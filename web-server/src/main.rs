//! Static file server for the hash-visualiser web UI.
//!
//! Run from the project root with:
//! ```sh
//! cargo serve
//! ```
//! This rebuilds the wasm package (`wasm-pack build --target web --features web`) and then serves the project root, so
//! the UI lives at <http://127.0.0.1:8000/>.
//!
//! The port number can be overridden using the `PORT` environment variable, e.g. `PORT=9000 cargo serve`.
//!
//! Live-reload: rather than have the browser poll for changes, the server watches the `hv/` folder and *pushes* the
//! `.hv` source to connected clients over a Server-Sent Events stream at `/events` (see [`events`]). The client renders
//! each message it receives, so editing the `.hv` re-renders the diagram with no network traffic between edits.

use std::{
    path::{Path, PathBuf},
    process::{self, Command},
};

use actix_files::Files;
use actix_web::{App, HttpResponse, HttpServer, get, middleware::Logger, web};
use futures_util::stream::{self, StreamExt};
use notify::{Event, RecursiveMode, Watcher, recommended_watcher};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

/// The `.hv` source rendered by the UI; watched for live-reload and streamed to clients.
const HV_FILE: &str = "hv/sha3.hv";

/// Shared state handed to request handlers.
struct AppState {
    /// Broadcasts the latest `.hv` source to every connected SSE client.
    tx: broadcast::Sender<String>,
    /// Absolute path of the watched `.hv` file, used to send each new client an initial snapshot.
    hv_path: PathBuf,
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

    let hv_path = root.join(HV_FILE);

    // Fan-out channel: the watcher publishes; each SSE client subscribes. A small buffer is plenty since clients only
    // care about the most recent version (a lagging client simply skips to the latest).
    let (tx, _rx) = broadcast::channel::<String>(16);

    // Watch the file for changes and push each new version. The watcher must stay alive for the life of the process,
    // so it is held in this binding (kept across the `.run().await` below).
    let _watcher = spawn_watcher(&hv_path, tx.clone());

    let state = web::Data::new(AppState {
        tx,
        hv_path: hv_path.clone(),
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
            .service(events)
            .service(Files::new("/", root.clone()).index_file("index.html"))
    })
    .bind(addr)?
    .run()
    .await
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Server-Sent Events stream of the `.hv` source.
///
/// On connect the client receives one event with the current file contents (so it renders immediately), then a fresh
/// event every time the watched file changes. The browser's `EventSource` reconnects automatically if the stream drops.
#[get("/events")]
async fn events(state: web::Data<AppState>) -> HttpResponse {
    let rx = state.tx.subscribe();

    // Snapshot first, so a freshly-connected client renders without waiting for the next edit.
    let initial = std::fs::read_to_string(&state.hv_path).unwrap_or_default();
    let initial = stream::once(async move { Ok::<_, std::io::Error>(sse_event(&initial)) });

    // Then forward each broadcast version; a `Lagged` error just means we skip ahead to the latest.
    let updates = BroadcastStream::new(rx).filter_map(|msg| async move {
        msg.ok()
            .map(|content| Ok::<_, std::io::Error>(sse_event(&content)))
    });

    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/event-stream"))
        .insert_header(("Cache-Control", "no-cache"))
        .streaming(initial.chain(updates))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Encodes `content` as a single SSE message. Each line becomes its own `data:` field (the spec joins them back with
/// `\n`), and the leading space after `data:` is the optional separator the client strips — so the file's own
/// indentation is preserved.
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
/// Watches the directory containing `hv_path` and broadcasts the file's contents whenever they change.
///
/// The parent directory is watched (not the file directly) so atomic-save renames used by many editors are still seen.
/// The last-sent contents are remembered so duplicate filesystem events don't trigger duplicate broadcasts. Returns the
/// watcher, which the caller must keep alive.
fn spawn_watcher(hv_path: &Path, tx: broadcast::Sender<String>) -> impl Watcher {
    let file = hv_path.to_path_buf();
    let watch_dir = hv_path
        .parent()
        .expect("hv file must live in a directory")
        .to_path_buf();

    let mut last = std::fs::read_to_string(&file).ok();

    let mut watcher = recommended_watcher(move |res: notify::Result<Event>| {
        if res.is_err() {
            return;
        }
        if let Ok(content) = std::fs::read_to_string(&file)
            && last.as_deref() != Some(content.as_str())
        {
            last = Some(content.clone());
            let _ = tx.send(content); // Err only means no clients are connected — harmless.
        }
    })
    .expect("create filesystem watcher");

    watcher
        .watch(&watch_dir, RecursiveMode::NonRecursive)
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
