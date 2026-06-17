use crate::{
    ast::{ebnf_08::FlowDirection, ebnf_11::BinOp},
    graph::{ValidatedGraph, build},
    render::{
        Rect, apply_reduce, cell_width, description_html, effective_matrix, expr_label, format_cell,
        grid_size, grid_spec, inferred_grid_shape, op_symbol, reduction_label_source, reduction_op,
        step_back, step_forward, step_range,
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

fn eq<T, U>(actual: T, expected: U) -> Result<(), String>
where
    T: PartialEq<U> + std::fmt::Debug,
    U: std::fmt::Debug,
{
    if actual == expected {
        Ok(())
    } else {
        Err(format!("expected {expected:?}, got {actual:?}"))
    }
}

fn check(cond: bool, msg: &str) -> Result<(), String> {
    if cond { Ok(()) } else { Err(msg.to_string()) }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_place_one_node_per_layer_left_to_right() -> Result<(), String> {
    let g = parse_and_build(
        "
        node a : register {}
        node b : operation {}
        wire a -> b
    ",
    );
    let pos = layout(&g);

    eq(
        pos["a"],
        Rect {
            top_left: Point::new(MARGIN, MARGIN),
            size: Size::new(NODE_W, NODE_H),
        },
    )?;
    eq(
        pos["b"],
        Rect {
            top_left: Point::new(MARGIN + NODE_W + LAYER_GAP, MARGIN),
            size: Size::new(NODE_W, NODE_H),
        },
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_stack_siblings_along_the_cross_axis() -> Result<(), String> {
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

    eq(pos["a"].top_left.x, MARGIN)?;
    eq(pos["b"].top_left.x, MARGIN)?;
    check(
        pos["a"].top_left.y != pos["b"].top_left.y,
        "siblings should not share a row",
    )?;
    eq(pos["b"].top_left.y - pos["a"].top_left.y, NODE_H + NODE_GAP)?;
    eq(pos["c"].top_left.x, MARGIN + NODE_W + LAYER_GAP)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_reverse_main_axis_for_right_to_left() -> Result<(), String> {
    let g = parse_and_build(
        "
        layout: right_to_left
        node a : register {}
        node b : operation {}
        wire a -> b
    ",
    );
    let pos = layout(&g);

    // Source sits downstream (further right) of its target under right-to-left flow.
    eq(pos["b"].top_left.x, MARGIN)?;
    eq(pos["a"].top_left.x, MARGIN + NODE_W + LAYER_GAP)?;
    eq(g.flow, FlowDirection::RightToLeft)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_advance_main_axis_vertically_for_top_to_bottom() -> Result<(), String> {
    let g = parse_and_build(
        "
        layout: top_to_bottom
        node a : register {}
        node b : operation {}
        wire a -> b
    ",
    );
    let pos = layout(&g);

    eq(pos["a"].top_left.y, MARGIN)?;
    eq(pos["b"].top_left.y, MARGIN + NODE_H + LAYER_GAP)?;
    eq(pos["a"].top_left.x, pos["b"].top_left.x)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_place_every_declared_node() -> Result<(), String> {
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
    eq(pos.len(), g.nodes.len())?;
    check(
        pos.contains_key("c"),
        "expected isolated node 'c' to be placed",
    )
}

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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Step-button range: clamped to the comprehension's range, no wrap-around.
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_derive_step_range_from_comprehension() -> Result<(), String> {
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
    eq(step_range("state", &g, 5), (0, 5))
}

#[test]
fn should_fall_back_to_row_span_without_comprehension() -> Result<(), String> {
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

    eq(step_range("state", &g, 5), (0, 5))
}

#[test]
fn should_clamp_step_forward_at_last_row() -> Result<(), String> {
    let range = (0, 5); // visits rows 0..=4
    eq(step_forward(0, range), 1)?;
    eq(step_forward(3, range), 4)?;
    eq(step_forward(4, range), 4) // no wrap past the last row
}

#[test]
fn should_clamp_step_back_at_first_row() -> Result<(), String> {
    let range = (0, 5);
    eq(step_back(4, range), 3)?;
    eq(step_back(1, range), 0)?;
    eq(step_back(0, range), 0) // no wrap before the first row
}

#[test]
fn should_respect_a_nonzero_range_start() -> Result<(), String> {
    let range = (1, 4); // visits rows 1..=3
    eq(step_back(1, range), 1)?; // clamped at start, not 0
    eq(step_forward(3, range), 3) // clamped at end-1
}

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
fn should_label_reduction_operators() -> Result<(), String> {
    eq(op_symbol(&BinOp::Xor), "xor")?;
    eq(op_symbol(&BinOp::And), "and")?;
    eq(op_symbol(&BinOp::Or), "or")?;
    eq(op_symbol(&BinOp::Add), "+")
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

    let (var, expr) = reduction_label_source("state", &g).ok_or("expected a comprehension reduction")?;
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
    let (var, _) = reduction_label_source("state", &g).ok_or("expected reduction found via compute")?;
    eq(var.as_str(), "x")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Node descriptions: a node's markdown `description` is rendered to HTML for the docs panel.
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_render_node_description_markdown_to_html() -> Result<(), String> {
    let g = parse_and_build(
        "
        node c : operation {
            symbol: \"ThetaC\",
            description: \"\"\"
# Theta-C

XOR the five lanes via `theta_c`.

```rust
fn theta_c() {}
```
\"\"\"
        }
    ",
    );

    let html = description_html(&g.nodes["c"]).ok_or("expected a rendered description")?;
    check(html.contains("<h1>"), "heading should render to <h1>")?;
    check(html.contains("<code>theta_c</code>"), "inline code should render")?;
    check(html.contains("<pre><code"), "fenced block should render to <pre><code>")
}

#[test]
fn should_have_no_description_html_when_absent() -> Result<(), String> {
    let g = parse_and_build("node plain : register { format: hex8 }");
    check(
        description_html(&g.nodes["plain"]).is_none(),
        "a node without a description should produce no HTML",
    )
}
