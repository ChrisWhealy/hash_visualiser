use crate::{
    ast::ebnf_08::FlowDirection,
    error::graph_error::GraphError,
    graph::{ValidatedGraph, build},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Helpers
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn parse_and_build(src: &str) -> Result<ValidatedGraph, String> {
    let program = crate::parse(src).map_err(|e| e.to_string())?;
    build(&program).map_err(|errs| errs.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("; "))
}

fn graph_errors_for(src: &str) -> Result<Vec<GraphError>, String> {
    let program = crate::parse(src).map_err(|e| e.to_string())?;
    match build(&program) {
        Err(errs) => Ok(errs),
        Ok(_) => Err("expected graph error, but build succeeded".into()),
    }
}

fn eq<T, U>(actual: T, expected: U) -> Result<(), String>
where
    T: PartialEq<U> + std::fmt::Debug,
    U: std::fmt::Debug,
{
    if actual == expected {
        Ok(())
    } else {
        Err(format!("expected {expected:?}, got {actual:?}"))
    }
}

fn check(cond: bool, msg: &str) -> Result<(), String> {
    if cond { Ok(()) } else { Err(msg.to_string()) }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Happy-path: flat programs (nodes and wires at top level)
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_produce_correct_layers_from_two_flat_wired_nodes() -> Result<(), String> {
    let g = parse_and_build("
        node a : register {}
        node b : operation {}
        wire a -> b
    ")?;

    eq(g.layers, vec![vec!["a"], vec!["b"]])?;
    eq(g.edges, vec![("a".to_string(), "b".to_string())])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_produce_three_layers_from_flat_diamond() -> Result<(), String> {
    let g = parse_and_build("
        node a : constant {}
        node b : operation {}
        node c : operation {}
        node d : register {}
        wire a -> b
        wire a -> c
        wire b -> d
        wire c -> d
    ")?;

    eq(g.layers.len(), 3)?;
    eq(&g.layers[0], &vec!["a"])?;
    eq(&g.layers[1], &vec!["b", "c"])?;
    eq(&g.layers[2], &vec!["d"])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_set_flow_direction_from_layout_directive() -> Result<(), String> {
    let g = parse_and_build("
        layout: top_to_bottom
        node a : register {}
    ")?;

    eq(g.flow, FlowDirection::TopToBottom)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_capture_context_word_size() -> Result<(), String> {
    let g = parse_and_build("
        context { word_size: 64 }
    ")?;

    eq(g.word_size, Some(64))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_index_named_wire_by_name() -> Result<(), String> {
    let g = parse_and_build("
        node src : register {}
        node dst : register {}
        wire carry: src -> dst
    ")?;

    check(g.named_wires.contains_key("carry"), "expected a named wire 'carry'")?;
    eq(g.named_wires.len(), 1)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Happy-path: hash blocks
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_produce_correct_layers_from_hash_block_nodes_and_wires() -> Result<(), String> {
    let g = parse_and_build("
        hash SHA256 {
            node input  : register  { label: \"Input\" }
            node mixer  : operation { label: \"Mix\"   }
            node output : register  { label: \"Output\" }
            wire input -> mixer
            wire mixer -> output
        }
    ")?;

    eq(g.layers, vec![vec!["input"], vec!["mixer"], vec!["output"]])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_use_hash_block_layout_directive_to_override_default_flow() -> Result<(), String> {
    let g = parse_and_build("
        hash SHA256 {
            layout: right_to_left
            node a : register {}
        }
    ")?;

    eq(g.flow, FlowDirection::RightToLeft)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_capture_hash_block_context_word_size() -> Result<(), String> {
    let g = parse_and_build("
        hash SHA256 {
            context { word_size: 32 }
            node a : register {}
        }
    ")?;

    eq(g.word_size, Some(32))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Happy-path: functions
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_validate_and_collect_context_functions() -> Result<(), String> {
    let g = parse_and_build("
        context {
            fn base(x: u32) = x rotr_u 2
            fn derived(x: u32) = base(x) xor base(x)
        }
    ")?;

    check(g.fn_defs.contains_key("base"), "expected function 'base'")?;
    check(g.fn_defs.contains_key("derived"), "expected function 'derived'")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_validate_parameterised_sigma_style_functions() -> Result<(), String> {
    let g = parse_and_build("
        context {
            word_size: 64
            fn Sigma(e: u64, r1: u64, r2: u64, r3: u64) = (e rotr_u r1) xor (e rotr_u r2) xor (e shr_u r3)
            fn Sigma1(e: u64) = Sigma(e, 14, 18, 41)
        }
    ")?;

    eq(g.word_size, Some(64))?;
    check(g.fn_defs.contains_key("Sigma"), "expected function 'Sigma'")?;
    check(g.fn_defs.contains_key("Sigma1"), "expected function 'Sigma1'")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_merge_context_and_hash_block_items() -> Result<(), String> {
    let g = parse_and_build("
        context { word_size: 32 }
        hash SHA256 {
            node a : register {}
            node b : operation {}
            wire a -> b
        }
    ")?;

    eq(g.word_size, Some(32))?;
    eq(g.layers, vec![vec!["a"], vec!["b"]])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Error paths
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_reject_wire_to_undeclared_node() -> Result<(), String> {
    let errs = graph_errors_for("
        node a : register {}
        wire a -> ghost
    ")?;

    check(
        errs.iter().any(
            |e| matches!(e, GraphError::UndeclaredNode { endpoint, .. } if endpoint == "ghost"),
        ),
        "expected an UndeclaredNode error for 'ghost'",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_reject_duplicate_node() -> Result<(), String> {
    let errs = graph_errors_for("
        node a : register {}
        node a : operation {}
    ")?;

    check(
        errs.iter().any(|e| matches!(e, GraphError::DuplicateNode(n) if n == "a")),
        "expected a DuplicateNode error for 'a'",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_reject_undeclared_function_call() -> Result<(), String> {
    let errs = graph_errors_for("
        fn foo(x: u32) = missing(x)
    ")?;

    check(
        errs.iter().any(
            |e| matches!(e, GraphError::UndeclaredFn { callee, .. } if callee == "missing"),
        ),
        "expected an UndeclaredFn error for 'missing'",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_detect_undeclared_function_called_inside_comprehension() -> Result<(), String> {
    // Confirms call-graph analysis recurses into comprehension/reduce/index bodies, not just flat expressions.
    let errs = graph_errors_for("
        fn f(a: [[u8; 5]; 5]) -> [u8; 5] = [ for x in 0..5 => reduce xor over missing(a[x]) ]
    ")?;

    check(
        errs.iter().any(
            |e| matches!(e, GraphError::UndeclaredFn { callee, .. } if callee == "missing"),
        ),
        "expected an UndeclaredFn error for 'missing'",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_validate_array_processing_function() -> Result<(), String> {
    let g = parse_and_build("
        hash SHA3 {
            fn ThetaC(a: [[u8; 5]; 5]) -> [u8; 5] = [ for x in 0..5 => reduce xor over a[x] ]
        }
    ")?;

    check(g.fn_defs.contains_key("ThetaC"), "expected function 'ThetaC'")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Data bindings
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_collect_data_binding_and_node_referencing_it() -> Result<(), String> {
    let g = parse_and_build("
        data A = [[0x1, 0x2], [0x3, 0x4]]
        node state : register { format: hex8, source: A }
    ")?;

    check(g.data.contains_key("A"), "expected data binding 'A'")?;
    check(g.nodes.contains_key("state"), "expected node 'state'")
}

#[test]
fn should_pass_data_node_to_function_via_compute() -> Result<(), String> {
    let g = parse_and_build("
        fn ThetaC(a: [[u64; 5]; 5]) -> [u64; 5] = [ for x in 0..5 => reduce xor over a[x] ]
        data A = [[0x1, 0x2, 0x3, 0x4, 0x5], [0x1, 0x2, 0x3, 0x4, 0x5], [0x1, 0x2, 0x3, 0x4, 0x5], [0x1, 0x2, 0x3, 0x4, 0x5], [0x1, 0x2, 0x3, 0x4, 0x5]]
        node state : register  { format: hex64, source: A }
        node c     : operation { symbol: \"ThetaC\", compute: ThetaC(state) }
        wire state -> c
    ")?;

    check(g.data.contains_key("A"), "expected data binding 'A'")?;
    check(g.nodes.contains_key("c"), "expected node 'c'")
}

#[test]
fn should_reject_node_referencing_undeclared_data() -> Result<(), String> {
    let errs = graph_errors_for("
        node state : register { format: hex8, source: Missing }
    ")?;

    check(
        errs.iter().any(
            |e| matches!(e, GraphError::UndeclaredData { name, .. } if name == "Missing"),
        ),
        "expected an UndeclaredData error for 'Missing'",
    )
}

#[test]
fn should_reject_duplicate_data_binding() -> Result<(), String> {
    let errs = graph_errors_for("
        data A = [0x1]
        data A = [0x2]
    ")?;

    check(
        errs.iter().any(|e| matches!(e, GraphError::DuplicateData(n) if n == "A")),
        "expected a DuplicateData error for 'A'",
    )
}

#[test]
fn should_reject_compute_calling_undeclared_function() -> Result<(), String> {
    let errs = graph_errors_for("
        node c : operation { symbol: \"X\", compute: Missing(state) }
    ")?;

    check(
        errs.iter().any(
            |e| matches!(e, GraphError::UndeclaredFn { callee, .. } if callee == "Missing"),
        ),
        "expected an UndeclaredFn error for 'Missing'",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_reject_cyclical_node_graph() -> Result<(), String> {
    let errs = graph_errors_for("
        node a : register {}
        node b : operation {}
        wire a -> b
        wire b -> a
    ")?;

    check(
        errs.iter().any(|e| matches!(e, GraphError::Cycle(nodes) if nodes.contains(&"a".to_string()))),
        "expected a Cycle error involving 'a'",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_reject_mutually_recursive_functions() -> Result<(), String> {
    let errs = graph_errors_for("
        fn ping(x: u32) = pong(x)
        fn pong(x: u32) = ping(x)
    ")?;

    check(
        errs.iter().any(|e| matches!(e, GraphError::FnCycle(_))),
        "expected an FnCycle error",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_reject_function_call_with_bad_arity() -> Result<(), String> {
    let errs = graph_errors_for("
        fn identity(x: u32) = x
        fn bad() = identity(1, 2)
    ")?;

    check(
        errs.iter().any(
            |e| matches!(e, GraphError::ArityMismatch { name, expected: 1, got: 2 } if name == "identity"),
        ),
        "expected an ArityMismatch error for 'identity'",
    )
}
