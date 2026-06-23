use super::{
    CELL_GAP_CH, CELL_H_CH, GRID_LABEL_H_CH, cell_width, eval, format_digits, import_for_node, inferred_grid_shape,
    is_map_operation, node_data_matrix, step_range,
};
use crate::{ast::ebnf_06::NodeDecl, graph::ValidatedGraph};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How a node should be drawn as a table: its dimensions plus the per-cell width and hex-digit count derived from the
/// node's `format` specifier.
pub(crate) struct GridSpec {
    pub rows: usize,
    pub cols: usize,
    pub digits: usize,
    /// Half-open `[start, end)` range the step buttons walk, from the fed function's comprehension.
    pub step_range: (usize, usize),
    /// The node's declared data (`source: NAME`), if any. When present the grid shows these values; otherwise it
    /// shows placeholders.
    pub values: Option<Vec<Vec<u64>>>,
    /// All in user units, derived from the measured monospace advance `ch`.
    pub cell_w: f64,
    pub cell_h: f64,
    pub cell_gap: f64,
    pub label_h: f64,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Resolves the grid spec for a node, or `None` if it is not an array node. `ch` is the measured monospace advance;
/// every metric (cell width/height and inter-cell gap) is a multiple of it.
///
/// The shape comes from the node's declared data (`source: NAME`) when present, otherwise it is inferred from the type
/// of the function the node feeds.
pub(crate) fn grid_spec(
    name: &str,
    decl: &NodeDecl,
    graph: &ValidatedGraph,
    ch: f64,
) -> Option<GridSpec> {
    // A node whose `compute` applies an IMPORTED function renders as an expandable box (no value grid) — its inner
    // workings live in that file and are reached by clicking to open it in a modal, not drawn inline here.
    if import_for_node(decl, graph).is_some() {
        return None;
    }

    // The operation node that applies a map renders as a plain box; its output is shown by the map visualisation on
    // the input grid, so it must not also draw its own value grid.
    if is_map_operation(decl, graph) {
        return None;
    }

    // Prefer a 2D-array `source` (a grid of values); then a 1-D array (a `[u64; N]` data source or a comprehension
    // result) shown as a single COLUMN — one value per row — so a vector of wide (e.g. hex64) values stays narrow
    // rather than forcing the diagram off-screen horizontally; then a single scalar value shown in a 1×1 cell. So
    // "before"/"after" values are visualised whether they're a word, a vector, or a matrix.
    let values = node_data_matrix(decl, graph)
        .or_else(|| eval::node_matrix(decl, graph))
        .or_else(|| eval::node_array(decl, graph).map(|col| col.into_iter().map(|v| vec![v]).collect()))
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
