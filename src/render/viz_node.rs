use crate::ast::ebnf_11::Expr;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One node of a map body's expression tree, laid out as a grid of cells: leaves (array reads / literals) at row 0,
/// operations stacked below them, the final result at the deepest row.
pub(crate) struct VizNode {
    /// The sub-expression, re-evaluated for each `x` to fill the cell.
    pub expr: Expr,
    /// Indices of child nodes feeding this one.
    pub children: Vec<usize>,
    /// Operator glyph for an internal node (`xor`, `rotl_u`, …); `None` for a leaf.
    pub op_label: Option<String>,
    /// Cross-axis column (fractional for centred internal nodes) and depth row.
    pub col: f64,
    pub row: usize,
    /// For an array read `arr[index]`, the index expression — so the source cell can be highlighted.
    pub read_index: Option<Expr>,
}

/// Recursively builds the expression tree, appending nodes (children before parents) and returning the root's index.
/// Leaves are assigned successive columns; an internal node is centred over its children, one row deeper.
pub(crate) fn build_viz_tree(expr: &Expr, nodes: &mut Vec<VizNode>, next_leaf: &mut f64) -> usize {
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
            nodes.push(VizNode {
                expr: expr.clone(),
                children: vec![],
                op_label: None,
                col,
                row: 0,
                read_index,
            });
        }
    }
    nodes.len() - 1
}
