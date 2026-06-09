use super::Parser;
use crate::{
    ast::ebnf_07::{WireDecl, WireEndpoint},
    error::parse_error::ParseError,
    lexer::token::Token,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// EBNF §7 Wire declarations
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Parser {
    pub fn parse_wire_decl(&mut self) -> Result<WireDecl, ParseError> {
        self.expect(&Token::Wire, "`wire`")?;

        // Disambiguate: [IDENT ":"] endpoint "->" endpoint
        // Look one token ahead: if pos+0 is IDENT and pos+1 is ":" → named wire.
        let name = if matches!(self.peek_nth(0), Some(Token::Ident(_)))
            && self.peek_nth(1) == Some(&Token::Colon)
        {
            let n = self.expect_ident()?;
            self.advance(); // consume ":"
            Some(n)
        } else {
            None
        };

        let source = self.parse_wire_endpoint()?;
        self.expect(&Token::Arrow, "`->`")?;
        let target = self.parse_wire_endpoint()?;

        Ok(WireDecl {
            name,
            source,
            target,
        })
    }

    fn parse_wire_endpoint(&mut self) -> Result<WireEndpoint, ParseError> {
        let err_msg: &str = "expected wire endpoint (name or `?`), instead found";

        match self.peek_nth(0).cloned() {
            Some(Token::Question) => {
                self.advance();
                Ok(WireEndpoint::Open)
            }
            Some(Token::Ident(s)) => {
                self.advance();
                Ok(WireEndpoint::Node(s))
            }
            Some(t) => Err(self.err(format!("{err_msg} {t}"))),
            None => Err(self.err("{err_msg} end of input")),
        }
    }
}
