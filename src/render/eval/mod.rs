//! A small scalar evaluator: it computes a node's numeric value so the renderer can show the "before" (input data) and
//! "after" (computed result) values flowing through an operation, even when there is no comprehension to step through.
//!
//! Only scalar (`u64`) values are handled. Array-valued expressions (comprehensions, reductions, indexing) return
//! `None` — those are visualised separately by the reduction renderer.

use std::collections::HashMap;

use crate::{
    ast::{
        ebnf_06::{NodeDecl, PropValue},
        ebnf_11::{BinOp, Expr},
    },
    graph::ValidatedGraph,
};

/// Guards against runaway recursion through cyclic node/`compute` references.
/// The graph is expected to be acyclic, but parsing of a malformed `.hv` file should fail gracefully (returning `None`)
/// rather than overflowing the stack.
const MAX_DEPTH: u32 = 64;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The scalar value a node carries: the literal behind its `source` data binding, or the result of evaluating its
/// `compute` expression.
/// 
/// Returns `None` for a node with no value, or whose value is an array rather than a scalar.
pub(super) fn node_value(decl: &NodeDecl, graph: &ValidatedGraph) -> Option<u64> {
    node_value_at(decl, graph, 0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn node_value_at(decl: &NodeDecl, graph: &ValidatedGraph, depth: u32) -> Option<u64> {
    if depth >= MAX_DEPTH {
        return None;
    }

    // A literal data source, e.g. `source: InputValue` with `data InputValue = 0x…`.
    if let Some(Expr::Ident(source)) = prop_expr(decl, "source") {
        return eval(graph.data.get(source)?, &HashMap::new(), graph, depth);
    }

    // A computed value, e.g. `compute: ShiftRight(value, amount)`.
    if let Some(expr) = prop_expr(decl, "compute") {
        return eval(expr, &HashMap::new(), graph, depth);
    }

    None
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Evaluates a scalar expression. `env` binds function parameters to values; a bare identifier not found in `env` is
/// treated as a node reference and resolved to that node's value (so `compute: f(a, b)` resolves `a`/`b` to the data of
/// the `a`/`b` nodes).
fn eval(expr: &Expr, env: &HashMap<&str, u64>, graph: &ValidatedGraph, depth: u32) -> Option<u64> {
    if depth >= MAX_DEPTH {
        return None;
    }

    match expr {
        Expr::Integer(n) | Expr::HexLit(n) => Some(*n),

        Expr::Ident(name) => match env.get(name.as_str()) {
            Some(v) => Some(*v),
            None => node_value_at(graph.nodes.get(name)?, graph, depth + 1),
        },

        Expr::Not(e) => Some(mask(!eval(e, env, graph, depth)?, word_bits(graph))),

        Expr::BinOp { op, lhs, rhs } => {
            let a = eval(lhs, env, graph, depth)?;
            let b = eval(rhs, env, graph, depth)?;
            Some(apply_binop(op, a, b, word_bits(graph)))
        }

        Expr::Call { name, args } => {
            let def = graph.fn_defs.get(name)?;
            if def.params.len() != args.len() {
                return None;
            }
            let mut child: HashMap<&str, u64> = HashMap::with_capacity(args.len());
            for (param, arg) in def.params.iter().zip(args) {
                child.insert(param.name.as_str(), eval(arg, env, graph, depth + 1)?);
            }
            eval(&def.body, &child, graph, depth + 1)
        }

        // Array-valued expressions are not scalars; the reduction renderer visualises those.
        Expr::Index { .. } | Expr::Comprehension { .. } | Expr::Reduce { .. } | Expr::Array(_) => None,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Applies a binary operator within a `bits`-wide word, matching the Rust/WASM operator semantics the DSL borrows
/// (shift/rotate counts are taken modulo the word width; shifts and rotates respect signedness where it matters).
fn apply_binop(op: &BinOp, a: u64, b: u64, bits: u32) -> u64 {
    let a = mask(a, bits);
    let b = mask(b, bits);
    let shift = (b % bits as u64) as u32;

    match op {
        BinOp::Or => a | b,
        BinOp::Xor => a ^ b,
        BinOp::And => a & b,
        BinOp::Add => mask(a.wrapping_add(b), bits),
        BinOp::Sub => mask(a.wrapping_sub(b), bits),
        BinOp::Shl => mask(a << shift, bits),
        BinOp::ShrU => a >> shift,
        BinOp::ShrS => arith_shr(a, shift, bits),
        // Rotations don't depend on signedness, so the `_s` and `_u` variants are identical.
        BinOp::RotrU | BinOp::RotrS => mask((a >> shift) | (a << ((bits - shift) % bits)), bits),
        BinOp::RotlU | BinOp::RotlS => mask((a << shift) | (a >> ((bits - shift) % bits)), bits),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Arithmetic (sign-propagating) right shift of a `bits`-wide value.
fn arith_shr(a: u64, shift: u32, bits: u32) -> u64 {
    if shift == 0 {
        return a;
    }

    let logical = a >> shift;
    let negative = (a >> (bits - 1)) & 1 == 1;

    if negative {
        mask(logical | (!0u64 << (bits - shift)), bits)
    } else {
        logical
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The configured word size in bits (default 64), clamped to a representable range.
fn word_bits(graph: &ValidatedGraph) -> u32 {
    graph.word_size.unwrap_or(64).clamp(1, 64) as u32
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Keeps only the low `bits` bits of `v`.
fn mask(v: u64, bits: u32) -> u64 {
    if bits >= 64 { v } else { v & ((1u64 << bits) - 1) }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The expression value of a node property, if it has one (e.g. `source`, `compute`, `format`).
fn prop_expr<'a>(decl: &'a NodeDecl, name: &str) -> Option<&'a Expr> {
    decl.properties
        .iter()
        .find(|p| p.name == name)
        .and_then(|p| match &p.value {
            PropValue::Expr(e) => Some(e),
            PropValue::Str(_) => None,
        })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
