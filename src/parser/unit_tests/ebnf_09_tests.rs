use super::*;
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §9  Event handlers
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_parse_handler_with_no_params() -> Result<(), String> {
    let h = make_parser("a on receive() {}")?
        .parse_event_handler()
        .map_err(|e| e.to_string())?;
    eq(h.node.as_str(), "a")?;
    eq(h.event.as_str(), "receive")?;
    eq(h.params.len(), 0)?;
    eq(h.body.len(), 0)
}

#[test]
fn should_parse_handler_with_one_param() -> Result<(), String> {
    let h = make_parser("a on receive(value) {}")?
        .parse_event_handler()
        .map_err(|e| e.to_string())?;
    params_eq(&h.params, &["value"])
}

#[test]
fn should_parse_handler_with_multiple_params() -> Result<(), String> {
    let h = make_parser("a on receive(e, f, g) {}")?
        .parse_event_handler()
        .map_err(|e| e.to_string())?;
    params_eq(&h.params, &["e", "f", "g"])
}

#[test]
fn should_allow_trailing_comma_in_handler_params() -> Result<(), String> {
    let h = make_parser("a on receive(x, y,) {}")?
        .parse_event_handler()
        .map_err(|e| e.to_string())?;
    params_eq(&h.params, &["x", "y"])
}

#[test]
fn should_accept_reroute_as_event_name() -> Result<(), String> {
    // reroute is a keyword that is also a valid built-in event name
    let h = make_parser("w1 on reroute(new_src) {}")?
        .parse_event_handler()
        .map_err(|e| e.to_string())?;
    eq(h.event.as_str(), "reroute")
}

#[test]
fn should_parse_handler_with_multiple_effects() -> Result<(), String> {
    let h = make_parser("a on receive(v) { let r = v set value }")?
        .parse_event_handler()
        .map_err(|e| e.to_string())?;
    eq(h.body.len(), 2)
}

#[test]
fn should_error_on_handler_missing_on_keyword() -> Result<(), String> {
    let err = make_parser("a receive() {}")?
        .parse_event_handler()
        .unwrap_err();
    msg_contains(&err.message, "`on`")
}

#[test]
fn should_error_on_handler_missing_open_paren() -> Result<(), String> {
    let err = make_parser("a on receive {}")?
        .parse_event_handler()
        .unwrap_err();
    msg_contains(&err.message, "`(`")
}
