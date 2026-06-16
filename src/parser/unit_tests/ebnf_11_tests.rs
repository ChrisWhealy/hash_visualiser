use super::*;

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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §11  Primary Expressions
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_parse_integer_literal() -> Result<(), String> {
    match expr_of("fn f() = 42")? {
        Expr::Integer(42) => Ok(()),
        other => Err(format!("expected Integer(42), got {other:?}")),
    }
}

#[test]
fn should_parse_hex_literal() -> Result<(), String> {
    match expr_of("fn f() = 0xff")? {
        Expr::HexLit(0xff) => Ok(()),
        other => Err(format!("expected HexLit(0xff), got {other:?}")),
    }
}

#[test]
fn should_parse_identifier_reference() -> Result<(), String> {
    match expr_of("fn f() = x")? {
        Expr::Ident(ref s) if s == "x" => Ok(()),
        other => Err(format!("expected Ident(\"x\"), got {other:?}")),
    }
}

#[test]
fn should_parse_function_call_with_no_args() -> Result<(), String> {
    match expr_of("fn f() = g()")? {
        Expr::Call { name, args } => {
            eq(name.as_str(), "g")?;
            eq(args.len(), 0)
        }
        other => Err(format!("expected Call, got {other:?}")),
    }
}

#[test]
fn should_parse_function_call_with_one_arg() -> Result<(), String> {
    match expr_of("fn f() = g(x)")? {
        Expr::Call { args, .. } => eq(args.len(), 1),
        other => Err(format!("expected Call, got {other:?}")),
    }
}

#[test]
fn should_parse_function_call_with_multiple_args() -> Result<(), String> {
    match expr_of("fn f() = Sigma(e, 6, 11, 25)")? {
        Expr::Call { name, args } => {
            eq(name.as_str(), "Sigma")?;
            eq(args.len(), 4)
        }
        other => Err(format!("expected Call, got {other:?}")),
    }
}

#[test]
fn should_be_transparent_for_parenthesised_expression() -> Result<(), String> {
    // (x) must not introduce a new AST node — it should yield just Ident("x")
    match expr_of("fn f() = (x)")? {
        Expr::Ident(_) => Ok(()),
        other => Err(format!(
            "expected Ident (parens are transparent), got {other:?}"
        )),
    }
}

#[test]
fn should_parse_unary_not() -> Result<(), String> {
    match expr_of("fn f() = not x")? {
        Expr::Not(_) => Ok(()),
        other => Err(format!("expected Not, got {other:?}")),
    }
}

#[test]
fn should_nest_double_not() -> Result<(), String> {
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
binop_test!(should_parse_binop_or, "or", BinOp::Or);
binop_test!(should_parse_binop_xor, "xor", BinOp::Xor);
binop_test!(should_parse_binop_and, "and", BinOp::And);
binop_test!(should_parse_binop_add, "+", BinOp::Add);
binop_test!(should_parse_binop_sub, "-", BinOp::Sub);
binop_test!(should_parse_binop_shl, "shl", BinOp::Shl);
binop_test!(should_parse_binop_shr_u, "shr_u", BinOp::ShrU);
binop_test!(should_parse_binop_shr_s, "shr_s", BinOp::ShrS);
binop_test!(should_parse_binop_rotr_u, "rotr_u", BinOp::RotrU);
binop_test!(should_parse_binop_rotr_s, "rotr_s", BinOp::RotrS);
binop_test!(should_parse_binop_rotl_u, "rotl_u", BinOp::RotlU);
binop_test!(should_parse_binop_rotl_s, "rotl_s", BinOp::RotlS);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §11  Operator precedence
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_bind_xor_tighter_than_or() -> Result<(), String> {
    // a or b xor c  →  a or (b xor c)
    match expr_of("fn f() = a or b xor c")? {
        Expr::BinOp {
            op: BinOp::Or,
            lhs,
            rhs,
        } => {
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
fn should_bind_and_tighter_than_xor() -> Result<(), String> {
    // a xor b and c  →  a xor (b and c)
    match expr_of("fn f() = a xor b and c")? {
        Expr::BinOp {
            op: BinOp::Xor,
            rhs,
            ..
        } => match *rhs {
            Expr::BinOp { op: BinOp::And, .. } => Ok(()),
            other => Err(format!("expected And rhs, got {other:?}")),
        },
        other => Err(format!("expected Xor at root, got {other:?}")),
    }
}

#[test]
fn should_bind_add_tighter_than_and() -> Result<(), String> {
    // a and b + c  →  a and (b + c)
    match expr_of("fn f() = a and b + c")? {
        Expr::BinOp {
            op: BinOp::And,
            rhs,
            ..
        } => match *rhs {
            Expr::BinOp { op: BinOp::Add, .. } => Ok(()),
            other => Err(format!("expected Add rhs, got {other:?}")),
        },
        other => Err(format!("expected And at root, got {other:?}")),
    }
}

#[test]
fn should_bind_shift_tighter_than_add() -> Result<(), String> {
    // a shl 2 + b  →  (a shl 2) + b
    match expr_of("fn f() = a shl 2 + b")? {
        Expr::BinOp {
            op: BinOp::Add,
            lhs,
            ..
        } => match *lhs {
            Expr::BinOp { op: BinOp::Shl, .. } => Ok(()),
            other => Err(format!("expected Shl lhs, got {other:?}")),
        },
        other => Err(format!("expected Add at root, got {other:?}")),
    }
}

#[test]
fn should_bind_rotation_tighter_than_shift() -> Result<(), String> {
    // a rotr_u 6 shl 2  →  (a rotr_u 6) shl 2
    match expr_of("fn f() = a rotr_u 6 shl 2")? {
        Expr::BinOp {
            op: BinOp::Shl,
            lhs,
            ..
        } => match *lhs {
            Expr::BinOp {
                op: BinOp::RotrU, ..
            } => Ok(()),
            other => Err(format!("expected RotrU lhs, got {other:?}")),
        },
        other => Err(format!("expected Shl at root, got {other:?}")),
    }
}

#[test]
fn should_bind_not_tighter_than_rotation() -> Result<(), String> {
    // a rotr_u not b  →  a rotr_u (not b)
    match expr_of("fn f() = a rotr_u not b")? {
        Expr::BinOp {
            op: BinOp::RotrU,
            rhs,
            ..
        } => match *rhs {
            Expr::Not(_) => Ok(()),
            other => Err(format!("expected Not rhs, got {other:?}")),
        },
        other => Err(format!("expected RotrU at root, got {other:?}")),
    }
}

#[test]
fn should_override_precedence_with_parentheses() -> Result<(), String> {
    // (a or b) and c  →  BinOp(And, BinOp(Or, a, b), c)
    match expr_of("fn f() = (a or b) and c")? {
        Expr::BinOp {
            op: BinOp::And,
            lhs,
            ..
        } => match *lhs {
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
fn should_be_left_associative_for_same_level_operator() -> Result<(), String> {
    // a xor b xor c  →  (a xor b) xor c
    match expr_of("fn f() = a xor b xor c")? {
        Expr::BinOp {
            op: BinOp::Xor,
            lhs,
            rhs,
        } => {
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
fn should_be_left_associative_for_addition() -> Result<(), String> {
    // a + b + c  →  (a + b) + c
    match expr_of("fn f() = a + b + c")? {
        Expr::BinOp {
            op: BinOp::Add,
            lhs,
            ..
        } => match *lhs {
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
fn should_error_on_empty_expression() -> Result<(), String> {
    msg_contains(&expect_parse_err("fn f() =")?, "end of input")
}

#[test]
fn should_error_on_plus_as_primary_expression() -> Result<(), String> {
    msg_contains(&expect_parse_err("fn f() = +")?, "expression")
}

#[test]
fn should_error_on_unclosed_parenthesis() -> Result<(), String> {
    msg_contains(&expect_parse_err("fn f() = (a xor b")?, "`)`")
}
