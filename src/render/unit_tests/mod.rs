mod array_grid;
mod cell_formatting;
mod declared_node;
mod description;
mod inner_reduction;
mod layout;
mod nested_map;
mod step_buttons;

use crate::{
    graph::{ValidatedGraph, build},
    render::{
        cell_width, description_html, format_value, grid_size, grid_spec, inferred_grid_shape,
        is_map_operation, nested_map, placeholder_value, step_back, step_forward, step_range,
    },
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// SvgNode creation needs a live browser DOM, so the rendering itself is exercised by the svg-dom crate's
// wasm-bindgen tests. Here we cover the pure layout geometry, which is what positions every translated object.
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn parse_and_build(src: &str) -> ValidatedGraph {
    let program = crate::parse(src).expect("parse failed");
    build(&program).expect("build failed")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn check(cond: bool, msg: &str) -> Result<(), String> {
    if cond { Ok(()) } else { Err(msg.to_string()) }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn format_cell(index: usize, digits: usize) -> String {
    format_value(placeholder_value(index), digits)
}
