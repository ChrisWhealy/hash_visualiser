use std::fmt;
use super::{duration_unit::DurationUnit, span::Span, token::Token, Lexer};

// ── Comparison helpers ────────────────────────────────────────────────────────

fn eq<T: PartialEq + fmt::Debug>(actual: T, expected: T) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("expected {expected:?}, got {actual:?}"))
    }
}

fn msg_contains(msg: &str, needle: &str) -> Result<(), String> {
    if msg.contains(needle) {
        Ok(())
    } else {
        Err(format!("expected error message to contain {needle:?}, got:\n  {msg}"))
    }
}

// ── Lexer helpers ─────────────────────────────────────────────────────────────

fn lex(src: &str) -> Result<Vec<Token>, String> {
    Lexer::new(src)
        .tokenise()
        .map(|ts| ts.into_iter().map(|st| st.token).collect())
        .map_err(|e| e.to_string())
}

fn lex_spans(src: &str) -> Result<Vec<(Token, Span)>, String> {
    Lexer::new(src)
        .tokenise()
        .map(|ts| ts.into_iter().map(|st| (st.token, st.span)).collect())
        .map_err(|e| e.to_string())
}

// Returns the error message when the input is expected to fail tokenisation.
fn expect_lex_err(src: &str) -> Result<String, String> {
    match Lexer::new(src).tokenise() {
        Err(e) => Ok(e.to_string()),
        Ok(_) => Err(format!("expected lex error for {src:?}, but tokenisation succeeded")),
    }
}

fn token_at(tokens: &[(Token, Span)], idx: usize) -> Result<&(Token, Span), String> {
    tokens.get(idx).ok_or_else(|| {
        format!("expected token at index {idx}, but stream has only {} tokens", tokens.len())
    })
}

// ── §1.1  Keywords ────────────────────────────────────────────────────────────

#[test]
fn all_keywords_should_map_to_their_token() -> Result<(), String> {
    let cases: &[(&str, Token)] = &[
        ("all",           Token::All),
        ("and",           Token::And),
        ("animate",       Token::Animate),
        ("arrange",       Token::Arrange),
        ("auto",          Token::Auto),
        ("bottom_to_top", Token::BottomToTop),
        ("button",        Token::Button),
        ("compute",       Token::Compute),
        ("constant",      Token::Constant),
        ("contains",      Token::Contains),
        ("context",       Token::Context),
        ("emit",          Token::Emit),
        ("fn",            Token::Fn),
        ("for",           Token::For),
        ("format",        Token::Format),
        ("from",          Token::From),
        ("grid",          Token::Grid),
        ("group",         Token::Group),
        ("hash",          Token::Hash),
        ("horizontal",    Token::Horizontal),
        ("label",         Token::Label),
        ("layout",        Token::Layout),
        ("left_to_right", Token::LeftToRight),
        ("let",           Token::Let),
        ("node",          Token::Node),
        ("not",           Token::Not),
        ("on",            Token::On),
        ("operation",     Token::Operation),
        ("or",            Token::Or),
        ("over",          Token::Over),
        ("pinned",        Token::Pinned),
        ("register",      Token::Register),
        ("reroute",       Token::Reroute),
        ("right_to_left", Token::RightToLeft),
        ("rotl_s",        Token::RotlS),
        ("rotl_u",        Token::RotlU),
        ("rotr_s",        Token::RotrS),
        ("rotr_u",        Token::RotrU),
        ("set",           Token::Set),
        ("shl",           Token::Shl),
        ("shr_s",         Token::ShrS),
        ("shr_u",         Token::ShrU),
        ("symbol",        Token::Symbol),
        ("to",            Token::To),
        ("top_to_bottom", Token::TopToBottom),
        ("vertical",      Token::Vertical),
        ("via",           Token::Via),
        ("wire",          Token::Wire),
        ("word_size",     Token::WordSize),
        ("xor",           Token::Xor),
    ];

    for (src, expected) in cases {
        let tokens = lex(src)?;
        eq(&tokens, &vec![expected.clone()])
            .map_err(|e| format!("keyword `{src}`: {e}"))?;
    }

    Ok(())
}

#[test]
fn keyword_as_prefix_of_longer_word_should_be_an_ident() -> Result<(), String> {
    // "context1" starts with the keyword "context" but must be lexed as an identifier
    eq(lex("context1")?,   vec![Token::Ident("context1".into())])?;
    eq(lex("nodes")?,      vec![Token::Ident("nodes".into())])?;
    eq(lex("format_hex")?, vec![Token::Ident("format_hex".into())])?;
    eq(lex("wires")?,      vec![Token::Ident("wires".into())])?;
    Ok(())
}

// ── §1  IDENT ─────────────────────────────────────────────────────────────────

#[test]
fn should_get_plain_identifier() -> Result<(), String> {
    eq(lex("foo")?, vec![Token::Ident("foo".into())])
}

#[test]
fn should_get_identifier_with_leading_underscore() -> Result<(), String> {
    eq(lex("_bar")?, vec![Token::Ident("_bar".into())])
}

#[test]
fn should_get_identifier_containing_digits() -> Result<(), String> {
    eq(lex("r1")?,   vec![Token::Ident("r1".into())])?;
    eq(lex("a42z")?, vec![Token::Ident("a42z".into())])
}

#[test]
fn should_get_identifier_with_internal_underscores() -> Result<(), String> {
    eq(lex("big_sigma")?, vec![Token::Ident("big_sigma".into())])
}

#[test]
fn should_get_mixed_case_identifier() -> Result<(), String> {
    eq(lex("SHA256")?, vec![Token::Ident("SHA256".into())])?;
    eq(lex("Sigma1")?, vec![Token::Ident("Sigma1".into())])
}

// ── §1  INTEGER ───────────────────────────────────────────────────────────────

#[test]
fn should_get_integer_zero() -> Result<(), String> {
    eq(lex("0")?, vec![Token::Integer(0)])
}

#[test]
fn should_get_integer_positive_values() -> Result<(), String> {
    eq(lex("1")?,    vec![Token::Integer(1)])?;
    eq(lex("42")?,   vec![Token::Integer(42)])?;
    eq(lex("1024")?, vec![Token::Integer(1024)])
}

#[test]
fn should_get_integer_after_ignoring_leading_zeros() -> Result<(), String> {
    eq(lex("01")?,      vec![Token::Integer(1)])?;
    eq(lex("0042")?,    vec![Token::Integer(42)])?;
    eq(lex("0001024")?, vec![Token::Integer(1024)])
}

#[test]
fn should_get_multiple_integers_separated_by_whitespace() -> Result<(), String> {
    eq(
        lex("1 2 3")?,
        vec![Token::Integer(1), Token::Integer(2), Token::Integer(3)],
    )
}

// ── §1  HEX_LIT ───────────────────────────────────────────────────────────────

#[test]
fn should_get_hex_literal_from_lowercase_digits() -> Result<(), String> {
    eq(lex("0xdeadbeef")?, vec![Token::HexLit(0xdeadbeef)])
}

#[test]
fn should_get_hex_literal_from_uppercase_digits() -> Result<(), String> {
    eq(lex("0xDEADBEEF")?, vec![Token::HexLit(0xDEADBEEF)])
}

#[test]
fn should_get_hex_literal_from_mixed_case_digits() -> Result<(), String> {
    eq(lex("0xDeAdBeEf")?, vec![Token::HexLit(0xDeAdBeEf)])
}

#[test]
fn should_get_hex_literal_of_zero() -> Result<(), String> {
    eq(lex("0x0")?, vec![Token::HexLit(0)])
}

#[test]
fn should_get_hex_literal_of_single_digit() -> Result<(), String> {
    eq(lex("0xf")?, vec![Token::HexLit(0xf)])
}

// ── §1  STRING ────────────────────────────────────────────────────────────────

#[test]
fn should_get_empty_string_literal() -> Result<(), String> {
    eq(lex("\"\"")?, vec![Token::Str(String::new())])
}

#[test]
fn should_get_ascii_string_literal() -> Result<(), String> {
    eq(lex("\"hello\"")?, vec![Token::Str("hello".into())])
}

#[test]
fn should_get_string_literal_with_spaces() -> Result<(), String> {
    eq(lex("\"Step forward\"")?, vec![Token::Str("Step forward".into())])
}

#[test]
fn should_get_string_literal_with_unicode_content() -> Result<(), String> {
    // Display glyphs such as Σ₁ and ⊕ must survive the lexer unchanged
    eq(lex("\"Σ₁\"")?,    vec![Token::Str("Σ₁".into())])?;
    eq(lex("\"⊕\"")?,     vec![Token::Str("⊕".into())])?;
    eq(lex("\"Step →\"")?, vec![Token::Str("Step →".into())])
}

// ── §1  DURATION ──────────────────────────────────────────────────────────────

#[test]
fn should_get_duration_in_milliseconds() -> Result<(), String> {
    eq(lex("250ms")?, vec![Token::Duration(250, DurationUnit::Ms)])?;
    eq(lex("0ms")?,   vec![Token::Duration(0, DurationUnit::Ms)])
}

#[test]
fn should_get_duration_in_seconds() -> Result<(), String> {
    eq(lex("1s")?,  vec![Token::Duration(1, DurationUnit::S)])?;
    eq(lex("30s")?, vec![Token::Duration(30, DurationUnit::S)])
}

#[test]
fn integer_followed_by_ms_ident_suffix_should_not_be_a_duration() -> Result<(), String> {
    // "1msg": peek(0)='m', peek(1)='s', peek(2)='g' (ident-continue) → not a suffix
    eq(lex("1msg")?, vec![Token::Integer(1), Token::Ident("msg".into())])
}

#[test]
fn integer_followed_by_s_ident_suffix_should_not_be_a_duration() -> Result<(), String> {
    // "1second": peek(0)='s', peek(1)='e' (ident-continue) → not a suffix
    eq(lex("1second")?, vec![Token::Integer(1), Token::Ident("second".into())])
}

#[test]
fn integer_then_ms_at_end_of_input_should_be_a_duration() -> Result<(), String> {
    eq(lex("300ms")?, vec![Token::Duration(300, DurationUnit::Ms)])
}

#[test]
fn integer_then_s_followed_by_punctuation_should_be_a_duration() -> Result<(), String> {
    // "1s}" — '}' is not ident-continue so the 's' is consumed as a duration suffix
    eq(lex("1s}")?, vec![Token::Duration(1, DurationUnit::S), Token::RBrace])
}

// ── §1  Punctuation ───────────────────────────────────────────────────────────

#[test]
fn should_get_all_punctuation_tokens() -> Result<(), String> {
    eq(
        lex("-> : = { } ( ) [ ] , ? + -")?,
        vec![
            Token::Arrow,    Token::Colon,    Token::Equals,
            Token::LBrace,   Token::RBrace,
            Token::LParen,   Token::RParen,
            Token::LBracket, Token::RBracket,
            Token::Comma,    Token::Question,
            Token::Plus,     Token::Minus,
        ],
    )
}

#[test]
fn minus_not_followed_by_gt_should_be_minus_token() -> Result<(), String> {
    eq(lex("-")?,   vec![Token::Minus])?;
    eq(lex("- 5")?, vec![Token::Minus, Token::Integer(5)])
}

#[test]
fn hyphen_immediately_followed_by_gt_should_be_arrow() -> Result<(), String> {
    eq(lex("->")?, vec![Token::Arrow])
}

#[test]
fn arrow_surrounded_by_identifiers() -> Result<(), String> {
    eq(
        lex("a -> b")?,
        vec![Token::Ident("a".into()), Token::Arrow, Token::Ident("b".into())],
    )
}

// ── §1  Whitespace and line comments ─────────────────────────────────────────

#[test]
fn leading_and_trailing_whitespace_should_be_stripped() -> Result<(), String> {
    eq(lex("  fn  ")?, vec![Token::Fn])
}

#[test]
fn newlines_should_be_treated_as_whitespace() -> Result<(), String> {
    eq(lex("\nfn\n")?, vec![Token::Fn])
}

#[test]
fn tabs_should_be_treated_as_whitespace() -> Result<(), String> {
    eq(lex("\t fn \t")?, vec![Token::Fn])
}

#[test]
fn line_comment_should_be_discarded() -> Result<(), String> {
    eq(lex("fn // this is a comment\n")?, vec![Token::Fn])
}

#[test]
fn line_comment_without_trailing_newline_should_be_discarded() -> Result<(), String> {
    eq(lex("fn // end of file")?, vec![Token::Fn])
}

#[test]
fn comment_should_not_consume_the_following_line() -> Result<(), String> {
    eq(lex("fn // first\nnode")?, vec![Token::Fn, Token::Node])
}

#[test]
fn source_consisting_only_of_a_comment_should_produce_empty_token_stream() -> Result<(), String> {
    eq(lex("// just a comment")?, vec![])
}

#[test]
fn empty_input_should_produce_empty_token_stream() -> Result<(), String> {
    eq(lex("")?, vec![])
}

// ── §1  Span positions ────────────────────────────────────────────────────────

#[test]
fn first_token_should_start_at_line_1_col_1() -> Result<(), String> {
    let toks = lex_spans("fn")?;
    let (_, span) = token_at(&toks, 0)?;
    eq(*span, Span { line: 1, col: 1 })
}

#[test]
fn second_token_column_should_be_after_first_token_and_space() -> Result<(), String> {
    // "fn node" — "node" starts at col 4 (col 1 + len("fn") + 1 space)
    let toks = lex_spans("fn node")?;
    let (_, span) = token_at(&toks, 1)?;
    eq(*span, Span { line: 1, col: 4 })
}

#[test]
fn token_on_second_line_should_have_line_2() -> Result<(), String> {
    let toks = lex_spans("fn\nnode")?;
    let (_, span) = token_at(&toks, 1)?;
    eq(*span, Span { line: 2, col: 1 })
}

#[test]
fn token_after_comment_should_be_on_next_line() -> Result<(), String> {
    let toks = lex_spans("fn // comment\nnode")?;
    let (_, span) = token_at(&toks, 1)?;
    eq(*span, Span { line: 2, col: 1 })
}

// ── §1  Lex errors ────────────────────────────────────────────────────────────

#[test]
fn unterminated_string_literal_should_be_an_error() -> Result<(), String> {
    msg_contains(&expect_lex_err("\"unterminated")?, "unterminated string")
}

#[test]
fn string_literal_containing_a_bare_newline_should_be_an_error() -> Result<(), String> {
    msg_contains(&expect_lex_err("\"line one\nline two\"")?, "unterminated string")
}

#[test]
fn unexpected_character_should_be_an_error() -> Result<(), String> {
    msg_contains(&expect_lex_err("@")?, "unexpected character")
}

#[test]
fn hex_prefix_without_digits_should_be_an_error() -> Result<(), String> {
    msg_contains(&expect_lex_err("0x")?, "0x")
}

#[test]
fn hex_prefix_followed_by_non_hex_character_should_be_an_error() -> Result<(), String> {
    msg_contains(&expect_lex_err("0xg")?, "0x")
}

#[test]
fn error_message_should_include_line_and_column() -> Result<(), String> {
    let msg = expect_lex_err("fn @")?;
    msg_contains(&msg, "line")?;
    msg_contains(&msg, "col")
}
