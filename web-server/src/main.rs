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

use std::{
    path::{Path, PathBuf},
    process::{self, Command},
};

use actix_files::Files;
use actix_web::{App, HttpServer, middleware::Logger};

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

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8000);
    let addr = ("127.0.0.1", port);

    println!("\n  hash-visualiser running on http://127.0.0.1:{port}/web\n");

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .service(Files::new("/", root.clone()).index_file("index.html"))
    })
    .bind(addr)?
    .run()
    .await
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
