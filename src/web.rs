//! Browser/WebAssembly entry point.
//!
//! Compiled into the `pkg/` bundle by `wasm-pack build --target web --features web` and driven from `index.html`.
//! Parsing and rendering both run in the browser: the `<svg>` tree is built directly in the live DOM via `svg_dom`.

use wasm_bindgen::prelude::*;
use svg_dom::SvgRoot;

use crate::{SHA3_EXAMPLE, graph, parse, render};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Parses the built-in `.hv` example, lays it out, and renders it as live SVG inside the element `parent_id`.
///
/// Currently showcases the SHA3 theta_c example: the 5x5 state is drawn as a hex8 grid with Step buttons that walk the
/// highlight through each row.
///
/// Returns the parse/validation/DOM error as a `JsValue` so the page can surface it to the user.
#[wasm_bindgen]
pub fn run(parent_id: &str) -> Result<(), JsValue> {
    let to_js = |msg: String| JsValue::from_str(&msg);

    let program = parse(SHA3_EXAMPLE).map_err(|e| to_js(e.to_string()))?;
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

    // The Step buttons own their click closures; leak the scene so they stay live for the page lifetime.
    std::mem::forget(scene);

    Ok(())
}
