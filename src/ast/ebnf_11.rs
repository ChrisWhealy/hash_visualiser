// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §11 Expressions
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone)]
pub enum Expr {
    Integer(u64),
    HexLit(u64),
    Ident(String),
    Call {
        name: String,
        args: Vec<Expr>,
    },
    BinOp {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Not(Box<Expr>),

    // §11.1 Array expressions
    /// Array indexing: `base[index]`. Chains for multi-dimensional access, e.g. `state[x][y]`.
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },

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
    Reduce {
        op: BinOp,
        array: Box<Expr>,
    },

    /// Array literal `[ e0, e1, … ]`. Nests for multi-dimensional data, e.g. `[[1, 2], [3, 4]]`.
    Array(Vec<Expr>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinOp {
    Or,
    Xor,
    And,
    Add,
    Sub,
    Shl,
    ShrU,
    ShrS,
    RotrU,
    RotrS,
    RotlU,
    RotlS,
}

impl std::fmt::Display for BinOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinOp::Or => write!(f, "or")?,
            BinOp::Xor => write!(f, "xor")?,
            BinOp::And => write!(f, "and")?,
            BinOp::Add => write!(f, "+")?,
            BinOp::Sub => write!(f, "-")?,
            BinOp::Shl => write!(f, "shl")?,
            BinOp::ShrU => write!(f, "shr_u")?,
            BinOp::ShrS => write!(f, "shr_s")?,
            BinOp::RotrU => write!(f, "rotr_u")?,
            BinOp::RotrS => write!(f, "rotr_s")?,
            BinOp::RotlU => write!(f, "rotl_u")?,
            BinOp::RotlS => write!(f, "rotl_s")?,
        }
        Ok(())
    }
}
