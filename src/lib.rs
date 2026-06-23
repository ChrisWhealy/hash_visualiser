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
