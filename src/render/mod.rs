mod layout;
pub(crate) mod rect;

use std::{cell::Cell, collections::HashMap, rc::Rc};
use svg_dom::{
    Error, SvgNode, SvgRoot,
    root::utils::{Point, Size},
};

use crate::{
    ast::{
        ebnf_04::Type,
        ebnf_06::{NodeDecl, NodeKind, PropValue},
        ebnf_07::{WireDecl, WireEndpoint},
        ebnf_11::Expr,
    },
    graph::ValidatedGraph,
};
use layout::{
    MARGIN, NODE_H, NODE_W, downstream, entry_point, exit_point, layout, layout_sized, upstream,
};
use rect::Rect;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Array-grid visualisation constants (user units / CSS colours)
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const GRID_LABEL_H_CH: f64 = 3.2; // vertical band above the grid for the node label, in ch
const GRID_LABEL_BASELINE: f64 = 0.6; // label baseline within that band, as a fraction of the band height
const BTN_W: f64 = 120.0;
const BTN_H: f64 = 28.0;
const BTN_GAP: f64 = 12.0;

// Cell metrics are expressed as multiples of the monospace advance ("ch"), which is measured at render time (see
// `measure_char`) rather than hard-coded, so spacing tracks the actual font.
const BYTE_GAP_CH: f64 = 0.5; // gap between bytes, in ch (half a character)
const CELL_PAD_CH: f64 = 2.0; // total horizontal padding inside a cell, in ch
const CELL_H_CH: f64 = 3.5; // cell height, in ch
const CELL_GAP_CH: f64 = 1.0; // whitespace between adjacent cells (rows and columns), in ch
const FALLBACK_CH: f64 = 7.5; // used only if the browser cannot measure (e.g. the SVG is not yet laid out)

const CELL_FONT_FAMILY: &str = "ui-monospace, monospace";
const CELL_FONT_SIZE: &str = "12";

const CELL_FILL: &str = "#e6ecf5";
const CELL_TEXT: &str = "#0f1420";
const CELL_STROKE: &str = "#2a3650";
const HILITE_FILL: &str = "#6db3f2"; // background of the row currently being processed

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
    /// Renderer-injected interactive controls (e.g. the array step buttons) that are not declared in the DSL.
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
        .keys()
        .map(|name| {
            let size = grids
                .get(name)
                .map(grid_size)
                .unwrap_or_else(|| Size::new(NODE_W, NODE_H));
            (name.clone(), size)
        })
        .collect();

    let placement = layout_sized(graph, &sizes);

    let mut wires = Vec::with_capacity(graph.wires.len());
    for wire in &graph.wires {
        if let Some(line) = render_wire(svg, graph, &placement, wire)? {
            wires.push(line);
        }
    }

    let mut nodes = HashMap::with_capacity(graph.nodes.len());
    let mut controls = Vec::new();
    // Track the bottom-right of everything actually drawn: grids and their step buttons can extend well beyond the
    // node box, so we fit the viewport to the real content at the end.
    let (mut max_x, mut max_y) = (0.0_f64, 0.0_f64);

    for (name, rect) in &placement {
        let decl = &graph.nodes[name];
        max_x = max_x.max(rect.top_left.x + rect.size.width);
        max_y = max_y.max(rect.top_left.y + rect.size.height);

        if let Some(spec) = grids.get(name) {
            let origin: Point = (*rect).into();
            let (group, cells) = render_array_node(svg, decl, origin, spec)?;

            let btn_origin = Point::new(origin.x, origin.y + rect.size.height + BTN_GAP);
            controls.extend(render_step_controls(svg, btn_origin, spec.rows, cells)?);

            nodes.insert(name.clone(), group);
            max_y = max_y.max(btn_origin.y + BTN_H);
        } else {
            nodes.insert(name.clone(), render_node(svg, decl, *rect)?);
        }
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
// Array-grid visualisation
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

/// Infers a 2D `(rows, cols)` shape for `name` from the function it feeds.
///
/// A node is drawn as a grid when it is the source of a wire into an `operation` node whose `symbol` names a function
/// whose first parameter is a 2D array type, e.g. `[[u8; 5]; 5]`. The DSL carries no node data yet, so this is the only
/// way the renderer learns a node's shape. Returns `None` for ordinary scalar nodes.
fn inferred_grid_shape(name: &str, graph: &ValidatedGraph) -> Option<(usize, usize)> {
    graph.wires.iter().find_map(|wire| {
        let (WireEndpoint::Node(src), WireEndpoint::Node(dst)) = (&wire.source, &wire.target) else {
            return None;
        };
        if src != name {
            return None;
        }

        let symbol = string_prop(graph.nodes.get(dst)?, "symbol")?;
        let first_param = graph.fn_defs.get(&symbol)?.params.first()?;

        match &first_param.ty {
            Type::Array { element, len: rows } => match element.as_ref() {
                Type::Array { len: cols, .. } => Some((*rows, *cols)),
                _ => None,
            },
            _ => None,
        }
    })
}

/// How a node should be drawn as a table: its dimensions plus the per-cell width and hex-digit count derived from the
/// node's `format` specifier.
struct GridSpec {
    rows: usize,
    cols: usize,
    digits: usize,
    /// All in user units, derived from the measured monospace advance `ch`.
    cell_w: f64,
    cell_h: f64,
    cell_gap: f64,
    label_h: f64,
}

/// Resolves the grid spec for a node, or `None` if it is not an array node. `ch` is the measured monospace advance;
/// every metric (cell width/height and inter-cell gap) is a multiple of it.
fn grid_spec(name: &str, decl: &NodeDecl, graph: &ValidatedGraph, ch: f64) -> Option<GridSpec> {
    let (rows, cols) = inferred_grid_shape(name, graph)?;
    let digits = format_digits(decl);

    Some(GridSpec {
        rows,
        cols,
        digits,
        cell_w: cell_width(digits, ch),
        cell_h: CELL_H_CH * ch,
        cell_gap: CELL_GAP_CH * ch,
        label_h: GRID_LABEL_H_CH * ch,
    })
}

/// Measures the monospace advance ("ch") in user units for the cell font, by probing a single "0" glyph.
///
/// SVG geometry attributes (a cell's `width`, element `x`/`y`) take user-unit numbers and cannot use font-relative CSS
/// units like `ch`, so the value must be known numerically — we discover it from the rendered font rather than guess.
/// Falls back to [`FALLBACK_CH`] if the browser cannot measure (e.g. the SVG is not yet laid out, or off the wasm
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

    // The probe has served its purpose; keep it out of view.
    let _ = probe.set_attr("visibility", "hidden");
    ch
}

/// The drawn footprint of a grid: cells plus inter-cell gaps, with the label band above.
fn grid_size(spec: &GridSpec) -> Size {
    let cols = spec.cols as f64;
    let rows = spec.rows as f64;
    Size::new(
        cols * spec.cell_w + (cols - 1.0).max(0.0) * spec.cell_gap,
        spec.label_h + rows * spec.cell_h + (rows - 1.0).max(0.0) * spec.cell_gap,
    )
}

/// Cell width (user units) for a value of `digits` hex digits, given the measured monospace advance `ch`.
///
/// Budgets one `ch` per hex digit, [`BYTE_GAP_CH`] per inter-byte gap (see [`format_cell`]), and [`CELL_PAD_CH`] of
/// total padding — all as multiples of the real font metric.
fn cell_width(digits: usize, ch: f64) -> f64 {
    let glyphs = digits as f64 + byte_separators(digits) as f64 * BYTE_GAP_CH + CELL_PAD_CH;
    glyphs * ch
}

/// Number of inter-byte gaps in a `digits`-digit value: one fewer than the byte count (0 for a single-byte hex8 value).
fn byte_separators(digits: usize) -> usize {
    (digits / 2).saturating_sub(1)
}

/// Number of hex digits the node's `format` displays. Defaults to 2 (hex8) when absent or unrecognised.
fn format_digits(decl: &NodeDecl) -> usize {
    match ident_prop(decl, "format").as_deref() {
        Some("hex16") => 4,
        Some("hex32") => 8,
        Some("hex64") => 16,
        _ => 2,
    }
}

/// Placeholder cell value: a deterministic spread so the full width is visibly used, masked to the format's bit width,
/// and grouped into bytes (pairs of hex digits) separated by a small gap. A single-byte hex8 value has no gap.
fn format_cell(index: usize, digits: usize) -> String {
    let spread = (index as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    let bits = digits * 4;
    let value = if bits >= 64 {
        spread
    } else {
        spread & ((1u64 << bits) - 1)
    };

    let hex = format!("{value:0digits$x}");
    hex.chars()
        .collect::<Vec<_>>()
        .chunks(2)
        .map(|byte| byte.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(" ")
}

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

/// Draws the table described by `spec` anchored at `origin`, with the node label above it.
///
/// Cell values are placeholders since the DSL cannot yet carry node data. Returns the node group and the cell handles
/// grouped by row, so the caller can re-colour a row to highlight it.
fn render_array_node(
    svg: &SvgRoot,
    decl: &NodeDecl,
    origin: Point,
    spec: &GridSpec,
) -> Result<(SvgNode, Vec<Vec<SvgNode>>), Error> {
    let group = svg.group()?;
    group.set_attr("data-node", &decl.name)?;

    let title = svg.text(
        Point::new(origin.x, origin.y + spec.label_h * GRID_LABEL_BASELINE),
        &node_label(decl),
    )?;
    title.set_fill("#e6ecf5")?;
    title.set_attr("font-family", "sans-serif")?;
    title.set_attr("font-size", "13")?;
    title.set_attr("font-weight", "600")?;
    group.append(&title)?;

    let grid_top = origin.y + spec.label_h;
    let mut cells: Vec<Vec<SvgNode>> = Vec::with_capacity(spec.rows);

    for r in 0..spec.rows {
        let mut row_cells = Vec::with_capacity(spec.cols);

        for c in 0..spec.cols {
            let x = origin.x + c as f64 * (spec.cell_w + spec.cell_gap);
            let y = grid_top + r as f64 * (spec.cell_h + spec.cell_gap);

            let cell = svg.rect(Point::new(x, y), Size::new(spec.cell_w, spec.cell_h))?;
            cell.set_fill(CELL_FILL)?;
            cell.set_stroke(CELL_STROKE)?;
            cell.set_stroke_width(1.0)?;
            group.append(&cell)?;

            let text = svg.text(
                Point::new(x + spec.cell_w / 2.0, y + spec.cell_h / 2.0),
                &format_cell(r * spec.cols + c, spec.digits),
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

/// Sets every cell's background to the default, then paints the `active` row with the highlight colour.
fn highlight_row(cells: &[Vec<SvgNode>], active: usize) {
    for (r, row) in cells.iter().enumerate() {
        let fill = if r == active { HILITE_FILL } else { CELL_FILL };
        for cell in row {
            let _ = cell.set_fill(fill);
        }
    }
}

/// Draws the "step back / step forward" buttons and wires their clicks to the row highlight.
///
/// The shared `current` row index and the cell handles are captured by the click closures (which live inside the
/// returned button nodes), so the buttons must be kept alive for the controls to keep working.
fn render_step_controls(
    svg: &SvgRoot,
    origin: Point,
    rows: usize,
    cells: Vec<Vec<SvgNode>>,
) -> Result<Vec<SvgNode>, Error> {
    let cells = Rc::new(cells);
    let current = Rc::new(Cell::new(0usize));

    // Start with the first row highlighted.
    highlight_row(&cells[..], 0);

    let back = make_button(svg, origin, "\u{2190} Step back")?;
    {
        let cells = Rc::clone(&cells);
        let current = Rc::clone(&current);
        back.on_click(move |_| {
            let r = (current.get() + rows - 1) % rows;
            current.set(r);
            highlight_row(&cells[..], r);
        })?;
    }

    let fwd = make_button(
        svg,
        Point::new(origin.x + BTN_W + BTN_GAP, origin.y),
        "Step forward \u{2192}",
    )?;
    {
        let cells = Rc::clone(&cells);
        let current = Rc::clone(&current);
        fwd.on_click(move |_| {
            let r = (current.get() + 1) % rows;
            current.set(r);
            highlight_row(&cells[..], r);
        })?;
    }

    Ok(vec![back, fwd])
}

/// A clickable labelled button: a rounded rect with centred text, returned as a `<g>` to attach the handler to.
fn make_button(svg: &SvgRoot, origin: Point, label: &str) -> Result<SvgNode, Error> {
    let group = svg.group()?;
    group.set_attr("style", "cursor: pointer")?;

    let rect = svg.rect(origin, Size::new(BTN_W, BTN_H))?;
    rect.set_fill("#2a3650")?;
    rect.set_stroke("#6db3f2")?;
    rect.set_stroke_width(1.0)?;
    rect.set_attr("rx", "6")?;
    group.append(&rect)?;

    let text = svg.text(
        Point::new(origin.x + BTN_W / 2.0, origin.y + BTN_H / 2.0),
        label,
    )?;
    text.set_fill("#e6ecf5")?;
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
