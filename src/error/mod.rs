pub mod graph_error;
pub mod lex_error;
pub mod parse_error;

use lex_error::LexError;
use parse_error::ParseError;
use std::fmt;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug)]
pub enum Error {
    Lex(LexError),
    Parse(ParseError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Lex(e) => write!(f, "{e}"),
            Error::Parse(e) => write!(f, "{e}"),
        }
    }
}

impl From<ParseError> for Error {
    fn from(e: ParseError) -> Self {
        Error::Parse(e)
    }
}
