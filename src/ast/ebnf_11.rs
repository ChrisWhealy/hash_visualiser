// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §11 Expressions
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone)]
pub enum Expr {
    Integer(u64),
    HexLit(u64),
    Ident(String),
    Call { name: String, args: Vec<Expr> },
    BinOp { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    Not(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinOp {
    Or, Xor, And,
    Add, Sub,
    Shl, ShrU, ShrS,
    RotrU, RotrS, RotlU, RotlS,
}
