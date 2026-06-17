use std::collections::HashMap;

use svg_dom::root::utils::{Point, Size};

use crate::render::Rect;
use crate::{ast::ebnf_08::FlowDirection, graph::ValidatedGraph};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Layout constants (user units)
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub const NODE_W: f64 = 120.0;
pub const NODE_H: f64 = 60.0;
/// Gap between adjacent layers along the flow (main) axis.
pub const LAYER_GAP: f64 = 80.0;
/// Gap between sibling nodes within a layer along the cross axis.
pub const NODE_GAP: f64 = 40.0;
/// Padding between the diagram and the edge of the viewport.
pub const MARGIN: f64 = 40.0;
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Uniform-box layout — every node is `NODE_W`×`NODE_H`. Used for sizing estimates ([`super::diagram_size`]) and as
/// the default when actual node sizes are not yet known.
pub fn layout(graph: &ValidatedGraph) -> HashMap<String, Rect> {
    let sizes: HashMap<String, Size> = graph
        .nodes
        .keys()
        .map(|n| (n.clone(), Size::new(NODE_W, NODE_H)))
        .collect();

    layout_sized(graph, &sizes)
}

/// Assigns a [`Rect`] to every node from its topological layers and flow direction, honouring each node's actual size.
///
/// Each topological layer occupies one slot along the *main* axis (the flow direction); nodes within a layer are
/// stacked along the *cross* axis.  Layers advance along the main axis by the largest node in each preceding layer, so
/// an oversized node (e.g. an array grid) pushes downstream layers clear of it rather than overlapping them.
/// `RightToLeft` / `BottomToTop` reverse which screen slot each topological layer takes.  Nodes absent from `sizes`
/// fall back to `NODE_W`×`NODE_H`.
pub fn layout_sized(graph: &ValidatedGraph, sizes: &HashMap<String, Size>) -> HashMap<String, Rect> {
    let horizontal = matches!(
        graph.flow,
        FlowDirection::LeftToRight | FlowDirection::RightToLeft
    );
    let n = graph.layers.len();

    // Screen order of the topological layers: reversed for RtL / BtT.
    let screen_order: Vec<usize> = match graph.flow {
        FlowDirection::LeftToRight | FlowDirection::TopToBottom => (0..n).collect(),
        FlowDirection::RightToLeft | FlowDirection::BottomToTop => (0..n).rev().collect(),
    };

    let size_of = |name: &str| sizes.get(name).copied().unwrap_or(Size::new(NODE_W, NODE_H));
    let main_of = |s: Size| if horizontal { s.width } else { s.height };
    let cross_of = |s: Size| if horizontal { s.height } else { s.width };

    let mut out = HashMap::with_capacity(graph.nodes.len());
    let mut main_off = MARGIN;

    for &topo_i in &screen_order {
        let layer = &graph.layers[topo_i];
        let layer_main = layer
            .iter()
            .map(|name| main_of(size_of(name)))
            .fold(0.0_f64, f64::max);

        let mut cross_off = MARGIN;
        for name in layer {
            let sz = size_of(name);
            let p = if horizontal {
                Point::new(main_off, cross_off)
            } else {
                Point::new(cross_off, main_off)
            };
            out.insert(name.clone(), Rect::new(p, sz));
            cross_off += cross_of(sz) + NODE_GAP;
        }

        main_off += layer_main + LAYER_GAP;
    }

    out
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Point on a node's *downstream* edge — where a wire leaving the node begins.
pub fn exit_point(rect: &Rect, flow: &FlowDirection) -> Point {
    let cp = rect.centre();

    match flow {
        FlowDirection::LeftToRight => Point::new(rect.top_left.x + rect.size.width, cp.y),
        FlowDirection::RightToLeft => Point::new(rect.top_left.x, cp.y),
        FlowDirection::TopToBottom => Point::new(cp.x, rect.top_left.y + rect.size.height),
        FlowDirection::BottomToTop => Point::new(cp.x, rect.top_left.y),
    }
}

/// Point on a node's *upstream* edge — where a wire arriving at the node ends.
pub fn entry_point(rect: &Rect, flow: &FlowDirection) -> Point {
    let cp = rect.centre();

    match flow {
        FlowDirection::LeftToRight => Point::new(rect.top_left.x, cp.y),
        FlowDirection::RightToLeft => Point::new(rect.top_left.x + rect.size.width, cp.y),
        FlowDirection::TopToBottom => Point::new(cp.x, rect.top_left.y),
        FlowDirection::BottomToTop => Point::new(cp.x, rect.top_left.y + rect.size.height),
    }
}

/// Shifts a point one layer-gap further downstream — used to anchor an open (`?`) wire target.
pub fn downstream(p: Point, flow: &FlowDirection) -> Point {
    match flow {
        FlowDirection::LeftToRight => Point::new(p.x + LAYER_GAP, p.y),
        FlowDirection::RightToLeft => Point::new(p.x - LAYER_GAP, p.y),
        FlowDirection::TopToBottom => Point::new(p.x, p.y + LAYER_GAP),
        FlowDirection::BottomToTop => Point::new(p.x, p.y - LAYER_GAP),
    }
}

/// Shifts a point one layer-gap further upstream — used to anchor an open (`?`) wire source.
pub fn upstream(p: Point, flow: &FlowDirection) -> Point {
    match flow {
        FlowDirection::LeftToRight => Point::new(p.x - LAYER_GAP, p.y),
        FlowDirection::RightToLeft => Point::new(p.x + LAYER_GAP, p.y),
        FlowDirection::TopToBottom => Point::new(p.x, p.y - LAYER_GAP),
        FlowDirection::BottomToTop => Point::new(p.x, p.y + LAYER_GAP),
    }
}
