# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

@AGENTS.md

Wirecraft is a Rust + GPUI source-to-source migration of Threadworks 2.6.7 (Kotlin/Swing), a dataflow graph designer that stores `.orch` files and generates code from them. `README.md` describes the user-facing features. `docs/migration/status.md`, `parity.md` and `architecture.md` (the ADRs) record what is migrated, what is pending and why.

## Commands

```sh
cargo build --workspace --locked
cargo test --workspace --locked
cargo test -p wirecraft-runtime --test components --locked          # one integration test file
cargo test -p wirecraft-core --locked routing::                      # tests matching a path filter
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings

cargo run -p wirecraft-desktop -- path/to/project.orch --state-dir /tmp/some-state
cargo run -p wirecraft-runtime -- validate|compile|update-references|serve|templates ...
```

- The binary built from `wirecraft-runtime` is named `wirecraft`.
- The compiler tests need clang, clangd, Go, Node.js and rustc on PATH. Kotlin, PHP and systemJS (run on Bun) tests are `#[ignore]`d unless run with `--include-ignored`; `WIRECRAFT_TEST_KOTLINC` / `WIRECRAFT_TEST_PHP` / `WIRECRAFT_TEST_BUN` point at the tools (see README).
- Native desktop smokes are the `scripts/*_smoke.py` files. They launch their own window process with temporary state and need a real X11/XWayland display with Vulkan, plus xdotool and ImageMagick; Xvfb does not work here. Screenshots go under `target/<name>-smoke/` and should be inspected.

## Architecture

Three crates. Business logic is shared, never duplicated per transport.

- **`wirecraft-core`**: the pure domain. It has no UI or network code.
  - `model.rs`: `Document`/`Node`/`Link`. Unknown JSON fields are kept in `extra` maps so they round-trip.
  - `store.rs`: `.orch` decode/encode.
  - `analysis.rs`: link relations and stereotypes.
  - `drawing.rs`: a single projection of topology into shapes and ports, shared by the desktop canvas and SVG/PDF export.
  - `routing.rs`: 45° link routing.
  - `c17.rs`: the typed C17 emitter.
  - `compiler.rs` + `templates.rs`: template compilers for Go, Kotlin, Node.js, PHP, systemJS (the baseline QuickJS set) and Rust, using MiniJinja with Pebble compatibility. The templates under `resources/compiler-templates*` are embedded by `build.rs`: the byte-identical baseline, its generated overlay (`scripts/template_overlay.py`, which rewrites that directory) and `compiler-templates-native/` for hand-written sets without a baseline (Rust).
  - `components.rs`: pluggable components (Exterior/Reference nodes, resolve/pin/normalize/expand).
  - Mutations run on a candidate document and are committed only after `ensure_valid`.
- **`wirecraft-runtime`**: the application service and its adapters.
  - `service.rs` `Service::execute` is the single command dispatcher, a `match` on command strings such as `threadwork.create_entity`, `components.publish` or `file.save`. The `threadwork.` prefix is stripped before matching. It owns revisions, undo/redo, dirty state and recovery, and runs `components::normalize` before each commit.
  - `backend.rs` serves `/api`, `/mcp` and `/events` on loopback with a bearer token.
  - `mcp.rs` maps MCP tools onto the same commands. Tool schemas live in `resources/tools.json`; `tests/services.rs` asserts the tool count, so update that count when adding a tool.
  - `cli.rs` holds the `wirecraft` subcommands.
  - `lsp.rs` / `generated_analysis.rs` supervise clangd. `jobs.rs` / `process.rs` supervise build subprocesses.
  - Built-in archetypes (`resources/archetypes/*.orch`) are embedded by `build.rs`.
- **`wirecraft-desktop`**: the GPUI client.
  - It spawns its own backend child process and talks to it over HTTP from a worker (`client.rs`). Snapshots are immutable.
  - `scene.rs` paints the canvas, with world-space culling. `interaction.rs` handles pointer gestures for each mode (Select/Node/Type/Link).
  - `workbench.rs` holds menus, the command palette and views. `inspector.rs` is the property editor. `compiler_issues.rs` shows background C17 diagnostics.
  - A gesture sends one backend mutation on release; the canvas previews it locally until then.

## Domain conventions worth knowing

- **Nodes:** Processors, Types, composites and links are all `Node`s. A link node has `link: Some(...)` with `sourceNodeId`, `targetNodeId` and port names (defaults `out`/`in`). `Document::sync_links` keeps `incomingLinks`/`outgoingLinks`/`ports` consistent.
- **Wirecraft-only state:** stored in `Node.extra`, for example `stereotype: "Exterior" | "Reference"`. This lets the original app still open the files.
- **C17 runtime semantics:**
  - Each tick moves at most one packet per link.
  - `push`/`pop` copy by value.
  - A self-link gives a body `<link>_in`/`<link>_out`.
  - Library links pass a `const Lib_xLibrary *<linkname>` struct of function pointers.
- **Components:**
  - **Exterior:** a node with exactly one data link. Its name is derived as `<link>Out` or `<link>In`.
  - **Reference:** points to a sibling `.orch` file pinned by SHA-256. `components::expand` inlines it during C17 generation; copied IDs are prefixed `<refId>::`.
  - The template compilers reject both. The spec and the list of pending work are in `docs/design/components.md`.
- **Layout units:** the grid is 25 units, and the minimum node height is 73.

## Working rules beyond AGENTS.md

- **Live apps:** a running Threadworks JVM app (`:8765/mcp`) and a running Wirecraft app may be open on the user's files. Use their MCP endpoints read-only for comparisons, unless the user explicitly asks you to drive them. Never start, stop or restart either app. Never print or store their bearer tokens.
- **User files:** `examples/WebServer.orch`, `examples/example.copy.orch`, `examples/*.pdf` and `examples/generated-project/` belong to the user and are untracked. Never commit them. Mutation checks use temporary copies.
- **Releases:** after each verified change, commit with a Conventional Commits/Commitizen message, bump the patch or minor version in the workspace `Cargo.toml` (`[workspace.package] version`), tag `vX.Y.Z` and push.
- **Docs:** when behaviour changes, update `docs/migration/status.md` / `parity.md` and the README. Keep pending gaps stated explicitly.
