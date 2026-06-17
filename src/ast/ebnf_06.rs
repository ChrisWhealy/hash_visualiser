use crate::ast::ebnf_11::Expr;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §6 Node declarations
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone)]
pub struct NodeDecl {
    pub name: String,
    pub kind: NodeKind,
    pub properties: Vec<Property>,
}

#[derive(Debug, Clone)]
pub enum NodeKind {
    Register,
    Operation,
    Constant,
    Button,
    User(String),
}

#[derive(Debug, Clone)]
pub struct Property {
    pub name: String,
    pub value: PropValue,
}

#[derive(Debug, Clone)]
pub enum PropValue {
    Str(String),
    Expr(Expr),
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A named data binding: `data <name> = <value>`, where `value` is a literal (typically a nested array literal).
///
/// A node references a binding by name through its `source` property, and the bound value can be passed to a function
/// (e.g. an operation node's `compute: ThetaC(state)`).
#[derive(Debug, Clone)]
pub struct DataDecl {
    pub name: String,
    pub value: Expr,
}
