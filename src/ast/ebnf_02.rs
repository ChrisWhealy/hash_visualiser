use crate::ast::{
    ebnf_03::ContextBlock,
    ebnf_04::FnDef,
    ebnf_05::HashBlock,
    ebnf_06::{DataDecl, NodeDecl},
    ebnf_07::WireDecl,
    ebnf_08::{FlowDirection, GroupDecl},
    ebnf_09::EventHandler,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §2 Top-level
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<TopItem>,
}

/// `import "<path>"` — brings the function definitions from another `.hv` file into scope (so they can be called by
/// `compute:` here) without copying their bodies. The path is resolved relative to the `hv/` root.
#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub path: String,
}

#[derive(Debug, Clone)]
pub enum TopItem {
    Context(ContextBlock),
    FnDef(FnDef),
    Import(ImportDecl),
    Hash(HashBlock),
    Node(NodeDecl),
    Wire(WireDecl),
    Group(GroupDecl),
    Layout(FlowDirection),
    EventHandler(EventHandler),
    Data(DataDecl),
}
