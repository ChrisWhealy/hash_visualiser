// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §7 Wire declarations
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone)]
pub struct WireDecl {
    pub name: Option<String>,
    pub source: WireEndpoint,
    pub target: WireEndpoint,
}

#[derive(Debug, Clone)]
pub enum WireEndpoint {
    Node(String),
    Open,
}

