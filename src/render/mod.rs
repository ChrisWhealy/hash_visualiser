mod eval;
mod layout;
pub(crate) mod rect;
mod routing;

use std::{cell::Cell, collections::HashMap, rc::Rc};
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
use layout::{MARGIN, NODE_H, NODE_W, layout, layout_sized};
use rect::Rect;

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
/// Wires are created before nodes so that the node boxes paint on top of the connecting lines.
pub fn render(svg: &SvgRoot, graph: &ValidatedGraph) -> Result<Scene, Error> {
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
        reduction_op(name, graph).is_some()
            || comprehension_map(name, graph).is_some()
            || nested_map(name, graph).is_some()
    }) {
        SvgRoot::create_in("transport", Size::new(TRANSPORT_W, TRANSPORT_H)).ok()
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
            attach_description(svg, &group, decl, *rect)?;
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
        Point::new(from.x + (to.x - from.x) / len * r, from.y + (to.y - from.y) / len * r)
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
    let markdown = decl.properties.iter().find(|p| p.name == "description").and_then(|p| match &p.value {
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
fn attach_description(svg: &SvgRoot, group: &SvgNode, decl: &NodeDecl, rect: Rect) -> Result<(), Error> {
    if let Some(html) = description_html(decl) {
        group.set_attr("style", "cursor: pointer")?;

        // Discoverability badge, just inside the node's top-right corner.
        let badge = svg.text(
            Point::new(rect.top_left.x + rect.size.width - DESC_ICON_INSET, rect.top_left.y + DESC_ICON_INSET),
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
        let (WireEndpoint::Node(src), WireEndpoint::Node(dst)) = (&wire.source, &wire.target) else {
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
            if args.iter().any(|a| matches!(a, Expr::Ident(s) if s == name)) {
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
/// How a node should be drawn as a table: its dimensions plus the per-cell width and hex-digit count derived from the
/// node's `format` specifier.
struct GridSpec {
    rows: usize,
    cols: usize,
    digits: usize,
    /// Half-open `[start, end)` range the step buttons walk, from the fed function's comprehension.
    step_range: (usize, usize),
    /// The node's declared data (`source: NAME`), if any. When present the grid shows these values; otherwise it
    /// shows placeholders.
    values: Option<Vec<Vec<u64>>>,
    /// All in user units, derived from the measured monospace advance `ch`.
    cell_w: f64,
    cell_h: f64,
    cell_gap: f64,
    label_h: f64,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Resolves the grid spec for a node, or `None` if it is not an array node. `ch` is the measured monospace advance;
/// every metric (cell width/height and inter-cell gap) is a multiple of it.
///
/// The shape comes from the node's declared data (`source: NAME`) when present, otherwise it is inferred from the type
/// of the function the node feeds.
fn grid_spec(name: &str, decl: &NodeDecl, graph: &ValidatedGraph, ch: f64) -> Option<GridSpec> {
    // The operation node that applies a map renders as a plain box; its output is shown by the map visualisation on
    // the input grid, so it must not also draw its own value grid.
    if is_map_operation(decl, graph) {
        return None;
    }

    // Prefer a 2D-array `source` (a grid of values); then a 1-D array (a `[u64; N]` data source or a comprehension
    // result) shown as a single row; then a single scalar value shown in a 1×1 cell. So "before"/"after" values are
    // visualised whether they're a word, a vector, or a matrix.
    let values = node_data_matrix(decl, graph)
        .or_else(|| eval::node_matrix(decl, graph))
        .or_else(|| eval::node_array(decl, graph).map(|row| vec![row]))
        .or_else(|| eval::node_value(decl, graph).map(|v| vec![vec![v]]));
    let (rows, cols) = match &values {
        Some(rows) if !rows.is_empty() => (rows.len(), rows[0].len()),
        _ => inferred_grid_shape(name, graph)?,
    };
    let digits = format_digits(decl);

    Some(GridSpec {
        rows,
        cols,
        digits,
        step_range: step_range(name, graph, rows),
        values,
        cell_w: cell_width(digits, ch),
        cell_h: CELL_H_CH * ch,
        cell_gap: CELL_GAP_CH * ch,
        label_h: GRID_LABEL_H_CH * ch,
    })
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
            .map(|r| (0..spec.cols).map(|c| placeholder_value(r * spec.cols + c)).collect())
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
        Expr::Index { base, index } => {
            find_reduction_op(base).or_else(|| find_reduction_op(index))
        }
        Expr::BinOp { lhs, rhs, .. } => {
            find_reduction_op(lhs).or_else(|| find_reduction_op(rhs))
        }
        Expr::Call { args, .. } => args.iter().find_map(find_reduction_op),
        Expr::Array(elems) => elems.iter().find_map(find_reduction_op),
        Expr::Integer(_) | Expr::HexLit(_) | Expr::Ident(_) => None,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A map comprehension a grid node feeds: `[ for <var> in start..end => <body> ]` whose body is NOT a reduction.
struct MapInfo {
    var: String,
    body: Expr,
    range: (usize, usize),
    /// Name of the array parameter the body indexes into (the fed function's first parameter).
    array_param: String,
}

/// A comprehension body the map visualiser can draw as an expression tree: a single (scalar) element, not a nested
/// comprehension or a reduction (those are visualised differently / not yet).
fn is_simple_map_body(body: &Expr) -> bool {
    !matches!(body, Expr::Comprehension { .. }) && find_reduction_op(body).is_none()
}

/// If `name` feeds a function that is a simple (1-D, non-reduce) map comprehension, returns its details so the renderer
/// can visualise the per-element computation. Reduce- and nested-comprehensions aren't handled here.
fn comprehension_map(name: &str, graph: &ValidatedGraph) -> Option<MapInfo> {
    let func = fed_function(name, graph)?;
    let Expr::Comprehension { var, start, end, body } = &func.body else {
        return None;
    };
    if !is_simple_map_body(body) {
        return None;
    }
    Some(MapInfo {
        var: var.clone(),
        body: (**body).clone(),
        range: (*start as usize, *end as usize),
        array_param: func.params.first()?.name.clone(),
    })
}

/// A nested map a matrix grid node feeds: `fn f(a: [[..];M], d: [..]) = [ for x => [ for y => a[x][y] op d[x] ] ]`
/// — the outer loop walks rows of the matrix `a` (each paired with one broadcast value `d[x]`); the inner loop folds
/// that value into every element of the row. Detected for the *matrix* input only, so the visualiser draws it once.
struct NestedMapInfo {
    /// Half-open range the step buttons walk — the outer comprehension's range.
    outer_range: (usize, usize),
    /// The matrix and broadcast-vector parameter names (for the `a[x]` / `d[x]` labels).
    matrix_param: String,
    vec_param: String,
    /// Node supplying the broadcast vector `d`, and the operation node holding the computed output.
    vec_node: String,
    op_node: String,
    /// Glyph for the per-element operation box (the inner body's operator, e.g. `XOR`).
    op_label: String,
}

/// If the matrix grid node `matrix_node` is the first argument of an operation that applies a nested map, returns its
/// details so the renderer can visualise the row-by-row fold. `None` for the broadcast-vector input or any other node.
fn nested_map(matrix_node: &str, graph: &ValidatedGraph) -> Option<NestedMapInfo> {
    graph.nodes.iter().find_map(|(op_name, decl)| {
        let Expr::Call { name: callee, args } = compute_expr(decl)? else {
            return None;
        };
        // The matrix must be the first argument and the broadcast vector the second.
        let (Expr::Ident(first), Some(Expr::Ident(vec_node))) = (args.first()?, args.get(1)) else {
            return None;
        };
        if first != matrix_node {
            return None;
        }

        let func = graph.fn_defs.get(callee)?;
        let Expr::Comprehension { start, end, body: inner, .. } = &func.body else {
            return None;
        };
        let Expr::Comprehension { body: leaf, .. } = inner.as_ref() else {
            return None;
        };
        if !is_simple_map_body(leaf) || func.params.len() < 2 {
            return None;
        }

        let op_label = match leaf.as_ref() {
            Expr::BinOp { op, .. } => op.to_string(),
            _ => "f".to_string(),
        };
        Some(NestedMapInfo {
            outer_range: (*start as usize, *end as usize),
            matrix_param: func.params[0].name.clone(),
            vec_param: func.params[1].name.clone(),
            vec_node: vec_node.clone(),
            op_node: op_name.clone(),
            op_label,
        })
    })
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

/// Whether `decl` is the operation node that applies a map comprehension — 1-D (`[ for x => … ]`) or nested
/// (`[ for x => [ for y => … ] ]`). Such a node renders as a plain box: its output is shown by the map visualisation
/// attached to the *input* grid (the per-element computation building up the result), so a separate output grid would
/// just duplicate it.
fn is_map_operation(decl: &NodeDecl, graph: &ValidatedGraph) -> bool {
    let Some(Expr::Call { name, .. }) = compute_expr(decl) else {
        return false;
    };
    graph.fn_defs.get(name).map(|f| is_map_body(&f.body)).unwrap_or(false)
}

/// Whether `body` is a map comprehension over a simple (non-reduce) leaf: one comprehension layer (1-D) or several
/// nested (N-D).
fn is_map_body(body: &Expr) -> bool {
    match body {
        Expr::Comprehension { body: inner, .. } => is_simple_map_body(inner) || is_map_body(inner),
        _ => false,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Left-folds `values` with the reduction operator, matching the comprehension semantics (`w0 op w1 op …`).
/// `None` for an empty row or a non-associative operator (the latter is already rejected by the parser).
fn apply_reduce(op: &BinOp, values: &[u64]) -> Option<u64> {
    let f: fn(u64, u64) -> u64 = match op {
        BinOp::Xor => |a, b| a ^ b,
        BinOp::And => |a, b| a & b,
        BinOp::Or => |a, b| a | b,
        BinOp::Add => |a, b| a.wrapping_add(b),
        _ => return None,
    };
    values.iter().copied().reduce(f)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Display glyph for a reduction operator, shown in the operation-row boxes.
fn op_symbol(op: &BinOp) -> &'static str {
    match op {
        BinOp::Xor => "xor",
        BinOp::And => "and",
        BinOp::Or => "or",
        BinOp::Add => "+",
        _ => "?",
    }
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
            format!("{}[{}]", expr_label(base, var, value), expr_label(index, var, value))
        }
        Expr::Call { name, args } => {
            let inner: Vec<String> = args.iter().map(|a| expr_label(a, var, value)).collect();
            format!("{name}({})", inner.join(", "))
        }
        Expr::BinOp { op, lhs, rhs } => {
            // Parenthesise binary operands so a nested expression reads unambiguously, e.g. `(x + 4) mod 5`.
            let paren = |e: &Expr| {
                let s = expr_label(e, var, value);
                if matches!(e, Expr::BinOp { .. }) { format!("({s})") } else { s }
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

    let width = probe.computed_text_length().filter(|w| *w > 0.0).unwrap_or(estimate);

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
    if matches!(decl.kind, NodeKind::Operation) { OP_CARD_PAD } else { 0.0 }
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
                        let h = spec.rows as f64 * spec.cell_h + (spec.rows as f64 - 1.0) * spec.cell_gap;
                        (top + h / 2.0, h)
                    } else {
                        let left = rect.top_left.x + pad;
                        let w = spec.cols as f64 * spec.cell_w + (spec.cols as f64 - 1.0) * spec.cell_gap;
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
        Point::new(top_left.x + size.width / 2.0, top_left.y + size.height / 2.0),
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

/// Which way a Step button moves the reduction cursor.
#[derive(Clone, Copy)]
enum StepAction {
    Init,
    Forward,
    Back,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Visualises the inner reduction `reduce <op> over <expr>` below the grid, as a static left-fold for the selected row.
///
/// Layout (all aligned to the grid's columns):
///   * a **working row** — a live copy of the selected row's values (the inputs to this iteration), labelled with the
///     `over` expression (e.g. `a[x]`) with the index variable substituted for the current row;
///   * an **operation row** — `op` boxes in columns `1..cols`, forming a left-fold (each box folds the running result
///     with the next working element);
///   * a **result row** — a single cell in the rightmost column, directly below the final fold box, showing this
///     iteration's output (the reduction of the working row); and
///   * an **output-state row** — `c[x]` for each outer row. Stepping forward computes and fills `c[x]`; stepping back
///     blanks the cell just left, so the output reflects only the rows reached so far.
///
/// The Step buttons live in the fixed transport bar (`transport`) when one is present, otherwise below the diagram.
/// They drive a single shared closure that re-points the working values/label, the output value, and the highlights.
/// Returns the buttons (which own that closure) and the drawn area's bottom-right corner for viewport fitting.
#[allow(clippy::too_many_arguments)]
fn render_reduction(
    svg: &SvgRoot,
    transport: Option<&SvgRoot>,
    spec: &GridSpec,
    matrix: &[Vec<u64>],
    op: &BinOp,
    label_source: Option<(String, Expr)>,
    grid_origin: Point,
    grid_bottom: f64,
    state_cells: Vec<Vec<SvgNode>>,
) -> Result<(Vec<SvgNode>, Point), Error> {
    let (cell_w, cell_h, gap) = (spec.cell_w, spec.cell_h, spec.cell_gap);
    let (cols, rows, digits) = (spec.cols, spec.rows, spec.digits);
    let range = spec.step_range;
    let start = range.0;

    let col_x = |c: usize| grid_origin.x + c as f64 * (cell_w + gap);

    // The fold is drawn below the grid; the buttons are no longer interleaved here (they live in the transport bar).
    let content_top = grid_bottom + cell_h;
    let label_baseline = content_top + cell_h * 0.7;
    let working_y = content_top + cell_h;
    let op_y = working_y + 2.0 * cell_h; // a `cell_h` band is left clear between rows for the connector wires
    let result_y = op_y + 2.0 * cell_h; // this iteration's output, directly below the final fold box
    let output_y = result_y + 2.0 * cell_h;

    // --- working-row label: the `over` expression with the index variable bound to the current row ---
    fn label_text(source: &Option<(String, Expr)>, x: usize) -> String {
        source
            .as_ref()
            .map(|(var, expr)| expr_label(expr, var, x))
            .unwrap_or_default()
    }
    let working_label =
        svg.text(Point::new(grid_origin.x, label_baseline), &label_text(&label_source, start))?;
    working_label.set_fill(PALE_BLUE_GREY)?;
    working_label.set_attr("font-family", CELL_FONT_FAMILY)?;
    working_label.set_attr("font-size", "13")?;

    // --- working row: a live copy of the selected row ---
    let start_vals = matrix.get(start).cloned().unwrap_or_default();
    let mut working_texts = Vec::with_capacity(cols);
    for c in 0..cols {
        let content = format_value(start_vals.get(c).copied().unwrap_or(0), digits);
        let (_, text) =
            draw_value_cell(svg, Point::new(col_x(c), working_y), Size::new(cell_w, cell_h), &content)?;
        working_texts.push(text);
    }

    // --- operation row: `op` boxes in columns 1..cols (column 0 is just the first operand) ---
    for c in 1..cols {
        let box_ = svg.rect(Point::new(col_x(c), op_y), Size::new(cell_w, cell_h))?;
        box_.set_fill(DEEP_SLATE_BLUE)?;
        box_.set_stroke(HILITE_FILL)?;
        box_.set_stroke_width(1.0)?;
        box_.set_attr("rx", "4")?;

        let label = svg.text(Point::new(col_x(c) + cell_w / 2.0, op_y + cell_h / 2.0), op_symbol(op))?;
        label.set_fill(PALE_BLUE_GREY)?;
        label.set_attr("text-anchor", "middle")?;
        label.set_attr("dominant-baseline", "central")?;
        label.set_attr("font-family", "sans-serif")?;
        label.set_attr("font-size", "12")?;
    }

    // --- fold connector wires (static) ---
    let working_bottom = working_y + cell_h;
    let op_mid = op_y + cell_h / 2.0;
    let op_top = |c: usize| Point::new(col_x(c) + cell_w / 2.0, op_y); // "next element" input
    let op_left = |c: usize| Point::new(col_x(c), op_mid); // "running result" input
    let op_right = |c: usize| Point::new(col_x(c) + cell_w, op_mid);

    for c in 1..cols {
        // The next working element drops straight down into the box's top.
        let from = Point::new(col_x(c) + cell_w / 2.0, working_bottom);
        wire(svg, &[from, op_top(c)])?;

        if c == 1 {
            // The first operand (working[0]) elbows into the left of the first box.
            let w0 = Point::new(col_x(0) + cell_w / 2.0, working_bottom);
            wire(svg, &[w0, Point::new(w0.x, op_mid), op_left(1)])?;
        } else {
            // The running result flows horizontally from the previous box into this one's left.
            wire(svg, &[op_right(c - 1), op_left(c)])?;
        }
    }

    // --- result row: this iteration's output value, a single cell in the rightmost column ---
    let last_col = cols.saturating_sub(1);
    let (_, result_text) =
        draw_value_cell(svg, Point::new(col_x(last_col), result_y), Size::new(cell_w, cell_h), "")?;
    if cols >= 2 {
        // The final fold box's output drops straight down into the result cell.
        let x_mid = col_x(last_col) + cell_w / 2.0;
        wire(svg, &[Point::new(x_mid, op_y + cell_h), Point::new(x_mid, result_y)])?;
    }

    // --- output-state row: c[x] for each outer row, all initially blank ---
    let mut output_rects = Vec::with_capacity(rows);
    let mut output_texts = Vec::with_capacity(rows);
    for r in 0..rows {
        let (rect, text) =
            draw_value_cell(svg, Point::new(col_x(r), output_y), Size::new(cell_w, cell_h), "")?;
        output_rects.push(rect);
        output_texts.push(text);
    }

    // --- shared navigation: forward computes c[x]; back blanks the cell left behind ---
    let matrix = matrix.to_vec(); // own the data so the closure can be 'static
    let op = op.clone();
    let current = Cell::new(start);

    let go: Rc<dyn Fn(StepAction)> = Rc::new(move |action: StepAction| {
        let old = current.get();
        let x = match action {
            StepAction::Init => old,
            StepAction::Forward => step_forward(old, range),
            StepAction::Back => step_back(old, range),
        };
        current.set(x);

        highlight_row(&state_cells[..], x);

        if let Some(vals) = matrix.get(x) {
            for (c, t) in working_texts.iter().enumerate() {
                t.set_text(&format_value(vals.get(c).copied().unwrap_or(0), digits));
            }
            // The result row always shows the current iteration's output (recomputed each step).
            result_text.set_text(&apply_reduce(&op, vals).map(|r| format_value(r, digits)).unwrap_or_default());
        }
        working_label.set_text(&label_text(&label_source, x));

        match action {
            // Arriving at a row (forward, or the initial display) computes and writes its reduction.
            StepAction::Init | StepAction::Forward => {
                if let (Some(vals), Some(t)) = (matrix.get(x), output_texts.get(x))
                    && let Some(result) = apply_reduce(&op, vals)
                {
                    t.set_text(&format_value(result, digits));
                }
            }
            // Stepping back un-computes: blank the cell we just left.
            StepAction::Back => {
                if x < old
                    && let Some(t) = output_texts.get(old)
                {
                    t.set_text("");
                }
            }
        }

        for (r, rect) in output_rects.iter().enumerate() {
            let _ = rect.set_fill(if r == x { HILITE_FILL } else { CELL_FILL });
        }
    });
    go(StepAction::Init);

    // --- step buttons: in the transport bar if available, otherwise below the diagram ---
    let (button_root, back_pt) = match transport {
        Some(t) => (t, Point::new(TRANSPORT_PAD, (TRANSPORT_H - BTN_H) / 2.0)),
        None => (svg, Point::new(grid_origin.x, output_y + 2.0 * cell_h)),
    };
    let fwd_pt = Point::new(back_pt.x + BTN_W + BTN_GAP, back_pt.y);

    let back = make_button(button_root, back_pt, "\u{2190} Step back")?;
    {
        let go = Rc::clone(&go);
        back.on_click(move |_| go(StepAction::Back))?;
    }
    let fwd = make_button(button_root, fwd_pt, "Step forward \u{2192}")?;
    {
        let go = Rc::clone(&go);
        fwd.on_click(move |_| go(StepAction::Forward))?;
    }

    // Viewport extent of what we drew on the main SVG (buttons in the transport bar don't count).
    let content_right = col_x(cols.max(rows).saturating_sub(1)) + cell_w;
    let (right, bottom) = match transport {
        Some(_) => (content_right, output_y + cell_h),
        None => (content_right.max(fwd_pt.x + BTN_W), fwd_pt.y + BTN_H),
    };
    Ok((vec![back, fwd], Point::new(right, bottom)))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One node of a map body's expression tree, laid out as a grid of cells: leaves (array reads / literals) at row 0,
/// operations stacked below them, the final result at the deepest row.
struct VizNode {
    /// The sub-expression, re-evaluated for each `x` to fill the cell.
    expr: Expr,
    /// Indices of child nodes feeding this one.
    children: Vec<usize>,
    /// Operator glyph for an internal node (`xor`, `rotl_u`, …); `None` for a leaf.
    op_label: Option<String>,
    /// Cross-axis column (fractional for centred internal nodes) and depth row.
    col: f64,
    row: usize,
    /// For an array read `arr[index]`, the index expression — so the source cell can be highlighted.
    read_index: Option<Expr>,
}

/// Recursively builds the expression tree, appending nodes (children before parents) and returning the root's index.
/// Leaves are assigned successive columns; an internal node is centred over its children, one row deeper.
fn build_viz_tree(expr: &Expr, nodes: &mut Vec<VizNode>, next_leaf: &mut f64) -> usize {
    match expr {
        Expr::BinOp { op, lhs, rhs } => {
            let l = build_viz_tree(lhs, nodes, next_leaf);
            let r = build_viz_tree(rhs, nodes, next_leaf);
            let col = (nodes[l].col + nodes[r].col) / 2.0;
            let row = 1 + nodes[l].row.max(nodes[r].row);
            nodes.push(VizNode {
                expr: expr.clone(),
                children: vec![l, r],
                op_label: Some(op.to_string()),
                col,
                row,
                read_index: None,
            });
        }
        Expr::Not(inner) => {
            let c = build_viz_tree(inner, nodes, next_leaf);
            let (col, row) = (nodes[c].col, nodes[c].row + 1);
            nodes.push(VizNode {
                expr: expr.clone(),
                children: vec![c],
                op_label: Some("not".to_string()),
                col,
                row,
                read_index: None,
            });
        }
        other => {
            let read_index = match other {
                Expr::Index { index, .. } => Some((**index).clone()),
                _ => None,
            };
            let col = *next_leaf;
            *next_leaf += 1.0;
            nodes.push(VizNode { expr: expr.clone(), children: vec![], op_label: None, col, row: 0, read_index });
        }
    }
    nodes.len() - 1
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Visualises a map comprehension below its input grid: for the selected `x`, the body's expression tree is drawn as
/// live value cells (reads highlighted in the input row), and the result fills the output-state row. The transport
/// buttons step `x` across the comprehension's range.
#[allow(clippy::too_many_arguments)]
fn render_map(
    svg: &SvgRoot,
    transport: Option<&SvgRoot>,
    spec: &GridSpec,
    matrix: &[Vec<u64>],
    map: &MapInfo,
    grid_origin: Point,
    grid_bottom: f64,
    input_cells: Vec<Vec<SvgNode>>,
    graph: &ValidatedGraph,
) -> Result<(Vec<SvgNode>, Point), Error> {
    let (cell_w, cell_h, gap) = (spec.cell_w, spec.cell_h, spec.cell_gap);
    let digits = spec.digits;
    let range = map.range;
    let start = range.0;
    let input_values: Vec<u64> = matrix.first().cloned().unwrap_or_default();

    // Build and place the body's expression tree.
    let mut nodes: Vec<VizNode> = Vec::new();
    let mut next_leaf = 0.0_f64;
    let root = build_viz_tree(&map.body, &mut nodes, &mut next_leaf);
    let depth = nodes.iter().map(|n| n.row).max().unwrap_or(0);

    // Each node is a labelled box: a label band (the read expression for a leaf, or the operator for an internal
    // node) above a value cell. Internal nodes get the coloured operation card, matching the expanded-Maj tutorial.
    let label_h = spec.label_h;
    let node_h = label_h + cell_h;
    let row_h = node_h + cell_h; // a clear band below each node for the connector wires
    let content_top = grid_bottom + cell_h;
    let node_x = |col: f64| grid_origin.x + col * (cell_w + gap);
    let node_top = |row: usize| content_top + row as f64 * row_h;
    let value_top = |row: usize| node_top(row) + label_h;
    let node_bottom = |row: usize| node_top(row) + node_h;

    // Connector wires (drawn first, behind the cells): each child elbows down into its parent's top.
    for node in &nodes {
        let px = node_x(node.col) + cell_w / 2.0;
        let py = node_top(node.row);
        for &c in &node.children {
            let child = &nodes[c];
            let cx = node_x(child.col) + cell_w / 2.0;
            let cy = node_bottom(child.row);
            let mid = (cy + py) / 2.0;
            wire(
                svg,
                &[Point::new(cx, cy), Point::new(cx, mid), Point::new(px, mid), Point::new(px, py)],
            )?;
        }
    }

    // Precompute every node's value, and which input cells each step reads, for each x in the range.
    let xs: Vec<usize> = (range.0..range.1).collect();
    let eval_at = |expr: &Expr, x: usize| {
        eval::eval_scalar_with(expr, &map.var, x as u64, &map.array_param, &input_values, graph)
    };
    let node_values: Vec<Vec<Option<u64>>> =
        xs.iter().map(|&x| nodes.iter().map(|n| eval_at(&n.expr, x)).collect()).collect();
    let reads: Vec<Vec<usize>> = xs
        .iter()
        .map(|&x| {
            nodes
                .iter()
                .filter_map(|n| n.read_index.as_ref().and_then(|idx| eval_at(idx, x)).map(|i| i as usize))
                .collect()
        })
        .collect();

    // Draw each node as a labelled box; the returned text node is updated as `x` steps.
    let mut node_texts: Vec<SvgNode> = Vec::with_capacity(nodes.len());
    for (i, node) in nodes.iter().enumerate() {
        let top = Point::new(node_x(node.col), node_top(node.row));
        let value_pos = Point::new(top.x, value_top(node.row));
        let content = node_values
            .first()
            .and_then(|vals| vals.get(i).copied().flatten())
            .map(|v| format_value(v, digits))
            .unwrap_or_default();

        // The label band: an operator glyph (internal node), or the read expression (array-read leaf).
        let band_label = match &node.op_label {
            Some(op) => Some(op.clone()),
            None if node.read_index.is_some() => Some(expr_label(&node.expr, "", 0)),
            None => None,
        };

        if node.op_label.is_some() {
            // Coloured operation card framing the label band + value cell.
            let card = svg.rect(top, Size::new(cell_w, node_h))?;
            card.set_fill(fill_for(&NodeKind::Operation))?;
            card.set_stroke("black")?;
            card.set_stroke_width(1.5)?;
            card.set_attr("rx", "6")?;
        }

        if let Some(label) = &band_label {
            let lbl = svg.text(Point::new(top.x + cell_w / 2.0, top.y + label_h * GRID_LABEL_BASELINE), label)?;
            lbl.set_fill(PALE_BLUE_GREY)?;
            lbl.set_attr("text-anchor", "middle")?;
            lbl.set_attr("font-family", if node.op_label.is_some() { "sans-serif" } else { CELL_FONT_FAMILY })?;
            lbl.set_attr("font-size", if node.op_label.is_some() { "12" } else { "11" })?;
            if node.op_label.is_some() {
                lbl.set_attr("font-weight", "600")?;
            }
        }

        let (_, text) = draw_value_cell(svg, value_pos, Size::new(cell_w, cell_h), &content)?;
        node_texts.push(text);
    }

    // Output-state row: D[x] for each x, filling at column x as you step.
    let output_y = node_top(depth) + row_h;
    let mut output_rects: Vec<SvgNode> = Vec::with_capacity(xs.len());
    let mut output_texts: Vec<SvgNode> = Vec::with_capacity(xs.len());
    for &x in &xs {
        let (rect, text) =
            draw_value_cell(svg, Point::new(node_x(x as f64), output_y), Size::new(cell_w, cell_h), "")?;
        output_rects.push(rect);
        output_texts.push(text);
    }

    // Shared navigation: forward/init fills D[x]; back blanks the cell just left.
    let input_row: Vec<SvgNode> = input_cells.into_iter().next().unwrap_or_default();
    let current = Cell::new(start);

    let go: Rc<dyn Fn(StepAction)> = Rc::new(move |action: StepAction| {
        let old = current.get();
        let x = match action {
            StepAction::Init => old,
            StepAction::Forward => step_forward(old, range),
            StepAction::Back => step_back(old, range),
        };
        current.set(x);
        let xi = x - start;

        if let Some(vals) = node_values.get(xi) {
            for (text, value) in node_texts.iter().zip(vals) {
                text.set_text(&value.map(|v| format_value(v, digits)).unwrap_or_default());
            }
        }

        for cell in &input_row {
            let _ = cell.set_fill(CELL_FILL);
        }
        if let Some(read) = reads.get(xi) {
            for &idx in read {
                if let Some(cell) = input_row.get(idx) {
                    let _ = cell.set_fill(HILITE_FILL);
                }
            }
        }

        match action {
            StepAction::Init | StepAction::Forward => {
                if let (Some(vals), Some(text)) = (node_values.get(xi), output_texts.get(xi))
                    && let Some(d) = vals.get(root).copied().flatten()
                {
                    text.set_text(&format_value(d, digits));
                }
            }
            StepAction::Back => {
                if x < old
                    && let Some(text) = output_texts.get(old - start)
                {
                    text.set_text("");
                }
            }
        }

        for (i, rect) in output_rects.iter().enumerate() {
            let _ = rect.set_fill(if i == xi { HILITE_FILL } else { CELL_FILL });
        }
    });
    go(StepAction::Init);

    // Step buttons: in the transport bar if available, otherwise below the diagram.
    let (button_root, back_pt) = match transport {
        Some(t) => (t, Point::new(TRANSPORT_PAD, (TRANSPORT_H - BTN_H) / 2.0)),
        None => (svg, Point::new(grid_origin.x, output_y + 2.0 * cell_h)),
    };
    let fwd_pt = Point::new(back_pt.x + BTN_W + BTN_GAP, back_pt.y);

    let back = make_button(button_root, back_pt, "\u{2190} Step back")?;
    {
        let go = Rc::clone(&go);
        back.on_click(move |_| go(StepAction::Back))?;
    }
    let fwd = make_button(button_root, fwd_pt, "Step forward \u{2192}")?;
    {
        let go = Rc::clone(&go);
        fwd.on_click(move |_| go(StepAction::Forward))?;
    }

    let max_col = nodes
        .iter()
        .map(|n| n.col)
        .fold(0.0_f64, f64::max)
        .max(range.1.saturating_sub(1) as f64);
    let content_right = node_x(max_col) + cell_w;
    let (right, bottom) = match transport {
        Some(_) => (content_right, output_y + cell_h),
        None => (content_right.max(fwd_pt.x + BTN_W), fwd_pt.y + BTN_H),
    };
    Ok((vec![back, fwd], Point::new(right, bottom)))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Visualises a nested map below its matrix input grid. For the selected outer row `x`, the row `a[x]` is folded
/// element-wise with the single broadcast value `d[x]` (e.g. `a[x][y] XOR d[x]`), and the resulting row is written
/// into a building-up output grid.
///
/// Layout (all aligned to the grid's columns):
///   * a **working row** — a live copy of the selected matrix row `a[x]`, with the broadcast cell `d[x]` beside it;
///   * an **operation row** — one `op` box per lane; each lane drops in from above and `d[x]` arrives off a broadcast
///     bus from the side;
///   * a **result row** — this iteration's output lanes, directly below the boxes; and
///   * an **output grid** — `rows × cols`. Stepping forward writes row `x`; stepping back blanks the row just left, so
///     the grid reflects only the rows reached so far.
///
/// The Step buttons step the outer index `x` across the comprehension's range, filling the output one row at a time.
#[allow(clippy::too_many_arguments)]
fn render_nested_map(
    svg: &SvgRoot,
    transport: Option<&SvgRoot>,
    spec: &GridSpec,
    matrix: &[Vec<u64>],
    info: &NestedMapInfo,
    grid_origin: Point,
    grid_bottom: f64,
    input_cells: Vec<Vec<SvgNode>>,
    vec_cells: Option<Vec<Vec<SvgNode>>>,
    graph: &ValidatedGraph,
) -> Result<(Vec<SvgNode>, Point), Error> {
    let (cell_w, cell_h, gap) = (spec.cell_w, spec.cell_h, spec.cell_gap);
    let (cols, rows, digits) = (spec.cols, spec.rows, spec.digits);
    let range = info.outer_range;
    let start = range.0;

    let col_x = |c: f64| grid_origin.x + c * (cell_w + gap);

    // Input data: the broadcast vector and the precomputed output matrix.
    let vector: Vec<u64> =
        graph.nodes.get(&info.vec_node).and_then(|d| eval::node_array(d, graph)).unwrap_or_default();
    let output: Vec<Vec<u64>> =
        graph.nodes.get(&info.op_node).and_then(|o| eval::node_matrix(o, graph)).unwrap_or_default();

    // Layout bands below the grid (mirrors the reduction viz).
    let content_top = grid_bottom + cell_h;
    let label_baseline = content_top + cell_h * 0.7;
    let working_y = content_top + cell_h;
    let working_bottom = working_y + cell_h;
    let op_y = working_y + 2.0 * cell_h;
    let op_mid = op_y + cell_h / 2.0;
    let result_y = op_y + 2.0 * cell_h;
    let output_y = result_y + 2.0 * cell_h;
    let rail_y = working_bottom + cell_h / 2.0; // broadcast bus, in the clear band above the op row
    let d_col = cols as f64 + 0.5; // the d[x] cell sits a column-gap to the right of the lanes

    // --- labels (a[x] over the working row, d[x] over the broadcast cell) ---
    let mk_label = |pos: Point, text: &str| -> Result<SvgNode, Error> {
        let lbl = svg.text(pos, text)?;
        lbl.set_fill(PALE_BLUE_GREY)?;
        lbl.set_attr("font-family", CELL_FONT_FAMILY)?;
        lbl.set_attr("font-size", "13")?;
        Ok(lbl)
    };
    let working_label =
        mk_label(Point::new(grid_origin.x, label_baseline), &format!("{}[{}]", info.matrix_param, start))?;
    let d_label = mk_label(Point::new(col_x(d_col), label_baseline), &format!("{}[{}]", info.vec_param, start))?;

    // --- working row: a live copy of the selected matrix row a[x] ---
    let start_row = matrix.get(start).cloned().unwrap_or_default();
    let mut working_texts = Vec::with_capacity(cols);
    for c in 0..cols {
        let content = format_value(start_row.get(c).copied().unwrap_or(0), digits);
        let (_, text) =
            draw_value_cell(svg, Point::new(col_x(c as f64), working_y), Size::new(cell_w, cell_h), &content)?;
        working_texts.push(text);
    }

    // --- broadcast cell d[x] ---
    let (_, d_text) = draw_value_cell(
        svg,
        Point::new(col_x(d_col), working_y),
        Size::new(cell_w, cell_h),
        &format_value(vector.get(start).copied().unwrap_or(0), digits),
    )?;

    // --- operation row: one `op` box per lane ---
    for c in 0..cols {
        let box_ = svg.rect(Point::new(col_x(c as f64), op_y), Size::new(cell_w, cell_h))?;
        box_.set_fill(DEEP_SLATE_BLUE)?;
        box_.set_stroke(HILITE_FILL)?;
        box_.set_stroke_width(1.0)?;
        box_.set_attr("rx", "4")?;

        let label = svg.text(Point::new(col_x(c as f64) + cell_w / 2.0, op_mid), &info.op_label)?;
        label.set_fill(PALE_BLUE_GREY)?;
        label.set_attr("text-anchor", "middle")?;
        label.set_attr("dominant-baseline", "central")?;
        label.set_attr("font-family", "sans-serif")?;
        label.set_attr("font-size", "12")?;
    }

    // --- wires: lane drops, the broadcast bus, and result drops ---
    for c in 0..cols {
        let cx = col_x(c as f64) + cell_w / 2.0;
        // Each lane drops straight into its box top.
        wire(svg, &[Point::new(cx, working_bottom), Point::new(cx, op_y)])?;
        // The broadcast value enters each box from its left edge, off the bus.
        let lx = col_x(c as f64);
        wire(svg, &[Point::new(lx, rail_y), Point::new(lx, op_mid)])?;
        // Each box drops into its result cell.
        wire(svg, &[Point::new(cx, op_y + cell_h), Point::new(cx, result_y)])?;
    }
    // The broadcast bus: from d[x] down to the rail, then left across all the boxes.
    let d_cx = col_x(d_col) + cell_w / 2.0;
    wire(svg, &[Point::new(d_cx, working_bottom), Point::new(d_cx, rail_y), Point::new(col_x(0.0), rail_y)])?;

    // --- result row: this iteration's output lanes ---
    let mut result_texts = Vec::with_capacity(cols);
    for c in 0..cols {
        let content =
            output.get(start).and_then(|r| r.get(c)).map(|&v| format_value(v, digits)).unwrap_or_default();
        let (_, text) =
            draw_value_cell(svg, Point::new(col_x(c as f64), result_y), Size::new(cell_w, cell_h), &content)?;
        result_texts.push(text);
    }

    // --- output grid: rows × cols, filling one row per step ---
    let mut output_rects: Vec<Vec<SvgNode>> = Vec::with_capacity(rows);
    let mut output_texts: Vec<Vec<SvgNode>> = Vec::with_capacity(rows);
    for r in 0..rows {
        let mut rect_row = Vec::with_capacity(cols);
        let mut text_row = Vec::with_capacity(cols);
        for c in 0..cols {
            let y = output_y + r as f64 * (cell_h + gap);
            let (rect, text) =
                draw_value_cell(svg, Point::new(col_x(c as f64), y), Size::new(cell_w, cell_h), "")?;
            rect_row.push(rect);
            text_row.push(text);
        }
        output_rects.push(rect_row);
        output_texts.push(text_row);
    }

    // --- shared navigation: forward writes output row x; back blanks the row just left ---
    let matrix = matrix.to_vec();
    let (matrix_param, vec_param) = (info.matrix_param.clone(), info.vec_param.clone());
    let current = Cell::new(start);

    let go: Rc<dyn Fn(StepAction)> = Rc::new(move |action: StepAction| {
        let old = current.get();
        let x = match action {
            StepAction::Init => old,
            StepAction::Forward => step_forward(old, range),
            StepAction::Back => step_back(old, range),
        };
        current.set(x);

        highlight_row(&input_cells[..], x);
        // Highlight the selected element d[x] in the original broadcast-vector grid (a 1×N row).
        if let Some(row) = vec_cells.as_ref().and_then(|c| c.first()) {
            for (i, cell) in row.iter().enumerate() {
                let _ = cell.set_fill(if i == x { HILITE_FILL } else { CELL_FILL });
            }
        }

        if let Some(row) = matrix.get(x) {
            for (c, t) in working_texts.iter().enumerate() {
                t.set_text(&format_value(row.get(c).copied().unwrap_or(0), digits));
            }
        }
        d_text.set_text(&format_value(vector.get(x).copied().unwrap_or(0), digits));
        for (c, t) in result_texts.iter().enumerate() {
            t.set_text(
                &output.get(x).and_then(|r| r.get(c)).map(|&v| format_value(v, digits)).unwrap_or_default(),
            );
        }
        working_label.set_text(&format!("{matrix_param}[{x}]"));
        d_label.set_text(&format!("{vec_param}[{x}]"));

        match action {
            // Arriving at a row (forward, or the initial display) writes it into the output grid.
            StepAction::Init | StepAction::Forward => {
                if let (Some(vals), Some(texts)) = (output.get(x), output_texts.get(x)) {
                    for (t, v) in texts.iter().zip(vals) {
                        t.set_text(&format_value(*v, digits));
                    }
                }
            }
            // Stepping back un-computes: blank the row we just left.
            StepAction::Back => {
                if x < old
                    && let Some(texts) = output_texts.get(old)
                {
                    for t in texts {
                        t.set_text("");
                    }
                }
            }
        }

        for (r, row) in output_rects.iter().enumerate() {
            let fill = if r == x { HILITE_FILL } else { CELL_FILL };
            for rect in row {
                let _ = rect.set_fill(fill);
            }
        }
    });
    go(StepAction::Init);

    // --- step buttons: in the transport bar if available, otherwise below the diagram ---
    let output_bottom = output_y + rows as f64 * (cell_h + gap);
    let (button_root, back_pt) = match transport {
        Some(t) => (t, Point::new(TRANSPORT_PAD, (TRANSPORT_H - BTN_H) / 2.0)),
        None => (svg, Point::new(grid_origin.x, output_bottom + cell_h)),
    };
    let fwd_pt = Point::new(back_pt.x + BTN_W + BTN_GAP, back_pt.y);

    let back = make_button(button_root, back_pt, "\u{2190} Step back")?;
    {
        let go = Rc::clone(&go);
        back.on_click(move |_| go(StepAction::Back))?;
    }
    let fwd = make_button(button_root, fwd_pt, "Step forward \u{2192}")?;
    {
        let go = Rc::clone(&go);
        fwd.on_click(move |_| go(StepAction::Forward))?;
    }

    let content_right = col_x(d_col) + cell_w;
    let (right, bottom) = match transport {
        Some(_) => (content_right, output_bottom),
        None => (content_right.max(fwd_pt.x + BTN_W), fwd_pt.y + BTN_H),
    };
    Ok((vec![back, fwd], Point::new(right, bottom)))
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
