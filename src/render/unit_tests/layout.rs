use super::{check, eq, parse_and_build};
use crate::{
    ast::ebnf_08::FlowDirection,
    render::{
        Rect,
        layout::{LAYER_GAP, MARGIN, NODE_GAP, NODE_H, NODE_W, layout},
    },
};
use svg_dom::root::utils::{Point, Size};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Layout
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
fn should_place_inputs_adjacent_to_their_consumer_and_centre_the_result() -> Result<(), String> {
    // (a AND b) then XOR with c: a,b feed `ab`; `ab` and `c` feed `result`.
    let g = parse_and_build(
        "
        node a : register {}
        node b : register {}
        node c : register {}
        node ab : operation {}
        node result : operation {}
        wire a -> ab
        wire b -> ab
        wire ab -> result
        wire c -> result
    ",
    );
    let pos = layout(&g);

    // `c` feeds `result` (the last column), so it sits one column before it — the same column as `ab`, not column 0.
    eq(pos["c"].top_left.x, pos["ab"].top_left.x)?;
    check(
        pos["ab"].top_left.x > pos["a"].top_left.x,
        "the AND operation is downstream of its inputs",
    )?;
    check(
        pos["result"].top_left.x > pos["ab"].top_left.x,
        "the result is downstream of the AND operation",
    )?;

    // `result` is vertically centred between the two nodes that feed it (`ab` and `c`).
    let centre_y = |r: &Rect| r.top_left.y + r.size.height / 2.0;
    let midpoint = (centre_y(&pos["ab"]) + centre_y(&pos["c"])) / 2.0;
    check(
        (centre_y(&pos["result"]) - midpoint).abs() < 0.001,
        "result should be centred between its inputs",
    )
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
