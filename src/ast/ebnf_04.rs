use crate::ast::ebnf_11::Expr;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §4 Function definitions
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: String,
    pub params: Vec<Param>,
    /// The declared return type. A function with no `-> type` annotation returns [`Type::Unit`].
    pub return_type: Type,
    pub body: Expr,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A function parameter: a name bound to a type, e.g. `a: [[u8; 5]; 5]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// §4.1 Types — primitive numeric types, fixed-size arrays built from them, and the unit type.
///
/// Examples: `u8`, `u32`, `[u16; 8]`, `[[u8; 5]; 5]`.
///
/// [`Type::Unit`] has no surface syntax: it is the return type of a function declared without a `-> type` annotation,
/// and never appears as a parameter or array-element type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    U8,
    U16,
    U32,
    U64,
    /// A fixed-size array `[element; len]`. Nesting yields multi-dimensional arrays.
    Array { element: Box<Type>, len: usize },
    /// The unit type — "nothing". The return type of a function that yields no value.
    Unit,
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::U8 => write!(f, "u8"),
            Type::U16 => write!(f, "u16"),
            Type::U32 => write!(f, "u32"),
            Type::U64 => write!(f, "u64"),
            Type::Array { element, len } => write!(f, "[{element}; {len}]"),
            Type::Unit => write!(f, "unit"),
        }
    }
}
