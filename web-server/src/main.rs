//! Static file server for the hash-visualiser web UI.
//!
//! Run from the project root with:
//! ```sh
//! cargo serve
//! ```
//! This rebuilds the wasm package (`wasm-pack build --target web --features web`) and then serves the project root.
//! The UI lives at <http://127.0.0.1:8000/web>.
//!
//! The port number can be overridden using the `PORT` environment variable, e.g. `PORT=9000 cargo serve`.
//!
//! Live-reload: rather than have the browser poll for changes, the server watches the `hv/` folder and *pushes* the
//! `.hv` source to connected clients over a Server-Sent Events stream at `/events` (see [`events`]). The client renders
//! each message it receives, so editing the `.hv` re-renders the diagram without the need for continuous polling
//! between edits.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::{self, Command},
};

use actix_files::Files;
use actix_web::{App, HttpResponse, HttpServer, get, middleware::Logger, web};
use futures_util::stream::StreamExt;
use notify::{Event, RecursiveMode, Watcher, recommended_watcher};
use serde::Serialize;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

/// Directory (relative to the project root) holding the `.hv` sources the UI can render. Its files are listed for the
/// sidebar, served statically, and watched for live-reload.
const HV_DIR: &str = "hv";

/// A live-reload message pushed to clients over the SSE stream, serialised to JSON and tagged by `kind`:
/// - `file`  — a `.hv` file's contents changed; the client re-renders if it is the one currently selected.
/// - `files` — the directory listing changed (a file was created or deleted); the client refreshes its sidebar.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Message {
    File { file: String, content: String },
    Files { files: Vec<String> },
}

/// Shared state handed to request handlers.
struct AppState {
    /// Broadcasts each changed `.hv` file (as JSON) to every connected SSE client.
    tx: broadcast::Sender<String>,
    /// Absolute path of the watched `hv/` directory, used to list the available files.
    hv_dir: PathBuf,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // The web-server crate lives one level below the project root.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("web-server must live inside the project")
        .to_path_buf();

    // Rebuild the wasm bundle so the served `pkg/` is up to date.
    build_wasm(&root);

    let hv_dir = root.join(HV_DIR);

    // Fan-out channel: the watcher publishes; each SSE client subscribes. A small buffer is plenty since clients only
    // care about the most recent version (a lagging client simply skips to the latest).
    let (tx, _rx) = broadcast::channel::<String>(16);

    // Watch the directory containing the .hv files for changes and push each new version.
    // Since the watcher must stay alive for the lifespan of this process, it is held in this binding (kept across the
    // `.run().await` below).
    let _watcher = spawn_watcher(&hv_dir, tx.clone());

    let state = web::Data::new(AppState {
        tx,
        hv_dir: hv_dir.clone(),
    });

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8000);
    let addr = ("127.0.0.1", port);

    println!("\n  hash-visualiser running on http://127.0.0.1:{port}/web\n");

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(state.clone())
            .service(list_files)
            .service(events)
            .service(Files::new("/", root.clone()).index_file("index.html"))
    })
    .bind(addr)?
    .run()
    .await
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Lists the `.hv` files available to render, as a JSON array of file names (sorted), for the sidebar.
#[get("/api/files")]
async fn list_files(state: web::Data<AppState>) -> web::Json<Vec<String>> {
    web::Json(read_hv_files(&state.hv_dir).into_keys().collect())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Server-Sent Events stream of `.hv` changes.
///
/// The client receives a JSON [`Message`] whenever the watched directory changes: a `file` message (re-render if it is
/// the selected file) or a `files` message (refresh the sidebar listing). The initial render and listing come from the
/// client fetching them directly, so no snapshot is sent on connect. The browser's `EventSource` reconnects
/// automatically if the stream drops.
#[get("/events")]
async fn events(state: web::Data<AppState>) -> HttpResponse {
    let rx = state.tx.subscribe();

    // Forward each broadcast message; a `Lagged` error just means we skip ahead to the latest.
    let updates = BroadcastStream::new(rx)
        .filter_map(|msg| async move { msg.ok().map(|json| Ok::<_, std::io::Error>(sse_event(&json))) });

    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/event-stream"))
        .insert_header(("Cache-Control", "no-cache"))
        .streaming(updates)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Encodes `content` as a single server sent event (SSE) message.
/// Each line becomes its own `data:` field (the spec joins them back with `\n`), and the leading space after `data:` is
/// the optional separator the client strips — so the file's own indentation is preserved.
fn sse_event(content: &str) -> web::Bytes {
    let mut out = String::with_capacity(content.len() + 16);
    for line in content.split('\n') {
        out.push_str("data: ");
        out.push_str(line);
        out.push('\n');
    }
    out.push('\n'); // blank line terminates the event
    web::Bytes::from(out)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Reads every `.hv` file under `dir` (recursively, into subdirectories), returning a relative-path → contents map.
/// Sorted (it's a `BTreeMap`) so the file list is stable; unreadable files and non-`.hv` entries are skipped.
fn read_hv_files(dir: &Path) -> BTreeMap<String, String> {
    let mut sources = BTreeMap::new();
    collect_hv_files(dir, dir, &mut sources);
    sources
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Recursively collects the `.hv` files under `root`, keyed by their path *relative* to `root` (forward-slashed, e.g.
/// `binary_operations/01_add.hv`) so subdirectories are represented in the listing and served URLs.
fn collect_hv_files(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_hv_files(root, &path, out);
        } else if path.extension().is_some_and(|ext| ext == "hv")
            && let Ok(rel) = path.strip_prefix(root)
            && let Some(rel) = rel.to_str()
            && let Ok(content) = std::fs::read_to_string(&path)
        {
            out.insert(rel.replace(std::path::MAIN_SEPARATOR, "/"), content);
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Broadcasts `msg` (as JSON) to every connected SSE client. A send error just means no clients are connected.
fn broadcast(tx: &broadcast::Sender<String>, msg: &Message) {
    if let Ok(json) = serde_json::to_string(msg) {
        let _ = tx.send(json);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Watches `hv_dir` and broadcasts the appropriate [`Message`] whenever its content changes.
///
/// The directory is watched recursively (not the individual files) so:
/// - file creations/deletions, changes in subdirectories, and atomic-save renames are all seen
/// - all `.hv` files share a single watcher
///
/// On every filesystem event the directory is re-scanned against the last seen state: a changed listing emits a
/// `files` message (so clients refresh their sidebar) and each file whose contents changed emits a `file` message.
/// 
/// Comparing against the last state means duplicate filesystem events don't trigger duplicate broadcasts. Returns the
/// watcher, which the caller must keep alive.
fn spawn_watcher(hv_dir: &Path, tx: broadcast::Sender<String>) -> impl Watcher {
    let dir = hv_dir.to_path_buf();
    let mut last = read_hv_files(&dir);

    let mut watcher = recommended_watcher(move |res: notify::Result<Event>| {
        if res.is_err() {
            return;
        }
        let current = read_hv_files(&dir);

        // A created or deleted `.hv` file changes the listing: tell clients to refresh their file list.
        if current.keys().ne(last.keys()) {
            broadcast(&tx, &Message::Files { files: current.keys().cloned().collect() });
        }

        // Push the new contents of any file whose body changed (covers edits and newly created files).
        for (file, content) in &current {
            if last.get(file).map(String::as_str) != Some(content.as_str()) {
                broadcast(&tx, &Message::File { file: file.clone(), content: content.clone() });
            }
        }

        last = current;
    })
    .expect("create filesystem watcher");

    watcher
        .watch(hv_dir, RecursiveMode::Recursive)
        .expect("watch hv directory");

    watcher
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Rebuilds the wasm package so the served `pkg/` is up to date.
///
/// A failure here is fatal: rather than silently serving a stale `pkg/`, the error is reported and the process exits.
fn build_wasm(root: &Path) {
    println!("Building wasm package: wasm-pack build --target web --features web");

    match Command::new("wasm-pack")
        .current_dir(root)
        .args(["build", "--target", "web", "--features", "web"])
        .status()
    {
        Ok(status) if status.success() => {}
        Ok(status) => {
            eprintln!("aborting: wasm-pack exited with {status}");
            process::exit(1);
        }
        Err(err) => {
            eprintln!("aborting: could not run wasm-pack ({err})");
            process::exit(1);
        }
    }
}
