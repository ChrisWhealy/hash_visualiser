use super::Parser;
use crate::{
    ast::ebnf_04::{FnDef, Param, Type},
    error::parse_error::ParseError,
    lexer::token::Token,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// EBNF §4 Function definitions
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Parser {
    // fn_def ::= "fn" IDENT "(" [ typed_param_list ] ")" [ "->" type ] "=" expr ;
    pub fn parse_fn_def(&mut self) -> Result<FnDef, ParseError> {
        self.expect(&Token::Fn, "`fn`")?;
        let name = self.expect_ident()?;
        self.expect(&Token::LParen, "`(`")?;
        let params = self.parse_comma_sep(Self::parse_param, &Token::RParen)?;
        self.expect(&Token::RParen, "`)`")?;

        // An explicit `-> type` declares the return type; its absence means the function returns UNIT.
        let return_type = if self.peek_nth(0) == Some(&Token::Arrow) {
            self.advance();
            self.parse_type()?
        } else {
            Type::Unit
        };

        self.expect(&Token::Equals, "`=`")?;
        let body = self.parse_expr()?;

        Ok(FnDef {
            name,
            params,
            return_type,
            body,
        })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    // param ::= IDENT ":" type
    fn parse_param(&mut self) -> Result<Param, ParseError> {
        let name = self.expect_ident()?;
        self.expect(&Token::Colon, "`:`")?;
        let ty = self.parse_type()?;

        Ok(Param { name, ty })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    // type       ::= primitive | array_type
    // primitive  ::= "u8" | "u16" | "u32" | "u64"
    // array_type ::= "[" type ";" INTEGER "]"
    fn parse_type(&mut self) -> Result<Type, ParseError> {
        match self.peek_nth(0) {
            Some(Token::LBracket) => {
                self.advance();
                let element = self.parse_type()?;
                self.expect(&Token::Semicolon, "`;`")?;
                let len = self.expect_integer()? as usize;
                self.expect(&Token::RBracket, "`]`")?;

                Ok(Type::Array {
                    element: Box::new(element),
                    len,
                })
            }
            Some(Token::Ident(_)) => {
                let name = self.expect_ident()?;
                match name.as_str() {
                    "u8" => Ok(Type::U8),
                    "u16" => Ok(Type::U16),
                    "u32" => Ok(Type::U32),
                    "u64" => Ok(Type::U64),
                    other => Err(self.err(format!(
                        "unknown type `{other}` (expected u8, u16, u32, u64, or an array type such as [u8; 5])"
                    ))),
                }
            }
            Some(t) => Err(self.err(format!("expected a type, instead found {t}"))),
            None => Err(self.err("expected a type, instead found end of input")),
        }
    }
}
