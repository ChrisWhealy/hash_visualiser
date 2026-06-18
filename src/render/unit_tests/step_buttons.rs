use super::{parse_and_build, step_back, step_forward, step_range, eq};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Step-button range: clamped to the comprehension's range, no wrap-around.
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_derive_step_range_from_comprehension() -> Result<(), String> {
    let g = parse_and_build(
        "
        hash SHA3 {
            fn ThetaC(a: [[u64; 5]; 5]) -> [u64; 5] = [ for x in 0..5 => reduce xor over a[x] ]
            node state : register  { format: hex64 }
            node c     : operation { symbol: \"ThetaC\" }
            wire state -> c
        }
    ",
    );

    // Taken from the comprehension `for x in 0..5`, not just the row count.
    eq(step_range("state", &g, 5), (0, 5))
}

#[test]
fn should_fall_back_to_row_span_without_comprehension() -> Result<(), String> {
    let g = parse_and_build(
        "
        hash SHA3 {
            fn ThetaC(a: [[u64; 5]; 5]) -> [u64; 5] = a
            node state : register  { format: hex64 }
            node c     : operation { symbol: \"ThetaC\" }
            wire state -> c
        }
    ",
    );

    eq(step_range("state", &g, 5), (0, 5))
}

#[test]
fn should_clamp_step_forward_at_last_row() -> Result<(), String> {
    let range = (0, 5); // visits rows 0..=4
    eq(step_forward(0, range), 1)?;
    eq(step_forward(3, range), 4)?;
    eq(step_forward(4, range), 4) // no wrap past the last row
}

#[test]
fn should_clamp_step_back_at_first_row() -> Result<(), String> {
    let range = (0, 5);
    eq(step_back(4, range), 3)?;
    eq(step_back(1, range), 0)?;
    eq(step_back(0, range), 0) // no wrap before the first row
}

#[test]
fn should_respect_a_nonzero_range_start() -> Result<(), String> {
    let range = (1, 4); // visits rows 1..=3
    eq(step_back(1, range), 1)?; // clamped at start, not 0
    eq(step_forward(3, range), 3) // clamped at end-1
}
