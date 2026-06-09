use std::fmt;
use super::Parser;
use crate::{
    ast::{
        ebnf_02::{Program, TopItem},
        ebnf_03::ContextItem,
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §2  Top-level structure
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
mod ebnf_02 {
    use super::*;

    #[test]
    fn empty_program_should_have_no_items() -> Result<(), String> {
        eq(parse("")?.items.len(), 0)
    }

    #[test]
    fn program_should_accept_context_block_at_top_level() -> Result<(), String> {
        let prog = parse("context {}")?;
        match prog.items.first() {
            Some(TopItem::Context(_)) => Ok(()),
            other => Err(format!("expected TopItem::Context, got {other:?}")),
        }
    }

    #[test]
    fn program_should_accept_fn_def_at_top_level() -> Result<(), String> {
        let prog = parse("fn f() = 1")?;
        match prog.items.first() {
            Some(TopItem::FnDef(_)) => Ok(()),
            other => Err(format!("expected TopItem::FnDef, got {other:?}")),
        }
    }

    #[test]
    fn program_should_accept_hash_block_at_top_level() -> Result<(), String> {
        let prog = parse("hash SHA256 {}")?;
        match prog.items.first() {
            Some(TopItem::Hash(_)) => Ok(()),
            other => Err(format!("expected TopItem::Hash, got {other:?}")),
        }
    }

    #[test]
    fn program_should_accept_node_decl_at_top_level() -> Result<(), String> {
        let prog = parse("node a : register {}")?;
        match prog.items.first() {
            Some(TopItem::Node(_)) => Ok(()),
            other => Err(format!("expected TopItem::Node, got {other:?}")),
        }
    }

    #[test]
    fn program_should_accept_wire_decl_at_top_level() -> Result<(), String> {
        let prog = parse("wire a -> b")?;
        match prog.items.first() {
            Some(TopItem::Wire(_)) => Ok(()),
            other => Err(format!("expected TopItem::Wire, got {other:?}")),
        }
    }

    #[test]
    fn program_should_accept_group_decl_at_top_level() -> Result<(), String> {
        let prog = parse("group g { contains: [a] }")?;
        match prog.items.first() {
            Some(TopItem::Group(_)) => Ok(()),
            other => Err(format!("expected TopItem::Group, got {other:?}")),
        }
    }

    #[test]
    fn program_should_accept_layout_decl_at_top_level() -> Result<(), String> {
        let prog = parse("layout: left_to_right")?;
        match prog.items.first() {
            Some(TopItem::Layout(_)) => Ok(()),
            other => Err(format!("expected TopItem::Layout, got {other:?}")),
        }
    }

    #[test]
    fn program_should_accept_event_handler_at_top_level() -> Result<(), String> {
        let prog = parse("a on receive() {}")?;
        match prog.items.first() {
            Some(TopItem::EventHandler(_)) => Ok(()),
            other => Err(format!("expected TopItem::EventHandler, got {other:?}")),
        }
    }

    #[test]
    fn program_should_accept_multiple_items() -> Result<(), String> {
        let prog = parse("context {} hash SHA256 {} node a : register {}")?;
        eq(prog.items.len(), 3)
    }

    #[test]
    fn unexpected_token_at_top_level_should_be_an_error() -> Result<(), String> {
        msg_contains(&expect_parse_err("+")?, "unexpected token")
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §3  Context block
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
mod ebnf_03 {
    use super::*;

    #[test]
    fn empty_context_block_should_have_no_items() -> Result<(), String> {
        let ctx = make_parser("context {}")?.parse_context_block().map_err(|e| e.to_string())?;
        eq(ctx.items.len(), 0)
    }

    #[test]
    fn context_block_should_accept_word_size() -> Result<(), String> {
        let ctx = make_parser("context { word_size: 32 }")?
            .parse_context_block().map_err(|e| e.to_string())?;
        eq(ctx.items.len(), 1)?;
        match &ctx.items[0] {
            ContextItem::WordSize(32) => Ok(()),
            other => Err(format!("expected WordSize(32), got {other:?}")),
        }
    }

    #[test]
    fn context_block_should_accept_fn_def() -> Result<(), String> {
        let ctx = make_parser("context { fn f() = 1 }")?
            .parse_context_block().map_err(|e| e.to_string())?;
        match ctx.items.first() {
            Some(ContextItem::FnDef(_)) => Ok(()),
            other => Err(format!("expected FnDef context item, got {other:?}")),
        }
    }

    #[test]
    fn context_block_should_accept_word_size_and_fn_def() -> Result<(), String> {
        let ctx = make_parser("context { word_size: 64 fn f() = 1 }")?
            .parse_context_block().map_err(|e| e.to_string())?;
        eq(ctx.items.len(), 2)?;
        match &ctx.items[0] {
            ContextItem::WordSize(64) => {}
            other => return Err(format!("expected WordSize(64), got {other:?}")),
        }
        match &ctx.items[1] {
            ContextItem::FnDef(_) => Ok(()),
            other => Err(format!("expected FnDef, got {other:?}")),
        }
    }

    #[test]
    fn node_decl_inside_context_block_should_be_an_error() -> Result<(), String> {
        let err = make_parser("context { node a : register {} }")?
            .parse_context_block().unwrap_err();
        if err.message.contains("word_size") || err.message.contains("fn") {
            Ok(())
        } else {
            Err(format!("expected error to mention `word_size` or `fn`, got: {}", err.message))
        }
    }

    #[test]
    fn unterminated_context_block_should_be_an_error() -> Result<(), String> {
        let err = make_parser("context {")?
            .parse_context_block().unwrap_err();
        if err.message.contains("terminated") || err.message.contains("}") {
            Ok(())
        } else {
            Err(format!("expected error about unterminated block, got: {}", err.message))
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §4  Function definitions
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
mod ebnf_04 {
    use super::*;

    #[test]
    fn fn_with_no_params_should_parse() -> Result<(), String> {
        let f = make_parser("fn f() = 1")?.parse_fn_def().map_err(|e| e.to_string())?;
        eq(f.name.as_str(), "f")?;
        eq(f.params.len(), 0)?;
        match f.body {
            Expr::Integer(1) => Ok(()),
            other => Err(format!("expected Integer(1) body, got {other:?}")),
        }
    }

    #[test]
    fn fn_with_one_param_should_parse() -> Result<(), String> {
        let f = make_parser("fn f(x) = x")?.parse_fn_def().map_err(|e| e.to_string())?;
        params_eq(&f.params, &["x"])?;
        match f.body {
            Expr::Ident(ref s) if s == "x" => Ok(()),
            other => Err(format!("expected Ident(\"x\") body, got {other:?}")),
        }
    }

    #[test]
    fn fn_with_multiple_params_should_parse() -> Result<(), String> {
        let f = make_parser("fn Sigma(e, r1, r2, r3) = e rotr_u r1")?
            .parse_fn_def().map_err(|e| e.to_string())?;
        eq(f.name.as_str(), "Sigma")?;
        params_eq(&f.params, &["e", "r1", "r2", "r3"])
    }

    #[test]
    fn fn_params_should_allow_trailing_comma() -> Result<(), String> {
        let f = make_parser("fn f(a, b,) = a")?.parse_fn_def().map_err(|e| e.to_string())?;
        params_eq(&f.params, &["a", "b"])
    }

    #[test]
    fn fn_with_missing_equals_should_be_an_error() -> Result<(), String> {
        let err = make_parser("fn f() x")?.parse_fn_def().unwrap_err();
        msg_contains(&err.message, "`=`")
    }

    #[test]
    fn fn_with_missing_open_paren_should_be_an_error() -> Result<(), String> {
        let err = make_parser("fn f x")?.parse_fn_def().unwrap_err();
        msg_contains(&err.message, "`(`")
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §5  Hash block
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
mod ebnf_05 {
    use super::*;

    #[test]
    fn empty_hash_block_should_parse() -> Result<(), String> {
        let h = make_parser("hash SHA256 {}")?.parse_hash_block().map_err(|e| e.to_string())?;
        eq(h.name.as_str(), "SHA256")?;
        eq(h.items.len(), 0)
    }

    #[test]
    fn hash_block_should_accept_context_and_nodes() -> Result<(), String> {
        let h = make_parser("hash SHA256 { context { word_size: 32 } node a : register {} }")?
            .parse_hash_block().map_err(|e| e.to_string())?;
        eq(h.items.len(), 2)
    }

    #[test]
    fn hash_block_missing_name_should_be_an_error() -> Result<(), String> {
        let err = make_parser("hash {}")?.parse_hash_block().unwrap_err();
        msg_contains(&err.message, "identifier")
    }

    #[test]
    fn unterminated_hash_block_should_be_an_error() -> Result<(), String> {
        let err = make_parser("hash SHA256 {")?.parse_hash_block().unwrap_err();
        if err.message.contains("terminated") || err.message.contains("}") {
            Ok(())
        } else {
            Err(format!("expected error about unterminated block, got: {}", err.message))
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §6  Node declarations
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
mod ebnf_06 {
    use super::*;

    #[test]
    fn node_with_register_kind_should_parse() -> Result<(), String> {
        let n = make_parser("node a : register {}")?.parse_node_decl().map_err(|e| e.to_string())?;
        eq(n.name.as_str(), "a")?;
        match n.kind {
            NodeKind::Register => Ok(()),
            other => Err(format!("expected Register kind, got {other:?}")),
        }
    }

    #[test]
    fn node_with_operation_kind_should_parse() -> Result<(), String> {
        let n = make_parser("node s1 : operation {}")?.parse_node_decl().map_err(|e| e.to_string())?;
        match n.kind {
            NodeKind::Operation => Ok(()),
            other => Err(format!("expected Operation kind, got {other:?}")),
        }
    }

    #[test]
    fn node_with_constant_kind_should_parse() -> Result<(), String> {
        let n = make_parser("node k : constant {}")?.parse_node_decl().map_err(|e| e.to_string())?;
        match n.kind {
            NodeKind::Constant => Ok(()),
            other => Err(format!("expected Constant kind, got {other:?}")),
        }
    }

    #[test]
    fn node_with_button_kind_should_parse() -> Result<(), String> {
        let n = make_parser("node btn : button {}")?.parse_node_decl().map_err(|e| e.to_string())?;
        match n.kind {
            NodeKind::Button => Ok(()),
            other => Err(format!("expected Button kind, got {other:?}")),
        }
    }

    #[test]
    fn node_with_user_defined_kind_should_parse() -> Result<(), String> {
        let n = make_parser("node x : mux {}")?.parse_node_decl().map_err(|e| e.to_string())?;
        match n.kind {
            NodeKind::User(ref s) if s == "mux" => Ok(()),
            other => Err(format!("expected User(\"mux\") kind, got {other:?}")),
        }
    }

    #[test]
    fn node_with_no_properties_should_parse() -> Result<(), String> {
        let n = make_parser("node a : register {}")?.parse_node_decl().map_err(|e| e.to_string())?;
        eq(n.properties.len(), 0)
    }

    #[test]
    fn node_with_string_property_should_parse() -> Result<(), String> {
        let n = make_parser("node a : register { label: \"a\" }")?
            .parse_node_decl().map_err(|e| e.to_string())?;
        eq(n.properties.len(), 1)?;
        eq(n.properties[0].name.as_str(), "label")?;
        match &n.properties[0].value {
            PropValue::Str(s) if s == "a" => Ok(()),
            other => Err(format!("expected PropValue::Str(\"a\"), got {other:?}")),
        }
    }

    #[test]
    fn node_with_identifier_property_should_parse() -> Result<(), String> {
        let n = make_parser("node a : register { format: hex32 }")?
            .parse_node_decl().map_err(|e| e.to_string())?;
        match &n.properties[0].value {
            PropValue::Expr(Expr::Ident(s)) if s == "hex32" => Ok(()),
            other => Err(format!("expected PropValue::Expr(Ident(\"hex32\")), got {other:?}")),
        }
    }

    #[test]
    fn node_with_multiple_properties_should_parse() -> Result<(), String> {
        let n = make_parser("node a : register { label: \"a\", format: hex32 }")?
            .parse_node_decl().map_err(|e| e.to_string())?;
        eq(n.properties.len(), 2)
    }

    #[test]
    fn node_property_list_should_allow_trailing_comma() -> Result<(), String> {
        let n = make_parser("node a : register { label: \"a\", format: hex32, }")?
            .parse_node_decl().map_err(|e| e.to_string())?;
        eq(n.properties.len(), 2)
    }

    #[test]
    fn node_layout_auto_property_should_parse() -> Result<(), String> {
        let n = make_parser("node a : register { layout: auto }")?
            .parse_node_decl().map_err(|e| e.to_string())?;
        match &n.properties[0].value {
            PropValue::Expr(Expr::Ident(s)) if s == "auto" => Ok(()),
            other => Err(format!("expected PropValue::Expr(Ident(\"auto\")), got {other:?}")),
        }
    }

    #[test]
    fn node_layout_pinned_property_should_parse() -> Result<(), String> {
        let n = make_parser("node a : register { layout: pinned(10, 20) }")?
            .parse_node_decl().map_err(|e| e.to_string())?;
        match &n.properties[0].value {
            PropValue::Expr(Expr::Call { name, args }) if name == "pinned" => eq(args.len(), 2),
            other => Err(format!("expected PropValue::Expr(Call{{pinned, [10, 20]}}), got {other:?}")),
        }
    }

    #[test]
    fn node_missing_kind_separator_should_be_an_error() -> Result<(), String> {
        let err = make_parser("node a {}")?.parse_node_decl().unwrap_err();
        msg_contains(&err.message, "`:`")
    }

    #[test]
    fn node_missing_body_brace_should_be_an_error() -> Result<(), String> {
        let err = make_parser("node a : register")?.parse_node_decl().unwrap_err();
        msg_contains(&err.message, "`{`")
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §7  Wire declarations
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
mod ebnf_07 {
    use super::*;

    #[test]
    fn unnamed_wire_should_parse() -> Result<(), String> {
        let w = make_parser("wire a -> b")?.parse_wire_decl().map_err(|e| e.to_string())?;
        eq(w.name.as_deref(), None)?;
        match &w.source {
            WireEndpoint::Node(s) if s == "a" => {}
            other => return Err(format!("expected source Node(\"a\"), got {other:?}")),
        }
        match &w.target {
            WireEndpoint::Node(s) if s == "b" => Ok(()),
            other => Err(format!("expected target Node(\"b\"), got {other:?}")),
        }
    }

    #[test]
    fn named_wire_should_parse() -> Result<(), String> {
        let w = make_parser("wire carry: a -> b")?.parse_wire_decl().map_err(|e| e.to_string())?;
        eq(w.name.as_deref(), Some("carry"))
    }

    #[test]
    fn wire_with_open_source_endpoint_should_parse() -> Result<(), String> {
        let w = make_parser("wire ? -> b")?.parse_wire_decl().map_err(|e| e.to_string())?;
        match w.source {
            WireEndpoint::Open => Ok(()),
            other => Err(format!("expected Open source endpoint, got {other:?}")),
        }
    }

    #[test]
    fn wire_with_open_target_endpoint_should_parse() -> Result<(), String> {
        let w = make_parser("wire a -> ?")?.parse_wire_decl().map_err(|e| e.to_string())?;
        match w.target {
            WireEndpoint::Open => Ok(()),
            other => Err(format!("expected Open target endpoint, got {other:?}")),
        }
    }

    #[test]
    fn named_wire_with_open_source_should_parse() -> Result<(), String> {
        let w = make_parser("wire w1: ? -> dest")?.parse_wire_decl().map_err(|e| e.to_string())?;
        eq(w.name.as_deref(), Some("w1"))?;
        match w.source {
            WireEndpoint::Open => Ok(()),
            other => Err(format!("expected Open source, got {other:?}")),
        }
    }

    #[test]
    fn wire_missing_arrow_should_be_an_error() -> Result<(), String> {
        let err = make_parser("wire a b")?.parse_wire_decl().unwrap_err();
        msg_contains(&err.message, "`->`")
    }

    #[test]
    fn wire_missing_target_should_be_an_error() -> Result<(), String> {
        let err = make_parser("wire a ->")?.parse_wire_decl().unwrap_err();
        if err.message.contains("endpoint") || err.message.contains("end of input") {
            Ok(())
        } else {
            Err(format!("expected error about missing endpoint, got: {}", err.message))
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §8  Group and layout declarations
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
mod ebnf_08 {
    use super::*;

    #[test]
    fn group_with_contains_should_parse() -> Result<(), String> {
        let g = make_parser("group g { contains: [a, b, c] }")?
            .parse_group_decl().map_err(|e| e.to_string())?;
        eq(g.name.as_str(), "g")?;
        match &g.items[0] {
            GroupItem::Contains(names) => {
                let refs: Vec<&str> = names.iter().map(String::as_str).collect();
                eq(refs, vec!["a", "b", "c"])
            }
            other => Err(format!("expected Contains item, got {other:?}")),
        }
    }

    #[test]
    fn group_contains_should_allow_trailing_comma() -> Result<(), String> {
        let g = make_parser("group g { contains: [a, b,] }")?
            .parse_group_decl().map_err(|e| e.to_string())?;
        match &g.items[0] {
            GroupItem::Contains(names) => eq(names.len(), 2),
            other => Err(format!("expected Contains item, got {other:?}")),
        }
    }

    #[test]
    fn group_with_arrange_grid_should_parse() -> Result<(), String> {
        let g = make_parser("group g { arrange: grid }")?
            .parse_group_decl().map_err(|e| e.to_string())?;
        match &g.items[0] {
            GroupItem::Arrange(ArrangeMode::Grid) => Ok(()),
            other => Err(format!("expected Arrange(Grid), got {other:?}")),
        }
    }

    #[test]
    fn group_with_arrange_horizontal_should_parse() -> Result<(), String> {
        let g = make_parser("group g { arrange: horizontal }")?
            .parse_group_decl().map_err(|e| e.to_string())?;
        match &g.items[0] {
            GroupItem::Arrange(ArrangeMode::Horizontal) => Ok(()),
            other => Err(format!("expected Arrange(Horizontal), got {other:?}")),
        }
    }

    #[test]
    fn group_with_arrange_vertical_should_parse() -> Result<(), String> {
        let g = make_parser("group g { arrange: vertical }")?
            .parse_group_decl().map_err(|e| e.to_string())?;
        match &g.items[0] {
            GroupItem::Arrange(ArrangeMode::Vertical) => Ok(()),
            other => Err(format!("expected Arrange(Vertical), got {other:?}")),
        }
    }

    #[test]
    fn layout_left_to_right_should_parse() -> Result<(), String> {
        eq(
            make_parser("layout: left_to_right")?.parse_layout_decl().map_err(|e| e.to_string())?,
            FlowDirection::LeftToRight,
        )
    }

    #[test]
    fn layout_top_to_bottom_should_parse() -> Result<(), String> {
        eq(
            make_parser("layout: top_to_bottom")?.parse_layout_decl().map_err(|e| e.to_string())?,
            FlowDirection::TopToBottom,
        )
    }

    #[test]
    fn layout_right_to_left_should_parse() -> Result<(), String> {
        eq(
            make_parser("layout: right_to_left")?.parse_layout_decl().map_err(|e| e.to_string())?,
            FlowDirection::RightToLeft,
        )
    }

    #[test]
    fn layout_bottom_to_top_should_parse() -> Result<(), String> {
        eq(
            make_parser("layout: bottom_to_top")?.parse_layout_decl().map_err(|e| e.to_string())?,
            FlowDirection::BottomToTop,
        )
    }

    #[test]
    fn invalid_flow_direction_should_be_an_error() -> Result<(), String> {
        let err = make_parser("layout: diagonal")?.parse_layout_decl().unwrap_err();
        msg_contains(&err.message, "flow direction")
    }

    #[test]
    fn invalid_arrange_mode_should_be_an_error() -> Result<(), String> {
        let err = make_parser("group g { arrange: diagonal }")?.parse_group_decl().unwrap_err();
        msg_contains(&err.message, "arrange mode")
    }

    #[test]
    fn unknown_group_item_keyword_should_be_an_error() -> Result<(), String> {
        let err = make_parser("group g { emit: foo }")?.parse_group_decl().unwrap_err();
        if err.message.contains("contains") || err.message.contains("arrange") {
            Ok(())
        } else {
            Err(format!("expected error mentioning `contains` or `arrange`, got: {}", err.message))
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §9  Event handlers
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
mod ebnf_09 {
    use super::*;

    #[test]
    fn handler_with_no_params_should_parse() -> Result<(), String> {
        let h = make_parser("a on receive() {}")?
            .parse_event_handler().map_err(|e| e.to_string())?;
        eq(h.node.as_str(), "a")?;
        eq(h.event.as_str(), "receive")?;
        eq(h.params.len(), 0)?;
        eq(h.body.len(), 0)
    }

    #[test]
    fn handler_with_one_param_should_parse() -> Result<(), String> {
        let h = make_parser("a on receive(value) {}")?
            .parse_event_handler().map_err(|e| e.to_string())?;
        params_eq(&h.params, &["value"])
    }

    #[test]
    fn handler_with_multiple_params_should_parse() -> Result<(), String> {
        let h = make_parser("a on receive(e, f, g) {}")?
            .parse_event_handler().map_err(|e| e.to_string())?;
        params_eq(&h.params, &["e", "f", "g"])
    }

    #[test]
    fn handler_params_should_allow_trailing_comma() -> Result<(), String> {
        let h = make_parser("a on receive(x, y,) {}")?
            .parse_event_handler().map_err(|e| e.to_string())?;
        params_eq(&h.params, &["x", "y"])
    }

    #[test]
    fn handler_should_accept_reroute_as_event_name() -> Result<(), String> {
        // reroute is a keyword that is also a valid built-in event name
        let h = make_parser("w1 on reroute(new_src) {}")?
            .parse_event_handler().map_err(|e| e.to_string())?;
        eq(h.event.as_str(), "reroute")
    }

    #[test]
    fn handler_with_multiple_effects_should_parse() -> Result<(), String> {
        let h = make_parser("a on receive(v) { let r = v set value }")?
            .parse_event_handler().map_err(|e| e.to_string())?;
        eq(h.body.len(), 2)
    }

    #[test]
    fn handler_missing_on_keyword_should_be_an_error() -> Result<(), String> {
        let err = make_parser("a receive() {}")?.parse_event_handler().unwrap_err();
        msg_contains(&err.message, "`on`")
    }

    #[test]
    fn handler_missing_open_paren_should_be_an_error() -> Result<(), String> {
        let err = make_parser("a on receive {}")?.parse_event_handler().unwrap_err();
        msg_contains(&err.message, "`(`")
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §10  Effects
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
mod ebnf_10 {
    use super::*;

    #[test]
    fn set_prop_assign_should_parse() -> Result<(), String> {
        match make_parser("set label: incoming")?.parse_effect().map_err(|e| e.to_string())? {
            Effect::Set(SetEffect::Prop { name, .. }) => eq(name.as_str(), "label"),
            other => Err(format!("expected Set(Prop), got {other:?}")),
        }
    }

    #[test]
    fn set_var_assign_should_parse() -> Result<(), String> {
        match make_parser("set result = a xor b")?.parse_effect().map_err(|e| e.to_string())? {
            Effect::Set(SetEffect::Var { name, .. }) => eq(name.as_str(), "result"),
            other => Err(format!("expected Set(Var), got {other:?}")),
        }
    }

    #[test]
    fn set_bare_ident_should_parse() -> Result<(), String> {
        match make_parser("set value")?.parse_effect().map_err(|e| e.to_string())? {
            Effect::Set(SetEffect::Bare(name)) => eq(name.as_str(), "value"),
            other => Err(format!("expected Set(Bare), got {other:?}")),
        }
    }

    #[test]
    fn let_binding_should_parse() -> Result<(), String> {
        match make_parser("let r = a rotr_u 6")?.parse_effect().map_err(|e| e.to_string())? {
            Effect::Let(b) => {
                eq(b.name.as_str(), "r")?;
                match b.value {
                    Expr::BinOp { op: BinOp::RotrU, .. } => Ok(()),
                    other => Err(format!("expected BinOp(RotrU), got {other:?}")),
                }
            }
            other => Err(format!("expected Let, got {other:?}")),
        }
    }

    #[test]
    fn animate_fill_pulse_should_parse() -> Result<(), String> {
        match make_parser("animate fill: pulse \"gold\" for 250ms")?
            .parse_effect().map_err(|e| e.to_string())?
        {
            Effect::Animate(a) => match a.spec {
                AnimateSpec::FillPulse { colour, duration } => {
                    eq(colour.as_str(), "gold")?;
                    eq(duration.value, 250u64)
                }
                other => Err(format!("expected FillPulse, got {other:?}")),
            },
            other => Err(format!("expected Animate, got {other:?}")),
        }
    }

    #[test]
    fn animate_prop_transition_should_parse() -> Result<(), String> {
        match make_parser("animate opacity from 0 to 1 over 300ms")?
            .parse_effect().map_err(|e| e.to_string())?
        {
            Effect::Animate(a) => match a.spec {
                AnimateSpec::Transition { prop, duration, .. } => {
                    eq(prop.as_str(), "opacity")?;
                    eq(duration.value, 300u64)
                }
                other => Err(format!("expected Transition, got {other:?}")),
            },
            other => Err(format!("expected Animate, got {other:?}")),
        }
    }

    #[test]
    fn animate_with_wrong_pulse_keyword_should_be_an_error() -> Result<(), String> {
        let err = make_parser("animate fill: flash \"red\" for 100ms")?
            .parse_effect().unwrap_err();
        msg_contains(&err.message, "pulse")
    }

    #[test]
    fn emit_with_no_target_should_parse() -> Result<(), String> {
        match make_parser("emit forward(v)")?.parse_effect().map_err(|e| e.to_string())? {
            Effect::Emit(e) => {
                eq(e.event.as_str(), "forward")?;
                eq(e.args.len(), 1)?;
                eq(e.target.is_none(), true)
            }
            other => Err(format!("expected Emit, got {other:?}")),
        }
    }

    #[test]
    fn emit_broadcast_to_all_should_parse() -> Result<(), String> {
        match make_parser("emit step(1) -> all")?.parse_effect().map_err(|e| e.to_string())? {
            Effect::Emit(e) => match e.target {
                Some(EmitTarget::All) => Ok(()),
                other => Err(format!("expected EmitTarget::All, got {other:?}")),
            },
            other => Err(format!("expected Emit, got {other:?}")),
        }
    }

    #[test]
    fn emit_to_named_node_should_parse() -> Result<(), String> {
        match make_parser("emit forward(v) -> sink")?.parse_effect().map_err(|e| e.to_string())? {
            Effect::Emit(e) => match e.target {
                Some(EmitTarget::Node(ref s)) if s == "sink" => Ok(()),
                other => Err(format!("expected EmitTarget::Node(\"sink\"), got {other:?}")),
            },
            other => Err(format!("expected Emit, got {other:?}")),
        }
    }

    #[test]
    fn emit_via_named_wire_should_parse() -> Result<(), String> {
        match make_parser("emit forward(v) via carry")?.parse_effect().map_err(|e| e.to_string())? {
            Effect::Emit(e) => match e.target {
                Some(EmitTarget::Via(ref s)) if s == "carry" => Ok(()),
                other => Err(format!("expected EmitTarget::Via(\"carry\"), got {other:?}")),
            },
            other => Err(format!("expected Emit, got {other:?}")),
        }
    }

    #[test]
    fn emit_with_multiple_args_should_parse() -> Result<(), String> {
        match make_parser("emit send(a, b, c)")?.parse_effect().map_err(|e| e.to_string())? {
            Effect::Emit(e) => eq(e.args.len(), 3),
            other => Err(format!("expected Emit, got {other:?}")),
        }
    }

    #[test]
    fn reroute_to_should_parse() -> Result<(), String> {
        match make_parser("reroute w1 to dest")?.parse_effect().map_err(|e| e.to_string())? {
            Effect::Reroute(r) => {
                eq(r.wire.as_str(), "w1")?;
                eq(r.direction, RerouteDir::To)?;
                eq(r.node.as_str(), "dest")
            }
            other => Err(format!("expected Reroute, got {other:?}")),
        }
    }

    #[test]
    fn reroute_from_should_parse() -> Result<(), String> {
        match make_parser("reroute w1 from src")?.parse_effect().map_err(|e| e.to_string())? {
            Effect::Reroute(r) => eq(r.direction, RerouteDir::From),
            other => Err(format!("expected Reroute, got {other:?}")),
        }
    }

    #[test]
    fn reroute_with_invalid_direction_should_be_an_error() -> Result<(), String> {
        let err = make_parser("reroute w1 above dest")?.parse_effect().unwrap_err();
        if err.message.contains("`to`") || err.message.contains("`from`") {
            Ok(())
        } else {
            Err(format!("expected error mentioning `to` or `from`, got: {}", err.message))
        }
    }

    #[test]
    fn unknown_effect_keyword_should_be_an_error() -> Result<(), String> {
        // `node` is not a valid effect keyword
        let err = make_parser("node a : register {}")?.parse_effect().unwrap_err();
        msg_contains(&err.message, "effect")
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §11  Expressions
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
mod ebnf_11 {
    use super::*;

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    // §11  Primary expressions
    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    #[test]
    fn integer_literal_should_parse() -> Result<(), String> {
        match expr_of("fn f() = 42")? {
            Expr::Integer(42) => Ok(()),
            other => Err(format!("expected Integer(42), got {other:?}")),
        }
    }

    #[test]
    fn hex_literal_should_parse() -> Result<(), String> {
        match expr_of("fn f() = 0xff")? {
            Expr::HexLit(0xff) => Ok(()),
            other => Err(format!("expected HexLit(0xff), got {other:?}")),
        }
    }

    #[test]
    fn identifier_reference_should_parse() -> Result<(), String> {
        match expr_of("fn f() = x")? {
            Expr::Ident(ref s) if s == "x" => Ok(()),
            other => Err(format!("expected Ident(\"x\"), got {other:?}")),
        }
    }

    #[test]
    fn function_call_with_no_args_should_parse() -> Result<(), String> {
        match expr_of("fn f() = g()")? {
            Expr::Call { name, args } => {
                eq(name.as_str(), "g")?;
                eq(args.len(), 0)
            }
            other => Err(format!("expected Call, got {other:?}")),
        }
    }

    #[test]
    fn function_call_with_one_arg_should_parse() -> Result<(), String> {
        match expr_of("fn f() = g(x)")? {
            Expr::Call { args, .. } => eq(args.len(), 1),
            other => Err(format!("expected Call, got {other:?}")),
        }
    }

    #[test]
    fn function_call_with_multiple_args_should_parse() -> Result<(), String> {
        match expr_of("fn f() = Sigma(e, 6, 11, 25)")? {
            Expr::Call { name, args } => {
                eq(name.as_str(), "Sigma")?;
                eq(args.len(), 4)
            }
            other => Err(format!("expected Call, got {other:?}")),
        }
    }

    #[test]
    fn parenthesised_expression_should_be_transparent() -> Result<(), String> {
        // (x) must not introduce a new AST node — it should yield just Ident("x")
        match expr_of("fn f() = (x)")? {
            Expr::Ident(_) => Ok(()),
            other => Err(format!("expected Ident (parens are transparent), got {other:?}")),
        }
    }

    #[test]
    fn unary_not_should_parse() -> Result<(), String> {
        match expr_of("fn f() = not x")? {
            Expr::Not(_) => Ok(()),
            other => Err(format!("expected Not, got {other:?}")),
        }
    }

    #[test]
    fn double_not_should_nest() -> Result<(), String> {
        match expr_of("fn f() = not not x")? {
            Expr::Not(inner) => match *inner {
                Expr::Not(_) => Ok(()),
                other => Err(format!("expected inner Not, got {other:?}")),
            },
            other => Err(format!("expected outer Not, got {other:?}")),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    // §11  All binary operators
    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    macro_rules! binop_test {
        ($name:ident, $src:literal, $expected:expr) => {
            #[test]
            fn $name() -> Result<(), String> {
                match expr_of(concat!("fn f() = a ", $src, " b"))? {
                    Expr::BinOp { op, .. } => eq(op, $expected),
                    other => Err(format!(
                        "expected BinOp with op {:?}, got {other:?}",
                        $expected
                    )),
                }
            }
        };
    }

    binop_test!(binop_or,     "or",     BinOp::Or);
    binop_test!(binop_xor,    "xor",    BinOp::Xor);
    binop_test!(binop_and,    "and",    BinOp::And);
    binop_test!(binop_add,    "+",      BinOp::Add);
    binop_test!(binop_sub,    "-",      BinOp::Sub);
    binop_test!(binop_shl,    "shl",    BinOp::Shl);
    binop_test!(binop_shr_u,  "shr_u",  BinOp::ShrU);
    binop_test!(binop_shr_s,  "shr_s",  BinOp::ShrS);
    binop_test!(binop_rotr_u, "rotr_u", BinOp::RotrU);
    binop_test!(binop_rotr_s, "rotr_s", BinOp::RotrS);
    binop_test!(binop_rotl_u, "rotl_u", BinOp::RotlU);
    binop_test!(binop_rotl_s, "rotl_s", BinOp::RotlS);

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    // §11  Operator precedence
    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    #[test]
    fn xor_should_bind_tighter_than_or() -> Result<(), String> {
        // a or b xor c  →  a or (b xor c)
        match expr_of("fn f() = a or b xor c")? {
            Expr::BinOp { op: BinOp::Or, lhs, rhs } => {
                match *lhs {
                    Expr::Ident(_) => {}
                    other => return Err(format!("expected Ident lhs, got {other:?}")),
                }
                match *rhs {
                    Expr::BinOp { op: BinOp::Xor, .. } => Ok(()),
                    other => Err(format!("expected Xor rhs, got {other:?}")),
                }
            }
            other => Err(format!("expected Or at root, got {other:?}")),
        }
    }

    #[test]
    fn and_should_bind_tighter_than_xor() -> Result<(), String> {
        // a xor b and c  →  a xor (b and c)
        match expr_of("fn f() = a xor b and c")? {
            Expr::BinOp { op: BinOp::Xor, rhs, .. } => match *rhs {
                Expr::BinOp { op: BinOp::And, .. } => Ok(()),
                other => Err(format!("expected And rhs, got {other:?}")),
            },
            other => Err(format!("expected Xor at root, got {other:?}")),
        }
    }

    #[test]
    fn add_should_bind_tighter_than_and() -> Result<(), String> {
        // a and b + c  →  a and (b + c)
        match expr_of("fn f() = a and b + c")? {
            Expr::BinOp { op: BinOp::And, rhs, .. } => match *rhs {
                Expr::BinOp { op: BinOp::Add, .. } => Ok(()),
                other => Err(format!("expected Add rhs, got {other:?}")),
            },
            other => Err(format!("expected And at root, got {other:?}")),
        }
    }

    #[test]
    fn shift_should_bind_tighter_than_add() -> Result<(), String> {
        // a shl 2 + b  →  (a shl 2) + b
        match expr_of("fn f() = a shl 2 + b")? {
            Expr::BinOp { op: BinOp::Add, lhs, .. } => match *lhs {
                Expr::BinOp { op: BinOp::Shl, .. } => Ok(()),
                other => Err(format!("expected Shl lhs, got {other:?}")),
            },
            other => Err(format!("expected Add at root, got {other:?}")),
        }
    }

    #[test]
    fn rotation_should_bind_tighter_than_shift() -> Result<(), String> {
        // a rotr_u 6 shl 2  →  (a rotr_u 6) shl 2
        match expr_of("fn f() = a rotr_u 6 shl 2")? {
            Expr::BinOp { op: BinOp::Shl, lhs, .. } => match *lhs {
                Expr::BinOp { op: BinOp::RotrU, .. } => Ok(()),
                other => Err(format!("expected RotrU lhs, got {other:?}")),
            },
            other => Err(format!("expected Shl at root, got {other:?}")),
        }
    }

    #[test]
    fn not_should_bind_tighter_than_rotation() -> Result<(), String> {
        // a rotr_u not b  →  a rotr_u (not b)
        match expr_of("fn f() = a rotr_u not b")? {
            Expr::BinOp { op: BinOp::RotrU, rhs, .. } => match *rhs {
                Expr::Not(_) => Ok(()),
                other => Err(format!("expected Not rhs, got {other:?}")),
            },
            other => Err(format!("expected RotrU at root, got {other:?}")),
        }
    }

    #[test]
    fn parentheses_should_override_precedence() -> Result<(), String> {
        // (a or b) and c  →  BinOp(And, BinOp(Or, a, b), c)
        match expr_of("fn f() = (a or b) and c")? {
            Expr::BinOp { op: BinOp::And, lhs, .. } => match *lhs {
                Expr::BinOp { op: BinOp::Or, .. } => Ok(()),
                other => Err(format!("expected Or lhs (from parens), got {other:?}")),
            },
            other => Err(format!("expected And at root, got {other:?}")),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    // §11  Left-associativity
    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    #[test]
    fn same_level_operator_should_be_left_associative() -> Result<(), String> {
        // a xor b xor c  →  (a xor b) xor c
        match expr_of("fn f() = a xor b xor c")? {
            Expr::BinOp { op: BinOp::Xor, lhs, rhs } => {
                match *lhs {
                    Expr::BinOp { op: BinOp::Xor, .. } => {}
                    other => return Err(format!("expected Xor lhs (left assoc), got {other:?}")),
                }
                match *rhs {
                    Expr::Ident(_) => Ok(()),
                    other => Err(format!("expected Ident rhs, got {other:?}")),
                }
            }
            other => Err(format!("expected Xor at root, got {other:?}")),
        }
    }

    #[test]
    fn addition_should_be_left_associative() -> Result<(), String> {
        // a + b + c  →  (a + b) + c
        match expr_of("fn f() = a + b + c")? {
            Expr::BinOp { op: BinOp::Add, lhs, .. } => match *lhs {
                Expr::BinOp { op: BinOp::Add, .. } => Ok(()),
                other => Err(format!("expected Add lhs (left assoc), got {other:?}")),
            },
            other => Err(format!("expected Add at root, got {other:?}")),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    // §11  Error cases
    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    #[test]
    fn empty_expression_should_be_an_error() -> Result<(), String> {
        msg_contains(&expect_parse_err("fn f() =")?, "end of input")
    }

    #[test]
    fn plus_as_primary_expression_should_be_an_error() -> Result<(), String> {
        msg_contains(&expect_parse_err("fn f() = +")?, "expression")
    }

    #[test]
    fn unclosed_parenthesis_should_be_an_error() -> Result<(), String> {
        msg_contains(&expect_parse_err("fn f() = (a xor b")?, "`)`")
    }
}
