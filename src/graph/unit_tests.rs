use crate::{
    ast::{
        ebnf_02::{Program, TopItem},
        ebnf_04::{FnDef, Param, Type},
        ebnf_06::{NodeDecl, NodeKind},
        ebnf_07::{WireDecl, WireEndpoint},
        ebnf_11::Expr,
    },
    error::graph_error::GraphError,
    graph::{ValidatedGraph, build, build_with_imports, imported_paths},
};
use std::collections::HashMap;

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
        // These tests only exercise arity and the call graph, so every parameter is given the same placeholder type
        // and the return type is left as unit.
        params: params
            .iter()
            .map(|s| Param {
                name: s.to_string(),
                ty: Type::U32,
            })
            .collect(),
        return_type: Type::Unit,
        body,
    })
}

fn program(items: Vec<TopItem>) -> Program {
    Program { items }
}

/// Builds a program, flattening the error list into one string so a test can `?` it.
fn build_ok(items: Vec<TopItem>) -> Result<ValidatedGraph, String> {
    build(&program(items))
        .map_err(|errs| errs.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("; "))
}

/// Builds a program expected to fail, returning its errors (or an error if it unexpectedly succeeded).
fn build_errs(items: Vec<TopItem>) -> Result<Vec<GraphError>, String> {
    match build(&program(items)) {
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
// Happy-path tests
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_build_empty_program() -> Result<(), String> {
    let g = build_ok(vec![])?;
    check(g.nodes.is_empty(), "nodes should be empty")?;
    check(g.layers.is_empty(), "layers should be empty")?;
    check(g.edges.is_empty(), "edges should be empty")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_produce_two_layers_from_two_nodes_connected_by_one_wire() -> Result<(), String> {
    let g = build_ok(vec![node("a"), node("b"), wire("a", "b")])?;
    eq(g.layers, vec![vec!["a".to_string()], vec!["b".to_string()]])?;
    eq(g.edges, vec![("a".to_string(), "b".to_string())])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_produce_three_layers_from_linear_chain() -> Result<(), String> {
    let g = build_ok(vec![
        node("a"),
        node("b"),
        node("c"),
        wire("a", "b"),
        wire("b", "c"),
    ])?;
    eq(
        g.layers,
        vec![
            vec!["a".to_string()],
            vec!["b".to_string()],
            vec!["c".to_string()],
        ],
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_produce_three_layers_from_diamond_topology() -> Result<(), String> {
    // A → B, A → C, B → D, C → D  →  [[A], [B,C], [D]]  (each layer sorted alphabetically)
    let g = build_ok(vec![
        node("a"),
        node("b"),
        node("c"),
        node("d"),
        wire("a", "b"),
        wire("a", "c"),
        wire("b", "d"),
        wire("c", "d"),
    ])?;
    eq(
        g.layers,
        vec![
            vec!["a".to_string()],
            vec!["b".to_string(), "c".to_string()],
            vec!["d".to_string()],
        ],
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_not_count_open_endpoint_wire_as_edge() -> Result<(), String> {
    let g = build_ok(vec![
        node("a"),
        TopItem::Wire(WireDecl {
            name: None,
            source: WireEndpoint::Open,
            target: WireEndpoint::Node("a".into()),
        }),
    ])?;
    eq(g.edges.len(), 0)?;
    eq(g.layers, vec![vec!["a".to_string()]])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_assign_isolated_nodes_t0_their_own_layers() -> Result<(), String> {
    let g = build_ok(vec![node("x"), node("y")])?;
    // No edges: both are in layer 0, sorted alphabetically
    eq(g.layers, vec![vec!["x".to_string(), "y".to_string()]])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Error tests
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_reject_duplicate_node_name() -> Result<(), String> {
    let errs = build_errs(vec![node("a"), node("a")])?;
    check(
        errs.iter().any(|e| matches!(e, GraphError::DuplicateNode(n) if n == "a")),
        "expected a DuplicateNode error for 'a'",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_reject_duplicate_wire_name() -> Result<(), String> {
    let errs = build_errs(vec![
        node("a"),
        node("b"),
        node("c"),
        named_wire("w", "a", "b"),
        named_wire("w", "b", "c"),
    ])?;
    check(
        errs.iter().any(|e| matches!(e, GraphError::DuplicateWireName(n) if n == "w")),
        "expected a DuplicateWireName error for 'w'",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_reject_wire_to_undeclared_node() -> Result<(), String> {
    let errs = build_errs(vec![node("a"), wire("a", "ghost")])?;
    check(
        errs.iter()
            .any(|e| matches!(e, GraphError::UndeclaredNode { endpoint, .. } if endpoint == "ghost")),
        "expected an UndeclaredNode error for 'ghost'",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_reject_cyclical_node_graph() -> Result<(), String> {
    let errs = build_errs(vec![
        node("a"),
        node("b"),
        wire("a", "b"),
        wire("b", "a"),
    ])?;
    check(
        errs.iter().any(|e| matches!(e, GraphError::Cycle(_))),
        "expected a Cycle error",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_reject_call_to_undeclared_function() -> Result<(), String> {
    let errs = build_errs(vec![fn_def(
        "foo",
        &["x"],
        Expr::Call {
            name: "missing".into(),
            args: vec![Expr::Ident("x".into())],
        },
    )])?;
    check(
        errs.iter().any(|e| matches!(e, GraphError::UndeclaredFn { callee, .. } if callee == "missing")),
        "expected an UndeclaredFn error for 'missing'",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_reject_bad_arity_function_call() -> Result<(), String> {
    let errs = build_errs(vec![
        fn_def("id", &["x"], Expr::Ident("x".into())),
        fn_def(
            "bad",
            &[],
            Expr::Call {
                name: "id".into(),
                args: vec![Expr::Integer(1), Expr::Integer(2)],
            },
        ),
    ])?;
    check(
        errs.iter().any(
            |e| matches!(e, GraphError::ArityMismatch { name, expected: 1, got: 2 } if name == "id"),
        ),
        "expected an ArityMismatch error for 'id' (1 vs 2)",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn shoulld_reject_mutually_recursive_functions() -> Result<(), String> {
    let errs = build_errs(vec![
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
    ])?;
    check(
        errs.iter().any(|e| matches!(e, GraphError::FnCycle(_))),
        "expected an FnCycle error",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Imports: `import "<path>"` brings another file's functions into scope (Phase 1 — fn reuse).
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn should_import_functions_into_scope() -> Result<(), String> {
    let main = std::fs::read_to_string("hv/sha3/theta.hv").map_err(|e| e.to_string())?;
    let program = crate::parser::parse(&main).map_err(|e| e.to_string())?;

    // theta.hv imports three sibling files (paths are relative to the importer; no transitive imports of their own).
    let paths = imported_paths(&program);
    eq(paths.clone(), vec!["theta_c.hv", "theta_d.hv", "theta_mix.hv"])?;

    // Map each relative path to the actual sibling file under hv/sha3/.
    let mut sources: HashMap<String, String> = HashMap::new();
    for p in &paths {
        let src = std::fs::read_to_string(format!("hv/sha3/{p}")).map_err(|e| e.to_string())?;
        sources.insert(p.clone(), src);
    }

    let graph = build_with_imports(&program, &sources)
        .map_err(|errs| errs.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("; "))?;

    // The imported functions are callable here, even though theta.hv defines no `fn` of its own; and each records the
    // file it came from (so the renderer can mark the calling node as expandable).
    for (f, file) in [("ThetaC", "theta_c.hv"), ("ThetaD", "theta_d.hv"), ("ThetaXor", "theta_mix.hv")] {
        check(graph.fn_defs.contains_key(f), &format!("expected imported fn {f} in scope"))?;
        eq(graph.fn_imports.get(f).map(String::as_str), Some(file))?;
    }
    Ok(())
}

#[test]
fn should_error_on_unresolved_import() -> Result<(), String> {
    let main = std::fs::read_to_string("hv/sha3/theta.hv").map_err(|e| e.to_string())?;
    let program = crate::parser::parse(&main).map_err(|e| e.to_string())?;

    // No sources supplied → every import is unresolved (and the calls to its fns can't be checked).
    let errs = match build_with_imports(&program, &HashMap::new()) {
        Err(errs) => errs,
        Ok(_) => return Err("expected build to fail without import sources".into()),
    };
    check(
        errs.iter().any(|e| matches!(e, GraphError::UnresolvedImport(_))),
        &format!("expected an UnresolvedImport error, got {errs:?}"),
    )
}
