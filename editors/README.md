# Editor support for `.hv` files

Two pieces, both in this directory:

| Directory          | What it is                                                                 |
| ------------------ | -------------------------------------------------------------------------- |
| `tree-sitter-hv/`  | The Tree-sitter grammar for the Hash Visualiser DSL (`grammar.js` + generated `src/` + highlight queries). |
| `zed/`             | The Zed editor extension that wires the grammar up for syntax highlighting, outline, and bracket matching. |

Zed does all syntax highlighting through Tree-sitter, so the grammar is the engine and the `zed/` extension is the thin wrapper that points Zed at it.

## What gets highlighted

* Keywords (`hash`, `context`, `fn`, `node`, `wire`, `data`, `layout`, `group`, event/effect words)
* Operators (`and`/`or`/`xor`/`not`, the shift & rotate ops, `+`/`-`)
* Node kinds (`register`/`operation`/`constant`/`button`) and primitive types (`u8`…`u64`) as types
* Flow/arrange modes as constants
* `data` bindings as constants
* Function definitions & calls as functions
* Property keys (`label`, `format`, `source`, `compute`, …) as properties
* Numbers (decimal, `0x…` hex, durations)
* Both `"…"` and triple-quoted `"""…"""` strings (the multi-line markdown descriptions)
* The document outline lists `hash`/`fn`/`node`/`data`/`group` definitions.

## Installing the Zed extension

Zed fetches an extension's grammar from a **git repository at a specific commit**, so the grammar has to be committed and pushed first.

1. **Generate and commit the grammar.**<br>From `editors/tree-sitter-hv/`:

   ```sh
   npm install            # one-time: pulls tree-sitter-cli
   npx tree-sitter generate
   ```

   Commit `grammar.js`, `tree-sitter.json`, `queries/`, and the generated `src/` (Zed compiles `src/parser.c`; `node_modules/` is git-ignored).

   Push to the repository named in `editors/zed/extension.toml`.

2. **Pin the grammar commit.**<br>Copy the commit SHA from step 1 into `editors/zed/extension.toml`:

   ```toml
   [grammars.hv]
   repository = "https://github.com/ChrisWhealy/hash_visualiser"
   path = "editors/tree-sitter-hv"   # grammar lives in a subdirectory of the repo
   rev = "REPLACE_WITH_COMMIT_SHA"
   ```

   (If your Zed build doesn't support the `path` sub-directory option, move `tree-sitter-hv/` into its own repository and point `repository` at that instead.)

3. **Install as a dev extension.**<br>In Zed: open the command palette (`cmd-shift-p`) → **`zed: install dev extension`** → choose the `editors/zed` directory.

4. Open any `.hv` file (e.g. `hv/composition/04_choose.hv`). Zed shows the language as **HV** in the status bar.

When you change the grammar or queries, re-run `tree-sitter generate`, commit, bump `rev`, and use **`zed: reload extensions`** (or reinstall the dev extension).

## Testing the grammar on its own

From `editors/tree-sitter-hv/`:

```sh
npx tree-sitter parse ../../hv/composition/04_choose.hv     # parse tree
npx tree-sitter highlight ../../hv/sha3.hv                  # ANSI-highlighted (needs a parser dir configured)
```

The grammar has been checked to parse every `.hv` file under `hv/`, including `sha256.hv` (event handlers, `animate`, `emit … via`).
