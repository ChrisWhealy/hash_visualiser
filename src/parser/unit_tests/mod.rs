mod ebnf_02_tests;
mod ebnf_03_tests;
mod ebnf_04_tests;
mod ebnf_05_tests;
mod ebnf_06_tests;
mod ebnf_07_tests;
mod ebnf_08_tests;
mod ebnf_09_tests;
mod ebnf_10_tests;
mod ebnf_11_tests;

use std::fmt;
use super::Parser;
use crate::{
    ast::{
        ebnf_02::{Program, TopItem},
        ebnf_03::ContextItem,
        ebnf_04::{Param, Type},
        ebnf_06::{NodeKind, PropValue},
        ebnf_07::WireEndpoint,
        ebnf_08::{ArrangeMode, FlowDirection, GroupItem},
        ebnf_10::{AnimateSpec, Effect, EmitTarget, RerouteDir, SetEffect},
        ebnf_11::{BinOp, Expr},
    },
    lexer::Lexer,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Comparison helpers
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

fn params_eq(actual: &[String], expected: &[&str]) -> Result<(), String> {
    let refs: Vec<&str> = actual.iter().map(String::as_str).collect();
    if refs == expected {
        Ok(())
    } else {
        Err(format!("expected params {expected:?}, got {refs:?}"))
    }
}

// Compares typed function parameters against expected (name, type) pairs.
fn fn_params_eq(actual: &[Param], expected: &[(&str, Type)]) -> Result<(), String> {
    let got: Vec<(&str, &Type)> = actual.iter().map(|p| (p.name.as_str(), &p.ty)).collect();
    let want: Vec<(&str, &Type)> = expected.iter().map(|(n, t)| (*n, t)).collect();
    if got == want {
        Ok(())
    } else {
        Err(format!("expected params {want:?}, got {got:?}"))
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Parser helpers
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn make_parser(src: &str) -> Result<Parser, String> {
    Lexer::new(src)
        .tokenise()
        .map(Parser::new)
        .map_err(|e| e.to_string())
}

fn parse(src: &str) -> Result<Program, String> {
    crate::parse(src).map_err(|e| e.to_string())
}

// Returns the error message when the input is expected to fail parsing.
fn expect_parse_err(src: &str) -> Result<String, String> {
    match crate::parse(src) {
        Err(e) => Ok(e.to_string()),
        Ok(_) => Err(format!("expected parse error for {src:?}, but parsing succeeded")),
    }
}

// Parses src as a complete program and extracts the body of the first fn def.
fn expr_of(src: &str) -> Result<Expr, String> {
    match parse(src)?.items.into_iter().next() {
        Some(TopItem::FnDef(f)) => Ok(f.body),
        Some(other) => Err(format!("expected FnDef at top level, got {other:?}")),
        None => Err("empty program — expected at least one item".into()),
    }
}
