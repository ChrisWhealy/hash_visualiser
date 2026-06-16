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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Happy-path: flat programs (nodes and wires at top level)
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_produce_correct_layers_from_two_flat_wired_nodes() {
    let g = parse_and_build("
        node a : register {}
        node b : operation {}
        wire a -> b
    ").unwrap();

    assert_eq!(g.layers, vec![vec!["a"], vec!["b"]]);
    assert_eq!(g.edges, vec![("a".to_string(), "b".to_string())]);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_produce_three_layers_from_flat_diamond() {
    let g = parse_and_build("
        node a : constant {}
        node b : operation {}
        node c : operation {}
        node d : register {}
        wire a -> b
        wire a -> c
        wire b -> d
        wire c -> d
    ").unwrap();

    assert_eq!(g.layers.len(), 3);
    assert_eq!(g.layers[0], vec!["a"]);
    assert_eq!(g.layers[1], vec!["b", "c"]);
    assert_eq!(g.layers[2], vec!["d"]);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_set_flow_direction_from_layout_directive() {
    let g = parse_and_build("
        layout: top_to_bottom
        node a : register {}
    ").unwrap();

    assert_eq!(g.flow, FlowDirection::TopToBottom);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_capture_context_word_size() {
    let g = parse_and_build("
        context { word_size: 64 }
    ").unwrap();

    assert_eq!(g.word_size, Some(64));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_index_named_wire_by_name() {
    let g = parse_and_build("
        node src : register {}
        node dst : register {}
        wire carry: src -> dst
    ").unwrap();

    assert!(g.named_wires.contains_key("carry"));
    assert_eq!(g.named_wires.len(), 1);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Happy-path: hash blocks
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_produce_correct_layers_from_hash_block_nodes_and_wires() {
    let g = parse_and_build("
        hash SHA256 {
            node input  : register  { label: \"Input\" }
            node mixer  : operation { label: \"Mix\"   }
            node output : register  { label: \"Output\" }
            wire input -> mixer
            wire mixer -> output
        }
    ").unwrap();

    assert_eq!(g.layers, vec![vec!["input"], vec!["mixer"], vec!["output"]]);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_use_hash_block_layout_directive_to_override_default_flow() {
    let g = parse_and_build("
        hash SHA256 {
            layout: right_to_left
            node a : register {}
        }
    ").unwrap();

    assert_eq!(g.flow, FlowDirection::RightToLeft);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_capture_hash_block_context_word_size() {
    let g = parse_and_build("
        hash SHA256 {
            context { word_size: 32 }
            node a : register {}
        }
    ").unwrap();

    assert_eq!(g.word_size, Some(32));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Happy-path: functions
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_validate_and_collect_context_functions() {
    let g = parse_and_build("
        context {
            fn base(x: u32) = x rotr_u 2
            fn derived(x: u32) = base(x) xor base(x)
        }
    ").unwrap();

    assert!(g.fn_defs.contains_key("base"));
    assert!(g.fn_defs.contains_key("derived"));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_validate_parameterised_sigma_style_functions() {
    let g = parse_and_build("
        context {
            word_size: 64
            fn Sigma(e: u64, r1: u64, r2: u64, r3: u64) = (e rotr_u r1) xor (e rotr_u r2) xor (e shr_u r3)
            fn Sigma1(e: u64) = Sigma(e, 14, 18, 41)
        }
    ").unwrap();

    assert_eq!(g.word_size, Some(64));
    assert!(g.fn_defs.contains_key("Sigma"));
    assert!(g.fn_defs.contains_key("Sigma1"));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_merge_context_and_hash_block_items() {
    let g = parse_and_build("
        context { word_size: 32 }
        hash SHA256 {
            node a : register {}
            node b : operation {}
            wire a -> b
        }
    ").unwrap();

    assert_eq!(g.word_size, Some(32));
    assert_eq!(g.layers, vec![vec!["a"], vec!["b"]]);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Error paths
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_reject_wire_to_undeclared_node() {
    let errs = graph_errors_for("
        node a : register {}
        wire a -> ghost
    ").unwrap();

    assert!(errs.iter().any(
        |e| matches!(e, GraphError::UndeclaredNode { endpoint, .. } if endpoint == "ghost")
    ));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_reject_duplicate_node() {
    let errs = graph_errors_for("
        node a : register {}
        node a : operation {}
    ").unwrap();

    assert!(errs.iter().any(|e| matches!(e, GraphError::DuplicateNode(n) if n == "a")));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_reject_undeclared_function_call() {
    let errs = graph_errors_for("
        fn foo(x: u32) = missing(x)
    ").unwrap();

    assert!(errs.iter().any(
        |e| matches!(e, GraphError::UndeclaredFn { callee, .. } if callee == "missing")
    ));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_reject_cyclical_node_graph() {
    let errs = graph_errors_for("
        node a : register {}
        node b : operation {}
        wire a -> b
        wire b -> a
    ").unwrap();

    assert!(errs.iter().any(|e| matches!(e, GraphError::Cycle(nodes) if nodes.contains(&"a".to_string()))));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_reject_mutually_recursive_functions() {
    let errs = graph_errors_for("
        fn ping(x: u32) = pong(x)
        fn pong(x: u32) = ping(x)
    ").unwrap();

    assert!(errs.iter().any(|e| matches!(e, GraphError::FnCycle(_))));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_reject_function_call_with_bad_arity() {
    let errs = graph_errors_for("
        fn identity(x: u32) = x
        fn bad() = identity(1, 2)
    ").unwrap();

    assert!(errs.iter().any(
        |e| matches!(e, GraphError::ArityMismatch { name, expected: 1, got: 2 } if name == "identity")
    ));
}
