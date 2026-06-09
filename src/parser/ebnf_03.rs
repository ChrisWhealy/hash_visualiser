use super::Parser;
use crate::{
    ast::ebnf_03::{ContextBlock, ContextItem},
    error::parse_error::ParseError,
    lexer::token::Token,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// EBNF §3 Context block
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Parser {
    pub fn parse_context_block(&mut self) -> Result<ContextBlock, ParseError> {
        self.expect(&Token::Context, "`context`")?;
        self.expect(&Token::LBrace, "`{`")?;

        let mut items = Vec::new();

        while !matches!(self.peek_nth(0), Some(Token::RBrace) | None) {
            let item = match self.peek_nth(0) {
                Some(Token::WordSize) => {
                    self.advance();
                    self.expect(&Token::Colon, "`:`")?;
                    ContextItem::WordSize(self.expect_integer()?)
                }
                Some(Token::Fn) => ContextItem::FnDef(self.parse_fn_def()?),
                Some(t) => {
                    return Err(self.err(format!(
                        "expected either of the keywords `word_size` or `fn`, instead found `{t}`"
                    )));
                }
                None => return Err(self.err("context block has not been terminated")),
            };

            items.push(item);
        }

        self.expect(&Token::RBrace, "`}`")?;
        Ok(ContextBlock { items })
    }
}
