use super::{check, eq, parse_and_build};
use crate::{
    ast::ebnf_11::BinOp,
    render::{
        reduce::apply_reduce, effective_matrix, expr_label, grid_spec, inferred_grid_shape,
        reduction_label_source, reduction_op,
    },
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Inner reduction visualisation — pure helpers (the SVG fold diagram itself needs a DOM).
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_extract_reduction_op_from_comprehension() -> Result<(), String> {
    let g = parse_and_build(
        "
        hash SHA3 {
            fn ThetaC(a: [[u64; 5]; 5]) -> [u64; 5] = [ for x in 0..5 => reduce xor over a[x] ]
            node state : register  { format: hex64 }
            node c     : operation { symbol: \"ThetaC\" }
            wire state -> c
        }
    ",
    );

    eq(reduction_op("state", &g), Some(BinOp::Xor))
}

#[test]
fn should_left_fold_the_reduction() -> Result<(), String> {
    eq(
        apply_reduce(&BinOp::Xor, &[0x1, 0x2, 0x3]),
        Some(0x1 ^ 0x2 ^ 0x3),
    )?; // == 0
    eq(apply_reduce(&BinOp::Add, &[10, 20, 12]), Some(42))?;
    eq(apply_reduce(&BinOp::Xor, &[0x5]), Some(0x5))?; // single element folds to itself
    check(
        apply_reduce(&BinOp::Xor, &[]).is_none(),
        "an empty row has no reduction",
    )
}

#[test]
fn should_label_reduction_operators_in_uppercase() -> Result<(), String> {
    // The operation-row boxes label themselves from the operator's `Display` (uppercase), matching the nested-map viz.
    eq(BinOp::Xor.to_string(), "XOR")?;
    eq(BinOp::And.to_string(), "AND")?;
    eq(BinOp::Or.to_string(), "OR")?;
    eq(BinOp::Add.to_string(), "+")
}

#[test]
fn should_use_declared_data_as_the_effective_matrix() -> Result<(), String> {
    let g = parse_and_build(
        "
        hash SHA3 {
            fn ThetaC(a: [[u64; 5]; 5]) -> [u64; 5] = [ for x in 0..5 => reduce xor over a[x] ]
            data A = [[0x1, 0x2], [0x3, 0x4]]
            node state : register  { format: hex64, source: A }
            node c     : operation { symbol: \"ThetaC\", compute: ThetaC(state) }
            wire state -> c
        }
    ",
    );

    let spec = grid_spec("state", &g.nodes["state"], &g, 7.5).ok_or("state should be a grid")?;
    eq(effective_matrix(&spec), vec![vec![1u64, 2], vec![3, 4]])
}

#[test]
fn should_label_working_row_with_substituted_index() -> Result<(), String> {
    let g = parse_and_build(
        "
        hash SHA3 {
            fn ThetaC(a: [[u64; 5]; 5]) -> [u64; 5] = [ for x in 0..5 => reduce xor over a[x] ]
            node state : register  { format: hex64 }
            node c     : operation { symbol: \"ThetaC\" }
            wire state -> c
        }
    ",
    );

    let (var, expr) =
        reduction_label_source("state", &g).ok_or("expected a comprehension reduction")?;
    eq(var.as_str(), "x")?;
    // The `over` operand `a[x]`, with the index variable bound to the current row.
    eq(expr_label(&expr, &var, 0), "a[0]")?;
    eq(expr_label(&expr, &var, 3), "a[3]")
}

#[test]
fn should_find_reduction_via_compute_without_a_wire() -> Result<(), String> {
    // No `wire state -> c`: the `compute: ThetaC(state)` link alone drives the reduction visualisation, so commenting
    // out the wire must not make the working/operation/result rows disappear.
    let g = parse_and_build(
        "
        hash SHA3 {
            fn ThetaC(a: [[u64; 5]; 5]) -> [u64; 5] = [ for x in 0..5 => reduce xor over a[x] ]
            data A = [[0x1, 0x2, 0x3, 0x4, 0x5], [0x1, 0x2, 0x3, 0x4, 0x5], [0x1, 0x2, 0x3, 0x4, 0x5], [0x1, 0x2, 0x3, 0x4, 0x5], [0x1, 0x2, 0x3, 0x4, 0x5]]
            node state : register  { format: hex64, source: A }
            node c     : operation { symbol: \"ThetaC\", compute: ThetaC(state) }
        }
    ",
    );

    eq(reduction_op("state", &g), Some(BinOp::Xor))?;
    eq(inferred_grid_shape("state", &g), Some((5, 5)))?;
    let (var, _) =
        reduction_label_source("state", &g).ok_or("expected reduction found via compute")?;
    eq(var.as_str(), "x")
}
