use super::{format_cell, check, eq, grid_size, grid_spec, parse_and_build, cell_width};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Cell sizing/formatting driven by the node's format specifier.
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_size_cells_and_values_by_format_width() -> Result<(), String> {
    // Cell width scales with the measured monospace advance; here we pass a representative `ch`.
    let ch = 7.5;
    check(
        cell_width(16, ch) > cell_width(2, ch),
        "hex64 cell should be wider than hex8",
    )?;

    // hex8: a single byte, no inter-byte gap
    let h8 = format_cell(0, 2);
    eq(h8.chars().filter(char::is_ascii_hexdigit).count(), 2)?;
    check(!h8.contains(' '), "hex8 should have no inter-byte gap")?;

    // hex16: two bytes separated by one gap -> "xx xx"
    let h16 = format_cell(3, 4);
    eq(h16.matches(' ').count(), 1)?;

    // hex64: eight bytes -> 16 hex digits with 7 gaps
    let h64 = format_cell(7, 16);
    eq(h64.chars().filter(char::is_ascii_hexdigit).count(), 16)?;
    eq(h64.matches(' ').count(), 7)
}

#[test]
fn should_derive_grid_spec_cell_width_from_hex64_format() -> Result<(), String> {
    let g = parse_and_build(
        "
        hash SHA3 {
            fn ThetaC(a: [[u64; 5]; 5]) -> [u64; 5] = a
            node state : register  { format: hex64 }
            node c     : operation { symbol: \"ThetaC\" }
            wire state -> c
        }
    ",
    );

    let ch = 7.5;
    let spec = grid_spec("state", &g.nodes["state"], &g, ch).ok_or("state should be a grid")?;
    eq((spec.rows, spec.cols), (5, 5))?;
    eq(spec.digits, 16)?;
    // A hex64 grid is much wider than the same 5x5 shape rendered at the hex8 cell width.
    check(
        grid_size(&spec).width > 5.0 * cell_width(2, ch),
        "hex64 grid should be wider than the same shape at hex8",
    )?;

    // Cell height and inter-cell gap are derived from the measured `ch`, not hard-coded pixels.
    eq(spec.cell_h, 3.5 * ch)?;
    eq(spec.cell_gap, ch)?;

    // Doubling the font metric doubles those metrics.
    let bigger =
        grid_spec("state", &g.nodes["state"], &g, 2.0 * ch).ok_or("state should be a grid")?;
    eq(bigger.cell_h, 2.0 * spec.cell_h)?;
    eq(bigger.cell_gap, 2.0 * spec.cell_gap)
}
