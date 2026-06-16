use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §5  Hash block
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_parse_empty_hash_block() -> Result<(), String> {
    let h = make_parser("hash SHA256 {}")?
        .parse_hash_block()
        .map_err(|e| e.to_string())?;
    eq(h.name.as_str(), "SHA256")?;
    eq(h.items.len(), 0)
}

#[test]
fn should_accept_context_and_nodes_in_hash_block() -> Result<(), String> {
    let h = make_parser("hash SHA256 { context { word_size: 32 } node a : register {} }")?
        .parse_hash_block()
        .map_err(|e| e.to_string())?;
    eq(h.items.len(), 2)
}

#[test]
fn should_error_on_hash_block_missing_name() -> Result<(), String> {
    let err = make_parser("hash {}")?.parse_hash_block().unwrap_err();
    msg_contains(&err.message, "identifier")
}

#[test]
fn should_error_on_unterminated_hash_block() -> Result<(), String> {
    let err = make_parser("hash SHA256 {")?
        .parse_hash_block()
        .unwrap_err();
    if err.message.contains("terminated") || err.message.contains("}") {
        Ok(())
    } else {
        Err(format!(
            "expected error about unterminated block, got: {}",
            err.message
        ))
    }
}
