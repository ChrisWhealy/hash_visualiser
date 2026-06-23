mod eval;
mod grid_spec;
mod layout;
mod map;
pub(crate) mod rect;
mod reduce;
mod routing;
mod viz_node;

use std::collections::HashMap;
use svg_dom::{
    Error, SvgNode, SvgRoot,
    root::utils::{Point, Size},
};

use crate::{
    ast::{
        ebnf_04::{FnDef, Type},
        ebnf_06::{NodeDecl, NodeKind, PropValue},
        ebnf_07::WireEndpoint,
        ebnf_08::FlowDirection,
        ebnf_11::{BinOp, Expr},
    },
    graph::ValidatedGraph,
};
use grid_spec::*;
use layout::{MARGIN, NODE_H, NODE_W, layout, layout_sized};
use map::*;
use rect::Rect;
use reduce::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Array-grid visualisation constants (user units / CSS colours)
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const GRID_LABEL_H_CH: f64 = 3.2; // vertical band above the grid for the node label, in ch
const GRID_LABEL_BASELINE: f64 = 0.6; // label baseline within that band, as a fraction of the band height
const BTN_W: f64 = 120.0;
const BTN_H: f64 = 28.0;
const BTN_GAP: f64 = 12.0;

// Transport bar (the `#transport` element) that holds the step buttons; sized to fit the two buttons.
const TRANSPORT_H: f64 = 56.0;
const TRANSPORT_PAD: f64 = 16.0;
const TRANSPORT_W: f64 = TRANSPORT_PAD * 2.0 + BTN_W * 2.0 + BTN_GAP;

// Cell metrics are expressed as multiples of the monospace advance ("ch"), which is measured at render time (see
// `measure_char`) rather than hard-coded, so spacing tracks the actual font.
const BYTE_GAP_CH: f64 = 0.5; // gap between bytes, in ch (half a character)
const CELL_PAD_CH: f64 = 2.0; // total horizontal padding inside a cell, in ch
const CELL_H_CH: f64 = 3.5; // cell height, in ch
const CELL_GAP_CH: f64 = 1.0; // whitespace between adjacent cells (rows and columns), in ch
const FALLBACK_CH: f64 = 7.5; // used only if the browser cannot measure (e.g. the SVG is not yet laid out)

const CELL_FONT_FAMILY: &str = "ui-monospace, monospace";
const CELL_FONT_SIZE: &str = "12";

/// Font for a plain node's centred label (e.g. an operation's `symbol`).
const NODE_LABEL_FONT_FAMILY: &str = "sans-serif";
const NODE_LABEL_FONT_SIZE: &str = "14";
/// Horizontal padding kept on each side of a node's label inside its box, so the text never touches the border.
const NODE_LABEL_PAD: f64 = 14.0;

/// Font + side padding for the small `op` boxes in a step visualisation's operation row (e.g. the `XOR` boxes in the
/// nested-map viz). The box takes its label's natural width plus this padding, narrower than the column, so the wire
/// entering its left edge has a visible horizontal run.
const OP_BOX_FONT_FAMILY: &str = "sans-serif";
const OP_BOX_FONT_SIZE: &str = "12";
const OP_BOX_PAD: f64 = 12.0;

const CELL_FILL: &str = "#e6ecf5";
const CELL_TEXT: &str = "#0f1420";
const CELL_STROKE: &str = "#2a3650";
const HILITE_FILL: &str = "#6db3f2"; // background of the row currently being processed

const MEDIUM_GREY: &str = "#888888";
const DEEP_SLATE_BLUE: &str = "#2a3650";
const PALE_BLUE_GREY: &str = "#e6ecf5";
const LIGHT_BLACK: &str = "#111111";

/// Inset (px) of the "ⓘ" description badge from a node's top-right corner.
const DESC_ICON_INSET: f64 = 12.0;

/// Padding (px) of an operation node's coloured card around its inner label + value cell, so the card fully frames it.
const OP_CARD_PAD: f64 = 10.0;

/// Corner radius (px) used to round each elbow in a wire instead of a sharp 90° turn.
/// Clamped per-corner to half the shorter adjacent segment.
const WIRE_CORNER_RADIUS: f64 = 5.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which way a Step button moves the reduction cursor.
#[derive(Clone, Copy)]
enum StepAction {
    Init,
    Forward,
    Back,
}

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
    /// Renderer-injected interactive controls (e.g. the array step buttons) that need to be present but do not need to
    /// be declared in the DSL.
    ///
    /// These own the click closures, so the caller must keep the `Scene` alive for as long as the controls should
    /// stay live (in the browser, leak it for the page lifetime via `std::mem::forget`).
    pub controls: Vec<SvgNode>,
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
/// Wires are created before nodes so that the node boxes paint on top of the connecting lines. `transport_id` is the
/// element the step-button transport bar is drawn into (the main page uses `"transport"`; a modal overlay passes its
/// own container so its controls don't leak into the page's bar).
pub fn render(svg: &SvgRoot, graph: &ValidatedGraph, transport_id: &str) -> Result<Scene, Error> {
    // Discover the real monospace advance ("ch") for this font so all cell spacing derives from it, not a guess.
    let ch = measure_char(svg);

    // Resolve which nodes render as grids (and at what cell size) up front, so the layout can reserve their real
    // footprint and keep downstream nodes (e.g. the ThetaC box) clear of the table.
    let mut grids: HashMap<String, GridSpec> = HashMap::new();
    for (name, decl) in &graph.nodes {
        if let Some(spec) = grid_spec(name, decl, graph, ch) {
            grids.insert(name.clone(), spec);
        }
    }

    let sizes: HashMap<String, Size> = graph
        .nodes
        .iter()
        .map(|(name, decl)| {
            let size = grids
                .get(name)
                .map(|spec| grid_footprint(decl, spec))
                .unwrap_or_else(|| node_box_size(svg, decl));
            (name.clone(), size)
        })
        .collect();

    let placement = layout_sized(graph, &sizes);

    // Per-node wire-attachment anchor on the cross axis (centre, extent). Register/constant value cells sit below a
    // label band, so wires must attach to the cell — not the footprint's centre, which lands near the cell's top edge.
    // Operation cards and plain boxes attach at their box centre.
    let conn = connection_anchors(graph, &placement, &grids);

    let routes = routing::route_all(graph, &placement, &conn);
    let mut wires = Vec::with_capacity(graph.wires.len());
    for (wire, route) in graph.wires.iter().zip(&routes) {
        if let Some(points) = route {
            wires.push(draw_wire(svg, points, wire.name.as_deref())?);
        }
    }

    let mut nodes = HashMap::with_capacity(graph.nodes.len());
    let mut controls = Vec::new();
    // Track the bottom-right of everything actually drawn: grids can extend well beyond the node box, so we fit the
    // viewport to the real content at the end.
    let (mut max_x, mut max_y) = (0.0_f64, 0.0_f64);

    // The reduction step buttons live in a fixed transport bar (`#transport`). Only create it when a node actually
    // feeds a `reduce` and so needs it; otherwise the bar is left empty and the page hides it (CSS `:empty`), so a
    // scalar function — which has nothing to step through — shows no empty transport footer at all.
    let transport = if grids.keys().any(|name| {
        !feeds_imported_fn(name, graph)
            && (reduction_op(name, graph).is_some()
                || comprehension_map(name, graph).is_some()
                || nested_map(name, graph).is_some())
    }) {
        SvgRoot::create_in(transport_id, Size::new(TRANSPORT_W, TRANSPORT_H)).ok()
    } else {
        None
    };

    // Phase 1: draw every node's body. Grid cells are kept so a step visualisation can highlight a *sibling* grid (the
    // nested-map viz highlights the selected element in the broadcast-vector grid, which is a different node).
    let mut grid_cells: HashMap<String, Vec<Vec<SvgNode>>> = HashMap::new();
    let mut grid_geom: HashMap<String, (Point, f64)> = HashMap::new(); // origin + grid bottom edge
    for (name, rect) in &placement {
        let decl = &graph.nodes[name];
        max_x = max_x.max(rect.top_left.x + rect.size.width);
        max_y = max_y.max(rect.top_left.y + rect.size.height);

        if let Some(spec) = grids.get(name) {
            let origin: Point = (*rect).into();
            let (group, cells) = render_array_node(svg, decl, origin, spec)?;
            attach_description(svg, &group, decl, *rect)?;
            nodes.insert(name.clone(), group);
            grid_geom.insert(name.clone(), (origin, origin.y + rect.size.height));
            grid_cells.insert(name.clone(), cells);
        } else {
            let group = render_node(svg, decl, *rect)?;
            // An operation box that applies an imported function is expandable (opens that file); otherwise it just
            // gets its description toggle, if any.
            match import_for_node(decl, graph) {
                Some(path) => attach_expand(svg, &group, *rect, &path)?,
                None => attach_description(svg, &group, decl, *rect)?,
            }
            nodes.insert(name.clone(), group);
        }
    }

    // Phase 2: draw each grid's step visualisation below it (now that every grid's cells exist).
    for name in placement.keys() {
        let Some(spec) = grids.get(name) else {
            continue;
        };
        let (origin, grid_bottom) = grid_geom[name];
        let cells = grid_cells[name].clone();

        // When this grid feeds an imported (expandable) operation, that operation's detail opens in a modal, so no
        // inline step viz is drawn here — the grid only draws itself.
        if feeds_imported_fn(name, graph) {
            max_x = max_x.max(origin.x + spec.cell_w);
            max_y = max_y.max(grid_bottom);
            continue;
        }

        // When the node feeds a `reduce`, visualise the inner fold below the grid; otherwise just the step buttons.
        let (mut ctrls, bottom_right) = match reduction_op(name, graph) {
            Some(op) => {
                let matrix = effective_matrix(spec);
                let label_source = reduction_label_source(name, graph);
                render_reduction(
                    svg,
                    transport.as_ref(),
                    spec,
                    &matrix,
                    &op,
                    label_source,
                    origin,
                    grid_bottom,
                    cells,
                )?
            }
            // When the node feeds a `map` comprehension, visualise the per-element body computation; otherwise the
            // grid is plain data (or a computed result) with nothing to step through.
            None => match comprehension_map(name, graph) {
                Some(map) => render_map(
                    svg,
                    transport.as_ref(),
                    spec,
                    &effective_matrix(spec),
                    &map,
                    origin,
                    grid_bottom,
                    cells,
                    graph,
                )?,
                // A nested map (matrix ⊕ broadcast vector) visualises the row-by-row fold building up the output.
                None => match nested_map(name, graph) {
                    Some(nmap) => render_nested_map(
                        svg,
                        transport.as_ref(),
                        spec,
                        &effective_matrix(spec),
                        &nmap,
                        origin,
                        grid_bottom,
                        cells,
                        grid_cells.get(&nmap.vec_node).cloned(),
                        graph,
                    )?,
                    None => (Vec::new(), Point::new(origin.x + spec.cell_w, grid_bottom)),
                },
            },
        };

        controls.append(&mut ctrls);
        max_x = max_x.max(bottom_right.x);
        max_y = max_y.max(bottom_right.y);
    }

    svg.set_viewport(Size::new(max_x + MARGIN, max_y + MARGIN))?;

    Ok(Scene {
        nodes,
        wires,
        controls,
    })
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
    label.set_attr("font-family", NODE_LABEL_FONT_FAMILY)?;
    label.set_attr("font-size", NODE_LABEL_FONT_SIZE)?;

    group.append(&box_)?;
    group.append(&label)?;

    Ok(group)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws a wire as an orthogonal polyline (an SVG `<path>`) from the points computed by [`routing::route_all`].
fn draw_wire(svg: &SvgRoot, points: &[Point], name: Option<&str>) -> Result<SvgNode, Error> {
    let d = wire_path_data(points, WIRE_CORNER_RADIUS);

    let path = svg.path(&d)?;
    path.set_fill("none")?;
    path.set_stroke(MEDIUM_GREY)?;
    path.set_stroke_width(2.0)?;
    if let Some(name) = name {
        path.set_attr("data-wire", name)?;
    }

    Ok(path)
}

/// Builds the SVG path data for an orthogonal polyline, rounding each interior corner with a quadratic curve of (up to)
/// `radius` px. Each corner's radius is clamped to half the shorter adjacent segment, so short legs never overshoot.
fn wire_path_data(points: &[Point], radius: f64) -> String {
    let Some(first) = points.first() else {
        return String::new();
    };

    let mut d = format!("M {} {}", first.x, first.y);
    let n = points.len();

    for i in 1..n.saturating_sub(1) {
        let (prev, corner, next) = (points[i - 1], points[i], points[i + 1]);
        let r = radius
            .min(point_distance(prev, corner) / 2.0)
            .min(point_distance(corner, next) / 2.0);

        if r <= 0.0 {
            d.push_str(&format!(" L {} {}", corner.x, corner.y));
            continue;
        }

        // Cut the corner: line up to `r` before it, then curve through the corner to `r` past it.
        let enter = point_toward(corner, prev, r);
        let leave = point_toward(corner, next, r);
        d.push_str(&format!(
            " L {} {} Q {} {} {} {}",
            enter.x, enter.y, corner.x, corner.y, leave.x, leave.y
        ));
    }

    if n > 1 {
        let last = points[n - 1];
        d.push_str(&format!(" L {} {}", last.x, last.y));
    }

    d
}

fn point_distance(a: Point, b: Point) -> f64 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

/// The point `r` px from `from` toward `to`.
fn point_toward(from: Point, to: Point, r: f64) -> Point {
    let len = point_distance(from, to);
    if len == 0.0 {
        from
    } else {
        Point::new(
            from.x + (to.x - from.x) / len * r,
            from.y + (to.y - from.y) / len * r,
        )
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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
/// Renders a node's markdown `description` (if any) to HTML.
fn description_html(decl: &NodeDecl) -> Option<String> {
    let markdown = decl
        .properties
        .iter()
        .find(|p| p.name == "description")
        .and_then(|p| match &p.value {
            PropValue::Str(s) => Some(s.as_str()),
            _ => None,
        })?;

    let parser = pulldown_cmark::Parser::new_ext(markdown, pulldown_cmark::Options::all());
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    Some(html)
}

/// If `decl` has a `description`, makes the node clickable, marks it with a small "ⓘ" badge at its top-right corner
/// (so described nodes are discoverable), and toggles the rendered markdown in the page's `#description` panel when
/// clicked.
fn attach_description(
    svg: &SvgRoot,
    group: &SvgNode,
    decl: &NodeDecl,
    rect: Rect,
) -> Result<(), Error> {
    if let Some(html) = description_html(decl) {
        group.set_attr("style", "cursor: pointer")?;

        // Discoverability badge, just inside the node's top-right corner.
        let badge = svg.text(
            Point::new(
                rect.top_left.x + rect.size.width - DESC_ICON_INSET,
                rect.top_left.y + DESC_ICON_INSET,
            ),
            "ⓘ",
        )?;
        badge.set_fill(LIGHT_BLACK)?;
        badge.set_attr("text-anchor", "middle")?;
        badge.set_attr("dominant-baseline", "central")?;
        badge.set_attr("font-family", "sans-serif")?;
        badge.set_attr("font-size", "14")?;
        group.append(&badge)?;

        let node = decl.name.clone();
        group.on_click(move |_| toggle_description(&node, &html))?;
    }
    Ok(())
}

/// Toggles `node`'s rendered `html` in the page's `#description` panel: shows it, or hides it (clears the panel) if that
/// same node's description is already showing. The currently-shown node is tracked via the panel's `data-shown`
/// attribute. A no-op outside the browser (no window/document).
fn toggle_description(node: &str, html: &str) {
    let Some(panel) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("description"))
    else {
        return;
    };

    if panel.get_attribute("data-shown").as_deref() == Some(node) {
        panel.set_inner_html("");
        let _ = panel.remove_attribute("data-shown");
    } else {
        panel.set_inner_html(html);
        let _ = panel.set_attribute("data-shown", node);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Array-grid visualisation
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

/// The function that `name` is an input to.
///
/// Found from the data flow itself, independent of any wire: a node whose `compute` expression applies a function to
/// `name` (e.g. `compute: ThetaC(state)`). Falls back to a wire `name -> op` whose `op` carries a `symbol` naming the
/// function. Because the `compute` link is checked first, the visualisation does not depend on the wire being present.
fn fed_function<'a>(name: &str, graph: &'a ValidatedGraph) -> Option<&'a FnDef> {
    // (a) A node computes a function applied to `name`.
    let via_compute = graph.nodes.values().find_map(|decl| {
        let value = decl.properties.iter().find(|p| p.name == "compute")?;
        match &value.value {
            PropValue::Expr(e) => compute_function(e, name, graph),
            _ => None,
        }
    });
    if via_compute.is_some() {
        return via_compute;
    }

    // (b) A wire carries `name` into an `operation` node whose `symbol` names the function.
    graph.wires.iter().find_map(|wire| {
        let (WireEndpoint::Node(src), WireEndpoint::Node(dst)) = (&wire.source, &wire.target)
        else {
            return None;
        };
        if src != name {
            return None;
        }

        let symbol = string_prop(graph.nodes.get(dst)?, "symbol")?;
        graph.fn_defs.get(&symbol)
    })
}

/// If `expr` applies a function to `name` (as a direct argument), returns that function; recurses into sub-expressions.
fn compute_function<'a>(expr: &Expr, name: &str, graph: &'a ValidatedGraph) -> Option<&'a FnDef> {
    match expr {
        Expr::Call { name: callee, args } => {
            if args
                .iter()
                .any(|a| matches!(a, Expr::Ident(s) if s == name))
            {
                graph.fn_defs.get(callee)
            } else {
                args.iter().find_map(|a| compute_function(a, name, graph))
            }
        }
        Expr::BinOp { lhs, rhs, .. } => {
            compute_function(lhs, name, graph).or_else(|| compute_function(rhs, name, graph))
        }
        Expr::Not(inner) => compute_function(inner, name, graph),
        Expr::Index { base, index } => {
            compute_function(base, name, graph).or_else(|| compute_function(index, name, graph))
        }
        Expr::Reduce { array, .. } => compute_function(array, name, graph),
        Expr::Comprehension { body, .. } => compute_function(body, name, graph),
        Expr::Array(elems) => elems.iter().find_map(|e| compute_function(e, name, graph)),
        Expr::Integer(_) | Expr::HexLit(_) | Expr::Ident(_) => None,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Infers a 2D `(rows, cols)` shape for `name` from the function it feeds.
///
/// A node is drawn as a grid when it is the source of a wire into an `operation` node whose `symbol` names a function
/// whose first parameter is a 2D array type, e.g. `[[u64; 5]; 5]`.
///
/// This is the fallback shape source: when a node declares its own `data` (via a `source` property), `grid_spec` takes
/// the shape directly from that literal instead. Returns `None` for ordinary scalar nodes.
fn inferred_grid_shape(name: &str, graph: &ValidatedGraph) -> Option<(usize, usize)> {
    match &fed_function(name, graph)?.params.first()?.ty {
        Type::Array { element, len: rows } => match element.as_ref() {
            Type::Array { len: cols, .. } => Some((*rows, *cols)),
            _ => None,
        },
        _ => None,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The half-open iteration range `[start, end)` the step buttons walk: the range of the (first) comprehension in the
/// fed function's body, e.g. `for x in 0..5`. Falls back to the full row span when the body has no comprehension.
fn step_range(name: &str, graph: &ValidatedGraph, rows: usize) -> (usize, usize) {
    fed_function(name, graph)
        .and_then(|f| find_comprehension_range(&f.body))
        .unwrap_or((0, rows))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Finds the range of the first comprehension reachable in an expression.
fn find_comprehension_range(expr: &Expr) -> Option<(usize, usize)> {
    match expr {
        Expr::Comprehension { start, end, .. } => Some((*start as usize, *end as usize)),
        Expr::Reduce { array, .. } => find_comprehension_range(array),
        Expr::Not(inner) => find_comprehension_range(inner),
        Expr::Index { base, index } => {
            find_comprehension_range(base).or_else(|| find_comprehension_range(index))
        }
        Expr::BinOp { lhs, rhs, .. } => {
            find_comprehension_range(lhs).or_else(|| find_comprehension_range(rhs))
        }
        Expr::Call { args, .. } => args.iter().find_map(find_comprehension_range),
        Expr::Array(elems) => elems.iter().find_map(find_comprehension_range),
        Expr::Integer(_) | Expr::HexLit(_) | Expr::Ident(_) => None,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Next highlighted index, clamped to the top of `[start, end)` — no wrap-around at the last row.
fn step_forward(current: usize, (start, end): (usize, usize)) -> usize {
    let last = end.saturating_sub(1).max(start);
    (current + 1).min(last)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Previous highlighted index, clamped to `start` — no wrap-around at the first row.
fn step_back(current: usize, (start, _end): (usize, usize)) -> usize {
    current.saturating_sub(1).max(start)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Resolves a node's declared data to a 2D matrix of values: follows its `source: NAME` property to the data binding
/// and interprets the bound array-of-arrays literal. `None` if the node has no `source` or the binding is not 2D.
fn node_data_matrix(decl: &NodeDecl, graph: &ValidatedGraph) -> Option<Vec<Vec<u64>>> {
    let source = ident_prop(decl, "source")?;
    let Expr::Array(rows) = graph.data.get(&source)? else {
        return None;
    };

    rows.iter()
        .map(|row| match row {
            Expr::Array(cells) => cells.iter().map(literal_u64).collect::<Option<Vec<u64>>>(),
            _ => None,
        })
        .collect::<Option<Vec<Vec<u64>>>>()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The numeric value of an integer or hex literal expression.
fn literal_u64(expr: &Expr) -> Option<u64> {
    match expr {
        Expr::Integer(n) | Expr::HexLit(n) => Some(*n),
        _ => None,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Inner reduction (`reduce <op> over a[x]`) — pure helpers used by its visualisation.
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

/// The grid's values as a full matrix: the declared data if present, otherwise the placeholder spread.
fn effective_matrix(spec: &GridSpec) -> Vec<Vec<u64>> {
    spec.values.clone().unwrap_or_else(|| {
        (0..spec.rows)
            .map(|r| {
                (0..spec.cols)
                    .map(|c| placeholder_value(r * spec.cols + c))
                    .collect()
            })
            .collect()
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The reduction operator of the (first) `reduce` in the function `name` feeds, if any — e.g. `Xor` for `reduce xor`.
fn reduction_op(name: &str, graph: &ValidatedGraph) -> Option<BinOp> {
    find_reduction_op(&fed_function(name, graph)?.body)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn find_reduction_op(expr: &Expr) -> Option<BinOp> {
    match expr {
        Expr::Reduce { op, .. } => Some(op.clone()),
        Expr::Comprehension { body, .. } => find_reduction_op(body),
        Expr::Not(inner) => find_reduction_op(inner),
        Expr::Index { base, index } => find_reduction_op(base).or_else(|| find_reduction_op(index)),
        Expr::BinOp { lhs, rhs, .. } => find_reduction_op(lhs).or_else(|| find_reduction_op(rhs)),
        Expr::Call { args, .. } => args.iter().find_map(find_reduction_op),
        Expr::Array(elems) => elems.iter().find_map(find_reduction_op),
        Expr::Integer(_) | Expr::HexLit(_) | Expr::Ident(_) => None,
    }
}

/// The `compute` expression of a node, if it has one.
fn compute_expr(decl: &NodeDecl) -> Option<&Expr> {
    decl.properties
        .iter()
        .find(|p| p.name == "compute")
        .and_then(|p| match &p.value {
            PropValue::Expr(e) => Some(e),
            PropValue::Str(_) => None,
        })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// If `decl`'s `compute` calls a function brought in by an `import`, returns that file's path — the node is then
/// expandable: clicking it opens the imported file's own visualisation.
fn import_for_node(decl: &NodeDecl, graph: &ValidatedGraph) -> Option<String> {
    let Expr::Call { name, .. } = compute_expr(decl)? else {
        return None;
    };
    graph.fn_imports.get(name).cloned()
}

/// Whether the grid node `name` feeds an operation that applies an IMPORTED function. Such an operation renders as an
/// expandable box and its detail opens in a modal, so no inline step visualisation is drawn on this input grid.
fn feeds_imported_fn(name: &str, graph: &ValidatedGraph) -> bool {
    fed_function(name, graph).is_some_and(|f| graph.fn_imports.contains_key(&f.name))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Marks an operation box as expandable: a `data-import` attribute (the page reads it to open the imported file's own
/// visualisation in a modal on click) plus a corner affordance so it reads as clickable.
fn attach_expand(svg: &SvgRoot, group: &SvgNode, rect: Rect, path: &str) -> Result<(), Error> {
    group.set_attr("data-import", path)?;
    group.set_attr("style", "cursor: pointer")?;

    let badge = svg.text(
        Point::new(
            rect.top_left.x + rect.size.width - DESC_ICON_INSET,
            rect.top_left.y + DESC_ICON_INSET,
        ),
        "\u{2922}", // ⤢ — diagonal expand arrows
    )?;
    badge.set_fill(LIGHT_BLACK)?;
    badge.set_attr("text-anchor", "middle")?;
    badge.set_attr("dominant-baseline", "central")?;
    badge.set_attr("font-family", "sans-serif")?;
    badge.set_attr("font-size", "14")?;
    group.append(&badge)?;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Source for the working-row label: the index variable of the (first) comprehension and the expression that its
/// reduction folds over (the operand after `over`), e.g. `("x", a[x])` for `[ for x in 0..5 => reduce xor over a[x] ]`.
fn reduction_label_source(name: &str, graph: &ValidatedGraph) -> Option<(String, Expr)> {
    find_comprehension_detail(&fed_function(name, graph)?.body)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn find_comprehension_detail(expr: &Expr) -> Option<(String, Expr)> {
    match expr {
        Expr::Comprehension { var, body, .. } => reduce_array(body).map(|arr| (var.clone(), arr)),
        Expr::Reduce { array, .. } => find_comprehension_detail(array),
        Expr::Not(inner) => find_comprehension_detail(inner),
        Expr::Index { base, index } => {
            find_comprehension_detail(base).or_else(|| find_comprehension_detail(index))
        }
        Expr::BinOp { lhs, rhs, .. } => {
            find_comprehension_detail(lhs).or_else(|| find_comprehension_detail(rhs))
        }
        Expr::Call { args, .. } => args.iter().find_map(find_comprehension_detail),
        Expr::Array(elems) => elems.iter().find_map(find_comprehension_detail),
        Expr::Integer(_) | Expr::HexLit(_) | Expr::Ident(_) => None,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The operand a reduction folds over (the expression after `over`).
fn reduce_array(expr: &Expr) -> Option<Expr> {
    match expr {
        Expr::Reduce { array, .. } => Some((**array).clone()),
        _ => None,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Renders an expression to a label string, substituting the index variable `var` with the concrete `value`.
/// e.g. `a[x]` with `var = "x"`, `value = 2` becomes `"a[2]"`.
fn expr_label(expr: &Expr, var: &str, value: usize) -> String {
    match expr {
        Expr::Ident(s) if s == var => value.to_string(),
        Expr::Ident(s) => s.clone(),
        Expr::Integer(n) => n.to_string(),
        Expr::HexLit(n) => format!("0x{n:x}"),
        Expr::Index { base, index } => {
            format!(
                "{}[{}]",
                expr_label(base, var, value),
                expr_label(index, var, value)
            )
        }
        Expr::Call { name, args } => {
            let inner: Vec<String> = args.iter().map(|a| expr_label(a, var, value)).collect();
            format!("{name}({})", inner.join(", "))
        }
        Expr::BinOp { op, lhs, rhs } => {
            // Parenthesise binary operands so a nested expression reads unambiguously, e.g. `(x + 4) mod 5`.
            let paren = |e: &Expr| {
                let s = expr_label(e, var, value);
                if matches!(e, Expr::BinOp { .. }) {
                    format!("({s})")
                } else {
                    s
                }
            };
            format!("{} {op} {}", paren(lhs), paren(rhs))
        }
        Expr::Not(inner) => format!("not {}", expr_label(inner, var, value)),
        Expr::Array(elems) => {
            let inner: Vec<String> = elems.iter().map(|e| expr_label(e, var, value)).collect();
            format!("[{}]", inner.join(", "))
        }
        Expr::Comprehension { .. } | Expr::Reduce { .. } => "…".to_string(),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Measures the monospace advance ("ch") in user units for the cell font, by probing a single "0" glyph.
///
/// SVG geometry attributes (a cell's `width`, element `x`/`y`) take user-unit numbers and cannot use font-relative CSS
/// units like `ch`, so the value must be known numerically.
///
/// We discover can its value from the rendered font rather than trying to guess.
///
/// Falls back to [`FALLBACK_CH`] if the browser cannot measure it (e.g. the SVG is not yet laid out, or off the wasm
/// target).
fn measure_char(svg: &SvgRoot) -> f64 {
    let Ok(probe) = svg.text(Point::origin(), "0") else {
        return FALLBACK_CH;
    };
    let _ = probe.set_attr("font-family", CELL_FONT_FAMILY);
    let _ = probe.set_attr("font-size", CELL_FONT_SIZE);

    let ch = probe
        .computed_text_length()
        .filter(|w| *w > 0.0)
        .unwrap_or(FALLBACK_CH);

    // The probe has served its purpose, so make sure it remains hidden
    let _ = probe.set_attr("visibility", "hidden");
    ch
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Measures the rendered width (user units) of `text` at the given font, via a hidden probe text node — the same
/// technique as [`measure_char`], but for a full string (so a node box can be widened to fit its label).
///
/// Falls back to a per-character estimate from [`FALLBACK_CH`] when the browser cannot measure (e.g. off the wasm
/// target, or before layout), so non-browser callers still get a sensible width.
fn measure_text(svg: &SvgRoot, text: &str, font_family: &str, font_size: &str) -> f64 {
    let estimate = FALLBACK_CH * text.chars().count() as f64;
    let Ok(probe) = svg.text(Point::origin(), text) else {
        return estimate;
    };
    let _ = probe.set_attr("font-family", font_family);
    let _ = probe.set_attr("font-size", font_size);

    let width = probe
        .computed_text_length()
        .filter(|w| *w > 0.0)
        .unwrap_or(estimate);

    // The probe has served its purpose; keep it hidden.
    let _ = probe.set_attr("visibility", "hidden");
    width
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The box width needed to hold a centred label of rendered width `text_w`: the default [`NODE_W`], or the label plus
/// [`NODE_LABEL_PAD`] clearance on each side when that is wider.
fn node_box_width(text_w: f64) -> f64 {
    (text_w + 2.0 * NODE_LABEL_PAD).max(NODE_W)
}

/// The footprint a plain (non-grid) node reserves: the default box, widened if its centred label is longer than the
/// box can hold. Height is unchanged.
fn node_box_size(svg: &SvgRoot, decl: &NodeDecl) -> Size {
    let label = node_label(decl);
    let text_w = measure_text(svg, &label, NODE_LABEL_FONT_FAMILY, NODE_LABEL_FONT_SIZE);
    Size::new(node_box_width(text_w), NODE_H)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The drawn footprint of a grid: cells plus inter-cell gaps, with the label band above.
fn grid_size(spec: &GridSpec) -> Size {
    let cols = spec.cols as f64;
    let rows = spec.rows as f64;
    Size::new(
        cols * spec.cell_w + (cols - 1.0).max(0.0) * spec.cell_gap,
        spec.label_h + rows * spec.cell_h + (rows - 1.0).max(0.0) * spec.cell_gap,
    )
}

/// Padding drawn around a node's inner content by its coloured card. Only operation nodes get a card (and so a frame);
/// everything else is zero.
fn card_pad(decl: &NodeDecl) -> f64 {
    if matches!(decl.kind, NodeKind::Operation) {
        OP_CARD_PAD
    } else {
        0.0
    }
}

/// A node's reserved footprint: its grid plus any card padding, so layout keeps neighbours clear of the framed card.
fn grid_footprint(decl: &NodeDecl, spec: &GridSpec) -> Size {
    let pad = card_pad(decl);
    let base = grid_size(spec);
    Size::new(base.width + 2.0 * pad, base.height + 2.0 * pad)
}

/// Cross-axis wire-attachment anchor `(centre, extent)` for every node. A register/constant value cell sits below its
/// label band, so wires anchor to the cells; operation cards and plain boxes anchor to the whole box centre.
fn connection_anchors(
    graph: &ValidatedGraph,
    placement: &HashMap<String, Rect>,
    grids: &HashMap<String, GridSpec>,
) -> HashMap<String, (f64, f64)> {
    let horizontal = matches!(
        graph.flow,
        FlowDirection::LeftToRight | FlowDirection::RightToLeft
    );

    placement
        .iter()
        .map(|(name, rect)| {
            let decl = &graph.nodes[name];
            let anchor = match grids.get(name) {
                // Register/constant value cells: anchor to the cells, below the label band.
                Some(spec) if !matches!(decl.kind, NodeKind::Operation) => {
                    let pad = card_pad(decl);
                    if horizontal {
                        let top = rect.top_left.y + pad + spec.label_h;
                        let h = spec.rows as f64 * spec.cell_h
                            + (spec.rows as f64 - 1.0) * spec.cell_gap;
                        (top + h / 2.0, h)
                    } else {
                        let left = rect.top_left.x + pad;
                        let w = spec.cols as f64 * spec.cell_w
                            + (spec.cols as f64 - 1.0) * spec.cell_gap;
                        (left + w / 2.0, w)
                    }
                }
                // Operation cards and plain boxes: anchor to the box centre.
                _ => {
                    if horizontal {
                        (rect.top_left.y + rect.size.height / 2.0, rect.size.height)
                    } else {
                        (rect.top_left.x + rect.size.width / 2.0, rect.size.width)
                    }
                }
            };
            (name.clone(), anchor)
        })
        .collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Cell width (user units) for a value of `digits` hex digits, given the measured monospace advance `ch`.
///
/// Budgets one `ch` per hex digit, [`BYTE_GAP_CH`] per inter-byte gap (see [`format_cell`]), and [`CELL_PAD_CH`] of
/// total padding — all as multiples of the real font metric.
fn cell_width(digits: usize, ch: f64) -> f64 {
    let glyphs = digits as f64 + byte_separators(digits) as f64 * BYTE_GAP_CH + CELL_PAD_CH;
    glyphs * ch
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Number of inter-byte gaps in a `digits`-digit value: one fewer than the byte count (0 for a single-byte hex8 value).
fn byte_separators(digits: usize) -> usize {
    (digits / 2).saturating_sub(1)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Number of hex digits the node's `format` displays. Defaults to 2 (hex8) when absent or unrecognised.
fn format_digits(decl: &NodeDecl) -> usize {
    match ident_prop(decl, "format").as_deref() {
        Some("hex16") => 4,
        Some("hex32") => 8,
        Some("hex64") => 16,
        _ => 2,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Formats a value for display: masked to the format's bit width and grouped into bytes (pairs of hex digits) separated
/// by a small gap. A single-byte hex8 value has no gap.
fn format_value(value: u64, digits: usize) -> String {
    let bits = digits * 4;
    let masked = if bits >= 64 {
        value
    } else {
        value & ((1u64 << bits) - 1)
    };

    let hex = format!("{masked:0digits$x}");
    hex.chars()
        .collect::<Vec<_>>()
        .chunks(2)
        .map(|byte| byte.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(" ")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Placeholder cell value used when a node declares no data: a deterministic spread so the full cell width is visibly
/// used.
fn placeholder_value(index: usize) -> u64 {
    (index as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Returns a node's string-valued property (e.g. `symbol: "ThetaC"`), if present.
fn string_prop(decl: &NodeDecl, key: &str) -> Option<String> {
    decl.properties
        .iter()
        .find(|p| p.name == key)
        .and_then(|p| match &p.value {
            PropValue::Str(s) => Some(s.clone()),
            _ => None,
        })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Returns a node's identifier-valued property (e.g. `format: hex64`), if present.
fn ident_prop(decl: &NodeDecl, key: &str) -> Option<String> {
    decl.properties
        .iter()
        .find(|p| p.name == key)
        .and_then(|p| match &p.value {
            PropValue::Expr(Expr::Ident(s)) => Some(s.clone()),
            _ => None,
        })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the table described by `spec` anchored at `origin`, with the node label above it.
///
/// Each cell shows the node's declared data value (`spec.values`, from its `data`/`source`) when present, otherwise a
/// placeholder. Returns the node group and the cell handles grouped by row, so the caller can re-colour a row to
/// highlight it.
fn render_array_node(
    svg: &SvgRoot,
    decl: &NodeDecl,
    origin: Point,
    spec: &GridSpec,
) -> Result<(SvgNode, Vec<Vec<SvgNode>>), Error> {
    let group = svg.group()?;
    group.set_attr("data-node", &decl.name)?;

    // An `operation` rendered as a value grid (its computed "after" result) keeps the coloured, bordered card of a
    // normal operation box, so it still reads as a transform rather than a bare data cell — and a description badge
    // stays visible against it. The card fully frames the inner label + value cell, which are inset by `pad`; data
    // nodes (registers/constants) have no padding and show their values as plain cells.
    let pad = card_pad(decl);
    if matches!(decl.kind, NodeKind::Operation) {
        let card = svg.rect(Point::new(origin.x, origin.y), grid_footprint(decl, spec))?;
        card.set_fill(fill_for(&decl.kind))?;
        card.set_stroke("black")?;
        card.set_stroke_width(1.5)?;
        card.set_attr("rx", "6")?;
        group.append(&card)?;
    }

    // Top-left of the inner content (label band + cells), framed by `pad` of card on every side.
    let inner = Point::new(origin.x + pad, origin.y + pad);

    let title = svg.text(
        Point::new(inner.x, inner.y + spec.label_h * GRID_LABEL_BASELINE),
        &node_label(decl),
    )?;
    title.set_fill(PALE_BLUE_GREY)?;
    title.set_attr("font-family", "sans-serif")?;
    title.set_attr("font-size", "13")?;
    title.set_attr("font-weight", "600")?;
    group.append(&title)?;

    let grid_top = inner.y + spec.label_h;
    let mut cells: Vec<Vec<SvgNode>> = Vec::with_capacity(spec.rows);

    for r in 0..spec.rows {
        let mut row_cells = Vec::with_capacity(spec.cols);

        for c in 0..spec.cols {
            let x = inner.x + c as f64 * (spec.cell_w + spec.cell_gap);
            let y = grid_top + r as f64 * (spec.cell_h + spec.cell_gap);

            let cell = svg.rect(Point::new(x, y), Size::new(spec.cell_w, spec.cell_h))?;
            cell.set_fill(CELL_FILL)?;
            cell.set_stroke(CELL_STROKE)?;
            cell.set_stroke_width(1.0)?;
            group.append(&cell)?;

            // Show the declared data value if present, otherwise a placeholder.
            let value = spec
                .values
                .as_ref()
                .and_then(|v| v.get(r))
                .and_then(|row| row.get(c))
                .copied()
                .unwrap_or_else(|| placeholder_value(r * spec.cols + c));

            let text = svg.text(
                Point::new(x + spec.cell_w / 2.0, y + spec.cell_h / 2.0),
                &format_value(value, spec.digits),
            )?;
            text.set_fill(CELL_TEXT)?;
            text.set_attr("text-anchor", "middle")?;
            text.set_attr("dominant-baseline", "central")?;
            text.set_attr("font-family", CELL_FONT_FAMILY)?;
            text.set_attr("font-size", CELL_FONT_SIZE)?;
            // The inter-byte separator is a full monospace space (1ch); shrink it to BYTE_GAP_CH via word-spacing,
            // expressed in `ch` so it tracks the font with no hard-coded pixel value.
            text.set_attr("style", &format!("word-spacing: {}ch", BYTE_GAP_CH - 1.0))?;
            group.append(&text)?;

            row_cells.push(cell);
        }

        cells.push(row_cells);
    }

    Ok((group, cells))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Sets every cell's background to the default, then paints the `active` row with the highlight colour.
fn highlight_row(cells: &[Vec<SvgNode>], active: usize) {
    for (r, row) in cells.iter().enumerate() {
        let fill = if r == active { HILITE_FILL } else { CELL_FILL };
        for cell in row {
            let _ = cell.set_fill(fill);
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws a value cell (rect + centred, byte-spaced text) and returns the (rect, text) handles.
fn draw_value_cell(
    svg: &SvgRoot,
    top_left: Point,
    size: Size,
    content: &str,
) -> Result<(SvgNode, SvgNode), Error> {
    let rect = svg.rect(top_left, size)?;
    rect.set_fill(CELL_FILL)?;
    rect.set_stroke(CELL_STROKE)?;
    rect.set_stroke_width(1.0)?;

    let text = svg.text(
        Point::new(
            top_left.x + size.width / 2.0,
            top_left.y + size.height / 2.0,
        ),
        content,
    )?;
    text.set_fill(CELL_TEXT)?;
    text.set_attr("text-anchor", "middle")?;
    text.set_attr("dominant-baseline", "central")?;
    text.set_attr("font-family", CELL_FONT_FAMILY)?;
    text.set_attr("font-size", CELL_FONT_SIZE)?;
    text.set_attr("style", &format!("word-spacing: {}ch", BYTE_GAP_CH - 1.0))?;

    Ok((rect, text))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws a connector wire through an orthogonal polyline (no fill, grey stroke), rounding each elbow.
fn wire(svg: &SvgRoot, points: &[Point]) -> Result<(), Error> {
    let path = svg.path(&wire_path_data(points, WIRE_CORNER_RADIUS))?;
    path.set_fill("none")?;
    path.set_stroke(MEDIUM_GREY)?;
    path.set_stroke_width(1.5)?;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A clickable labelled button:
/// Returns a rounded rect with centred text, wrapped in a `<g>` to which a click handler can be attached.
fn make_button(svg: &SvgRoot, origin: Point, label: &str) -> Result<SvgNode, Error> {
    let group = svg.group()?;
    group.set_attr("style", "cursor: pointer")?;

    let rect = svg.rect(origin, Size::new(BTN_W, BTN_H))?;
    rect.set_fill(DEEP_SLATE_BLUE)?;
    rect.set_stroke(HILITE_FILL)?;
    rect.set_stroke_width(1.0)?;
    rect.set_attr("rx", "6")?;
    group.append(&rect)?;

    let text = svg.text(
        Point::new(origin.x + BTN_W / 2.0, origin.y + BTN_H / 2.0),
        label,
    )?;
    text.set_fill(PALE_BLUE_GREY)?;
    text.set_attr("text-anchor", "middle")?;
    text.set_attr("dominant-baseline", "central")?;
    text.set_attr("font-family", "sans-serif")?;
    text.set_attr("font-size", "12")?;
    group.append(&text)?;

    Ok(group)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
