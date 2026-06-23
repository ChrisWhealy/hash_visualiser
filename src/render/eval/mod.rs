//! A small evaluator: it computes a node's value so the renderer can show the "before" (input data) and "after"
//! (computed result) values flowing through an operation.
//!
//! Values are a single word or a (possibly nested) array of words (e.g. SHA-3's column-parity vector, or a 5x5 state).
//! A comprehension maps to an array, a nested comprehension to a matrix, `reduce` folds a flat array to a word, and
//! indexing reads an element (which may itself be an array, e.g. `a[x]` of a matrix).
mod value;

use std::collections::HashMap;

use value::*;

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
/// `compute` expression. `None` if it has no value, or its value is an array.
pub(super) fn node_value(decl: &NodeDecl, graph: &ValidatedGraph) -> Option<u64> {
    as_scalar(&node_eval(decl, graph, 0)?)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The 1-D array value a node carries (e.g. a `[u64; N]` data source or a comprehension result). `None` if absent or
/// not a flat array of words.
pub(super) fn node_array(decl: &NodeDecl, graph: &ValidatedGraph) -> Option<Vec<u64>> {
    as_row(&node_eval(decl, graph, 0)?)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The 2-D matrix value a node carries (e.g. a `[[u64; N]; M]` data source or a nested comprehension result). `None` if
/// absent or not a matrix of words.
pub(super) fn node_matrix(decl: &NodeDecl, graph: &ValidatedGraph) -> Option<Vec<Vec<u64>>> {
    as_matrix(&node_eval(decl, graph, 0)?)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn node_eval(decl: &NodeDecl, graph: &ValidatedGraph, depth: u32) -> Option<Value> {
    if depth >= MAX_DEPTH {
        return None;
    }

    // A literal data source, e.g. `source: InputData` with `data InputData = [ 0x…, … ]`.
    if let Some(Expr::Ident(source)) = prop_expr(decl, "source") {
        return eval(graph.data.get(source)?, &HashMap::new(), graph, depth);
    }

    // A computed value, e.g. `compute: ThetaD(c)`.
    if let Some(expr) = prop_expr(decl, "compute") {
        return eval(expr, &HashMap::new(), graph, depth);
    }

    None
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Evaluates an expression. `env` binds function parameters / comprehension variables to values; a bare identifier not
/// in `env` is treated as a node reference and resolved to that node's value (so `compute: f(a)` resolves `a` to the
/// `a` node's data).
fn eval(
    expr: &Expr,
    env: &HashMap<String, Value>,
    graph: &ValidatedGraph,
    depth: u32,
) -> Option<Value> {
    if depth >= MAX_DEPTH {
        return None;
    }

    let bits = word_bits(graph);

    match expr {
        Expr::Integer(n) | Expr::HexLit(n) => Some(Value::Scalar(*n)),

        Expr::Ident(name) => match env.get(name) {
            Some(v) => Some(v.clone()),
            None => node_eval(graph.nodes.get(name)?, graph, depth + 1),
        },

        Expr::Not(e) => Some(Value::Scalar(mask(
            !eval_scalar(e, env, graph, depth)?,
            bits,
        ))),

        Expr::BinOp { op, lhs, rhs } => {
            let a = eval_scalar(lhs, env, graph, depth)?;
            let b = eval_scalar(rhs, env, graph, depth)?;
            Some(Value::Scalar(apply_binop(op, a, b, bits)))
        }

        Expr::Call { name, args } => {
            let def = graph.fn_defs.get(name)?;
            if def.params.len() != args.len() {
                return None;
            }
            let mut child: HashMap<String, Value> = HashMap::with_capacity(args.len());
            for (param, arg) in def.params.iter().zip(args) {
                child.insert(param.name.clone(), eval(arg, env, graph, depth + 1)?);
            }
            eval(&def.body, &child, graph, depth + 1)
        }

        // `base[index]`: read one element of an array (which may itself be an array, e.g. `a[x]` of a matrix).
        Expr::Index { base, index } => {
            let Value::Array(items) = eval(base, env, graph, depth)? else {
                return None;
            };
            let i = eval_scalar(index, env, graph, depth)? as usize;
            items.get(i).cloned()
        }

        // `[ e0, e1, … ]`: an array literal; each element may be a scalar or a nested array.
        Expr::Array(elems) => {
            let values: Option<Vec<Value>> =
                elems.iter().map(|e| eval(e, env, graph, depth)).collect();
            Some(Value::Array(values?))
        }

        // `[ for x in start..end => body ]`: map each `x` through the body (scalar or nested), collecting an array.
        Expr::Comprehension {
            var,
            start,
            end,
            body,
        } => {
            let mut out = Vec::with_capacity(end.saturating_sub(*start) as usize);
            for x in *start..*end {
                let mut child = env.clone();
                child.insert(var.clone(), Value::Scalar(x));
                out.push(eval(body, &child, graph, depth + 1)?);
            }
            Some(Value::Array(out))
        }

        // `reduce <op> over array`: fold a flat array to a scalar with an associative operator.
        Expr::Reduce { op, array } => {
            let folded = eval_row(array, env, graph, depth)?
                .into_iter()
                .reduce(|a, b| apply_binop(op, a, b, bits))?;
            Some(Value::Scalar(folded))
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Evaluates `expr` as a scalar with the comprehension variable `var` bound to `x` and the array parameter `array`
/// bound to `values`. The map visualiser uses this to compute each sub-expression of a (1-D) comprehension body for a
/// chosen `x` (e.g. `c[(x + 1) mod 5]`, `rotl(…, 1)`, the final `xor`).
pub(super) fn eval_scalar_with(
    expr: &Expr,
    var: &str,
    x: u64,
    array: &str,
    values: &[u64],
    graph: &ValidatedGraph,
) -> Option<u64> {
    let mut env: HashMap<String, Value> = HashMap::with_capacity(2);
    env.insert(var.to_string(), Value::Scalar(x));
    env.insert(
        array.to_string(),
        Value::Array(values.iter().map(|&v| Value::Scalar(v)).collect()),
    );
    eval_scalar(expr, &env, graph, 0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Evaluates `expr`, requiring a scalar result.
fn eval_scalar(
    expr: &Expr,
    env: &HashMap<String, Value>,
    graph: &ValidatedGraph,
    depth: u32,
) -> Option<u64> {
    as_scalar(&eval(expr, env, graph, depth)?)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Evaluates `expr`, requiring a flat array of words.
fn eval_row(
    expr: &Expr,
    env: &HashMap<String, Value>,
    graph: &ValidatedGraph,
    depth: u32,
) -> Option<Vec<u64>> {
    as_row(&eval(expr, env, graph, depth)?)
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
        BinOp::Mod => {
            if b == 0 {
                0
            } else {
                a % b
            }
        }
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
    if bits >= 64 {
        v
    } else {
        v & ((1u64 << bits) - 1)
    }
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
