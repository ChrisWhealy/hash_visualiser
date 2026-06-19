use super::*;
use crate::{graph::build, parser::parse};

fn value_of(src: &str, node: &str) -> Option<u64> {
    let program = parse(src).expect("parse");
    let graph = build(&program).expect("build");
    node_value(graph.nodes.get(node).expect("node exists"), &graph)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_apply_logical_shift_within_the_word() -> Result<(), String> {
    // 64-bit logical right shift: every nibble moves down one place.
    if apply_binop(&BinOp::ShrU, 0xF0F0_F0F0_F0F0_F0F0, 4, 64) != 0x0F0F_0F0F_0F0F_0F0F {
        return Err("shr_u failed".into());
    }
    // Left shift drops bits off the top of the word.
    if apply_binop(&BinOp::Shl, 0xF0F0_F0F0_F0F0_F0F0, 4, 64) != 0x0F0F_0F0F_0F0F_0F00 {
        return Err("shl failed".into());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_propagate_the_sign_bit_in_arithmetic_shift() -> Result<(), String> {
    // Negative (top bit set) → vacated high bits fill with 1s.
    if apply_binop(&BinOp::ShrS, 0x8000_0000_0000_0000, 4, 64) != 0xF800_0000_0000_0000 {
        return Err("shr_s (negative) failed".into());
    }
    // Positive → same as logical.
    if apply_binop(&BinOp::ShrS, 0x0800_0000_0000_0000, 4, 64) != 0x0080_0000_0000_0000 {
        return Err("shr_s (positive) failed".into());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_wrap_bits_around_the_word_during_rotate() -> Result<(), String> {
    // 8-bit rotate right by 4 of 0b1111_0000 == 0b0000_1111.
    if apply_binop(&BinOp::RotrU, 0xF0, 4, 8) != 0x0F {
        return Err("rotr failed".into());
    }
    // Rotating by 0 is the identity (and must not shift by the full word width).
    if apply_binop(&BinOp::RotlU, 0xF0, 0, 8) != 0xF0 {
        return Err("rotl by 0 failed".into());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_read_scalar_source_from_node_value() -> Result<(), String> {
    let v = value_of(
        "data N = 0x00000000000000B0\nnode x : register { source: N }",
        "x",
    );
    if v != Some(0xB0) {
        return Err(format!("expected 0xB0, got {v:?}"));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_resolve_node_args_when_computing_node_value() -> Result<(), String> {
    let src = "\
            context { word_size: 64 }\n\
            fn ShiftRight(value: u64, amount: u64) -> u64 = value shr_u amount\n\
            data InputValue = 0xF0F0F0F0F0F0F0F0\n\
            data ShiftAmount = 0x04\n\
            node value  : register  { source: InputValue }\n\
            node amount : constant  { source: ShiftAmount }\n\
            node result : operation { compute: ShiftRight(value, amount) }\n";

    // Inputs surface their literal data, and the operation surfaces the computed result.
    if value_of(src, "value") != Some(0xF0F0_F0F0_F0F0_F0F0) {
        return Err("input value wrong".into());
    }
    if value_of(src, "result") != Some(0x0F0F_0F0F_0F0F_0F0F) {
        return Err(format!(
            "computed result wrong: {:?}",
            value_of(src, "result")
        ));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_apply_unary_not() -> Result<(), String> {
    // hv/binary_operations/11_not.hv
    let src = "\
            context { word_size: 64 }\n\
            fn Not(a: u64) -> u64 = not a\n\
            data A = 0xFF00FF00FF00FF00\n\
            node a      : register  { source: A }\n\
            node result : operation { compute: Not(a) }\n";

    if value_of(src, "result") != Some(0x00FF_00FF_00FF_00FF) {
        return Err(format!("not wrong: {:?}", value_of(src, "result")));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_compute_three_input_majority() -> Result<(), String> {
    // hv/composition/02_majority.hv — a 3-input function (SHA-2's Maj) with a composed body.
    let src = "\
            context { word_size: 64 }\n\
            fn Maj(a: u64, b: u64, c: u64) -> u64 = (a and b) xor (a and c) xor (b and c)\n\
            data A = 0xFF00FF00FF00FF00\n\
            data B = 0xFFFF0000FFFF0000\n\
            data C = 0xFFFFFFFF00000000\n\
            node a : register { source: A }\n\
            node b : register { source: B }\n\
            node c : register { source: C }\n\
            node result : operation { compute: Maj(a, b, c) }\n";

    if value_of(src, "result") != Some(0xFFFF_FF00_FF00_0000) {
        return Err(format!("majority wrong: {:?}", value_of(src, "result")));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_expose_the_not_step_in_a_decomposed_choose() -> Result<(), String> {
    // hv/composition/04_choose.hv — Ch decomposed, with `NOT e` as its own node.
    let src = "\
            context { word_size: 64 }\n\
            fn Not(a: u64) -> u64 = not a\n\
            fn And(a: u64, b: u64) -> u64 = a and b\n\
            fn Xor(a: u64, b: u64) -> u64 = a xor b\n\
            data E = 0xFF00FF00FF00FF00\n\
            data F = 0xFFFF0000FFFF0000\n\
            data G = 0xFFFFFFFF00000000\n\
            node e : register { source: E }\n\
            node f : register { source: F }\n\
            node g : register { source: G }\n\
            node not_e : operation { compute: Not(e) }\n\
            node ef : operation { compute: And(e, f) }\n\
            node ng : operation { compute: And(not_e, g) }\n\
            node result : operation { compute: Xor(ef, ng) }\n";

    for (node, want) in [
        ("not_e", 0x00FF_00FF_00FF_00FFu64),
        ("ef", 0xFF00_0000_FF00_0000),
        ("ng", 0x00FF_00FF_0000_0000),
        ("result", 0xFFFF_00FF_FF00_0000),
    ] {
        if value_of(src, node) != Some(want) {
            return Err(format!("{node} = {:?}, want {want:#018x}", value_of(src, node)));
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_expose_each_intermediate_in_a_decomposed_majority() -> Result<(), String> {
    // hv/composition/03_majority_expanded.hv — Maj decomposed into per-step nodes; each must surface its own value.
    let src = "\
            context { word_size: 64 }\n\
            fn And(a: u64, b: u64) -> u64 = a and b\n\
            fn Xor(a: u64, b: u64) -> u64 = a xor b\n\
            data A = 0xFF00FF00FF00FF00\n\
            data B = 0xFFFF0000FFFF0000\n\
            data C = 0xFFFFFFFF00000000\n\
            node a : register { source: A }\n\
            node b : register { source: B }\n\
            node c : register { source: C }\n\
            node ab : operation { compute: And(a, b) }\n\
            node ac : operation { compute: And(a, c) }\n\
            node bc : operation { compute: And(b, c) }\n\
            node ab_ac : operation { compute: Xor(ab, ac) }\n\
            node result : operation { compute: Xor(ab_ac, bc) }\n";

    for (node, want) in [
        ("ab", 0xFF00_0000_FF00_0000u64),
        ("ac", 0xFF00_FF00_0000_0000),
        ("bc", 0xFFFF_0000_0000_0000),
        ("ab_ac", 0x0000_FF00_FF00_0000),
        ("result", 0xFFFF_FF00_FF00_0000),
    ] {
        if value_of(src, node) != Some(want) {
            return Err(format!("{node} = {:?}, want {want:#018x}", value_of(src, node)));
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_chain_one_operation_into_another() -> Result<(), String> {
    // hv/composition/01_and_then_xor.hv — the `result` node's `compute` references another *operation* node (`ab`),
    // so node_value must recurse through it.
    let src = "\
            context { word_size: 64 }\n\
            fn And(a: u64, b: u64) -> u64 = a and b\n\
            fn Xor(a: u64, b: u64) -> u64 = a xor b\n\
            data A = 0xFF00FF00FF00FF00\n\
            data B = 0x0F0F0F0F0F0F0F0F\n\
            data C = 0x00000000FFFFFFFF\n\
            node a : register { source: A }\n\
            node b : register { source: B }\n\
            node c : register { source: C }\n\
            node ab     : operation { compute: And(a, b) }\n\
            node result : operation { compute: Xor(ab, c) }\n";

    if value_of(src, "ab") != Some(0x0F00_0F00_0F00_0F00) {
        return Err(format!("stage 1 wrong: {:?}", value_of(src, "ab")));
    }
    if value_of(src, "result") != Some(0x0F00_0F00_F0FF_F0FF) {
        return Err(format!("stage 2 wrong: {:?}", value_of(src, "result")));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_compute_tutorial_examples_as_documented() -> Result<(), String> {
    // The exact (op, a, b, expected) tuples used by the 02–10 binary-operation tutorial files, so the rendered
    // "after" value always matches the data and the description's assert.
    let cases: &[(BinOp, u64, u64, u64)] = &[
        (BinOp::Shl, 0x0F0F_0F0F_0F0F_0F0F, 4, 0xF0F0_F0F0_F0F0_F0F0),
        (BinOp::ShrS, 0xFF00_0000_0000_0000, 8, 0xFFFF_0000_0000_0000),
        (
            BinOp::RotlU,
            0x1234_5678_9ABC_DEF0,
            8,
            0x3456_789A_BCDE_F012,
        ),
        (
            BinOp::RotrU,
            0x1234_5678_9ABC_DEF0,
            8,
            0xF012_3456_789A_BCDE,
        ),
        (
            BinOp::And,
            0xFF00_FF00_FF00_FF00,
            0x0F0F_0F0F_0F0F_0F0F,
            0x0F00_0F00_0F00_0F00,
        ),
        (
            BinOp::Or,
            0xFF00_FF00_FF00_FF00,
            0x00FF_00FF_00FF_00FF,
            0xFFFF_FFFF_FFFF_FFFF,
        ),
        (
            BinOp::Xor,
            0xFF00_FF00_FF00_FF00,
            0x0F0F_0F0F_0F0F_0F0F,
            0xF00F_F00F_F00F_F00F,
        ),
        (
            BinOp::Add,
            0xFFFF_FFFF_FFFF_FF00,
            0x0000_0000_0000_0200,
            0x0000_0000_0000_0100,
        ),
        (
            BinOp::Sub,
            0x0000_0000_0000_0100,
            0x0000_0000_0000_0200,
            0xFFFF_FFFF_FFFF_FF00,
        ),
    ];

    for (op, a, b, expected) in cases {
        let got = apply_binop(op, *a, *b, 64);
        if got != *expected {
            return Err(format!(
                "{op:?}({a:#018x}, {b:#018x}) = {got:#018x}, expected {expected:#018x}"
            ));
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_honour_nested_calls_and_word_size() -> Result<(), String> {
    // Sigma1 from SHA-256: a 32-bit value rotated and xored. Just check it evaluates and stays within 32 bits.
    let src = "\
            context { word_size: 32 }\n\
            fn Sigma(e: u32, r1: u32, r2: u32, r3: u32) -> u32 = (e rotr_u r1) xor (e rotr_u r2) xor (e rotr_u r3)\n\
            fn Sigma1(e: u32) -> u32 = Sigma(e, 6, 11, 25)\n\
            data E = 0x510E527F\n\
            node e : register  { source: E }\n\
            node s : operation { compute: Sigma1(e) }\n";

    let got = value_of(src, "s").ok_or("Sigma1 did not evaluate")?;
    // Reference computation in 32-bit space.
    let e: u32 = 0x510E_527F;
    let want = (e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25)) as u64;
    if got != want {
        return Err(format!("Sigma1 mismatch: got {got:#x}, want {want:#x}"));
    }
    Ok(())
}
