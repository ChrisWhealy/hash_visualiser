use super::Parser;
use crate::{
    ast::ebnf_06::{NodeDecl, NodeKind, PropValue, Property},
    ast::ebnf_11::Expr,
    error::parse_error::ParseError,
    lexer::token::Token,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// EBNF §6 Node declarations
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Parser {
    pub fn parse_node_decl(&mut self) -> Result<NodeDecl, ParseError> {
        self.expect(&Token::Node, "`node`")?;
        let name = self.expect_ident()?;
        self.expect(&Token::Colon, "`:`")?;
        let kind = self.parse_kind()?;
        self.expect(&Token::LBrace, "`{`")?;
        let properties = self.parse_property_list()?;
        self.expect(&Token::RBrace, "`}`")?;

        Ok(NodeDecl {
            name,
            kind,
            properties,
        })
    }

    fn parse_kind(&mut self) -> Result<NodeKind, ParseError> {
        let err_msg: &str = "expected a node kind, instead found";

        match self.peek_nth(0).cloned() {
            Some(Token::Register) => {
                self.advance();
                Ok(NodeKind::Register)
            }
            Some(Token::Operation) => {
                self.advance();
                Ok(NodeKind::Operation)
            }
            Some(Token::Constant) => {
                self.advance();
                Ok(NodeKind::Constant)
            }
            Some(Token::Button) => {
                self.advance();
                Ok(NodeKind::Button)
            }
            Some(Token::Ident(s)) => {
                self.advance();
                Ok(NodeKind::User(s))
            }
            Some(t) => Err(self.err(format!("{err_msg} {t}"))),
            None => Err(self.err("{err_msg} end of input")),
        }
    }

    fn parse_property_list(&mut self) -> Result<Vec<Property>, ParseError> {
        self.parse_comma_sep(|p| p.parse_property(), &Token::RBrace)
    }

    fn parse_property(&mut self) -> Result<Property, ParseError> {
        let name = self.parse_any_name()?;
        self.expect(&Token::Colon, "`:`")?;
        let value = self.parse_prop_value()?;
        Ok(Property { name, value })
    }

    fn parse_prop_value(&mut self) -> Result<PropValue, ParseError> {
        match self.peek_nth(0).cloned() {
            Some(Token::Str(s)) => {
                self.advance();
                Ok(PropValue::Str(s))
            }
            // Layout-value keywords that cannot appear in a general expr
            Some(Token::Auto) => {
                self.advance();
                Ok(PropValue::Expr(Expr::Ident("auto".into())))
            }
            Some(Token::Pinned) => {
                self.advance();
                self.expect(&Token::LParen, "`(`")?;
                let x = self.expect_integer()?;
                self.expect(&Token::Comma, "`,`")?;
                let y = self.expect_integer()?;
                self.expect(&Token::RParen, "`)`")?;

                Ok(PropValue::Expr(Expr::Call {
                    name: "pinned".into(),
                    args: vec![Expr::Integer(x), Expr::Integer(y)],
                }))
            }
            Some(Token::Grid) => {
                self.advance();
                Ok(PropValue::Expr(Expr::Ident("grid".into())))
            }
            Some(Token::Horizontal) => {
                self.advance();
                Ok(PropValue::Expr(Expr::Ident("horizontal".into())))
            }
            Some(Token::Vertical) => {
                self.advance();
                Ok(PropValue::Expr(Expr::Ident("vertical".into())))
            }
            Some(Token::LeftToRight) => {
                self.advance();
                Ok(PropValue::Expr(Expr::Ident("left_to_right".into())))
            }
            Some(Token::RightToLeft) => {
                self.advance();
                Ok(PropValue::Expr(Expr::Ident("right_to_left".into())))
            }
            Some(Token::TopToBottom) => {
                self.advance();
                Ok(PropValue::Expr(Expr::Ident("top_to_bottom".into())))
            }
            Some(Token::BottomToTop) => {
                self.advance();
                Ok(PropValue::Expr(Expr::Ident("bottom_to_top".into())))
            }
            _ => Ok(PropValue::Expr(self.parse_expr()?)),
        }
    }
}
