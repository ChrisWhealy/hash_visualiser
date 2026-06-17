pub mod ast;
pub mod error;
pub mod graph;
pub mod lexer;
pub mod parser;
pub mod render;

#[cfg(feature = "web")]
mod web;

pub use parser::parse;
pub use render::{Scene, render};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Default example
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub const BUILTIN_EXAMPLE: &str = include_str!("../hv/sha256.hv");

/// SHA3 theta_c example — demonstrates an array-typed node rendered as a hex8 grid with step-through highlighting.
pub const SHA3_EXAMPLE: &str = include_str!("../hv/sha3.hv");
