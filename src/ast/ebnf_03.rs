use crate::ast::ebnf_04::FnDef;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §3 Context block
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone)]
pub struct ContextBlock {
    pub items: Vec<ContextItem>,
}

#[derive(Debug, Clone)]
pub enum ContextItem {
    WordSize(u64),
    FnDef(FnDef),
}
