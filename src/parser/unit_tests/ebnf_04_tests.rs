use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §4  Function definitions
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_parse_fn_with_no_params() -> Result<(), String> {
    let f = make_parser("fn f() = 1")?
        .parse_fn_def()
        .map_err(|e| e.to_string())?;
    eq(f.name.as_str(), "f")?;
    eq(f.params.len(), 0)?;
    match f.body {
        Expr::Integer(1) => Ok(()),
        other => Err(format!("expected Integer(1) body, got {other:?}")),
    }
}

#[test]
fn should_parse_fn_with_one_param() -> Result<(), String> {
    let f = make_parser("fn f(x: u32) = x")?
        .parse_fn_def()
        .map_err(|e| e.to_string())?;
    fn_params_eq(&f.params, &[("x", Type::U32)])?;
    match f.body {
        Expr::Ident(ref s) if s == "x" => Ok(()),
        other => Err(format!("expected Ident(\"x\") body, got {other:?}")),
    }
}

#[test]
fn should_parse_fn_with_multiple_params() -> Result<(), String> {
    let f = make_parser("fn Sigma(e: u32, r1: u32, r2: u32, r3: u32) = e rotr_u r1")?
        .parse_fn_def()
        .map_err(|e| e.to_string())?;
    eq(f.name.as_str(), "Sigma")?;
    fn_params_eq(
        &f.params,
        &[
            ("e", Type::U32),
            ("r1", Type::U32),
            ("r2", Type::U32),
            ("r3", Type::U32),
        ],
    )
}

#[test]
fn should_allow_trailing_comma_in_fn_params() -> Result<(), String> {
    let f = make_parser("fn f(a: u8, b: u16,) = a")?
        .parse_fn_def()
        .map_err(|e| e.to_string())?;
    fn_params_eq(&f.params, &[("a", Type::U8), ("b", Type::U16)])
}

#[test]
fn should_parse_all_primitive_param_types() -> Result<(), String> {
    let f = make_parser("fn f(a: u8, b: u16, c: u32, d: u64) = a")?
        .parse_fn_def()
        .map_err(|e| e.to_string())?;
    fn_params_eq(
        &f.params,
        &[
            ("a", Type::U8),
            ("b", Type::U16),
            ("c", Type::U32),
            ("d", Type::U64),
        ],
    )
}

#[test]
fn should_parse_array_param_type() -> Result<(), String> {
    let f = make_parser("fn f(xs: [u16; 8]) = xs")?
        .parse_fn_def()
        .map_err(|e| e.to_string())?;
    fn_params_eq(
        &f.params,
        &[(
            "xs",
            Type::Array {
                element: Box::new(Type::U16),
                len: 8,
            },
        )],
    )
}

#[test]
fn should_parse_nested_array_param_type() -> Result<(), String> {
    // The SHA3 ThetaC input: an array of 5 arrays of 5 u8 values.
    let f = make_parser("fn ThetaC(a: [[u8; 5]; 5]) = a")?
        .parse_fn_def()
        .map_err(|e| e.to_string())?;
    let inner = Type::Array {
        element: Box::new(Type::U8),
        len: 5,
    };
    fn_params_eq(
        &f.params,
        &[(
            "a",
            Type::Array {
                element: Box::new(inner),
                len: 5,
            },
        )],
    )
}

#[test]
fn should_default_to_unit_return_type_when_omitted() -> Result<(), String> {
    let f = make_parser("fn f(x: u32) = x")?
        .parse_fn_def()
        .map_err(|e| e.to_string())?;
    eq(f.return_type, Type::Unit)
}

#[test]
fn should_parse_value_return_type() -> Result<(), String> {
    let f = make_parser("fn Ch(e: u32, f: u32, g: u32) -> u32 = e")?
        .parse_fn_def()
        .map_err(|e| e.to_string())?;
    eq(f.return_type, Type::U32)
}

#[test]
fn should_parse_array_return_type() -> Result<(), String> {
    // SHA3 ThetaC: takes the 5x5 state, returns the 5-element column-parity vector.
    let f = make_parser("fn ThetaC(a: [[u8; 5]; 5]) -> [u8; 5] = a")?
        .parse_fn_def()
        .map_err(|e| e.to_string())?;
    eq(
        f.return_type,
        Type::Array {
            element: Box::new(Type::U8),
            len: 5,
        },
    )
}

#[test]
fn should_error_on_missing_return_type_after_arrow() -> Result<(), String> {
    let err = make_parser("fn f(x: u32) -> = x")?
        .parse_fn_def()
        .unwrap_err();
    msg_contains(&err.message, "type")
}

#[test]
fn should_error_on_param_without_type() -> Result<(), String> {
    let err = make_parser("fn f(x) = x")?.parse_fn_def().unwrap_err();
    msg_contains(&err.message, "`:`")
}

#[test]
fn should_error_on_unknown_param_type() -> Result<(), String> {
    let err = make_parser("fn f(x: u128) = x")?
        .parse_fn_def()
        .unwrap_err();
    msg_contains(&err.message, "unknown type")
}

#[test]
fn should_error_on_array_type_missing_length() -> Result<(), String> {
    let err = make_parser("fn f(x: [u8]) = x")?
        .parse_fn_def()
        .unwrap_err();
    msg_contains(&err.message, "`;`")
}

#[test]
fn should_error_on_fn_with_missing_equals() -> Result<(), String> {
    let err = make_parser("fn f() x")?.parse_fn_def().unwrap_err();
    msg_contains(&err.message, "`=`")
}

#[test]
fn should_error_on_fn_with_missing_open_paren() -> Result<(), String> {
    let err = make_parser("fn f x")?.parse_fn_def().unwrap_err();
    msg_contains(&err.message, "`(`")
}
