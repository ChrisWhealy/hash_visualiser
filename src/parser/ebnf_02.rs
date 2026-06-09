use super::Parser;
use crate::{
    ast::ebnf_02::*,
    error::parse_error::ParseError,
    lexer::token::Token,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// EBNF §2 Top-level
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Parser {
    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut items = Vec::new();

        while self.peek_nth(0).is_some() {
            items.push(self.parse_top_item()?);
        }

        Ok(Program { items })
    }

    fn parse_top_item(&mut self) -> Result<TopItem, ParseError> {
        match self.peek_nth(0) {
            Some(Token::Context) => Ok(TopItem::Context(self.parse_context_block()?)),
            Some(Token::Fn) => Ok(TopItem::FnDef(self.parse_fn_def()?)),
            Some(Token::Hash) => Ok(TopItem::Hash(self.parse_hash_block()?)),
            Some(Token::Node) => Ok(TopItem::Node(self.parse_node_decl()?)),
            Some(Token::Wire) => Ok(TopItem::Wire(self.parse_wire_decl()?)),
            Some(Token::Group) => Ok(TopItem::Group(self.parse_group_decl()?)),
            Some(Token::Layout) => Ok(TopItem::Layout(self.parse_layout_decl()?)),
            Some(Token::Ident(_)) => Ok(TopItem::EventHandler(self.parse_event_handler()?)),
            Some(t) => Err(self.err(format!("unexpected token `{t}` found at top level"))),
            None => Err(self.err("unexpected end of input")),
        }
    }
}
