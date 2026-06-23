pub mod duration_unit;
pub mod span;
pub mod token;

use crate::error::lex_error::LexError;
use span::Span;
use duration_unit::DurationUnit;
use token::{SpannedToken, Token};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Lexer
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub struct Lexer<'src> {
    src: &'src str,
    pos: usize,
    line: u32,
    col: u32,
}

impl<'src> Lexer<'src> {
    pub fn new(src: &'src str) -> Self {
        Lexer {
            src,
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    pub fn tokenise(mut self) -> Result<Vec<SpannedToken>, LexError> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace_and_comments();

            if self.pos >= self.src.len() {
                break;
            }

            let span = Span {
                line: self.line,
                col: self.col,
            };

            let token = self.next_token()?;

            tokens.push(SpannedToken { token, span });
        }

        Ok(tokens)
    }

    // Returns the nth character ahead of the current position (0 = current).
    fn peek(&self, n: usize) -> Option<char> {
        self.src[self.pos..].chars().nth(n)
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek(0)?;

        self.pos += c.len_utf8();

        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }

        Some(c)
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            while matches!(self.peek(0), Some(c) if c.is_whitespace()) {
                self.advance();
            }

            if self.peek(0) == Some('/') && self.peek(1) == Some('/') {
                self.advance();
                self.advance();

                while matches!(self.peek(0), Some(c) if c != '\n') {
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    fn next_token(&mut self) -> Result<Token, LexError> {
        let c: char = self
            .peek(0)
            .ok_or_else(|| self.err("unexpected end of input"))?;
        match c {
            '{' => {
                self.advance();
                Ok(Token::LBrace)
            }
            '}' => {
                self.advance();
                Ok(Token::RBrace)
            }
            '(' => {
                self.advance();
                Ok(Token::LParen)
            }
            ')' => {
                self.advance();
                Ok(Token::RParen)
            }
            '[' => {
                self.advance();
                Ok(Token::LBracket)
            }
            ']' => {
                self.advance();
                Ok(Token::RBracket)
            }
            ',' => {
                self.advance();
                Ok(Token::Comma)
            }
            ';' => {
                self.advance();
                Ok(Token::Semicolon)
            }
            ':' => {
                self.advance();
                Ok(Token::Colon)
            }
            '=' => {
                self.advance();
                if self.peek(0) == Some('>') {
                    self.advance();
                    Ok(Token::FatArrow)
                } else {
                    Ok(Token::Equals)
                }
            }
            '.' => {
                self.advance();
                if self.peek(0) == Some('.') {
                    self.advance();
                    Ok(Token::DotDot)
                } else {
                    Err(self.err("unexpected character '.' (did you mean a range `..`?)"))
                }
            }
            '?' => {
                self.advance();
                Ok(Token::Question)
            }
            '+' => {
                self.advance();
                Ok(Token::Plus)
            }
            '-' => {
                self.advance();
                if self.peek(0) == Some('>') {
                    self.advance();
                    Ok(Token::Arrow)
                } else {
                    Ok(Token::Minus)
                }
            }
            '"' => {
                // A `"""` opener begins a multi-line (triple-quoted) string; a lone `"` a single-line one.
                if self.peek(1) == Some('"') && self.peek(2) == Some('"') {
                    self.scan_triple_string()
                } else {
                    self.scan_string()
                }
            }
            '0'..='9' => self.scan_number(),
            c if c.is_alphabetic() || c == '_' => self.scan_word(),
            c => Err(self.err(format!("unexpected character '{c}'"))),
        }
    }

    fn scan_string(&mut self) -> Result<Token, LexError> {
        self.advance(); // consume opening "
        let mut s = String::new();

        loop {
            match self.peek(0) {
                None | Some('\n') => return Err(self.err("unterminated string literal")),
                Some('"') => {
                    self.advance();
                    break;
                }
                Some(_) => s.push(self.advance().unwrap()),
            }
        }

        Ok(Token::Str(s))
    }

    // A triple-quoted string: spans newlines and may contain lone `"`; terminated by the next `"""`. Used for
    // multi-line values such as a node's markdown `description`.
    fn scan_triple_string(&mut self) -> Result<Token, LexError> {
        for _ in 0..3 {
            self.advance(); // consume opening """
        }
        let mut s = String::new();

        loop {
            if self.peek(0) == Some('"') && self.peek(1) == Some('"') && self.peek(2) == Some('"') {
                for _ in 0..3 {
                    self.advance(); // consume closing """
                }
                return Ok(Token::Str(s));
            }
            match self.advance() {
                Some(c) => s.push(c),
                None => return Err(self.err("unterminated triple-quoted string")),
            }
        }
    }

    fn scan_number(&mut self) -> Result<Token, LexError> {
        if self.peek(0) == Some('0') && self.peek(1) == Some('x') {
            return self.scan_hex();
        }

        let mut digits = String::new();

        while matches!(self.peek(0), Some(c) if c.is_ascii_digit()) {
            digits.push(self.advance().unwrap());
        }

        let value: u64 = digits
            .parse()
            .map_err(|_| self.err("integer literal out of u64 range"))?;

        // Check for duration suffix: ms or s, not followed by further identifier chars
        if self.peek(0) == Some('m') && self.peek(1) == Some('s') {
            if !self.peek(2).map_or(false, is_ident_continue) {
                self.advance();
                self.advance();
                return Ok(Token::Duration(value, DurationUnit::Ms));
            }
        } else if self.peek(0) == Some('s') {
            if !self.peek(1).map_or(false, is_ident_continue) {
                self.advance();
                return Ok(Token::Duration(value, DurationUnit::S));
            }
        }

        Ok(Token::Integer(value))
    }

    fn scan_hex(&mut self) -> Result<Token, LexError> {
        self.advance();
        self.advance(); // consume opening "0x"

        let mut digits = String::new();

        while matches!(self.peek(0), Some(c) if c.is_ascii_hexdigit()) {
            digits.push(self.advance().unwrap());
        }

        if digits.is_empty() {
            return Err(self.err("encountered a non-hex digit after `0x`"));
        }

        let value = u64::from_str_radix(&digits, 16)
            .map_err(|_| self.err("hex literal out of u64 range"))?;

        Ok(Token::HexLit(value))
    }

    fn scan_word(&mut self) -> Result<Token, LexError> {
        let mut word = String::new();
        while matches!(self.peek(0), Some(c) if is_ident_continue(c)) {
            word.push(self.advance().unwrap());
        }
        Ok(keyword_or_ident(&word))
    }

    fn err(&self, msg: impl Into<String>) -> LexError {
        LexError {
            message: msg.into(),
            span: Span {
                line: self.line,
                col: self.col,
            },
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// // Utilities
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn keyword_or_ident(s: &str) -> Token {
    match s {
        "all" => Token::All,
        "and" => Token::And,
        "animate" => Token::Animate,
        "arrange" => Token::Arrange,
        "auto" => Token::Auto,
        "bottom_to_top" => Token::BottomToTop,
        "button" => Token::Button,
        "compute" => Token::Compute,
        "constant" => Token::Constant,
        "contains" => Token::Contains,
        "context" => Token::Context,
        "data" => Token::Data,
        "emit" => Token::Emit,
        "fn" => Token::Fn,
        "for" => Token::For,
        "format" => Token::Format,
        "from" => Token::From,
        "grid" => Token::Grid,
        "group" => Token::Group,
        "hash" => Token::Hash,
        "horizontal" => Token::Horizontal,
        "import" => Token::Import,
        "in" => Token::In,
        "label" => Token::Label,
        "layout" => Token::Layout,
        "left_to_right" => Token::LeftToRight,
        "let" => Token::Let,
        "mod" => Token::Mod,
        "node" => Token::Node,
        "not" => Token::Not,
        "on" => Token::On,
        "operation" => Token::Operation,
        "or" => Token::Or,
        "over" => Token::Over,
        "pinned" => Token::Pinned,
        "reduce" => Token::Reduce,
        "register" => Token::Register,
        "reroute" => Token::Reroute,
        "right_to_left" => Token::RightToLeft,
        "rotl_s" => Token::RotlS,
        "rotl_u" => Token::RotlU,
        "rotr_s" => Token::RotrS,
        "rotr_u" => Token::RotrU,
        "set" => Token::Set,
        "shl" => Token::Shl,
        "shr_s" => Token::ShrS,
        "shr_u" => Token::ShrU,
        "symbol" => Token::Symbol,
        "to" => Token::To,
        "top_to_bottom" => Token::TopToBottom,
        "vertical" => Token::Vertical,
        "via" => Token::Via,
        "wire" => Token::Wire,
        "word_size" => Token::WordSize,
        "xor" => Token::Xor,
        _ => Token::Ident(s.to_owned()),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
