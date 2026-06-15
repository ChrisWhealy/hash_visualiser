use std::collections::HashMap;

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
/// Axis-aligned box giving a node's placement, in user units, with its top-left corner at `(x, y)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub fn center(&self) -> (f64, f64) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
}

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
            let (x, y) = match graph.flow {
                FlowDirection::LeftToRight | FlowDirection::RightToLeft => (
                    MARGIN + main_idx as f64 * (NODE_W + LAYER_GAP),
                    MARGIN + pi as f64 * (NODE_H + NODE_GAP),
                ),
                FlowDirection::TopToBottom | FlowDirection::BottomToTop => (
                    MARGIN + pi as f64 * (NODE_W + NODE_GAP),
                    MARGIN + main_idx as f64 * (NODE_H + LAYER_GAP),
                ),
            };

            out.insert(
                name.clone(),
                Rect {
                    x,
                    y,
                    w: NODE_W,
                    h: NODE_H,
                },
            );
        }
    }

    out
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Point on a node's *downstream* edge — where a wire leaving the node begins.
pub fn exit_point(rect: &Rect, flow: &FlowDirection) -> (f64, f64) {
    let (cx, cy) = rect.center();
    match flow {
        FlowDirection::LeftToRight => (rect.x + rect.w, cy),
        FlowDirection::RightToLeft => (rect.x, cy),
        FlowDirection::TopToBottom => (cx, rect.y + rect.h),
        FlowDirection::BottomToTop => (cx, rect.y),
    }
}

/// Point on a node's *upstream* edge — where a wire arriving at the node ends.
pub fn entry_point(rect: &Rect, flow: &FlowDirection) -> (f64, f64) {
    let (cx, cy) = rect.center();
    match flow {
        FlowDirection::LeftToRight => (rect.x, cy),
        FlowDirection::RightToLeft => (rect.x + rect.w, cy),
        FlowDirection::TopToBottom => (cx, rect.y),
        FlowDirection::BottomToTop => (cx, rect.y + rect.h),
    }
}

/// Shifts a point one layer-gap further downstream — used to anchor an open (`?`) wire target.
pub fn downstream((x, y): (f64, f64), flow: &FlowDirection) -> (f64, f64) {
    match flow {
        FlowDirection::LeftToRight => (x + LAYER_GAP, y),
        FlowDirection::RightToLeft => (x - LAYER_GAP, y),
        FlowDirection::TopToBottom => (x, y + LAYER_GAP),
        FlowDirection::BottomToTop => (x, y - LAYER_GAP),
    }
}

/// Shifts a point one layer-gap further upstream — used to anchor an open (`?`) wire source.
pub fn upstream((x, y): (f64, f64), flow: &FlowDirection) -> (f64, f64) {
    match flow {
        FlowDirection::LeftToRight => (x - LAYER_GAP, y),
        FlowDirection::RightToLeft => (x + LAYER_GAP, y),
        FlowDirection::TopToBottom => (x, y - LAYER_GAP),
        FlowDirection::BottomToTop => (x, y + LAYER_GAP),
    }
}
