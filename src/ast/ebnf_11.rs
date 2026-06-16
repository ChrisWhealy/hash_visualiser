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

    // §11.1 Array expressions
    /// Array indexing: `base[index]`. Chains for multi-dimensional access, e.g. `state[x][y]`.
    Index { base: Box<Expr>, index: Box<Expr> },

    /// Array comprehension `[ for <var> in <start>..<end> => <body> ]`.
    ///
    /// Binds `var` to each integer in the half-open range `start..end` and collects `body` into a fixed-size array of
    /// length `end - start`.
    Comprehension {
        var: String,
        start: u64,
        end: u64,
        body: Box<Expr>,
    },

    /// Reduction `reduce <op> over <array>`: folds an array to a scalar with an associative binary operator.
    Reduce { op: BinOp, array: Box<Expr> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinOp {
    Or, Xor, And,
    Add, Sub,
    Shl, ShrU, ShrS,
    RotrU, RotrS, RotlU, RotlS,
}
