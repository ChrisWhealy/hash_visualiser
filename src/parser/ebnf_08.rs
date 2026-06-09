use super::Parser;
use crate::{
    ast::ebnf_08::{ArrangeMode, FlowDirection, GroupDecl, GroupItem},
    error::parse_error::ParseError,
    lexer::token::Token,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// EBNF §8 Group and layout
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Parser {
    pub fn parse_group_decl(&mut self) -> Result<GroupDecl, ParseError> {
        self.expect(&Token::Group, "`group`")?;
        let name = self.expect_ident()?;
        self.expect(&Token::LBrace, "`{`")?;
        let mut items = Vec::new();

        while !matches!(self.peek_nth(0), Some(Token::RBrace) | None) {
            items.push(self.parse_group_item()?);
        }

        self.expect(&Token::RBrace, "`}`")?;
        Ok(GroupDecl { name, items })
    }

    fn parse_group_item(&mut self) -> Result<GroupItem, ParseError> {
        match self.peek_nth(0) {
            Some(Token::Contains) => {
                self.advance();
                self.expect(&Token::Colon, "`:`")?;
                self.expect(&Token::LBracket, "`[`")?;
                let names = self.parse_comma_sep(|p| p.expect_ident(), &Token::RBracket)?;
                self.expect(&Token::RBracket, "`]`")?;
                Ok(GroupItem::Contains(names))
            }
            Some(Token::Arrange) => {
                let err_msg = "expected an arrange mode, instead found";
                
                self.advance();
                self.expect(&Token::Colon, "`:`")?;

                let mode = match self.peek_nth(0) {
                    Some(Token::Grid) => {
                        self.advance();
                        ArrangeMode::Grid
                    }
                    Some(Token::Horizontal) => {
                        self.advance();
                        ArrangeMode::Horizontal
                    }
                    Some(Token::Vertical) => {
                        self.advance();
                        ArrangeMode::Vertical
                    }
                    Some(t) => return Err(self.err(format!("{err_msg} `{t}`"))),
                    None => return Err(self.err("{err_msg} end of input")),
                };
                Ok(GroupItem::Arrange(mode))
            }
            Some(t) => Err(self.err(format!(
                "expected either of the keywords `contains` or `arrange`, instead found {t}"
            ))),
            None => Err(self.err("group block has not been terminated")),
        }
    }

    pub fn parse_layout_decl(&mut self) -> Result<FlowDirection, ParseError> {
        self.expect(&Token::Layout, "`layout`")?;
        self.expect(&Token::Colon, "`:`")?;

        match self.peek_nth(0) {
            Some(Token::LeftToRight) => {
                self.advance();
                Ok(FlowDirection::LeftToRight)
            }
            Some(Token::TopToBottom) => {
                self.advance();
                Ok(FlowDirection::TopToBottom)
            }
            Some(Token::RightToLeft) => {
                self.advance();
                Ok(FlowDirection::RightToLeft)
            }
            Some(Token::BottomToTop) => {
                self.advance();
                Ok(FlowDirection::BottomToTop)
            }
            Some(t) => Err(self.err(format!("expected flow direction, found {t}"))),
            None => Err(self.err("expected flow direction, found end of input")),
        }
    }
}
