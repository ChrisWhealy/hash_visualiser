mod kahn_sort;

use crate::{
    ast::{
        ebnf_02::{Program, TopItem},
        ebnf_03::{ContextBlock, ContextItem},
        ebnf_04::FnDef,
        ebnf_05::{HashBlock, HashItem},
        ebnf_06::NodeDecl,
        ebnf_07::{WireDecl, WireEndpoint},
        ebnf_08::FlowDirection,
        ebnf_09::EventHandler,
        ebnf_10::{Effect, EmitTarget},
        ebnf_11::Expr,
    },
    error::graph_error::GraphError,
};
use kahn_sort::kahn_sort;
use std::collections::{HashMap, HashSet, VecDeque};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Output type — a validated, topology-resolved model ready for layout
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug)]
pub struct ValidatedGraph {
    pub nodes: HashMap<String, NodeDecl>,
    pub wires: Vec<WireDecl>,
    pub named_wires: HashMap<String, WireDecl>,
    pub fn_defs: HashMap<String, FnDef>,
    pub edges: Vec<(String, String)>,
    pub layers: Vec<Vec<String>>,
    pub flow: FlowDirection,
    pub word_size: Option<u64>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Entry point
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub fn build(program: &Program) -> Result<ValidatedGraph, Vec<GraphError>> {
    let mut errors: Vec<GraphError> = Vec::new();
    let mut nodes: HashMap<String, NodeDecl> = HashMap::new();
    let mut wires: Vec<WireDecl> = Vec::new();
    let mut named_wires: HashMap<String, WireDecl> = HashMap::new();
    let mut fn_defs: HashMap<String, FnDef> = HashMap::new();
    let mut event_handlers: Vec<EventHandler> = Vec::new();
    let mut flow = FlowDirection::LeftToRight;
    let mut word_size: Option<u64> = None;

    // Pass 1 — collect all declarations
    for item in &program.items {
        match item {
            TopItem::Node(n) => insert_node(n, &mut nodes, &mut errors),
            TopItem::Wire(w) => insert_wire(w, &mut wires, &mut named_wires, &mut errors),
            TopItem::FnDef(f) => insert_fn(f, &mut fn_defs, &mut errors),
            TopItem::Context(ctx) => {
                collect_context(ctx, &mut fn_defs, &mut word_size, &mut errors)
            }
            TopItem::Hash(h) => collect_hash(
                h,
                &mut nodes,
                &mut wires,
                &mut named_wires,
                &mut fn_defs,
                &mut event_handlers,
                &mut flow,
                &mut word_size,
                &mut errors,
            ),
            TopItem::Layout(f) => flow = f.clone(),
            TopItem::EventHandler(e) => event_handlers.push(e.clone()),
            TopItem::Group(_) => {}
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    // Pass 2 — validate references
    for wire in &wires {
        validate_wire(wire, &nodes, &mut errors);
    }

    for handler in &event_handlers {
        validate_handler(handler, &nodes, &named_wires, &mut errors);
    }

    validate_fn_calls(&fn_defs, &mut errors);

    if !errors.is_empty() {
        return Err(errors);
    }

    // Pass 3 — build topology + topo-sort
    let edges: Vec<(String, String)> = wires
        .iter()
        .filter_map(|w| match (&w.source, &w.target) {
            (WireEndpoint::Node(s), WireEndpoint::Node(t)) => Some((s.clone(), t.clone())),
            _ => None,
        })
        .collect();

    match kahn_sort(&nodes, &edges) {
        Ok(layers) => Ok(ValidatedGraph {
            nodes,
            wires,
            named_wires,
            fn_defs,
            edges,
            layers,
            flow,
            word_size,
        }),
        Err(cycle) => {
            errors.push(GraphError::Cycle(cycle));
            Err(errors)
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Pass 1 helpers — declaration collectors
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn insert_node(n: &NodeDecl, nodes: &mut HashMap<String, NodeDecl>, errors: &mut Vec<GraphError>) {
    if nodes.insert(n.name.clone(), n.clone()).is_some() {
        errors.push(GraphError::DuplicateNode(n.name.clone()));
    }
}

fn insert_wire(
    w: &WireDecl,
    wires: &mut Vec<WireDecl>,
    named_wires: &mut HashMap<String, WireDecl>,
    errors: &mut Vec<GraphError>,
) {
    if let Some(name) = &w.name {
        if named_wires.insert(name.clone(), w.clone()).is_some() {
            errors.push(GraphError::DuplicateWireName(name.clone()));
        }
    }
    wires.push(w.clone());
}

fn insert_fn(f: &FnDef, fn_defs: &mut HashMap<String, FnDef>, errors: &mut Vec<GraphError>) {
    if fn_defs.insert(f.name.clone(), f.clone()).is_some() {
        errors.push(GraphError::DuplicateFn(f.name.clone()));
    }
}

fn collect_context(
    ctx: &ContextBlock,
    fn_defs: &mut HashMap<String, FnDef>,
    word_size: &mut Option<u64>,
    errors: &mut Vec<GraphError>,
) {
    for item in &ctx.items {
        match item {
            ContextItem::FnDef(f) => insert_fn(f, fn_defs, errors),
            ContextItem::WordSize(n) => *word_size = Some(*n),
        }
    }
}

fn collect_hash(
    h: &HashBlock,
    nodes: &mut HashMap<String, NodeDecl>,
    wires: &mut Vec<WireDecl>,
    named_wires: &mut HashMap<String, WireDecl>,
    fn_defs: &mut HashMap<String, FnDef>,
    event_handlers: &mut Vec<EventHandler>,
    flow: &mut FlowDirection,
    word_size: &mut Option<u64>,
    errors: &mut Vec<GraphError>,
) {
    for item in &h.items {
        match item {
            HashItem::Node(n) => insert_node(n, nodes, errors),
            HashItem::Wire(w) => insert_wire(w, wires, named_wires, errors),
            HashItem::FnDef(f) => insert_fn(f, fn_defs, errors),
            HashItem::Context(ctx) => collect_context(ctx, fn_defs, word_size, errors),
            HashItem::EventHandler(e) => event_handlers.push(e.clone()),
            HashItem::Layout(f) => *flow = f.clone(),
            HashItem::Group(_) => {}
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Pass 2 helpers — reference validators
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn validate_wire(wire: &WireDecl, nodes: &HashMap<String, NodeDecl>, errors: &mut Vec<GraphError>) {
    let name = wire.name.as_deref();
    for ep in [&wire.source, &wire.target] {
        if let WireEndpoint::Node(n) = ep {
            if !nodes.contains_key(n) {
                errors.push(GraphError::UndeclaredNode {
                    wire_name: name.map(str::to_owned),
                    endpoint: n.clone(),
                });
            }
        }
    }
}

fn validate_handler(
    handler: &EventHandler,
    nodes: &HashMap<String, NodeDecl>,
    named_wires: &HashMap<String, WireDecl>,
    errors: &mut Vec<GraphError>,
) {
    if !nodes.contains_key(&handler.node) {
        errors.push(GraphError::HandlerOnUndeclaredNode(handler.node.clone()));
    }
    for effect in &handler.body {
        validate_effect(effect, nodes, named_wires, errors);
    }
}

fn validate_effect(
    effect: &Effect,
    nodes: &HashMap<String, NodeDecl>,
    named_wires: &HashMap<String, WireDecl>,
    errors: &mut Vec<GraphError>,
) {
    match effect {
        Effect::Emit(e) => match &e.target {
            Some(EmitTarget::Node(n)) => {
                if !nodes.contains_key(n) {
                    errors.push(GraphError::UndeclaredNode {
                        wire_name: None,
                        endpoint: n.clone(),
                    });
                }
            }
            Some(EmitTarget::Via(w)) => {
                if !named_wires.contains_key(w) {
                    errors.push(GraphError::UndeclaredWire(w.clone()));
                }
            }
            Some(EmitTarget::All) | None => {}
        },
        Effect::Reroute(r) => {
            if !named_wires.contains_key(&r.wire) {
                errors.push(GraphError::UndeclaredWire(r.wire.clone()));
            }
            if !nodes.contains_key(&r.node) {
                errors.push(GraphError::UndeclaredNode {
                    wire_name: None,
                    endpoint: r.node.clone(),
                });
            }
        }
        Effect::Set(_) | Effect::Animate(_) | Effect::Let(_) => {}
    }
}

fn validate_fn_calls(fn_defs: &HashMap<String, FnDef>, errors: &mut Vec<GraphError>) {
    let mut call_graph: HashMap<String, Vec<String>> = HashMap::new();

    for (caller_name, def) in fn_defs {
        let mut raw_calls: Vec<(String, usize)> = Vec::new();
        walk_expr_calls(&def.body, &mut raw_calls);

        let mut callees: HashSet<String> = HashSet::new();
        for (callee, arity) in raw_calls {
            match fn_defs.get(&callee) {
                None => errors.push(GraphError::UndeclaredFn {
                    caller: caller_name.clone(),
                    callee,
                }),
                Some(callee_def) if callee_def.params.len() != arity => {
                    errors.push(GraphError::ArityMismatch {
                        name: callee.clone(),
                        expected: callee_def.params.len(),
                        got: arity,
                    });
                }
                Some(_) => {
                    callees.insert(callee);
                }
            }
        }
        call_graph.insert(caller_name.clone(), callees.into_iter().collect());
    }

    // Kahn's on call graph to detect recursive cycles
    let mut in_deg: HashMap<String, usize> = fn_defs.keys().map(|n| (n.clone(), 0)).collect();
    for callees in call_graph.values() {
        for callee in callees {
            *in_deg.entry(callee.clone()).or_insert(0) += 1;
        }
    }

    let mut queue: VecDeque<String> = in_deg
        .iter()
        .filter(|&(_, &d)| d == 0)
        .map(|(n, _)| n.clone())
        .collect();
    let mut visited: HashSet<String> = HashSet::new();

    while let Some(node) = queue.pop_front() {
        for callee in call_graph.get(&node).into_iter().flatten() {
            let deg = in_deg.get_mut(callee).unwrap();
            *deg -= 1;
            if *deg == 0 {
                queue.push_back(callee.clone());
            }
        }
        visited.insert(node);
    }

    if visited.len() < fn_defs.len() {
        let mut cycle: Vec<String> = fn_defs
            .keys()
            .filter(|n| !visited.contains(*n))
            .cloned()
            .collect();
        cycle.sort();
        errors.push(GraphError::FnCycle(cycle));
    }
}

fn walk_expr_calls(expr: &Expr, calls: &mut Vec<(String, usize)>) {
    match expr {
        Expr::Call { name, args } => {
            calls.push((name.clone(), args.len()));
            for arg in args {
                walk_expr_calls(arg, calls);
            }
        }
        Expr::BinOp { lhs, rhs, .. } => {
            walk_expr_calls(lhs, calls);
            walk_expr_calls(rhs, calls);
        }
        Expr::Not(inner) => walk_expr_calls(inner, calls),
        Expr::Integer(_) | Expr::HexLit(_) | Expr::Ident(_) => {}
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod pipeline_tests;
#[cfg(test)]
mod unit_tests;
