// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug)]
pub struct LexError {
    pub message: String,
    pub span: crate::lexer::span::Span,
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "lex error at line {}, col {}: {}", self.span.line, self.span.col, self.message)
    }
}

impl From<LexError> for crate::error::Error {
    fn from(e: LexError) -> Self { crate::error::Error::Lex(e) }
}

