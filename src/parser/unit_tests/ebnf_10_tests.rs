use super::*;
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §10  Effects
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_parse_set_prop_assign() -> Result<(), String> {
    match make_parser("set label: incoming")?
        .parse_effect()
        .map_err(|e| e.to_string())?
    {
        Effect::Set(SetEffect::Prop { name, .. }) => eq(name.as_str(), "label"),
        other => Err(format!("expected Set(Prop), got {other:?}")),
    }
}

#[test]
fn should_parse_set_var_assign() -> Result<(), String> {
    match make_parser("set result = a xor b")?
        .parse_effect()
        .map_err(|e| e.to_string())?
    {
        Effect::Set(SetEffect::Var { name, .. }) => eq(name.as_str(), "result"),
        other => Err(format!("expected Set(Var), got {other:?}")),
    }
}

#[test]
fn should_parse_set_bare_ident() -> Result<(), String> {
    match make_parser("set value")?
        .parse_effect()
        .map_err(|e| e.to_string())?
    {
        Effect::Set(SetEffect::Bare(name)) => eq(name.as_str(), "value"),
        other => Err(format!("expected Set(Bare), got {other:?}")),
    }
}

#[test]
fn should_parse_let_binding() -> Result<(), String> {
    match make_parser("let r = a rotr_u 6")?
        .parse_effect()
        .map_err(|e| e.to_string())?
    {
        Effect::Let(b) => {
            eq(b.name.as_str(), "r")?;
            match b.value {
                Expr::BinOp {
                    op: BinOp::RotrU, ..
                } => Ok(()),
                other => Err(format!("expected BinOp(RotrU), got {other:?}")),
            }
        }
        other => Err(format!("expected Let, got {other:?}")),
    }
}

#[test]
fn should_parse_animate_fill_pulse() -> Result<(), String> {
    match make_parser("animate fill: pulse \"gold\" for 250ms")?
        .parse_effect()
        .map_err(|e| e.to_string())?
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
fn should_parse_animate_prop_transition() -> Result<(), String> {
    match make_parser("animate opacity from 0 to 1 over 300ms")?
        .parse_effect()
        .map_err(|e| e.to_string())?
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
fn should_error_on_animate_with_wrong_pulse_keyword() -> Result<(), String> {
    let err = make_parser("animate fill: flash \"red\" for 100ms")?
        .parse_effect()
        .unwrap_err();
    msg_contains(&err.message, "pulse")
}

#[test]
fn should_parse_emit_with_no_target() -> Result<(), String> {
    match make_parser("emit forward(v)")?
        .parse_effect()
        .map_err(|e| e.to_string())?
    {
        Effect::Emit(e) => {
            eq(e.event.as_str(), "forward")?;
            eq(e.args.len(), 1)?;
            eq(e.target.is_none(), true)
        }
        other => Err(format!("expected Emit, got {other:?}")),
    }
}

#[test]
fn should_parse_emit_broadcast_to_all() -> Result<(), String> {
    match make_parser("emit step(1) -> all")?
        .parse_effect()
        .map_err(|e| e.to_string())?
    {
        Effect::Emit(e) => match e.target {
            Some(EmitTarget::All) => Ok(()),
            other => Err(format!("expected EmitTarget::All, got {other:?}")),
        },
        other => Err(format!("expected Emit, got {other:?}")),
    }
}

#[test]
fn should_parse_emit_to_named_node() -> Result<(), String> {
    match make_parser("emit forward(v) -> sink")?
        .parse_effect()
        .map_err(|e| e.to_string())?
    {
        Effect::Emit(e) => match e.target {
            Some(EmitTarget::Node(ref s)) if s == "sink" => Ok(()),
            other => Err(format!(
                "expected EmitTarget::Node(\"sink\"), got {other:?}"
            )),
        },
        other => Err(format!("expected Emit, got {other:?}")),
    }
}

#[test]
fn should_parse_emit_via_named_wire() -> Result<(), String> {
    match make_parser("emit forward(v) via carry")?
        .parse_effect()
        .map_err(|e| e.to_string())?
    {
        Effect::Emit(e) => match e.target {
            Some(EmitTarget::Via(ref s)) if s == "carry" => Ok(()),
            other => Err(format!(
                "expected EmitTarget::Via(\"carry\"), got {other:?}"
            )),
        },
        other => Err(format!("expected Emit, got {other:?}")),
    }
}

#[test]
fn should_parse_emit_with_multiple_args() -> Result<(), String> {
    match make_parser("emit send(a, b, c)")?
        .parse_effect()
        .map_err(|e| e.to_string())?
    {
        Effect::Emit(e) => eq(e.args.len(), 3),
        other => Err(format!("expected Emit, got {other:?}")),
    }
}

#[test]
fn should_parse_reroute_to() -> Result<(), String> {
    match make_parser("reroute w1 to dest")?
        .parse_effect()
        .map_err(|e| e.to_string())?
    {
        Effect::Reroute(r) => {
            eq(r.wire.as_str(), "w1")?;
            eq(r.direction, RerouteDir::To)?;
            eq(r.node.as_str(), "dest")
        }
        other => Err(format!("expected Reroute, got {other:?}")),
    }
}

#[test]
fn should_parse_reroute_from() -> Result<(), String> {
    match make_parser("reroute w1 from src")?
        .parse_effect()
        .map_err(|e| e.to_string())?
    {
        Effect::Reroute(r) => eq(r.direction, RerouteDir::From),
        other => Err(format!("expected Reroute, got {other:?}")),
    }
}

#[test]
fn should_error_on_reroute_with_invalid_direction() -> Result<(), String> {
    let err = make_parser("reroute w1 above dest")?
        .parse_effect()
        .unwrap_err();
    if err.message.contains("`to`") || err.message.contains("`from`") {
        Ok(())
    } else {
        Err(format!(
            "expected error mentioning `to` or `from`, got: {}",
            err.message
        ))
    }
}

#[test]
fn should_error_on_unknown_effect_keyword() -> Result<(), String> {
    // `node` is not a valid effect keyword
    let err = make_parser("node a : register {}")?
        .parse_effect()
        .unwrap_err();
    msg_contains(&err.message, "effect")
}
