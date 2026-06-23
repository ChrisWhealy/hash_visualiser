use super::{eq, grid_spec, is_map_operation, nested_map, parse_and_build};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// A nested map `[ for x => [ for y => a[x][y] op d[x] ] ]` is detected for the MATRIX input only, so the row-by-row
// fold visualisation is drawn once. The broadcast-vector input renders as a plain grid; the operation node renders as
// a plain box (its output is shown by the visualisation building up).
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const THETA_MIX: &str = "\
    context { word_size: 64 }\n\
    fn ThetaXor(a: [[u64; 5]; 5], d: [u64; 5]) -> [[u64; 5]; 5] = [ for x in 0..5 => [ for y in 0..5 => a[x][y] xor d[x] ] ]\n\
    data StateA = [\n\
        [ 0x00, 0x01, 0x02, 0x03, 0x04 ],\n\
        [ 0x10, 0x11, 0x12, 0x13, 0x14 ],\n\
        [ 0x20, 0x21, 0x22, 0x23, 0x24 ],\n\
        [ 0x30, 0x31, 0x32, 0x33, 0x34 ],\n\
        [ 0x40, 0x41, 0x42, 0x43, 0x44 ] ]\n\
    data VecD = [ 0xFF, 0xF0, 0x0F, 0xFFFF0000, 0xFFFFFFFF00000000 ]\n\
    node state : register  { source: StateA }\n\
    node dvec  : register  { source: VecD }\n\
    node mixed : operation { compute: ThetaXor(state, dvec) }\n";

#[test]
fn should_detect_a_nested_map_for_the_matrix_input_only() -> Result<(), String> {
    let g = parse_and_build(THETA_MIX);

    let matrix = nested_map("state", &g).ok_or("expected the matrix input to feed a nested map")?;
    eq(matrix.outer_range, (0usize, 5usize))?;
    eq(matrix.op_label.as_str(), "XOR")?;
    eq(matrix.vec_node.as_str(), "dvec")?;
    eq(matrix.op_node.as_str(), "mixed")?;

    // The broadcast vector is NOT the matrix input, so it gets no fold visualisation of its own.
    if nested_map("dvec", &g).is_some() {
        return Err("the broadcast vector should not trigger the nested-map viz".into());
    }
    Ok(())
}

#[test]
fn should_render_the_nested_map_operation_as_a_plain_box() -> Result<(), String> {
    let g = parse_and_build(THETA_MIX);

    // The operation node renders as a plain box (its output is built up by the viz), so it has no value grid.
    if !is_map_operation(&g.nodes["mixed"], &g) {
        return Err("the nested-map operation should render as a map box".into());
    }
    if grid_spec("mixed", &g.nodes["mixed"], &g, 7.5).is_some() {
        return Err("the nested-map operation should not draw its own value grid".into());
    }

    // The inputs still render as grids: the 5x5 state matrix and the broadcast vector as a 5x1 column.
    let state = grid_spec("state", &g.nodes["state"], &g, 7.5).ok_or("state grid")?;
    eq((state.rows, state.cols), (5usize, 5usize))?;
    let dvec = grid_spec("dvec", &g.nodes["dvec"], &g, 7.5).ok_or("dvec grid")?;
    eq((dvec.rows, dvec.cols), (5usize, 1usize))
}
