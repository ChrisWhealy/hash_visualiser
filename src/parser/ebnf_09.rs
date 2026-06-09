use super::Parser;
use crate::{ast::ebnf_09::EventHandler, error::parse_error::ParseError, lexer::token::Token};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// EBNF §9 Event handlers
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Parser {
    pub fn parse_event_handler(&mut self) -> Result<EventHandler, ParseError> {
        let node = self.expect_ident()?;
        self.expect(&Token::On, "`on`")?;
        let event = self.parse_event_name()?;
        self.expect(&Token::LParen, "`(`")?;
        let params = self.parse_comma_sep(|p| p.expect_ident(), &Token::RParen)?;
        self.expect(&Token::RParen, "`)`")?;
        self.expect(&Token::LBrace, "`{`")?;
        let mut body = Vec::new();

        while !matches!(self.peek_nth(0), Some(Token::RBrace) | None) {
            body.push(self.parse_effect()?);
        }

        self.expect(&Token::RBrace, "`}`")?;
        Ok(EventHandler {
            node,
            event,
            params,
            body,
        })
    }

    // Event names are usually user-defined identifiers, but `reroute` is also a keyword that can be used as a built-in
    // event name.
    fn parse_event_name(&mut self) -> Result<String, ParseError> {
        let err_msg: &str = "expected an event name, instead found";

        match self.peek_nth(0).cloned() {
            Some(Token::Ident(s)) => {
                self.advance();
                Ok(s)
            }
            Some(Token::Reroute) => {
                self.advance();
                Ok("reroute".into())
            }
            Some(t) => Err(self.err(format!("{err_msg} {t}"))),
            None => Err(self.err("{err_msg} end of input")),
        }
    }
}
