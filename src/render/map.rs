use super::{
    BTN_GAP, BTN_H, BTN_W, CELL_FILL, CELL_FONT_FAMILY, DEEP_SLATE_BLUE, GRID_LABEL_BASELINE,
    GridSpec, HILITE_FILL, NodeKind, OP_BOX_FONT_FAMILY, OP_BOX_FONT_SIZE, OP_BOX_PAD,
    PALE_BLUE_GREY, StepAction, TRANSPORT_H, TRANSPORT_PAD, compute_expr, draw_value_cell, eval,
    expr_label, fed_function, fill_for, find_reduction_op, format_value, highlight_row,
    make_button, measure_text, step_back, step_forward, viz_node::*, wire,
};
use crate::{
    ast::{ebnf_06::NodeDecl, ebnf_11::Expr},
    graph::ValidatedGraph,
};
use std::{cell::Cell, rc::Rc};
use svg_dom::{
    Error, SvgNode, SvgRoot,
    root::utils::{Point, Size},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A map comprehension a grid node feeds: `[ for <var> in start..end => <body> ]` whose body is NOT a reduction.
pub(crate) struct MapInfo {
    pub var: String,
    pub body: Expr,
    pub range: (usize, usize),
    /// Name of the array parameter the body indexes into (the fed function's first parameter).
    pub array_param: String,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A nested map a matrix grid node feeds: `fn f(a: [[..];M], d: [..]) = [ for x => [ for y => a[x][y] op d[x] ] ]`
/// — the outer loop walks rows of the matrix `a` (each paired with one broadcast value `d[x]`); the inner loop folds
/// that value into every element of the row. Detected for the *matrix* input only, so the visualiser draws it once.
pub struct NestedMapInfo {
    /// Half-open range the step buttons walk — the outer comprehension's range.
    pub outer_range: (usize, usize),
    /// The matrix and broadcast-vector parameter names (for the `a[x]` / `d[x]` labels).
    pub matrix_param: String,
    pub vec_param: String,
    /// Node supplying the broadcast vector `d`, and the operation node holding the computed output.
    pub vec_node: String,
    pub op_node: String,
    /// Glyph for the per-element operation box (the inner body's operator, e.g. `XOR`).
    pub op_label: String,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A comprehension body the map visualiser can draw as an expression tree: a single (scalar) element, not a nested
/// comprehension or a reduction (those are visualised differently / not yet).
pub fn is_simple_map_body(body: &Expr) -> bool {
    !matches!(body, Expr::Comprehension { .. }) && find_reduction_op(body).is_none()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// If `name` feeds a function that is a simple (1-D, non-reduce) map comprehension, returns its details so the renderer
/// can visualise the per-element computation. Reduce- and nested-comprehensions aren't handled here.
pub fn comprehension_map(name: &str, graph: &ValidatedGraph) -> Option<MapInfo> {
    let func = fed_function(name, graph)?;
    let Expr::Comprehension {
        var,
        start,
        end,
        body,
    } = &func.body
    else {
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// If the matrix grid node `matrix_node` is the first argument of an operation that applies a nested map, returns its
/// details so the renderer can visualise the row-by-row fold. `None` for the broadcast-vector input or any other node.
pub(crate) fn nested_map(matrix_node: &str, graph: &ValidatedGraph) -> Option<NestedMapInfo> {
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
        let Expr::Comprehension {
            start,
            end,
            body: inner,
            ..
        } = &func.body
        else {
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

/// Whether `decl` is the operation node that applies a map comprehension — 1-D (`[ for x => … ]`) or nested
/// (`[ for x => [ for y => … ] ]`). Such a node renders as a plain box: its output is shown by the map visualisation
/// attached to the *input* grid (the per-element computation building up the result), so a separate output grid would
/// just duplicate it.
pub(crate) fn is_map_operation(decl: &NodeDecl, graph: &ValidatedGraph) -> bool {
    let Some(Expr::Call { name, .. }) = compute_expr(decl) else {
        return false;
    };
    graph
        .fn_defs
        .get(name)
        .map(|f| is_map_body(&f.body))
        .unwrap_or(false)
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
/// Visualises a map comprehension below its input grid: for the selected `x`, the body's expression tree is drawn as
/// live value cells (reads highlighted in the input row), and the result fills the output-state row. The transport
/// buttons step `x` across the comprehension's range.
#[allow(clippy::too_many_arguments)]
pub(super) fn render_map(
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
    // The input is a 1-D vector; flatten so it reads the same whether the grid is laid out as a row or a column.
    let input_values: Vec<u64> = matrix.iter().flatten().copied().collect();

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
                &[
                    Point::new(cx, cy),
                    Point::new(cx, mid),
                    Point::new(px, mid),
                    Point::new(px, py),
                ],
            )?;
        }
    }

    // Precompute every node's value, and which input cells each step reads, for each x in the range.
    let xs: Vec<usize> = (range.0..range.1).collect();
    let eval_at = |expr: &Expr, x: usize| {
        eval::eval_scalar_with(
            expr,
            &map.var,
            x as u64,
            &map.array_param,
            &input_values,
            graph,
        )
    };
    let node_values: Vec<Vec<Option<u64>>> = xs
        .iter()
        .map(|&x| nodes.iter().map(|n| eval_at(&n.expr, x)).collect())
        .collect();
    let reads: Vec<Vec<usize>> = xs
        .iter()
        .map(|&x| {
            nodes
                .iter()
                .filter_map(|n| {
                    n.read_index
                        .as_ref()
                        .and_then(|idx| eval_at(idx, x))
                        .map(|i| i as usize)
                })
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
            let lbl = svg.text(
                Point::new(top.x + cell_w / 2.0, top.y + label_h * GRID_LABEL_BASELINE),
                label,
            )?;
            lbl.set_fill(PALE_BLUE_GREY)?;
            lbl.set_attr("text-anchor", "middle")?;
            lbl.set_attr(
                "font-family",
                if node.op_label.is_some() {
                    "sans-serif"
                } else {
                    CELL_FONT_FAMILY
                },
            )?;
            lbl.set_attr(
                "font-size",
                if node.op_label.is_some() { "12" } else { "11" },
            )?;
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
        let (rect, text) = draw_value_cell(
            svg,
            Point::new(node_x(x as f64), output_y),
            Size::new(cell_w, cell_h),
            "",
        )?;
        output_rects.push(rect);
        output_texts.push(text);
    }

    // Shared navigation: forward/init fills D[x]; back blanks the cell just left. The input is a 1-D vector; flatten
    // its cells so read-index highlighting works whether the grid is laid out as a row or a column.
    let input_row: Vec<SvgNode> = input_cells.into_iter().flatten().collect();
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
pub(super) fn render_nested_map(
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
    let vector: Vec<u64> = graph
        .nodes
        .get(&info.vec_node)
        .and_then(|d| eval::node_array(d, graph))
        .unwrap_or_default();
    let output: Vec<Vec<u64>> = graph
        .nodes
        .get(&info.op_node)
        .and_then(|o| eval::node_matrix(o, graph))
        .unwrap_or_default();

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
    let working_label = mk_label(
        Point::new(grid_origin.x, label_baseline),
        &format!("{}[{}]", info.matrix_param, start),
    )?;
    let d_label = mk_label(
        Point::new(col_x(d_col), label_baseline),
        &format!("{}[{}]", info.vec_param, start),
    )?;

    // --- working row: a live copy of the selected matrix row a[x] ---
    let start_row = matrix.get(start).cloned().unwrap_or_default();
    let mut working_texts = Vec::with_capacity(cols);
    for c in 0..cols {
        let content = format_value(start_row.get(c).copied().unwrap_or(0), digits);
        let (_, text) = draw_value_cell(
            svg,
            Point::new(col_x(c as f64), working_y),
            Size::new(cell_w, cell_h),
            &content,
        )?;
        working_texts.push(text);
    }

    // --- broadcast cell d[x] ---
    let (_, d_text) = draw_value_cell(
        svg,
        Point::new(col_x(d_col), working_y),
        Size::new(cell_w, cell_h),
        &format_value(vector.get(start).copied().unwrap_or(0), digits),
    )?;

    // --- operation row: one `op` box per lane, sized to its label (narrower than the column) and centred, so the
    // broadcast wire entering its left edge has a visible run rather than hugging the column boundary. ---
    let op_box_w = (measure_text(svg, &info.op_label, OP_BOX_FONT_FAMILY, OP_BOX_FONT_SIZE)
        + 2.0 * OP_BOX_PAD)
        .min(cell_w);
    let col_centre = |c: usize| col_x(c as f64) + cell_w / 2.0;
    let box_left = |c: usize| col_centre(c) - op_box_w / 2.0;
    for c in 0..cols {
        let box_ = svg.rect(Point::new(box_left(c), op_y), Size::new(op_box_w, cell_h))?;
        box_.set_fill(DEEP_SLATE_BLUE)?;
        box_.set_stroke(HILITE_FILL)?;
        box_.set_stroke_width(1.0)?;
        box_.set_attr("rx", "4")?;

        let label = svg.text(Point::new(col_centre(c), op_mid), &info.op_label)?;
        label.set_fill(PALE_BLUE_GREY)?;
        label.set_attr("text-anchor", "middle")?;
        label.set_attr("dominant-baseline", "central")?;
        label.set_attr("font-family", OP_BOX_FONT_FAMILY)?;
        label.set_attr("font-size", OP_BOX_FONT_SIZE)?;
    }

    // --- wires: lane drops, the broadcast bus, and result drops ---
    for c in 0..cols {
        let cx = col_centre(c);
        // Each lane drops straight into its box top.
        wire(svg, &[Point::new(cx, working_bottom), Point::new(cx, op_y)])?;
        // The broadcast value drops at the column's left edge, then runs horizontally into the box's (inset) left edge.
        let lx = col_x(c as f64);
        wire(
            svg,
            &[
                Point::new(lx, rail_y),
                Point::new(lx, op_mid),
                Point::new(box_left(c), op_mid),
            ],
        )?;
        // Each box drops into its result cell.
        wire(
            svg,
            &[Point::new(cx, op_y + cell_h), Point::new(cx, result_y)],
        )?;
    }
    // The broadcast bus: from d[x] down to the rail, then left across all the boxes.
    let d_cx = col_x(d_col) + cell_w / 2.0;
    wire(
        svg,
        &[
            Point::new(d_cx, working_bottom),
            Point::new(d_cx, rail_y),
            Point::new(col_x(0.0), rail_y),
        ],
    )?;

    // --- result row: this iteration's output lanes ---
    let mut result_texts = Vec::with_capacity(cols);
    for c in 0..cols {
        let content = output
            .get(start)
            .and_then(|r| r.get(c))
            .map(|&v| format_value(v, digits))
            .unwrap_or_default();
        let (_, text) = draw_value_cell(
            svg,
            Point::new(col_x(c as f64), result_y),
            Size::new(cell_w, cell_h),
            &content,
        )?;
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
            let (rect, text) = draw_value_cell(
                svg,
                Point::new(col_x(c as f64), y),
                Size::new(cell_w, cell_h),
                "",
            )?;
            rect_row.push(rect);
            text_row.push(text);
        }
        output_rects.push(rect_row);
        output_texts.push(text_row);
    }

    // --- shared navigation: forward writes output row x; back blanks the row just left ---
    let matrix = matrix.to_vec();
    let (matrix_param, vec_param) = (info.matrix_param.clone(), info.vec_param.clone());
    // The broadcast vector is a 1-D grid; flatten its cells so d[x] highlighting works whether it's a row or a column.
    let vec_row: Vec<SvgNode> = vec_cells.into_iter().flatten().flatten().collect();
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
        // Highlight the selected element d[x] in the original broadcast-vector grid.
        for (i, cell) in vec_row.iter().enumerate() {
            let _ = cell.set_fill(if i == x { HILITE_FILL } else { CELL_FILL });
        }

        if let Some(row) = matrix.get(x) {
            for (c, t) in working_texts.iter().enumerate() {
                t.set_text(&format_value(row.get(c).copied().unwrap_or(0), digits));
            }
        }
        d_text.set_text(&format_value(vector.get(x).copied().unwrap_or(0), digits));
        for (c, t) in result_texts.iter().enumerate() {
            t.set_text(
                &output
                    .get(x)
                    .and_then(|r| r.get(c))
                    .map(|&v| format_value(v, digits))
                    .unwrap_or_default(),
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
