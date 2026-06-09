use super::Parser;
use crate::{ast::ebnf_04::FnDef, error::parse_error::ParseError, lexer::token::Token};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// EBNF §4 Function definitions
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Parser {
    pub fn parse_fn_def(&mut self) -> Result<FnDef, ParseError> {
        self.expect(&Token::Fn, "`fn`")?;
        let name = self.expect_ident()?;
        self.expect(&Token::LParen, "`(`")?;
        let params = self.parse_comma_sep(|p| p.expect_ident(), &Token::RParen)?;
        self.expect(&Token::RParen, "`)`")?;
        self.expect(&Token::Equals, "`=`")?;
        let body = self.parse_expr()?;

        Ok(FnDef { name, params, body })
    }
}
