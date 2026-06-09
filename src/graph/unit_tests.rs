use crate::{
    ast::{
        ebnf_02::{Program, TopItem},
        ebnf_04::FnDef,
        ebnf_06::{NodeDecl, NodeKind},
        ebnf_07::{WireDecl, WireEndpoint},
        ebnf_11::Expr,
    },
    error::graph_error::GraphError,
    graph::build,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Helpers
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn node(name: &str) -> TopItem {
    TopItem::Node(NodeDecl {
        name: name.into(),
        kind: NodeKind::Operation,
        properties: vec![],
    })
}

fn wire(src: &str, dst: &str) -> TopItem {
    TopItem::Wire(WireDecl {
        name: None,
        source: WireEndpoint::Node(src.into()),
        target: WireEndpoint::Node(dst.into()),
    })
}

fn named_wire(wire_name: &str, src: &str, dst: &str) -> TopItem {
    TopItem::Wire(WireDecl {
        name: Some(wire_name.into()),
        source: WireEndpoint::Node(src.into()),
        target: WireEndpoint::Node(dst.into()),
    })
}

fn fn_def(name: &str, params: &[&str], body: Expr) -> TopItem {
    TopItem::FnDef(FnDef {
        name: name.into(),
        params: params.iter().map(|s| s.to_string()).collect(),
        body,
    })
}

fn program(items: Vec<TopItem>) -> Program {
    Program { items }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Happy-path tests
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_build_empty_program() {
    let g = build(&program(vec![])).unwrap();
    assert!(g.nodes.is_empty());
    assert!(g.layers.is_empty());
    assert!(g.edges.is_empty());
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_produce_two_layers_from_two_nodes_connected_by_one_wire() {
    let g = build(&program(vec![node("a"), node("b"), wire("a", "b")])).unwrap();
    assert_eq!(g.layers, vec![vec!["a".to_string()], vec!["b".to_string()]]);
    assert_eq!(g.edges, vec![("a".to_string(), "b".to_string())]);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_produce_three_layers_from_linear_chain() {
    let g = build(&program(vec![
        node("a"),
        node("b"),
        node("c"),
        wire("a", "b"),
        wire("b", "c"),
    ]))
    .unwrap();
    assert_eq!(
        g.layers,
        vec![
            vec!["a".to_string()],
            vec!["b".to_string()],
            vec!["c".to_string()],
        ]
    );
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_produce_three_layers_from_diamond_topology() {
    // A → B, A → C, B → D, C → D  →  [[A], [B,C], [D]]
    let g = build(&program(vec![
        node("a"),
        node("b"),
        node("c"),
        node("d"),
        wire("a", "b"),
        wire("a", "c"),
        wire("b", "d"),
        wire("c", "d"),
    ]))
    .unwrap();
    assert_eq!(g.layers.len(), 3);
    assert_eq!(g.layers[0], vec!["a"]);
    assert_eq!(g.layers[1], vec!["b", "c"]); // sorted alphabetically
    assert_eq!(g.layers[2], vec!["d"]);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_not_count_open_endpoint_wire_as_edge() {
    let g = build(&program(vec![
        node("a"),
        TopItem::Wire(WireDecl {
            name: None,
            source: WireEndpoint::Open,
            target: WireEndpoint::Node("a".into()),
        }),
    ]))
    .unwrap();
    assert_eq!(g.edges.len(), 0);
    assert_eq!(g.layers, vec![vec!["a".to_string()]]);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_assign_isolated_nodes_t0_their_own_layers() {
    let g = build(&program(vec![node("x"), node("y")])).unwrap();
    // No edges: both are in layer 0, sorted alphabetically
    assert_eq!(g.layers, vec![vec!["x".to_string(), "y".to_string()]]);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Error tests
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_reject_duplicate_node_name() {
    let errs = build(&program(vec![node("a"), node("a")])).unwrap_err();
    assert!(
        errs.iter()
            .any(|e| matches!(e, GraphError::DuplicateNode(n) if n == "a"))
    );
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_reject_duplicate_wire_name() {
    let errs = build(&program(vec![
        node("a"),
        node("b"),
        node("c"),
        named_wire("w", "a", "b"),
        named_wire("w", "b", "c"),
    ]))
    .unwrap_err();
    assert!(
        errs.iter()
            .any(|e| matches!(e, GraphError::DuplicateWireName(n) if n == "w"))
    );
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_reject_wire_to_undeclared_node() {
    let errs = build(&program(vec![node("a"), wire("a", "ghost")])).unwrap_err();
    assert!(
        errs.iter().any(
            |e| matches!(e, GraphError::UndeclaredNode { endpoint, .. } if endpoint == "ghost")
        )
    );
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_reject_cyclical_node_graph() {
    let errs = build(&program(vec![
        node("a"),
        node("b"),
        wire("a", "b"),
        wire("b", "a"),
    ]))
    .unwrap_err();
    assert!(errs.iter().any(|e| matches!(e, GraphError::Cycle(_))));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_reject_call_to_undeclared_function() {
    let errs = build(&program(vec![fn_def(
        "foo",
        &["x"],
        Expr::Call {
            name: "missing".into(),
            args: vec![Expr::Ident("x".into())],
        },
    )]))
    .unwrap_err();
    assert!(
        errs.iter()
            .any(|e| matches!(e, GraphError::UndeclaredFn { callee, .. } if callee == "missing"))
    );
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_reject_bad_arity_function_call() {
    let errs = build(&program(vec![
        fn_def("id", &["x"], Expr::Ident("x".into())),
        fn_def(
            "bad",
            &[],
            Expr::Call {
                name: "id".into(),
                args: vec![Expr::Integer(1), Expr::Integer(2)],
            },
        ),
    ]))
    .unwrap_err();
    assert!(errs.iter().any(
        |e| matches!(e, GraphError::ArityMismatch { name, expected: 1, got: 2 } if name == "id")
    ));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn shoulld_reject_mutually_recursive_functions() {
    let errs = build(&program(vec![
        fn_def(
            "foo",
            &["x"],
            Expr::Call {
                name: "bar".into(),
                args: vec![Expr::Ident("x".into())],
            },
        ),
        fn_def(
            "bar",
            &["x"],
            Expr::Call {
                name: "foo".into(),
                args: vec![Expr::Ident("x".into())],
            },
        ),
    ]))
    .unwrap_err();
    assert!(errs.iter().any(|e| matches!(e, GraphError::FnCycle(_))));
}
