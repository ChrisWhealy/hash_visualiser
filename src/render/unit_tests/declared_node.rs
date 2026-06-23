use super::{eq, grid_spec, parse_and_build};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Declared node data: the grid takes its shape and values from `data`, overriding the function-param inference.
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_take_grid_values_and_shape_from_declared_data() -> Result<(), String> {
    let g = parse_and_build(
        "
        hash SHA3 {
            fn ThetaC(a: [[u64; 5]; 5]) -> [u64; 5] = [ for x in 0..5 => reduce xor over a[x] ]
            data A = [[0x1, 0x2, 0x3], [0x4, 0x5, 0x6]]
            node state : register  { format: hex64, source: A }
            node c     : operation { symbol: \"ThetaC\", compute: ThetaC(state) }
            wire state -> c
        }
    ",
    );

    let spec = grid_spec("state", &g.nodes["state"], &g, 7.5).ok_or("state should be a grid")?;
    // Shape comes from the 2x3 data literal, not the 5x5 function parameter type.
    eq((spec.rows, spec.cols), (2, 3))?;
    eq(spec.values, Some(vec![vec![1, 2, 3], vec![4, 5, 6]]))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_render_a_one_dimensional_array_as_a_single_column() -> Result<(), String> {
    // A 1-D vector (here a `[u64; 4]` data source) is laid out as a single column — one value per row — so wide
    // (e.g. hex64) values don't push the diagram off-screen horizontally.
    let g = parse_and_build(
        "
        data V = [ 0x1, 0x2, 0x3, 0x4 ]
        node v : register { format: hex64, source: V }
    ",
    );

    let spec = grid_spec("v", &g.nodes["v"], &g, 7.5).ok_or("v should be a grid")?;
    eq((spec.rows, spec.cols), (4, 1))?;
    eq(spec.values, Some(vec![vec![1], vec![2], vec![3], vec![4]]))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn imported_operation_renders_as_a_box_and_data_as_tables() -> Result<(), String> {
    // A node whose `compute` applies an IMPORTED function renders as a plain (expandable) box — its detail opens in a
    // modal — while register/data nodes still show their values as tables. (This replaces the old `collapsed` flag:
    // box-vs-inline is now derived from whether the function is imported.)
    use crate::graph::build_with_imports;
    use std::collections::HashMap;

    let lib = "fn RowXor(a: [[u64; 2]; 2]) -> [u64; 2] = [ for x in 0..2 => reduce xor over a[x] ]";
    let main = "
        import \"lib.hv\"
        data A = [[0x1, 0x2], [0x3, 0x4]]
        node state : register  { format: hex64, source: A }
        node c     : operation { symbol: \"RowXor\", compute: RowXor(state) }
        node out   : register  { format: hex64, compute: c }
        wire state -> c
        wire c -> out
    ";
    let program = crate::parse(main).map_err(|e| e.to_string())?;
    let sources: HashMap<String, String> = [("lib.hv".to_string(), lib.to_string())].into();
    let g = build_with_imports(&program, &sources)
        .map_err(|errs| errs.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("; "))?;

    // The operation applying the imported function is a box (no grid) ...
    if grid_spec("c", &g.nodes["c"], &g, 7.5).is_some() {
        return Err("an operation applying an imported function should render as a box, not a grid".into());
    }
    // ... while the input data and the register that mirrors the result stay tables.
    grid_spec("state", &g.nodes["state"], &g, 7.5).ok_or("input data should still be a table")?;
    grid_spec("out", &g.nodes["out"], &g, 7.5).ok_or("output register should still be a table")?;
    Ok(())
}
