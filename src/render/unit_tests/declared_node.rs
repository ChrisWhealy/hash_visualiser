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
