use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §3  Context block
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_be_no_items_in_empty_context_block() -> Result<(), String> {
    let ctx = make_parser("context {}")?
        .parse_context_block()
        .map_err(|e| e.to_string())?;
    eq(ctx.items.len(), 0)
}

#[test]
fn should_accept_word_size_in_context_block() -> Result<(), String> {
    let ctx = make_parser("context { word_size: 32 }")?
        .parse_context_block()
        .map_err(|e| e.to_string())?;
    eq(ctx.items.len(), 1)?;
    match &ctx.items[0] {
        ContextItem::WordSize(32) => Ok(()),
        other => Err(format!("expected WordSize(32), got {other:?}")),
    }
}

#[test]
fn should_accept_fn_def_in_context_block() -> Result<(), String> {
    let ctx = make_parser("context { fn f() = 1 }")?
        .parse_context_block()
        .map_err(|e| e.to_string())?;
    match ctx.items.first() {
        Some(ContextItem::FnDef(_)) => Ok(()),
        other => Err(format!("expected FnDef context item, got {other:?}")),
    }
}

#[test]
fn should_accept_word_size_and_fn_def_in_context_block() -> Result<(), String> {
    let ctx = make_parser("context { word_size: 64 fn f() = 1 }")?
        .parse_context_block()
        .map_err(|e| e.to_string())?;
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
fn should_error_on_node_decl_inside_context_block() -> Result<(), String> {
    let err = make_parser("context { node a : register {} }")?
        .parse_context_block()
        .unwrap_err();
    if err.message.contains("word_size") || err.message.contains("fn") {
        Ok(())
    } else {
        Err(format!(
            "expected error to mention `word_size` or `fn`, got: {}",
            err.message
        ))
    }
}

#[test]
fn should_error_on_unterminated_context_block() -> Result<(), String> {
    let err = make_parser("context {")?.parse_context_block().unwrap_err();
    if err.message.contains("terminated") || err.message.contains("}") {
        Ok(())
    } else {
        Err(format!(
            "expected error about unterminated block, got: {}",
            err.message
        ))
    }
}
