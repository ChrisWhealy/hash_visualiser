use crate::lexer::{span::Span, duration_unit::DurationUnit};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    All,
    And,
    Animate,
    Arrange,
    Auto,
    BottomToTop,
    Button,
    Compute,
    Constant,
    Contains,
    Context,
    Data,
    Emit,
    Fn,
    For,
    Format,
    From,
    Grid,
    Group,
    Hash,
    Horizontal,
    Import,
    In,
    Label,
    Layout,
    LeftToRight,
    Let,
    Node,
    Not,
    On,
    Operation,
    Or,
    Over,
    Pinned,
    Mod,
    Reduce,
    Register,
    Reroute,
    RightToLeft,
    RotlS,
    RotlU,
    RotrS,
    RotrU,
    Set,
    Shl,
    ShrS,
    ShrU,
    Symbol,
    To,
    TopToBottom,
    Vertical,
    Via,
    Wire,
    WordSize,
    Xor,

    // Literals
    Ident(String),
    Integer(u64),
    HexLit(u64),
    Str(String),
    Duration(u64, DurationUnit),

    // Punctuation
    Arrow,     // ->
    FatArrow,  // =>
    DotDot,    // ..
    Colon,     // :
    Equals,    // =
    LBrace,    // {
    RBrace,    // }
    LParen,    // (
    RParen,    // )
    LBracket,  // [
    RBracket,  // ]
    Comma,     // ,
    Semicolon, // ;
    Question,  // ?
    Plus,      // +
    Minus,     // -
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Ident(s) => write!(f, "identifier `{s}`"),
            Token::Integer(n) => write!(f, "integer `{n}`"),
            Token::HexLit(n) => write!(f, "hex literal `0x{n:x}`"),
            Token::Str(s) => write!(f, "string `\"{s}\"`"),
            Token::Duration(v, u) => write!(
                f,
                "duration `{v}{}`",
                if *u == DurationUnit::Ms { "ms" } else { "s" }
            ),
            Token::Arrow => write!(f, "`->`"),
            Token::FatArrow => write!(f, "`=>`"),
            Token::DotDot => write!(f, "`..`"),
            Token::Colon => write!(f, "`:`"),
            Token::Equals => write!(f, "`=`"),
            Token::LBrace => write!(f, "`{{`"),
            Token::RBrace => write!(f, "`}}`"),
            Token::LParen => write!(f, "`(`"),
            Token::RParen => write!(f, "`)`"),
            Token::LBracket => write!(f, "`[`"),
            Token::RBracket => write!(f, "`]`"),
            Token::Comma => write!(f, "`,`"),
            Token::Semicolon => write!(f, "`;`"),
            Token::Question => write!(f, "`?`"),
            Token::Plus => write!(f, "`+`"),
            Token::Minus => write!(f, "`-`"),
            other => write!(f, "keyword `{other:?}`"),
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Span,
}

