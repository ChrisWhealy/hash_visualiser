use super::Parser;
use crate::{
    ast::ebnf_11::{BinOp, Expr},
    error::parse_error::ParseError,
    lexer::token::Token,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// EBNF §11 Expressions
//
// Precedence (low → high): or → xor → and → +/- → shl/shr_u/shr_s → rotr/rotl → not → primary
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Parser {
    pub fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_xor()?;

        while self.peek_nth(0) == Some(&Token::Or) {
            self.advance();
            let rhs = self.parse_xor()?;
            lhs = Expr::BinOp {
                op: BinOp::Or,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }

        Ok(lhs)
    }

    fn parse_xor(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_and()?;

        while self.peek_nth(0) == Some(&Token::Xor) {
            self.advance();
            let rhs = self.parse_and()?;
            lhs = Expr::BinOp {
                op: BinOp::Xor,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }

        Ok(lhs)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_add()?;

        while self.peek_nth(0) == Some(&Token::And) {
            self.advance();
            let rhs = self.parse_add()?;
            lhs = Expr::BinOp {
                op: BinOp::And,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }

        Ok(lhs)
    }

    fn parse_add(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_shift()?;

        loop {
            let op = match self.peek_nth(0) {
                Some(Token::Plus) => BinOp::Add,
                Some(Token::Minus) => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_shift()?;
            lhs = Expr::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }

        Ok(lhs)
    }

    fn parse_shift(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_rot()?;

        loop {
            let op = match self.peek_nth(0) {
                Some(Token::Shl) => BinOp::Shl,
                Some(Token::ShrU) => BinOp::ShrU,
                Some(Token::ShrS) => BinOp::ShrS,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_rot()?;
            lhs = Expr::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }

        Ok(lhs)
    }

    fn parse_rot(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_mod()?;

        loop {
            let op = match self.peek_nth(0) {
                Some(Token::RotrU) => BinOp::RotrU,
                Some(Token::RotrS) => BinOp::RotrS,
                Some(Token::RotlU) => BinOp::RotlU,
                Some(Token::RotlS) => BinOp::RotlS,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_mod()?;
            lhs = Expr::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }

        Ok(lhs)
    }

    // `mod` binds tighter than the rotates/shifts/arithmetic above it (Rust-like multiplicative precedence), so a bare
    // `x + 1 mod 5` is `x + (1 mod 5)`. Index expressions such as `c[(x + 4) mod 5]` use explicit parentheses anyway.
    fn parse_mod(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_unary()?;

        while self.peek_nth(0) == Some(&Token::Mod) {
            self.advance();
            let rhs = self.parse_unary()?;
            lhs = Expr::BinOp {
                op: BinOp::Mod,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }

        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if self.peek_nth(0) == Some(&Token::Not) {
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expr::Not(Box::new(operand)));
        }

        self.parse_postfix()
    }

    // §11.1  postfix array indexing — binds tighter than any binary or unary operator.
    // postfix ::= primary { "[" expr "]" }
    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut base = self.parse_primary()?;

        while self.peek_nth(0) == Some(&Token::LBracket) {
            self.advance();
            let index = self.parse_expr()?;
            self.expect(&Token::RBracket, "`]`")?;
            base = Expr::Index {
                base: Box::new(base),
                index: Box::new(index),
            };
        }

        Ok(base)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let err_msg: &str = "expected an expression, instead found";

        match self.peek_nth(0).cloned() {
            Some(Token::Integer(n)) => {
                self.advance();
                Ok(Expr::Integer(n))
            }
            Some(Token::HexLit(n)) => {
                self.advance();
                Ok(Expr::HexLit(n))
            }
            Some(Token::LParen) => {
                self.advance();
                let e = self.parse_expr()?;
                self.expect(&Token::RParen, "`)`")?;
                Ok(e)
            }
            // A leading "[" begins a comprehension if followed by "for", otherwise an array literal. (Indexing is
            // handled postfix, after an atom.)
            Some(Token::LBracket) => {
                if self.peek_nth(1) == Some(&Token::For) {
                    self.parse_comprehension()
                } else {
                    self.parse_array_literal()
                }
            }
            Some(Token::Reduce) => self.parse_reduce(),
            // fn_call before bare ident: both start with IDENT, but call has "(" next
            Some(Token::Ident(_)) => {
                let name = self.expect_ident()?;
                if self.peek_nth(0) == Some(&Token::LParen) {
                    self.advance();
                    let args = self.parse_arg_list()?;
                    self.expect(&Token::RParen, "`)`")?;
                    Ok(Expr::Call { name, args })
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            Some(t) => Err(self.err(format!("{err_msg} {t}"))),
            None => Err(self.err("{err_msg} end of input")),
        }
    }

    // §11.1  comprehension ::= "[" "for" IDENT "in" INTEGER ".." INTEGER "=>" expr "]"
    fn parse_comprehension(&mut self) -> Result<Expr, ParseError> {
        self.expect(&Token::LBracket, "`[`")?;
        self.expect(&Token::For, "`for`")?;
        let var = self.expect_ident()?;
        self.expect(&Token::In, "`in`")?;
        let start = self.expect_integer()?;
        self.expect(&Token::DotDot, "`..`")?;
        let end = self.expect_integer()?;
        self.expect(&Token::FatArrow, "`=>`")?;
        let body = self.parse_expr()?;
        self.expect(&Token::RBracket, "`]`")?;

        Ok(Expr::Comprehension {
            var,
            start,
            end,
            body: Box::new(body),
        })
    }

    // §11.1  array_literal ::= "[" [ expr { "," expr } [","] ] "]"
    fn parse_array_literal(&mut self) -> Result<Expr, ParseError> {
        self.expect(&Token::LBracket, "`[`")?;
        let elems = self.parse_comma_sep(|p| p.parse_expr(), &Token::RBracket)?;
        self.expect(&Token::RBracket, "`]`")?;

        Ok(Expr::Array(elems))
    }

    // §11.1  reduction ::= "reduce" reduce_op "over" unary
    fn parse_reduce(&mut self) -> Result<Expr, ParseError> {
        self.expect(&Token::Reduce, "`reduce`")?;
        let op = self.parse_reduce_op()?;
        self.expect(&Token::Over, "`over`")?;
        let array = self.parse_unary()?;

        Ok(Expr::Reduce {
            op,
            array: Box::new(array),
        })
    }

    // A reduction folds with an associative binary operator. Non-associative ops (shifts, rotates, subtraction) are
    // rejected here rather than silently producing order-dependent nonsense.
    fn parse_reduce_op(&mut self) -> Result<BinOp, ParseError> {
        let msg = "`reduce` requires an associative operator (xor, and, or, +), instead found";
        let op = match self.peek_nth(0) {
            Some(Token::Or) => BinOp::Or,
            Some(Token::Xor) => BinOp::Xor,
            Some(Token::And) => BinOp::And,
            Some(Token::Plus) => BinOp::Add,
            Some(t) => return Err(self.err(format!("{msg} {t}"))),
            None => return Err(self.err(format!("{msg} end of input"))),
        };
        self.advance();
        Ok(op)
    }
}
