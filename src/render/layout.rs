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

/// Number of alternating barycentre passes used to balance the cross-axis positions.
const CROSS_PASSES: usize = 4;

/// Assigns a [`Rect`] to every node from its topology and flow direction, honouring each node's actual size.
///
/// Nodes are placed **as late as possible**: each one sits in the layer immediately before the operation it feeds (a
/// node with no successors keeps its longest-path depth). So inputs sit one column/row away from the operation they
/// feed, and a chain of functions converges toward the layer holding the final result.
///
/// Each layer occupies one slot along the *main* axis (the flow direction); within the *cross* axis a node is pulled
/// toward the barycentre of the nodes it connects to, so an operation sits centred between its inputs ("converging to
/// a point"). Layers advance along the main axis by the largest node in each preceding layer, so an oversized node
/// (e.g. an array grid) pushes downstream layers clear of it. `RightToLeft` / `BottomToTop` reverse which screen slot
/// each layer takes. Nodes absent from `sizes` fall back to `NODE_W`×`NODE_H`.
pub fn layout_sized(
    graph: &ValidatedGraph,
    sizes: &HashMap<String, Size>,
) -> HashMap<String, Rect> {
    let horizontal = matches!(
        graph.flow,
        FlowDirection::LeftToRight | FlowDirection::RightToLeft
    );

    let size_of = |name: &str| {
        sizes
            .get(name)
            .copied()
            .unwrap_or(Size::new(NODE_W, NODE_H))
    };
    let main_of = |s: Size| if horizontal { s.width } else { s.height };
    let cross_of = |s: Size| if horizontal { s.height } else { s.width };

    // Node→Node adjacency, both directions, from the wire edges.
    let mut succs: HashMap<String, Vec<String>> = HashMap::new();
    let mut preds: HashMap<String, Vec<String>> = HashMap::new();

    for (src, dst) in &graph.edges {
        succs.entry(src.clone()).or_default().push(dst.clone());
        preds.entry(dst.clone()).or_default().push(src.clone());
    }

    let layers = alap_layers(graph, &succs);
    let depth = layers.len();

    // Cross extent (height for horizontal flow) of every node.
    let extent: HashMap<String, f64> = graph
        .nodes
        .keys()
        .map(|name| (name.clone(), cross_of(size_of(name))))
        .collect();

    let centres = cross_centres(&layers, &preds, &succs, &extent);

    // Shift everything so the topmost node sits at MARGIN on the cross axis.
    let min_top = centres
        .iter()
        .map(|(name, c)| c - extent[name] / 2.0)
        .fold(f64::INFINITY, f64::min);
    let delta = if min_top.is_finite() {
        MARGIN - min_top
    } else {
        0.0
    };

    // Screen order of the layers: reversed for RtL / BtT.
    let screen_order: Vec<usize> = match graph.flow {
        FlowDirection::LeftToRight | FlowDirection::TopToBottom => (0..depth).collect(),
        FlowDirection::RightToLeft | FlowDirection::BottomToTop => (0..depth).rev().collect(),
    };

    let mut out = HashMap::with_capacity(graph.nodes.len());
    let mut main_off = MARGIN;

    for &li in &screen_order {
        let layer = &layers[li];
        let layer_main = layer
            .iter()
            .map(|name| main_of(size_of(name)))
            .fold(0.0_f64, f64::max);

        for name in layer {
            let sz = size_of(name);
            let cross_top = centres[name] + delta - cross_of(sz) / 2.0;
            let p = if horizontal {
                Point::new(main_off, cross_top)
            } else {
                Point::new(cross_top, main_off)
            };
            out.insert(name.clone(), Rect::new(p, sz));
        }

        main_off += layer_main + LAYER_GAP;
    }

    out
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Re-layers the graph "as late as possible": every node moves to the layer just before its earliest successor, so
/// inputs end up adjacent to the operation they feed. Nodes with no successors keep their longest-path depth (their
/// ASAP layer from [`ValidatedGraph::layers`]). Empty layers are dropped.
fn alap_layers(graph: &ValidatedGraph, succs: &HashMap<String, Vec<String>>) -> Vec<Vec<String>> {
    if graph.layers.is_empty() {
        return Vec::new();
    }

    // Longest-path depth per node (the existing ASAP layering).
    let mut asap: HashMap<&str, usize> = HashMap::new();
    for (i, layer) in graph.layers.iter().enumerate() {
        for name in layer {
            asap.insert(name.as_str(), i);
        }
    }

    // Visit sinks first (reverse ASAP order) so every node's successors are resolved before it.
    let mut alap: HashMap<String, usize> = HashMap::new();
    for layer in graph.layers.iter().rev() {
        for name in layer {
            let idx = match succs.get(name) {
                Some(s) if !s.is_empty() => {
                    s.iter().map(|m| alap[m]).min().unwrap().saturating_sub(1)
                }
                _ => asap[name.as_str()],
            };
            alap.insert(name.clone(), idx);
        }
    }

    let max_idx = alap.values().copied().max().unwrap_or(0);
    let mut buckets: Vec<Vec<String>> = vec![Vec::new(); max_idx + 1];
    for (name, &idx) in &alap {
        buckets[idx].push(name.clone());
    }
    for bucket in &mut buckets {
        bucket.sort();
    }
    buckets.into_iter().filter(|b| !b.is_empty()).collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Cross-axis centre for every node. Starting from a simple per-layer stack, alternating down/up barycentre passes pull
/// each node toward the average position of its neighbours, so operations sit centred between the nodes they connect.
fn cross_centres(
    layers: &[Vec<String>],
    preds: &HashMap<String, Vec<String>>,
    succs: &HashMap<String, Vec<String>>,
    extent: &HashMap<String, f64>,
) -> HashMap<String, f64> {
    let mut centre: HashMap<String, f64> = HashMap::new();

    // Initial packing: stack each layer along the cross axis.
    for layer in layers {
        let mut off = 0.0;
        for name in layer {
            let h = extent[name];
            centre.insert(name.clone(), off + h / 2.0);
            off += h + NODE_GAP;
        }
    }

    let last = layers.len().saturating_sub(1);
    for _ in 0..CROSS_PASSES {
        // Down pass: align each layer (after the first) to its predecessors.
        for layer in layers.iter().skip(1) {
            align_layer(layer, preds, &mut centre, extent);
        }
        // Up pass: align each layer (before the last) to its successors.
        for layer in layers[..last].iter().rev() {
            align_layer(layer, succs, &mut centre, extent);
        }
    }

    centre
}

/// Pulls one layer's nodes toward the barycentre of their `neighbours`, then resolves overlaps while keeping the layer
/// centred on those barycentres (so it doesn't drift along the cross axis).
fn align_layer(
    layer: &[String],
    neighbours: &HashMap<String, Vec<String>>,
    centre: &mut HashMap<String, f64>,
    extent: &HashMap<String, f64>,
) {
    if layer.is_empty() {
        return;
    }

    // Desired centre = mean of placed neighbours, or the current position when there are none.
    let mut want: Vec<(&String, f64)> = layer
        .iter()
        .map(|name| {
            let placed: Vec<f64> = neighbours
                .get(name)
                .into_iter()
                .flatten()
                .filter_map(|m| centre.get(m).copied())
                .collect();
            let d = if placed.is_empty() {
                centre[name]
            } else {
                placed.iter().sum::<f64>() / placed.len() as f64
            };
            (name, d)
        })
        .collect();

    want.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    // Place top-down with a minimum gap...
    let mut placed = Vec::with_capacity(want.len());
    let mut last_bottom = f64::NEG_INFINITY;
    for (name, d) in &want {
        let half = extent[*name] / 2.0;
        let c = d.max(last_bottom + NODE_GAP + half);
        placed.push(c);
        last_bottom = c + half;
    }

    // ...then re-centre the whole layer so its mean matches the desired mean (avoids downward drift when crowded).
    let want_mean = want.iter().map(|(_, d)| *d).sum::<f64>() / want.len() as f64;
    let got_mean = placed.iter().copied().sum::<f64>() / placed.len() as f64;
    let shift = want_mean - got_mean;

    for ((name, _), c) in want.iter().zip(placed) {
        centre.insert((*name).clone(), c + shift);
    }
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
