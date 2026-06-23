// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug)]
pub enum GraphError {
    DuplicateNode(String),
    DuplicateWireName(String),
    DuplicateFn(String),
    DuplicateData(String),
    UndeclaredData { node: String, name: String },
    UndeclaredNode { wire_name: Option<String>, endpoint: String },
    UndeclaredWire(String),
    UndeclaredFn { caller: String, callee: String },
    ArityMismatch { name: String, expected: usize, got: usize },
    HandlerOnUndeclaredNode(String),
    Cycle(Vec<String>),
    FnCycle(Vec<String>),
    /// `import "<path>"` could not be resolved — no source was supplied for that path.
    UnresolvedImport(String),
    /// An imported file failed to parse.
    ImportParse { path: String, message: String },
    /// Imports form a cycle (a file imports itself, directly or transitively).
    ImportCycle(String),
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphError::DuplicateNode(n) =>
                write!(f, "duplicate node name '{n}'"),
            GraphError::DuplicateWireName(n) =>
                write!(f, "duplicate wire name '{n}'"),
            GraphError::DuplicateFn(n) =>
                write!(f, "duplicate function name '{n}'"),
            GraphError::DuplicateData(n) =>
                write!(f, "duplicate data binding '{n}'"),
            GraphError::UndeclaredData { node, name } =>
                write!(f, "node '{node}' references undeclared data binding '{name}'"),
            GraphError::UndeclaredNode { wire_name: Some(w), endpoint } =>
                write!(f, "wire '{w}' references undeclared node '{endpoint}'"),
            GraphError::UndeclaredNode { wire_name: None, endpoint } =>
                write!(f, "wire references undeclared node '{endpoint}'"),
            GraphError::UndeclaredWire(n) =>
                write!(f, "reference to undeclared wire '{n}'"),
            GraphError::UndeclaredFn { caller, callee } =>
                write!(f, "function '{caller}' calls undeclared function '{callee}'"),
            GraphError::ArityMismatch { name, expected, got } =>
                write!(f, "function '{name}' expects {expected} argument(s) but got {got}"),
            GraphError::HandlerOnUndeclaredNode(n) =>
                write!(f, "event handler references undeclared node '{n}'"),
            GraphError::Cycle(nodes) =>
                write!(f, "cycle detected among nodes: {}", nodes.join(", ")),
            GraphError::FnCycle(fns) =>
                write!(f, "cycle detected among functions: {}", fns.join(", ")),
            GraphError::UnresolvedImport(path) =>
                write!(f, "could not resolve import '{path}' (no source supplied)"),
            GraphError::ImportParse { path, message } =>
                write!(f, "failed to parse imported file '{path}': {message}"),
            GraphError::ImportCycle(path) =>
                write!(f, "import cycle detected at '{path}'"),
        }
    }
}
