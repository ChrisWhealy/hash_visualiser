mod layout;
pub(crate) mod rect;

use std::collections::HashMap;
use svg_dom::{Error, SvgNode, SvgRoot, root::utils::Size};

use crate::{
    ast::{
        ebnf_06::{NodeDecl, NodeKind, PropValue},
        ebnf_07::{WireDecl, WireEndpoint},
    },
    graph::ValidatedGraph,
};
use layout::{MARGIN, downstream, entry_point, exit_point, layout, upstream};
use rect::Rect;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Create live SVG handles for everything described in a `.hv` file.
///
/// Every declared node becomes a [`SvgNode`] (a `<g>` wrapping a box and its label), keyed by node name, and every wire
/// becomes a `<line>`.  Since each handle points to a real DOM element, callers can later attach event handlers and
/// animations declared in the source by looking a node up by name.
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub struct Scene {
    pub nodes: HashMap<String, SvgNode>,
    pub wires: Vec<SvgNode>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Overall canvas size needed to contain the laid-out diagram, including a trailing margin.
///
/// Useful for sizing the `<svg>` viewport (e.g. via [`SvgRoot::create_in`](svg_dom::SvgRoot::create_in)) before
/// rendering.
pub fn diagram_size(graph: &ValidatedGraph) -> Size {
    let placement = layout(graph);
    let (mut max_x, mut max_y) = (0.0_f64, 0.0_f64);
    for rect in placement.values() {
        max_x = max_x.max(rect.top_left.x + rect.size.width);
        max_y = max_y.max(rect.top_left.y + rect.size.height);
    }
    Size::new(max_x + MARGIN, max_y + MARGIN)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Translates a validated graph into live SVG elements under `svg`.
///
/// Wires are created before nodes so that the node boxes paint on top of the connecting lines.
pub fn render(svg: &SvgRoot, graph: &ValidatedGraph) -> Result<Scene, Error> {
    let placement = layout(graph);

    let mut wires = Vec::with_capacity(graph.wires.len());
    for wire in &graph.wires {
        if let Some(line) = render_wire(svg, graph, &placement, wire)? {
            wires.push(line);
        }
    }

    let mut nodes = HashMap::with_capacity(graph.nodes.len());
    for (name, rect) in &placement {
        let decl = &graph.nodes[name];
        nodes.insert(name.clone(), render_node(svg, decl, *rect)?);
    }

    Ok(Scene { nodes, wires })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds a `<g>` holding the node's box and centred label, tagged with `data-node` for later lookup.
fn render_node(svg: &SvgRoot, decl: &NodeDecl, rect: Rect) -> Result<SvgNode, Error> {
    let group = svg.group()?;
    group.set_attr("data-node", &decl.name)?;

    let box_ = svg.rect(rect.into(), rect.into())?;
    box_.set_fill(fill_for(&decl.kind))?;
    box_.set_stroke("black")?;
    box_.set_stroke_width(1.5)?;
    box_.set_attr("rx", "6")?;

    let label = svg.text(rect.centre(), &node_label(decl))?;
    label.set_fill("white")?;
    label.set_attr("text-anchor", "middle")?;
    label.set_attr("dominant-baseline", "central")?;
    label.set_attr("font-family", "sans-serif")?;
    label.set_attr("font-size", "14")?;

    group.append(&box_)?;
    group.append(&label)?;
    Ok(group)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds a `<line>` between a wire's endpoints.
/// Open (`?`) endpoints are anchored one layer-gap beyond the concrete node they connect to; a wire with both endpoints
/// open is skipped (`Ok(None)`).
fn render_wire(
    svg: &SvgRoot,
    graph: &ValidatedGraph,
    placement: &HashMap<String, Rect>,
    wire: &WireDecl,
) -> Result<Option<SvgNode>, Error> {
    let flow = &graph.flow;
    let src = node_rect(placement, &wire.source);
    let dst = node_rect(placement, &wire.target);

    let (start_point, end_point) = match (src, dst) {
        (Some(s), Some(d)) => (exit_point(s, flow), entry_point(d, flow)),
        (Some(s), None) => {
            let start = exit_point(s, flow);
            (start, downstream(start, flow))
        }
        (None, Some(d)) => {
            let end = entry_point(d, flow);
            (upstream(end, flow), end)
        }
        (None, None) => return Ok(None),
    };

    let line = svg.line(start_point, end_point)?;
    line.set_stroke("#888888")?;
    line.set_stroke_width(2.0)?;

    if let Some(name) = &wire.name {
        line.set_attr("data-wire", name)?;
    }
    Ok(Some(line))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Resolves a wire endpoint to its placed [`Rect`]; `None` for open endpoints or unplaced nodes.
fn node_rect<'a>(
    placement: &'a HashMap<String, Rect>,
    endpoint: &WireEndpoint,
) -> Option<&'a Rect> {
    match endpoint {
        WireEndpoint::Node(name) => placement.get(name),
        WireEndpoint::Open => None,
    }
}

/// Box fill colour for each node kind.
fn fill_for(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::Register => "steelblue",
        NodeKind::Operation => "darkgoldenrod",
        NodeKind::Constant => "seagreen",
        NodeKind::Button => "slategray",
        NodeKind::User(_) => "dimgray",
    }
}

/// Visible label: the `label` or `symbol` string property if present, otherwise the node name.
fn node_label(decl: &NodeDecl) -> String {
    for key in ["label", "symbol"] {
        if let Some(PropValue::Str(s)) = decl
            .properties
            .iter()
            .find(|p| p.name == key)
            .map(|p| &p.value)
        {
            return s.clone();
        }
    }
    decl.name.clone()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
