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
/// Assigns a [`Rect`] to every node in the graph from its topological layers and flow direction.
///
/// Each layer occupies one position along the *main* axis (the flow direction); nodes within a layer are spread along
/// the *cross* axis.  `RightToLeft` and `BottomToTop` simply reverse the main-axis ordering of the layers.
pub fn layout(graph: &ValidatedGraph) -> HashMap<String, Rect> {
    let mut out = HashMap::with_capacity(graph.nodes.len());
    let n_layers = graph.layers.len();

    for (li, layer) in graph.layers.iter().enumerate() {
        let main_idx = match graph.flow {
            FlowDirection::RightToLeft | FlowDirection::BottomToTop => n_layers - 1 - li,
            FlowDirection::LeftToRight | FlowDirection::TopToBottom => li,
        };

        for (pi, name) in layer.iter().enumerate() {
            let p = match graph.flow {
                FlowDirection::LeftToRight | FlowDirection::RightToLeft => Point::new(
                    MARGIN + main_idx as f64 * (NODE_W + LAYER_GAP),
                    MARGIN + pi as f64 * (NODE_H + NODE_GAP),
                ),
                FlowDirection::TopToBottom | FlowDirection::BottomToTop => Point::new(
                    MARGIN + pi as f64 * (NODE_W + NODE_GAP),
                    MARGIN + main_idx as f64 * (NODE_H + LAYER_GAP),
                ),
            };

            out.insert(name.clone(), Rect::new(p, Size::new(NODE_W, NODE_H)));
        }
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
