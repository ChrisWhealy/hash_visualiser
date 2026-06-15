use crate::{
    ast::ebnf_08::FlowDirection,
    graph::{ValidatedGraph, build},
    render::layout::{LAYER_GAP, MARGIN, NODE_GAP, NODE_H, NODE_W, Rect, layout},
};

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
        Rect { x: MARGIN, y: MARGIN, w: NODE_W, h: NODE_H }
    );
    assert_eq!(
        pos["b"],
        Rect { x: MARGIN + NODE_W + LAYER_GAP, y: MARGIN, w: NODE_W, h: NODE_H }
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

    assert_eq!(pos["a"].x, MARGIN);
    assert_eq!(pos["b"].x, MARGIN);
    assert_ne!(pos["a"].y, pos["b"].y);
    assert_eq!(pos["b"].y - pos["a"].y, NODE_H + NODE_GAP);
    assert_eq!(pos["c"].x, MARGIN + NODE_W + LAYER_GAP);
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
    assert_eq!(pos["b"].x, MARGIN);
    assert_eq!(pos["a"].x, MARGIN + NODE_W + LAYER_GAP);
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

    assert_eq!(pos["a"].y, MARGIN);
    assert_eq!(pos["b"].y, MARGIN + NODE_H + LAYER_GAP);
    assert_eq!(pos["a"].x, pos["b"].x);
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
