use std::{cell::Cell, rc::Rc};
use svg_dom::{
    Error, SvgNode, SvgRoot,
    root::utils::{Point, Size},
};

use super::{
    BTN_GAP, BTN_H, BTN_W, CELL_FILL, CELL_FONT_FAMILY, DEEP_SLATE_BLUE, HILITE_FILL,
    OP_BOX_FONT_FAMILY, OP_BOX_FONT_SIZE, OP_BOX_PAD, PALE_BLUE_GREY, StepAction, TRANSPORT_H,
    TRANSPORT_PAD, draw_value_cell, expr_label, format_value, grid_spec::GridSpec, highlight_row,
    make_button, measure_text, step_back, step_forward, wire,
};
use crate::ast::ebnf_11::{BinOp, Expr};

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
pub(crate) fn render_reduction(
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
    let working_label = svg.text(
        Point::new(grid_origin.x, label_baseline),
        &label_text(&label_source, start),
    )?;
    working_label.set_fill(PALE_BLUE_GREY)?;
    working_label.set_attr("font-family", CELL_FONT_FAMILY)?;
    working_label.set_attr("font-size", "13")?;

    // --- working row: a live copy of the selected row ---
    let start_vals = matrix.get(start).cloned().unwrap_or_default();
    let mut working_texts = Vec::with_capacity(cols);
    for c in 0..cols {
        let content = format_value(start_vals.get(c).copied().unwrap_or(0), digits);
        let (_, text) = draw_value_cell(
            svg,
            Point::new(col_x(c), working_y),
            Size::new(cell_w, cell_h),
            &content,
        )?;
        working_texts.push(text);
    }

    // --- operation row: `op` boxes in columns 1..cols (column 0 is just the first operand). Each box takes its
    // label's natural width (narrower than the column) and is centred, so the carry-chain wires entering/leaving its
    // sides have a visible run rather than hugging the column boundary. ---
    let op_label = op.to_string();
    let op_box_w = (measure_text(svg, &op_label, OP_BOX_FONT_FAMILY, OP_BOX_FONT_SIZE)
        + 2.0 * OP_BOX_PAD)
        .min(cell_w);
    let col_centre = |c: usize| col_x(c) + cell_w / 2.0;
    let box_left_x = |c: usize| col_centre(c) - op_box_w / 2.0;

    for c in 1..cols {
        let box_ = svg.rect(Point::new(box_left_x(c), op_y), Size::new(op_box_w, cell_h))?;
        box_.set_fill(DEEP_SLATE_BLUE)?;
        box_.set_stroke(HILITE_FILL)?;
        box_.set_stroke_width(1.0)?;
        box_.set_attr("rx", "4")?;

        let label = svg.text(Point::new(col_centre(c), op_y + cell_h / 2.0), &op_label)?;
        label.set_fill(PALE_BLUE_GREY)?;
        label.set_attr("text-anchor", "middle")?;
        label.set_attr("dominant-baseline", "central")?;
        label.set_attr("font-family", OP_BOX_FONT_FAMILY)?;
        label.set_attr("font-size", OP_BOX_FONT_SIZE)?;
    }

    // --- fold connector wires (static) ---
    let working_bottom = working_y + cell_h;
    let op_mid = op_y + cell_h / 2.0;
    let op_top = |c: usize| Point::new(col_centre(c), op_y); // "next element" input (box top centre)
    let op_left = |c: usize| Point::new(box_left_x(c), op_mid); // "running result" input (box left edge)
    let op_right = |c: usize| Point::new(box_left_x(c) + op_box_w, op_mid); // box right edge

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
    let (_, result_text) = draw_value_cell(
        svg,
        Point::new(col_x(last_col), result_y),
        Size::new(cell_w, cell_h),
        "",
    )?;
    if cols >= 2 {
        // The final fold box's output drops straight down into the result cell.
        let x_mid = col_x(last_col) + cell_w / 2.0;
        wire(
            svg,
            &[
                Point::new(x_mid, op_y + cell_h),
                Point::new(x_mid, result_y),
            ],
        )?;
    }

    // --- output-state row: c[x] for each outer row, all initially blank ---
    let mut output_rects = Vec::with_capacity(rows);
    let mut output_texts = Vec::with_capacity(rows);
    for r in 0..rows {
        let (rect, text) = draw_value_cell(
            svg,
            Point::new(col_x(r), output_y),
            Size::new(cell_w, cell_h),
            "",
        )?;
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
            result_text.set_text(
                &apply_reduce(&op, vals)
                    .map(|r| format_value(r, digits))
                    .unwrap_or_default(),
            );
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
/// Left-folds `values` with the reduction operator, matching the comprehension semantics (`w0 op w1 op …`).
/// `None` for an empty row or a non-associative operator (the latter is already rejected by the parser).
pub(super) fn apply_reduce(op: &BinOp, values: &[u64]) -> Option<u64> {
    let f: fn(u64, u64) -> u64 = match op {
        BinOp::Xor => |a, b| a ^ b,
        BinOp::And => |a, b| a & b,
        BinOp::Or => |a, b| a | b,
        BinOp::Add => |a, b| a.wrapping_add(b),
        _ => return None,
    };
    values.iter().copied().reduce(f)
}
