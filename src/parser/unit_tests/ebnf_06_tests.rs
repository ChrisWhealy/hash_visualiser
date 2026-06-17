use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §6  Node declarations
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_parse_node_with_register_kind() -> Result<(), String> {
    let n = make_parser("node a : register {}")?
        .parse_node_decl()
        .map_err(|e| e.to_string())?;
    eq(n.name.as_str(), "a")?;
    match n.kind {
        NodeKind::Register => Ok(()),
        other => Err(format!("expected Register kind, got {other:?}")),
    }
}

#[test]
fn should_parse_node_with_operation_kind() -> Result<(), String> {
    let n = make_parser("node s1 : operation {}")?
        .parse_node_decl()
        .map_err(|e| e.to_string())?;
    match n.kind {
        NodeKind::Operation => Ok(()),
        other => Err(format!("expected Operation kind, got {other:?}")),
    }
}

#[test]
fn should_parse_node_with_constant_kind() -> Result<(), String> {
    let n = make_parser("node k : constant {}")?
        .parse_node_decl()
        .map_err(|e| e.to_string())?;
    match n.kind {
        NodeKind::Constant => Ok(()),
        other => Err(format!("expected Constant kind, got {other:?}")),
    }
}

#[test]
fn should_parse_node_with_button_kind() -> Result<(), String> {
    let n = make_parser("node btn : button {}")?
        .parse_node_decl()
        .map_err(|e| e.to_string())?;
    match n.kind {
        NodeKind::Button => Ok(()),
        other => Err(format!("expected Button kind, got {other:?}")),
    }
}

#[test]
fn should_parse_node_with_user_defined_kind() -> Result<(), String> {
    let n = make_parser("node x : mux {}")?
        .parse_node_decl()
        .map_err(|e| e.to_string())?;
    match n.kind {
        NodeKind::User(ref s) if s == "mux" => Ok(()),
        other => Err(format!("expected User(\"mux\") kind, got {other:?}")),
    }
}

#[test]
fn should_parse_node_with_no_properties() -> Result<(), String> {
    let n = make_parser("node a : register {}")?
        .parse_node_decl()
        .map_err(|e| e.to_string())?;
    eq(n.properties.len(), 0)
}

#[test]
fn should_parse_node_with_string_property() -> Result<(), String> {
    let n = make_parser("node a : register { label: \"a\" }")?
        .parse_node_decl()
        .map_err(|e| e.to_string())?;
    eq(n.properties.len(), 1)?;
    eq(n.properties[0].name.as_str(), "label")?;
    match &n.properties[0].value {
        PropValue::Str(s) if s == "a" => Ok(()),
        other => Err(format!("expected PropValue::Str(\"a\"), got {other:?}")),
    }
}

#[test]
fn should_parse_node_with_identifier_property() -> Result<(), String> {
    let n = make_parser("node a : register { format: hex32 }")?
        .parse_node_decl()
        .map_err(|e| e.to_string())?;
    match &n.properties[0].value {
        PropValue::Expr(Expr::Ident(s)) if s == "hex32" => Ok(()),
        other => Err(format!(
            "expected PropValue::Expr(Ident(\"hex32\")), got {other:?}"
        )),
    }
}

#[test]
fn should_parse_node_with_multiple_properties() -> Result<(), String> {
    let n = make_parser("node a : register { label: \"a\", format: hex32 }")?
        .parse_node_decl()
        .map_err(|e| e.to_string())?;
    eq(n.properties.len(), 2)
}

#[test]
fn should_allow_trailing_comma_in_node_property_list() -> Result<(), String> {
    let n = make_parser("node a : register { label: \"a\", format: hex32, }")?
        .parse_node_decl()
        .map_err(|e| e.to_string())?;
    eq(n.properties.len(), 2)
}

#[test]
fn should_parse_node_layout_auto_property() -> Result<(), String> {
    let n = make_parser("node a : register { layout: auto }")?
        .parse_node_decl()
        .map_err(|e| e.to_string())?;
    match &n.properties[0].value {
        PropValue::Expr(Expr::Ident(s)) if s == "auto" => Ok(()),
        other => Err(format!(
            "expected PropValue::Expr(Ident(\"auto\")), got {other:?}"
        )),
    }
}

#[test]
fn should_parse_node_layout_pinned_property() -> Result<(), String> {
    let n = make_parser("node a : register { layout: pinned(10, 20) }")?
        .parse_node_decl()
        .map_err(|e| e.to_string())?;
    match &n.properties[0].value {
        PropValue::Expr(Expr::Call { name, args }) if name == "pinned" => eq(args.len(), 2),
        other => Err(format!(
            "expected PropValue::Expr(Call{{pinned, [10, 20]}}), got {other:?}"
        )),
    }
}

#[test]
fn should_error_on_node_missing_kind_separator() -> Result<(), String> {
    let err = make_parser("node a {}")?.parse_node_decl().unwrap_err();
    msg_contains(&err.message, "`:`")
}

#[test]
fn should_error_on_node_missing_body_brace() -> Result<(), String> {
    let err = make_parser("node a : register")?
        .parse_node_decl()
        .unwrap_err();
    msg_contains(&err.message, "`{`")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Data declarations
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_parse_data_decl_with_array_literal() -> Result<(), String> {
    let d = make_parser("data A = [[0x1, 0x2], [0x3, 0x4]]")?
        .parse_data_decl()
        .map_err(|e| e.to_string())?;
    eq(d.name.as_str(), "A")?;
    match d.value {
        Expr::Array(rows) => eq(rows.len(), 2),
        other => Err(format!("expected Array value, got {other:?}")),
    }
}

#[test]
fn should_error_on_data_decl_missing_equals() -> Result<(), String> {
    let err = make_parser("data A [1, 2]")?.parse_data_decl().unwrap_err();
    msg_contains(&err.message, "`=`")
}
