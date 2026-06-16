use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §7  Wire declarations
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_parse_unnamed_wire() -> Result<(), String> {
    let w = make_parser("wire a -> b")?
        .parse_wire_decl()
        .map_err(|e| e.to_string())?;
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
fn should_parse_named_wire() -> Result<(), String> {
    let w = make_parser("wire carry: a -> b")?
        .parse_wire_decl()
        .map_err(|e| e.to_string())?;
    eq(w.name.as_deref(), Some("carry"))
}

#[test]
fn should_parse_wire_with_open_source_endpoint() -> Result<(), String> {
    let w = make_parser("wire ? -> b")?
        .parse_wire_decl()
        .map_err(|e| e.to_string())?;
    match w.source {
        WireEndpoint::Open => Ok(()),
        other => Err(format!("expected Open source endpoint, got {other:?}")),
    }
}

#[test]
fn should_parse_wire_with_open_target_endpoint() -> Result<(), String> {
    let w = make_parser("wire a -> ?")?
        .parse_wire_decl()
        .map_err(|e| e.to_string())?;
    match w.target {
        WireEndpoint::Open => Ok(()),
        other => Err(format!("expected Open target endpoint, got {other:?}")),
    }
}

#[test]
fn should_parse_named_wire_with_open_source() -> Result<(), String> {
    let w = make_parser("wire w1: ? -> dest")?
        .parse_wire_decl()
        .map_err(|e| e.to_string())?;
    eq(w.name.as_deref(), Some("w1"))?;
    match w.source {
        WireEndpoint::Open => Ok(()),
        other => Err(format!("expected Open source, got {other:?}")),
    }
}

#[test]
fn should_error_on_wire_missing_arrow() -> Result<(), String> {
    let err = make_parser("wire a b")?.parse_wire_decl().unwrap_err();
    msg_contains(&err.message, "`->`")
}

#[test]
fn should_error_on_wire_missing_target() -> Result<(), String> {
    let err = make_parser("wire a ->")?.parse_wire_decl().unwrap_err();
    if err.message.contains("endpoint") || err.message.contains("end of input") {
        Ok(())
    } else {
        Err(format!(
            "expected error about missing endpoint, got: {}",
            err.message
        ))
    }
}
