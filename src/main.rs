use hash_visualiser::BUILTIN_EXAMPLE;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Print the abstract syntax tree of the supplied .hv file, or the built-in example if no file is provided.
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn main() {
    let src = match std::env::args().nth(1) {
        Some(path) => std::fs::read_to_string(&path)
            .unwrap_or_else(|e| { eprintln!("cannot read {path}: {e}"); std::process::exit(1) }),
        None => BUILTIN_EXAMPLE.to_owned(),
    };

    match hash_visualiser::parse(&src) {
        Ok(program) => println!("{program:#?}"),
        Err(e)      => { eprintln!("{e}"); std::process::exit(1) }
    }
}
