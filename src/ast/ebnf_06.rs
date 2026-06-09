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
