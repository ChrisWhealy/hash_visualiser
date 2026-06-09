use crate::ast::{
    ebnf_03::ContextBlock,
    ebnf_04::FnDef,
    ebnf_06::NodeDecl,
    ebnf_07::WireDecl,
    ebnf_08::{FlowDirection, GroupDecl},
    ebnf_09::EventHandler,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §5 Hash block
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone)]
pub struct HashBlock {
    pub name: String,
    pub items: Vec<HashItem>,
}

#[derive(Debug, Clone)]
pub enum HashItem {
    Context(ContextBlock),
    FnDef(FnDef),
    Node(NodeDecl),
    Wire(WireDecl),
    Group(GroupDecl),
    Layout(FlowDirection),
    EventHandler(EventHandler),
}
