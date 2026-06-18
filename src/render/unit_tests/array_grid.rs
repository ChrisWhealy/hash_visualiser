use super::{check, eq, inferred_grid_shape, parse_and_build};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Array-grid shape inference (the pure half of the grid renderer; cell drawing needs a DOM).
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn should_infer_grid_shape_from_wired_function_param() -> Result<(), String> {
    let g = parse_and_build(
        "
        hash SHA3 {
            fn ThetaC(a: [[u8; 5]; 5]) -> [u8; 5] = [ for x in 0..5 => reduce xor over a[x] ]
            node state : register  { label: \"A\", format: hex8 }
            node c     : operation { symbol: \"ThetaC\" }
            wire state -> c
        }
    ",
    );

    eq(inferred_grid_shape("state", &g), Some((5, 5)))?;
    // The operation node itself feeds nothing, so it is not a grid.
    check(
        inferred_grid_shape("c", &g).is_none(),
        "operation node 'c' should not be a grid",
    )
}

#[test]
fn should_infer_non_square_grid_shape() -> Result<(), String> {
    let g = parse_and_build(
        "
        fn f(m: [[u8; 4]; 2]) -> u8 = reduce xor over m[0]
        node src : register  { format: hex8 }
        node op  : operation { symbol: \"f\" }
        wire src -> op
    ",
    );

    // outer length is rows, inner length is cols
    eq(inferred_grid_shape("src", &g), Some((2, 4)))
}

#[test]
fn should_not_infer_grid_for_scalar_function_param() -> Result<(), String> {
    let g = parse_and_build(
        "
        fn Ch(e: u32, f: u32, g: u32) -> u32 = (e and f) xor ((not e) and g)
        node e  : register  { format: hex32 }
        node ch : operation { symbol: \"Ch\" }
        wire e -> ch
    ",
    );

    check(
        inferred_grid_shape("e", &g).is_none(),
        "a scalar-param node should not be a grid",
    )
}

#[test]
fn should_not_infer_grid_for_unwired_node() -> Result<(), String> {
    let g = parse_and_build(
        "
        fn ThetaC(a: [[u8; 5]; 5]) -> [u8; 5] = a
        node lonely : register { format: hex8 }
    ",
    );

    check(
        inferred_grid_shape("lonely", &g).is_none(),
        "an unwired node should not be a grid",
    )
}
