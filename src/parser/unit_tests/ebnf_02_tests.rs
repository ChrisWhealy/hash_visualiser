use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §2  Top-level structure
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_be_no_items_in_empty_program() -> Result<(), String> {
    eq(parse("")?.items.len(), 0)
}

#[test]
fn should_accept_context_block_at_top_level() -> Result<(), String> {
    let prog = parse("context {}")?;
    match prog.items.first() {
        Some(TopItem::Context(_)) => Ok(()),
        other => Err(format!("expected TopItem::Context, got {other:?}")),
    }
}

#[test]
fn should_accept_fn_def_at_top_level() -> Result<(), String> {
    let prog = parse("fn f() = 1")?;
    match prog.items.first() {
        Some(TopItem::FnDef(_)) => Ok(()),
        other => Err(format!("expected TopItem::FnDef, got {other:?}")),
    }
}

#[test]
fn should_accept_hash_block_at_top_level() -> Result<(), String> {
    let prog = parse("hash SHA256 {}")?;
    match prog.items.first() {
        Some(TopItem::Hash(_)) => Ok(()),
        other => Err(format!("expected TopItem::Hash, got {other:?}")),
    }
}

#[test]
fn should_accept_node_decl_at_top_level() -> Result<(), String> {
    let prog = parse("node a : register {}")?;
    match prog.items.first() {
        Some(TopItem::Node(_)) => Ok(()),
        other => Err(format!("expected TopItem::Node, got {other:?}")),
    }
}

#[test]
fn should_accept_wire_decl_at_top_level() -> Result<(), String> {
    let prog = parse("wire a -> b")?;
    match prog.items.first() {
        Some(TopItem::Wire(_)) => Ok(()),
        other => Err(format!("expected TopItem::Wire, got {other:?}")),
    }
}

#[test]
fn should_accept_group_decl_at_top_level() -> Result<(), String> {
    let prog = parse("group g { contains: [a] }")?;
    match prog.items.first() {
        Some(TopItem::Group(_)) => Ok(()),
        other => Err(format!("expected TopItem::Group, got {other:?}")),
    }
}

#[test]
fn should_accept_layout_decl_at_top_level() -> Result<(), String> {
    let prog = parse("layout: left_to_right")?;
    match prog.items.first() {
        Some(TopItem::Layout(_)) => Ok(()),
        other => Err(format!("expected TopItem::Layout, got {other:?}")),
    }
}

#[test]
fn should_accept_event_handler_at_top_level() -> Result<(), String> {
    let prog = parse("a on receive() {}")?;
    match prog.items.first() {
        Some(TopItem::EventHandler(_)) => Ok(()),
        other => Err(format!("expected TopItem::EventHandler, got {other:?}")),
    }
}

#[test]
fn should_accept_multiple_items() -> Result<(), String> {
    let prog = parse("context {} hash SHA256 {} node a : register {}")?;
    eq(prog.items.len(), 3)
}

#[test]
fn should_error_on_unexpected_token_at_top_level() -> Result<(), String> {
    msg_contains(&expect_parse_err("+")?, "unexpected token")
}
