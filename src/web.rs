//! Browser/WebAssembly entry point.
//!
//! Compiled into the `pkg/` bundle by `wasm-pack build --target web --features web` and driven from `index.html`.
//! Parsing and rendering both run in the browser: the `<svg>` tree is built directly in the live DOM via `svg_dom`.
//!
//! The `.hv` source is supplied by the page (fetched at runtime), not embedded, so `index.html` can re-render on every
//! edit of the file — no wasm rebuild or server restart needed.

use std::cell::RefCell;
use std::collections::HashMap;

use wasm_bindgen::prelude::*;
use svg_dom::SvgRoot;

use crate::{graph, parse, render};

thread_local! {
    // Rendered scenes, keyed by the container element id they were drawn into. A scene's Step-button click closures
    // live inside it, so it must stay alive for the controls to keep working. Keying by container (rather than a single
    // slot) lets the main diagram AND any open modal overlays — each rendered into its own element — stay live at once;
    // re-rendering an element replaces its scene, and closing a modal drops its scene via `drop_scene`.
    static SCENES: RefCell<HashMap<String, render::Scene>> = RefCell::new(HashMap::new());
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
    run_with_imports(parent_id, "transport", source, Vec::new(), Vec::new())
}

/// The import paths declared directly in `source` (e.g. `["sha3/theta_c.hv", …]`). The page calls this to discover
/// which files to fetch before rendering, calling it again on each fetched file to walk the full dependency closure.
#[wasm_bindgen]
pub fn imports(source: &str) -> Result<Vec<String>, JsValue> {
    let program = parse(source).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(graph::imported_paths(&program))
}

/// Like [`run`], but with the `import`ed files' sources supplied as parallel `import_paths` / `import_sources` arrays
/// (the transitive closure the page fetched). Each imported file's `fn` definitions are pulled into scope before the
/// graph is built, so a file can call functions defined in another without copying them.
#[wasm_bindgen]
pub fn run_with_imports(
    parent_id: &str,
    transport_id: &str,
    source: &str,
    import_paths: Vec<String>,
    import_sources: Vec<String>,
) -> Result<(), JsValue> {
    let to_js = |msg: String| JsValue::from_str(&msg);

    let program = parse(source).map_err(|e| to_js(e.to_string()))?;
    let sources: HashMap<String, String> = import_paths.into_iter().zip(import_sources).collect();
    let graph = graph::build_with_imports(&program, &sources).map_err(|errs| {
        to_js(
            errs.iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("; "),
        )
    })?;

    let size = render::diagram_size(&graph);
    let svg = SvgRoot::create_in(parent_id, size).map_err(|e| to_js(format!("{e:?}")))?;
    let scene = render::render(&svg, &graph, transport_id).map_err(|e| to_js(format!("{e:?}")))?;

    // Keep the new scene alive (its controls own their click closures); any previous scene for this container is
    // dropped as it's replaced.
    SCENES.with(|s| s.borrow_mut().insert(parent_id.to_string(), scene));

    Ok(())
}

/// Releases the scene rendered into `parent_id` (e.g. when a modal overlay is closed), dropping its retained click
/// closures. The caller is expected to remove the element itself from the DOM.
#[wasm_bindgen]
pub fn drop_scene(parent_id: &str) {
    SCENES.with(|s| {
        s.borrow_mut().remove(parent_id);
    });
}
