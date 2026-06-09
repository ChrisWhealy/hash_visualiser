pub(crate) mod ebnf_02;
pub(crate) mod ebnf_03;
pub(crate) mod ebnf_04;
pub(crate) mod ebnf_05;
pub(crate) mod ebnf_06;
pub(crate) mod ebnf_07;
pub(crate) mod ebnf_08;
pub(crate) mod ebnf_09;
pub(crate) mod ebnf_10;
#[cfg(test)] mod tests;

use crate::{
    ast::{ebnf_02::Program, ebnf_10::Duration},
    error::parse_error::ParseError,
    lexer::{
        Lexer,
        span::Span,
        token::{SpannedToken, Token},
    },
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Common parser functionality
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Parser {
    pub fn new(tokens: Vec<SpannedToken>) -> Self {
        Parser { tokens, pos: 0 }
    }

    // Peek at the nth token where the zero'th token is the current one
    fn peek_nth(&self, n: usize) -> Option<&Token> {
        self.tokens.get(self.pos + n).map(|st| &st.token)
    }

    fn current_span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map(|st| st.span)
            .unwrap_or(Span::ZERO)
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn err(&self, msg: impl Into<String>) -> ParseError {
        ParseError {
            message: msg.into(),
            span: self.current_span(),
        }
    }

    fn expect(&mut self, tok: &Token, desc: &str) -> Result<(), ParseError> {
        let err_msg = format!("expected {desc}, instead found");

        match self.peek_nth(0) {
            Some(t) if t == tok => {
                self.advance();
                Ok(())
            }
            Some(t) => Err(self.err(format!("{err_msg} {t}"))),
            None => Err(self.err(format!("{err_msg} end of input"))),
        }
    }

    // Accept a user-defined identifier only.
    fn expect_ident(&mut self) -> Result<String, ParseError> {
        let err_msg: &str = "expected an identifier, instead found";

        match self.peek_nth(0).cloned() {
            Some(Token::Ident(s)) => {
                self.advance();
                Ok(s)
            }
            Some(t) => Err(self.err(format!("{err_msg} {t}"))),
            None => Err(self.err("{err_msg} end of input")),
        }
    }

    fn expect_string(&mut self) -> Result<String, ParseError> {
        let err_msg: &str = "expected a string, instead found";

        match self.peek_nth(0).cloned() {
            Some(Token::Str(s)) => {
                self.advance();
                Ok(s)
            }
            Some(t) => Err(self.err(format!("{err_msg} {t}"))),
            None => Err(self.err("{err_msg} end of input")),
        }
    }

    fn expect_integer(&mut self) -> Result<u64, ParseError> {
        let err_msg: &str = "expected an integer, instead found";

        match self.peek_nth(0).cloned() {
            Some(Token::Integer(n)) => {
                self.advance();
                Ok(n)
            }
            Some(t) => Err(self.err(format!("{err_msg} {t}"))),
            None => Err(self.err("{err_msg} end of input")),
        }
    }

    fn expect_duration(&mut self) -> Result<Duration, ParseError> {
        let err_msg: &str = "expected a duration (E.G. 250ms), instead found";

        match self.peek_nth(0).cloned() {
            Some(Token::Duration(v, u)) => {
                self.advance();
                Ok(Duration { value: v, unit: u })
            }
            Some(t) => Err(self.err(format!("{err_msg} {t}"))),
            None => Err(self.err("{err_msg} end of input")),
        }
    }

    // Accept an identifier OR any keyword that is legal as a name in
    // property/set/let positions (e.g. `set label: v`, `format: hex32`).
    fn parse_any_name(&mut self) -> Result<String, ParseError> {
        let err_msg: &str = "expected a name, instead found";

        let s = match self.peek_nth(0).cloned() {
            Some(Token::Ident(s)) => s,
            Some(Token::Label)    => "label".into(),
            Some(Token::Symbol)   => "symbol".into(),
            Some(Token::Format)   => "format".into(),
            Some(Token::Compute)  => "compute".into(),
            Some(Token::Contains) => "contains".into(),
            Some(Token::Arrange)  => "arrange".into(),
            Some(Token::Layout)   => "layout".into(),
            Some(Token::WordSize) => "word_size".into(),
            Some(Token::Register) => "register".into(),
            Some(Token::Operation) => "operation".into(),
            Some(Token::Constant) => "constant".into(),
            Some(Token::Button) => "button".into(),
            Some(Token::Auto) => "auto".into(),
            Some(Token::Pinned) => "pinned".into(),
            Some(Token::Grid) => "grid".into(),
            Some(Token::Horizontal) => "horizontal".into(),
            Some(Token::Vertical) => "vertical".into(),
            Some(Token::LeftToRight) => "left_to_right".into(),
            Some(Token::RightToLeft) => "right_to_left".into(),
            Some(Token::TopToBottom) => "top_to_bottom".into(),
            Some(Token::BottomToTop) => "bottom_to_top".into(),
            Some(t) => return Err(self.err(format!("{err_msg} {t}"))),
            None => return Err(self.err("{err_msg} end of input")),
        };

        self.advance();

        Ok(s)
    }

    fn parse_comma_sep<T, F>(&mut self, mut f: F, end: &Token) -> Result<Vec<T>, ParseError>
    where
        F: FnMut(&mut Self) -> Result<T, ParseError>,
    {
        let mut items = Vec::new();
        if self.peek_nth(0) == Some(end) {
            return Ok(items);
        }
        items.push(f(self)?);
        while self.peek_nth(0) == Some(&Token::Comma) {
            self.advance();
            if self.peek_nth(0) == Some(end) {
                break;
            } // trailing comma
            items.push(f(self)?);
        }
        Ok(items)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Public API
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub fn parse(src: &str) -> Result<Program, crate::error::Error> {
    let tokens = Lexer::new(src).tokenise()?;
    let mut parser = Parser::new(tokens);
    Ok(parser.parse_program()?)
}
