use super::Parser;
use crate::{
    ast::ebnf_05::{HashBlock, HashItem},
    error::parse_error::ParseError,
    lexer::token::Token,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// EBNF §5 Hash block
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Parser {
    pub fn parse_hash_block(&mut self) -> Result<HashBlock, ParseError> {
        self.expect(&Token::Hash, "`hash`")?;
        let name = self.expect_ident()?;
        self.expect(&Token::LBrace, "`{`")?;
        let mut items = Vec::new();

        while !matches!(self.peek_nth(0), Some(Token::RBrace) | None) {
            items.push(self.parse_hash_item()?);
        }

        self.expect(&Token::RBrace, "`}`")?;
        Ok(HashBlock { name, items })
    }

    fn parse_hash_item(&mut self) -> Result<HashItem, ParseError> {
        match self.peek_nth(0) {
            Some(Token::Context) => Ok(HashItem::Context(self.parse_context_block()?)),
            Some(Token::Fn) => Ok(HashItem::FnDef(self.parse_fn_def()?)),
            Some(Token::Node) => Ok(HashItem::Node(self.parse_node_decl()?)),
            Some(Token::Wire) => Ok(HashItem::Wire(self.parse_wire_decl()?)),
            Some(Token::Group) => Ok(HashItem::Group(self.parse_group_decl()?)),
            Some(Token::Layout) => Ok(HashItem::Layout(self.parse_layout_decl()?)),
            Some(Token::Data) => Ok(HashItem::Data(self.parse_data_decl()?)),
            Some(Token::Ident(_)) => Ok(HashItem::EventHandler(self.parse_event_handler()?)),
            Some(t) => Err(self.err(format!("unexpected token `{t}` found in hash block"))),
            None => Err(self.err("hash block has not been terminated")),
        }
    }
}
