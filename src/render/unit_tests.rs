use crate::{
    ast::ebnf_08::FlowDirection,
    graph::{ValidatedGraph, build},
    render::{
        Rect,
        layout::{LAYER_GAP, MARGIN, NODE_GAP, NODE_H, NODE_W, layout},
    },
};
use svg_dom::root::utils::{Point, Size};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// SvgNode creation needs a live browser DOM, so the rendering itself is exercised by the svg-dom crate's
// wasm-bindgen tests. Here we cover the pure layout geometry, which is what positions every translated object.
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn parse_and_build(src: &str) -> ValidatedGraph {
    let program = crate::parse(src).expect("parse failed");
    build(&program).expect("build failed")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_place_one_node_per_layer_left_to_right() {
    let g = parse_and_build(
        "
        node a : register {}
        node b : operation {}
        wire a -> b
    ",
    );
    let pos = layout(&g);

    assert_eq!(
        pos["a"],
        Rect {
            top_left: Point::new(MARGIN, MARGIN),
            size: Size::new(NODE_W, NODE_H)
        }
    );
    assert_eq!(
        pos["b"],
        Rect {
            top_left: Point::new(MARGIN + NODE_W + LAYER_GAP, MARGIN),
            size: Size::new(NODE_W, NODE_H)
        }
    );
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_stack_siblings_along_the_cross_axis() {
    // a and b have no incoming edges, so they share layer 0 and stack vertically (left-to-right flow).
    let g = parse_and_build(
        "
        node a : register {}
        node b : register {}
        node c : operation {}
        wire a -> c
        wire b -> c
    ",
    );
    let pos = layout(&g);

    assert_eq!(pos["a"].top_left.x, MARGIN);
    assert_eq!(pos["b"].top_left.x, MARGIN);
    assert_ne!(pos["a"].top_left.y, pos["b"].top_left.y);
    assert_eq!(pos["b"].top_left.y - pos["a"].top_left.y, NODE_H + NODE_GAP);
    assert_eq!(pos["c"].top_left.x, MARGIN + NODE_W + LAYER_GAP);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_reverse_main_axis_for_right_to_left() {
    let g = parse_and_build(
        "
        layout: right_to_left
        node a : register {}
        node b : operation {}
        wire a -> b
    ",
    );
    assert_eq!(g.flow, FlowDirection::RightToLeft);
    let pos = layout(&g);

    // Source sits downstream (further right) of its target under right-to-left flow.
    assert_eq!(pos["b"].top_left.x, MARGIN);
    assert_eq!(pos["a"].top_left.x, MARGIN + NODE_W + LAYER_GAP);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_advance_main_axis_vertically_for_top_to_bottom() {
    let g = parse_and_build(
        "
        layout: top_to_bottom
        node a : register {}
        node b : operation {}
        wire a -> b
    ",
    );
    let pos = layout(&g);

    assert_eq!(pos["a"].top_left.y, MARGIN);
    assert_eq!(pos["b"].top_left.y, MARGIN + NODE_H + LAYER_GAP);
    assert_eq!(pos["a"].top_left.x, pos["b"].top_left.x);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_place_every_declared_node() {
    let g = parse_and_build(
        "
        node a : register {}
        node b : operation {}
        node c : constant {}
        wire a -> b
    ",
    );
    let pos = layout(&g);

    // Includes the isolated node `c`, which has no wires.
    assert_eq!(pos.len(), g.nodes.len());
    assert!(pos.contains_key("c"));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Array-grid shape inference (the pure half of the grid renderer; cell drawing needs a DOM).
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
use crate::render::inferred_grid_shape;

#[test]
fn should_infer_grid_shape_from_wired_function_param() {
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

    assert_eq!(inferred_grid_shape("state", &g), Some((5, 5)));
    // The operation node itself feeds nothing, so it is not a grid.
    assert_eq!(inferred_grid_shape("c", &g), None);
}

#[test]
fn should_infer_non_square_grid_shape() {
    let g = parse_and_build(
        "
        fn f(m: [[u8; 4]; 2]) -> u8 = reduce xor over m[0]
        node src : register  { format: hex8 }
        node op  : operation { symbol: \"f\" }
        wire src -> op
    ",
    );

    // outer length is rows, inner length is cols
    assert_eq!(inferred_grid_shape("src", &g), Some((2, 4)));
}

#[test]
fn should_not_infer_grid_for_scalar_function_param() {
    let g = parse_and_build(
        "
        fn Ch(e: u32, f: u32, g: u32) -> u32 = (e and f) xor ((not e) and g)
        node e  : register  { format: hex32 }
        node ch : operation { symbol: \"Ch\" }
        wire e -> ch
    ",
    );

    assert_eq!(inferred_grid_shape("e", &g), None);
}

#[test]
fn should_not_infer_grid_for_unwired_node() {
    let g = parse_and_build(
        "
        fn ThetaC(a: [[u8; 5]; 5]) -> [u8; 5] = a
        node lonely : register { format: hex8 }
    ",
    );

    assert_eq!(inferred_grid_shape("lonely", &g), None);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Cell sizing/formatting driven by the node's format specifier.
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
use crate::render::{cell_width, format_cell, grid_size, grid_spec};

#[test]
fn should_size_cells_and_values_by_format_width() {
    // Cell width scales with the measured monospace advance; here we pass a representative `ch`.
    let ch = 7.5;
    assert!(cell_width(16, ch) > cell_width(2, ch)); // hex64 cell wider than hex8

    // hex8: a single byte, no inter-byte gap
    let h8 = format_cell(0, 2);
    assert_eq!(h8.chars().filter(char::is_ascii_hexdigit).count(), 2);
    assert!(!h8.contains(' '));

    // hex16: two bytes separated by one gap -> "xx xx"
    let h16 = format_cell(3, 4);
    assert_eq!(h16.matches(' ').count(), 1);

    // hex64: eight bytes -> 16 hex digits with 7 gaps
    let h64 = format_cell(7, 16);
    assert_eq!(h64.chars().filter(char::is_ascii_hexdigit).count(), 16);
    assert_eq!(h64.matches(' ').count(), 7);
}

#[test]
fn should_derive_grid_spec_cell_width_from_hex64_format() {
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
    let spec = grid_spec("state", &g.nodes["state"], &g, ch).expect("state should be a grid");
    assert_eq!((spec.rows, spec.cols), (5, 5));
    assert_eq!(spec.digits, 16);
    // A hex64 grid is much wider than the same 5x5 shape rendered at the hex8 cell width.
    assert!(grid_size(&spec).width > 5.0 * cell_width(2, ch));

    // Cell height and inter-cell gap are derived from the measured `ch`, not hard-coded pixels.
    assert_eq!(spec.cell_h, 3.5 * ch);
    assert_eq!(spec.cell_gap, ch);
    // Doubling the font metric doubles those metrics.
    let bigger = grid_spec("state", &g.nodes["state"], &g, 2.0 * ch).unwrap();
    assert_eq!(bigger.cell_h, 2.0 * spec.cell_h);
    assert_eq!(bigger.cell_gap, 2.0 * spec.cell_gap);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Step-button range: clamped to the comprehension's range, no wrap-around.
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
use crate::render::{step_back, step_forward, step_range};

#[test]
fn should_derive_step_range_from_comprehension() {
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

    // Taken from the comprehension `for x in 0..5`, not just the row count.
    assert_eq!(step_range("state", &g, 5), (0, 5));
}

#[test]
fn should_fall_back_to_row_span_without_comprehension() {
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

    assert_eq!(step_range("state", &g, 5), (0, 5));
}

#[test]
fn should_clamp_step_forward_at_last_row() {
    let range = (0, 5); // visits rows 0..=4
    assert_eq!(step_forward(0, range), 1);
    assert_eq!(step_forward(3, range), 4);
    assert_eq!(step_forward(4, range), 4); // no wrap past the last row
}

#[test]
fn should_clamp_step_back_at_first_row() {
    let range = (0, 5);
    assert_eq!(step_back(4, range), 3);
    assert_eq!(step_back(1, range), 0);
    assert_eq!(step_back(0, range), 0); // no wrap before the first row
}

#[test]
fn should_respect_a_nonzero_range_start() {
    let range = (1, 4); // visits rows 1..=3
    assert_eq!(step_back(1, range), 1); // clamped at start, not 0
    assert_eq!(step_forward(3, range), 3); // clamped at end-1
}
