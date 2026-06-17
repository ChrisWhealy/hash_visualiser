//! Browser/WebAssembly entry point.
//!
//! Compiled into the `pkg/` bundle by `wasm-pack build --target web --features web` and driven from `index.html`.
//! Parsing and rendering both run in the browser: the `<svg>` tree is built directly in the live DOM via `svg_dom`.
//!
//! The `.hv` source is supplied by the page (fetched at runtime), not embedded, so `index.html` can re-render on every
//! edit of the file — no wasm rebuild or server restart needed.

use std::cell::RefCell;

use wasm_bindgen::prelude::*;
use svg_dom::SvgRoot;

use crate::{graph, parse, render};

thread_local! {
    // Holds the most recently rendered scene. The Step-button click closures live inside it, so it must stay alive for
    // the controls to keep working; storing it here (rather than leaking) means each re-render drops the previous one.
    static CURRENT_SCENE: RefCell<Option<render::Scene>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Parses the `.hv` `source`, lays it out, and renders it as live SVG inside the element `parent_id`.
///
/// The caller is expected to clear `parent_id` (and the transport container) before each call, so this can be invoked
/// repeatedly to re-render after the source changes. The previous render is dropped, releasing its handles.
///
/// Returns the parse/validation/DOM error as a `JsValue` so the page can surface it to the user.
#[wasm_bindgen]
pub fn run(parent_id: &str, source: &str) -> Result<(), JsValue> {
    let to_js = |msg: String| JsValue::from_str(&msg);

    let program = parse(source).map_err(|e| to_js(e.to_string()))?;
    let graph = graph::build(&program).map_err(|errs| {
        to_js(
            errs.iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("; "),
        )
    })?;

    let size = render::diagram_size(&graph);
    let svg = SvgRoot::create_in(parent_id, size).map_err(|e| to_js(format!("{e:?}")))?;
    let scene = render::render(&svg, &graph).map_err(|e| to_js(format!("{e:?}")))?;

    // Keep the new scene alive (its controls own their click closures); the previous one is dropped here.
    CURRENT_SCENE.with(|s| *s.borrow_mut() = Some(scene));

    Ok(())
}
