use super::Parser;
use crate::{
    ast::{
        ebnf_10::{
            AnimateEffect, AnimateSpec, Effect, EmitEffect, EmitTarget, LetBinding, RerouteDir,
            RerouteEffect, SetEffect,
        },
        ebnf_11::Expr,
    },
    error::parse_error::ParseError,
    lexer::token::Token,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// EBNF §10 Effects
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Parser {
    pub fn parse_effect(&mut self) -> Result<Effect, ParseError> {
        match self.peek_nth(0) {
            Some(Token::Set) => Ok(Effect::Set(self.parse_set_effect()?)),
            Some(Token::Animate) => Ok(Effect::Animate(self.parse_animate_effect()?)),
            Some(Token::Emit) => Ok(Effect::Emit(self.parse_emit_effect()?)),
            Some(Token::Reroute) => Ok(Effect::Reroute(self.parse_reroute_effect()?)),
            Some(Token::Let) => Ok(Effect::Let(self.parse_let_binding()?)),
            Some(t) => Err(self.err(format!("expected effect keyword, found {t}"))),
            None => Err(self.err("expected effect, found end of input")),
        }
    }

    // §10.1  set effect
    // set IDENT ":"  → prop assign
    // set IDENT "="  → var assign
    // set IDENT      → bare (set node's primary value)
    fn parse_set_effect(&mut self) -> Result<SetEffect, ParseError> {
        self.expect(&Token::Set, "`set`")?;
        let name = self.parse_any_name()?;

        match self.peek_nth(0) {
            Some(Token::Colon) => {
                self.advance();
                Ok(SetEffect::Prop {
                    name,
                    value: self.parse_expr()?,
                })
            }
            Some(Token::Equals) => {
                self.advance();
                Ok(SetEffect::Var {
                    name,
                    value: self.parse_expr()?,
                })
            }
            _ => Ok(SetEffect::Bare(name)),
        }
    }

    // §10.2  let binding
    fn parse_let_binding(&mut self) -> Result<LetBinding, ParseError> {
        self.expect(&Token::Let, "`let`")?;
        let name = self.expect_ident()?;
        self.expect(&Token::Equals, "`=`")?;
        let value = self.parse_expr()?;

        Ok(LetBinding { name, value })
    }

    // §10.3  animate effect
    // Disambiguate on the token after the property name:
    //   ":"    → fill_pulse  (animate fill: pulse "colour" for DURATION)
    //   "from" → transition  (animate prop from expr to expr over DURATION)
    fn parse_animate_effect(&mut self) -> Result<AnimateEffect, ParseError> {
        self.expect(&Token::Animate, "`animate`")?;
        let prop = self.parse_any_name()?;

        match self.peek_nth(0) {
            Some(Token::Colon) => {
                self.advance();
                // expect the literal identifier "pulse"
                match self.peek_nth(0).cloned() {
                    Some(Token::Ident(s)) if s == "pulse" => {
                        self.advance();
                    }
                    Some(t) => return Err(self.err(format!("expected `pulse`, found {t}"))),
                    None => return Err(self.err("expected `pulse`, found end of input")),
                }

                let colour = self.expect_string()?;
                self.expect(&Token::For, "`for`")?;
                let duration = self.expect_duration()?;

                Ok(AnimateEffect {
                    spec: AnimateSpec::FillPulse { colour, duration },
                })
            }
            Some(Token::From) => {
                self.advance();
                let from = self.parse_expr()?;
                self.expect(&Token::To, "`to`")?;
                let to = self.parse_expr()?;
                self.expect(&Token::Over, "`over`")?;
                let duration = self.expect_duration()?;

                Ok(AnimateEffect {
                    spec: AnimateSpec::Transition {
                        prop,
                        from,
                        to,
                        duration,
                    },
                })
            }
            Some(t) => Err(self.err(format!(
                "expected `:` or `from` after animate property, found {t}"
            ))),
            None => Err(self.err("expected `:` or `from`, found end of input")),
        }
    }

    // §10.4  emit effect
    fn parse_emit_effect(&mut self) -> Result<EmitEffect, ParseError> {
        self.expect(&Token::Emit, "`emit`")?;
        let event = self.expect_ident()?;
        self.expect(&Token::LParen, "`(`")?;
        let args = self.parse_arg_list()?;
        self.expect(&Token::RParen, "`)`")?;

        let target = match self.peek_nth(0) {
            Some(Token::Arrow) => {
                let err_msg: &str = "expected an emit target, instead found";

                self.advance();
                match self.peek_nth(0).cloned() {
                    Some(Token::All) => {
                        self.advance();
                        Some(EmitTarget::All)
                    }
                    Some(Token::Ident(s)) => {
                        self.advance();
                        Some(EmitTarget::Node(s))
                    }
                    Some(t) => return Err(self.err(format!("{err_msg} {t}"))),
                    None => return Err(self.err("{err_msg} end of input")),
                }
            }
            Some(Token::Via) => {
                self.advance();
                Some(EmitTarget::Via(self.expect_ident()?))
            }
            _ => None,
        };
        Ok(EmitEffect {
            event,
            args,
            target,
        })
    }

    pub fn parse_arg_list(&mut self) -> Result<Vec<Expr>, ParseError> {
        self.parse_comma_sep(|p| p.parse_expr(), &Token::RParen)
    }

    // §10.5  reroute effect
    fn parse_reroute_effect(&mut self) -> Result<RerouteEffect, ParseError> {
        let err_msg: &str = "expected either of the keywords `to` or `from`, instead found";
        self.expect(&Token::Reroute, "`reroute`")?;
        let wire = self.expect_ident()?;

        let direction = match self.peek_nth(0) {
            Some(Token::To) => {
                self.advance();
                RerouteDir::To
            }
            Some(Token::From) => {
                self.advance();
                RerouteDir::From
            }
            Some(t) => return Err(self.err(format!("{err_msg} {t}"))),
            None => return Err(self.err("{err_msg} end of input")),
        };

        let node = self.expect_ident()?;

        Ok(RerouteEffect {
            wire,
            direction,
            node,
        })
    }
}
