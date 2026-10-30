# Wirecraft

Rust + GPUI migration of [Threadworks](https://github.com/alfu32/threadworks), based on revision `7d392d3a0e30b86445e00e7c03032a5188b26593` (2.6.7). This is an early working migration, **not feature-complete**. See [parity](docs/migration/parity.md), [architecture](docs/migration/architecture.md), and [current status](docs/migration/status.md). Planned Wirecraft extensions: [pluggable components](docs/design/components.md) (specification).

The framework-independent graph/storage crate is shared by a headless backend and a desktop client. The desktop owns an authenticated loopback backend process. MCP and the desktop invoke the same services. TCC and its native integration are absent.

## Build and run

Rust stable (tested 1.99.1), C toolchain and GPUI Linux development dependencies are needed. On Debian/Ubuntu these include pkg-config, libasound2-dev, libxkbcommon-dev, libxkbcommon-x11-dev, libxcb1-dev, libwayland-dev, libfontconfig1-dev, libvulkan1 and a functioning Vulkan driver. This list has not been tested in a clean system image. GPUI 0.2.2 is pinned; libc 0.2.189 is pinned for its older xattr dependency. Use the committed lockfile.

```sh
cargo build --workspace --locked
cargo run -p wirecraft-desktop -- path/to/project.orch
cargo run -p wirecraft-desktop -- --state-dir /tmp/wirecraft-independent-session
cargo run -p wirecraft-runtime -- serve --project path/to/project.orch
cargo run -p wirecraft-runtime -- validate path/to/project.orch
cargo run -p wirecraft-runtime -- new /tmp/new-project.orch
```

The workbench has Designer, Editor, Project Management, Analysis, Archetypes, Export, Tools and MCP views, with categorized menus and a searchable keyboard command palette (Ctrl-Shift-P, Up/Down, Enter, Escape). The icon set uses consistent GPU strokes rather than the old Swing toolbar artwork. [All 42 reference photos](docs/migration/threadworks/README.md) have chronological names and feature mappings; [walkthrough parity](docs/migration/walkthrough.md) records the implementation and its limits.

Designer supports node/type/link creation, component ports (the **Port** tool, or Link mode from an entity to empty canvas; ports inside a collapsed composite stay visible beside it), IO roles (**IO role** in the inspector: an amber IO-in/IO-out badge and dashed outline), live previews of the entity or routed link a click would create, link rewiring (select a link, pick up an end handle, click another entity or empty canvas), shift and box selection, composite subtree drag, resizing, collapse/expand, named typed ports and type-usage callouts. Data links choose the nearer side, use angular port order, separate routed channels and explicit composite boundary ports. [Native 45° routing](docs/migration/routing.md) uses source-derived candidates with Euclidean scoring and an eight-direction fallback; self-links reserve compact contours first. Original hollow output/filled notched input caps and 46-unit horizontal ends are retained. Crossings use hollow via pairs with gray backside connections; incomplete routing draws best effort without routing-error labels. Library dependencies retain right-side dependant boxes and top consumer flagpoles. Compiler warning/error badges and a clickable status issue list reveal mapped code. Wheel zooms about the pointer; middle-button drag pans; right-click opens the scrollable command menu, including grouped archetypes. Left-to-right windows select enclosed objects; right-to-left windows also select intersecting objects. Shift adds and Control removes; unmodified clicks replace selection. Dragging a selected member moves the selected group, with debounced background rerouting and one undo step. The canvas shows individual contours and selection bounds, and tree rows share the selection; drag a tree row onto another processing entity to reparent. Fit, hierarchy navigation and double-click entry/edit retain familiar workflows. Editor has the five source sections, C/C++ syntax highlighting, C compiler semantic colors and function/struct hover, section language, compact label/value rows and dropdowns, inline metadata/type/history tables, inherited values, revision tracking and advanced JSON. Inspector names commit on Enter or focus loss; inspector text fields also support Enter. A persistent draft strip in every view offers Apply, Discard and Show editor; Ctrl-Enter applies from Designer too. Apply body drafts before changing entities or sections; Reload entity explicitly discards a draft. Drafts, including invalid metadata tables, recover after restart. Ctrl N/O/S/Shift-S/Z/Y/Shift-Z/Enter/Q are supported. Unsaved graph changes block New/Open until saved.

Project Management provides six status lanes, drag/drop, remove-to-backlog and undo. Archetypes provides grouped descriptions and a read-only preview before explicit insertion. Analysis shows sparse P/E/D/U matrices, clickable endpoints, indicators and findings. Root master revision and inherited node revisions retain their source storage semantics. Sidebar widths, hierarchy expansion, viewport, selection and export profiles restore across restarts.

Session and draft data live in `$XDG_STATE_HOME/wirecraft` (fallback `~/.local/state/wirecraft`). `--state-dir` isolates them. Original Java Preferences are never changed. Closing the desktop stops only its owned backend; the Threadworks process is independent. A crash recovery snapshot preserves applied changes; UI session data preserves editor drafts. Independent sessions need separate state directories.

## MCP

For a desktop session, open **MCP → Manage Server**, then **Copy configuration**. It includes the ephemeral local endpoint and bearer header. Treat this configuration as access to the open design. Health checks and the bounded access log are visible here; Stop/Start disable/enable MCP without stopping the UI backend. The old `threadwork.*` tool names and `threadwork://` resource URIs are retained.

For headless stdio clients, use the built executable (stdout contains protocol only):

```json
{"mcpServers":{"wirecraft":{"command":"/absolute/path/target/debug/wirecraft","args":["mcp-stdio","--project","/absolute/path/project.orch"]}}}
```

For HTTP, `wirecraft serve` prints one JSON readiness record containing `endpoint`, `mcp`, and `token`; send `Authorization: Bearer <token>` to `/mcp`, `/api`, or `/events`. The backend binds only `127.0.0.1`, defaults to an OS-selected port, rejects browser Origin requests, and leaves the old server alone. `/health` is a non-mutating liveness route. Explicit `--port` and `--token` are available; `--recovery FILE` opts into recovery storage.

An API request is `{"command":"threadwork.get_design","arguments":{}}`. Mutations can add `"expectedRevision": 3` to reject stale updates. `/events` is an authenticated SSE stream; reconnecting clients must reload a snapshot after a resync event. `file.open`, `file.save`, `edit.undo`, `edit.redo` and document commands share this contract. Errors carry a stable code and message. MCP retains the original 19 tool schemas and adds 19 Wirecraft tools for project files, graph fragments, archetypes, pluggable components, reports, exports, graph-template/literal/C17 generation, mapped graph diagnostics, LSP and build jobs. `wirecraft.workbench` exposes sheet/document previews and exports, clipboard prompts, settings, MCP controls and printers through the same services. Unavailable commands and graph compilers return explicit limitations.

## Reusable workflows, reports and exports

Designer Copy/Cut/Paste retains subtrees and internal links, remapping IDs and typed references. Archetypes embeds the original reusable workflows; import a bundled design or a .orch file into the focused composite. File/Export can save a selected subtree as an archetype and copy Markdown component fiches.

Export has a real Plan/Documentation preview, ISO A0–A4/landscape/roll formats, 1:N scales, split/overlap, title block, parts/signature table and folding ticks. Plan writes SVG, PNG, Mermaid and searchable vector or compressed raster PDF; multipage output requires PDF. Documentation writes paginated PDF, escaped HTML or Markdown with header/footer/margins. PDF All combines plan and documentation. Plan preview defaults to the entire assembled drawing with page contours and alignment/folding marks; individual-sheet preview remains available. Auto refuses accidental cropping; fixed sheets report clipping. Raster allocation and page counts are bounded. CUPS printers are discovered and explicit Print submits a supervised job; no physical printer output was verified. Per-printer profiles persist. The Sheet toolbar opens this preview; the old editable in-canvas paper overlay and exact legacy typography remain gaps.

AI Support copies source/specification/test prompts with component dossiers and available compiler context. Options persists provider profiles and environment-variable names for credentials. All four network providers in the source baseline are stubs returning Unavailable; Wirecraft likewise reports network inference/browser authorization as unsupported. Clipboard assistance works without credentials.

Literal filesystem generation supports `file-export` / `multi-tech` entities, text/binary files, relative `metadata.path` or `metadata.file`, hierarchy directories and origin maps. It rejects unmigrated language graph technologies and unsafe/colliding paths. The output directory must be new or empty; generation never merges into a nonempty user directory. `generate.files` optionally takes `nodeId` and `outputDir`; omitting the output returns the virtual files for inspection.

## Compiler template sets

**Generate → Generate Project (effective technology)** supports C17, Go, Kotlin, Node.js, PHP, systemJS (engine-neutral JavaScript scripts, formerly QuickJS; stored `quickjs` technologies still select it) and Rust (a Wirecraft-authored set with no Threadworks counterpart; one dependency-free file plus `Cargo.toml`). Choose the language and technology in the inspector and Apply; inherited settings apply to descendants. Templates are embedded, so no source checkout is needed. [Compiler scope and compatibility](docs/migration/compiler-templates.md) distinguishes tested workflows from remaining parity.

```sh
cargo run -p wirecraft-runtime -- templates
cargo run -p wirecraft-runtime -- compile examples/nodejs.orch /tmp/wirecraft-nodejs-example
node /tmp/wirecraft-nodejs-example/hello_nodejs.js
cargo run -p wirecraft-runtime -- export-compiler examples/nodejs.orch /tmp/wirecraft-nodejs-compiler
cargo run -p wirecraft-runtime -- compile examples/nodejs.orch /tmp/wirecraft-custom-example --compiler /tmp/wirecraft-nodejs-compiler/compiler.wirecompiler.json
```

Five runnable graph examples are in `examples/`: Go (`go build`), Node.js (`node`), Kotlin (Kotlin 2.2.0/JVM 21 or the generated Gradle build), PHP (`php`), systemJS (`bun run`; the hello example also runs on `qjs` and `node`). Install these external tools separately; Wirecraft does not execute bundled foreign binaries. Model `@…` overrides, project files and portable JSON compiler bundles work through the shared CLI/API/MCP service. The generated JVM compiler-class ABI is replaced by that portable data format. C17 still uses its typed Rust emitter; C template execution, aggregate mixed technologies, composite boundary channels in file-based template layouts and wider collection/editor parity remain pending.

The WebServer example is a C17 HTTP/1.1 server modeled as a dataflow network, split into a [pluggable component](docs/design/components.md). `examples/webserver-component.orch` is the HTTP transport (accept, a pending-request feedback loop, respond, and an HTTP library lent as `libhttp`); it publishes a `request` output and a `response` input through Exterior circles. `examples/webserver.orch` places it as the Reference `http` and supplies the router and routes. Both specifications are in the models' Specification sections, starting with each root entity:

```sh
cargo run -p wirecraft-runtime -- compile examples/webserver.orch /tmp/wirecraft-webserver
clang -std=c17 -Wall -Wextra -pedantic /tmp/wirecraft-webserver/WebServerApp.c -o /tmp/wirecraft-webserver/webserver
WEBSERVER_PORT=8080 /tmp/wirecraft-webserver/webserver   # then: curl -i http://127.0.0.1:8080/home/about
```

The same server is transcoded into Go, Kotlin, Node.js (also run on Bun), PHP, systemJS (on Bun) and Rust as `examples/webserver-<language>.orch` by `scripts/transcode_webserver.py`; `tests/transcoded_webservers.rs` builds each and runs the same HTTP exchange (see [compiler-templates.md](docs/migration/compiler-templates.md#webserver-transcodings-wirecraft-viability-test-0160)).

Routes: `/home/about`, `/home/version`, `/api/user`; other paths answer 404, other methods 405, malformed requests 400. Set `WEBSERVER_MAX_REQUESTS=N` to exit after N requests and `WEBSERVER_BIND_ANY=1` to listen beyond loopback. After editing the component, run `cargo run -p wirecraft-runtime -- update-references examples/webserver.orch` (or **Graph → Update Component Reference**) to re-pin it.

## C17 topology generation

`Generate → Generate Project (effective technology)`, `compile.c17` through the application API, and MCP `wirecraft.compile_c17` invoke the same compiler. The existing CLI workflow is also available:

```sh
wirecraft compile project.orch /tmp/new-generated-directory
wirecraft desktop project.orch
```

A C-owned graph uses `technologyId: c-native`, `languageId: c`, `single-file` layout. Nodes contain recurring C statements in Source and setup statements in Initialization. Service-library declarations contain complete translation-unit declarations/includes. Links supply typed buffers named from their link name; processing code uses `push(packet, &item)`, `pop(packet, &item)` and `threadwork_buffer_count(packet)`. Type entities emit structs; array expressions use the existing ThreadworkArray runtime value. Composite execution initializes children, runs children in hierarchy order, then transports one packet per owned data/error link. Library links expose prefixed function pointers. Modeled code requests shutdown with `threadwork_runner__shutdown_request(&threadwork_runner)`; generated code drains residual traffic and frees remaining buffers.

The compiler emits one C17 translation unit and origin mappings. Tests compile and execute typed producer/consumer, nested composite/library and self-feedback fixtures with clang. Source capabilities, graph template overrides, parameterized collection types, aggregate mixed-language delegation and complete broader-language parity remain pending. C runnable capabilities remain unsupported, matching the original C17 restriction; no TCC integration is present.

Two deliberate runtime corrections: single-packet transport updates counts per packet and checks destination capacity for that packet; shutdown debug traces are opt-in on stderr with `-DWIRECRAFT_TRACE`. Self-links use explicit `<link>_in` / `<link>_out` body arguments, fixing the old duplicate generated parameter shape. These changes leave stored graph data intact.

## Language intelligence and builds

Install `clangd` and `clang`/`clang++` on PATH. Desktop **Tools** analyzes the selected Source section as a standalone C/C++ translation unit in an isolated intelligence workspace. It can also run an external C syntax check; refresh Build jobs to see authoritative stdout/stderr and exit status. `WIRECRAFT_CLANGD` and `WIRECRAFT_CC` select executable paths. **Analyze C17/Rust graph** analyzes the generated translation unit (or Rust program) and maps errors to node text sections. Click a mapped diagnostic to reveal its entity, section and position. The generated analyzer has an independent temporary workspace and stop control.

The backend supports other language servers through executable/argv/root configuration. Example commands (arguments are API JSON):

```json
{"command":"lsp.start","arguments":{"executable":"clangd","args":["--compile-commands-dir=/path/to/build"],"root":"/path/to/workspace"}}
{"command":"lsp.open","arguments":{"uri":"file:///path/to/workspace/example.c","languageId":"c","version":1,"text":"int main(void) { return 0; }"}}
{"command":"lsp.request","arguments":{"method":"textDocument/documentSymbol","params":{"textDocument":{"uri":"file:///path/to/workspace/example.c"}}}}
{"command":"lsp.diagnostics","arguments":{}}
{"command":"lsp.stop","arguments":{}}
```

`lsp.change` requires an increasing version and full text; `lsp.close` releases a document. Status reports actual negotiated capabilities and position encoding. Generic document/workspace requests support navigation, completion, symbols and hover when advertised by the server. `lsp.cancel` accepts `requestId` of an outstanding request. Versioned diagnostics from old document versions are ignored; a server omitting diagnostic versions cannot guarantee stale-result detection. Restart via lsp.start creates a new session and requires reopening documents.

`jobs.start` accepts `executable`, `args`, `cwd`, optional `timeoutSeconds`. `jobs.get/list/cancel` report progress/results. Output is capped at 64 KiB per stream; cancellation/timeouts terminate the owned process group on Unix. Editor diagnostics and external builds remain separate services.

## Configuration import and Linux installation

Wirecraft reads `settings.json` from its state directory. Options has staged Save/Cancel/defaults for general appearance, semantic color pickers and inference profiles. Themes, designer/editor fonts, composite title/reference size, wheel rows, grid/colors and indentation apply and restore. Monaspace Neon/Krypton are embedded under the SIL OFL; system font choices are also available. Missing legacy fonts use the system fallback. Settings include `schemaVersion: 1`, `editorIndentSpaces` (1–16; default 4), `appearance`, `inference`, `clangd: {executable, args}`, `cCompiler` and `cppCompiler`. Environment executable overrides take precedence.

Import an explicitly exported Java Preferences XML into a new JSON file:

```sh
wirecraft migrate-settings threadworks-preferences.xml /tmp/imported-settings.json
```

Review the output and place it at your chosen state directory's `settings.json`. Supported indentation, theme/font, composite, wheel and semantic colors activate; other panel values are retained in `legacyPreferences`. Identity/AI credentials are excluded. The importer never reads or writes the live preference store and refuses to overwrite an existing settings file.

```sh
scripts/package-linux.sh release
# Extract dist/wirecraft-0.4.1-linux-<architecture>.tar.gz, then:
./install.sh /your/prefix
```

The package includes both binaries, embedded archetypes, a desktop entry, MIME description, icon and migration documentation. Ensure the prefix's bin directory is in PATH. System graphics/audio libraries and Vulkan drivers remain prerequisites. Native file associations are not changed automatically. `scripts/package-linux.sh dev` can package a development build for install smoke checks.

## Verification and limits

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
python3 scripts/desktop_smoke.py
python3 scripts/walkthrough_smoke.py
python3 scripts/editor_smoke.py
python3 scripts/rust_editor_smoke.py   # also needs rust-analyzer and cargo
python3 scripts/canvas_smoke.py
python3 scripts/package_smoke.py dist/wirecraft-0.4.1-linux-x86_64.tar.gz --desktop --walkthrough --editor --canvas
```

Full integration verification also needs clang/clangd, uuid-dev, poppler-utils (pdfimages/pdftotext) and fonts-dejavu-core. The desktop smoke requires a real X11/XWayland display with Vulkan/DRI3, xdotool and ImageMagick. It launches its own PID-scoped window with temporary state. Xvfb on this machine lacks DRI3 and cannot present GPUI. Available-platform evidence is Linux only. macOS/Windows packaging and interaction have not been verified.

Ctrl-Space in C17 Declaration/Instantiation bodies queries the generated compilation context through clangd, showing runtime APIs, connected buffers, injected library functions and members of local typed values. Suggestions show signatures and insert into the draft; Enter/Tab accept, arrows select, Escape dismisses. A read-only generated function header identifies the wrapper. Unicode edits and stale replies are checked. Linux windows include minimize/maximize/restore/close controls.

Pending capabilities include remaining C17 capability/template features and Go/Kotlin/JS/PHP/systemJS graph compiler parity, inline completion and navigation for languages other than C and Rust, OAuth identity, editable canvas paper overlays and exact legacy layout fidelity. Network AI and automatic layout optimization were unfinished in the source as well. JVM plugin ABI is incompatible; plugin data survives. See the parity matrix for exact scope and continuation.

Rust Declaration/Instantiation bodies (technology `rust`) get the same editor features through rust-analyzer (`rustup component add rust-analyzer`; `rustAnalyzer.executable` in settings or `WIRECRAFT_RUST_ANALYZER` selects it): tree-sitter syntax coloring, semantic colors, Ctrl-Space completion of runtime methods, link buffers and lent library instances (`math.maximum`), hover with the library function's `///` documentation and the model provenance of injected arguments, and native diagnostics while typing. rust-analyzer analyzes the whole generated program with its `Cargo.toml` in a temporary workspace, and its results are remapped to the visible body; the read-only header shows the generated `fn run_…`/`fn init_…` signature. The background compiler check of a Rust design runs rust-analyzer's `cargo check` on the applied graph and lists rustc errors in the status issue list, mapped to entity, section, line and column. The first request waits for rust-analyzer to load the workspace and standard library (about 5 s). Designs with component Exteriors or References are not checked, since the Rust generator rejects them.

**Go to Definition** (F12, Ctrl-click, or the command palette) and **Find References** (Shift-F12) work in C17, Rust, Go, Kotlin, JavaScript (Node.js and systemJS) and PHP bodies. The language server analyzes the generated single-file program, and every location is mapped back to the model:
- **Bodies:** a target in an authored body moves the cursor there, or opens that entity's section at the line and column.
- **Model elements:** a link buffer or library argument reveals its link or library entity; a modeled struct reveals its Type entity.
- **Library members:** `math.maximum` (`math->maximum` in C, `$math->maximum` in PHP) resolves through the generated instance to the function in the library's body.
- **Generated and external code:** the runtime and other scaffolding, and std or system headers, are named in the status bar.

Language servers, each configurable in Options (settings key, environment override):

| Bodies | Server | Settings | Override |
| --- | --- | --- | --- |
| C17 | clangd | `clangd` | `WIRECRAFT_CLANGD` |
| Rust | rust-analyzer | `rustAnalyzer` | `WIRECRAFT_RUST_ANALYZER` |
| Go | gopls (also found in `$GOPATH/bin` or `~/go/bin`) | `gopls` | `WIRECRAFT_GOPLS` |
| Kotlin | kotlin-language-server | `kotlinLanguageServer` | `WIRECRAFT_KOTLIN_LANGUAGE_SERVER` |
| Node.js, systemJS | typescript-language-server `--stdio` | `javascriptLanguageServer` | `WIRECRAFT_JAVASCRIPT_LANGUAGE_SERVER` |
| PHP | intelephense `--stdio` | `phpLanguageServer` | `WIRECRAFT_PHP_LANGUAGE_SERVER` |

JavaScript and PHP library arguments are untyped, so their servers cannot follow `libhttp.f(...)`; Wirecraft resolves those calls, model arguments and modeled types from the model itself, and finds a library function's references in every consumer body. When a body's server is not installed, this model navigation is all that works, and the status bar says the server is missing; locals then need the server. Completion, hover, semantic colors and compiler checks remain C17 and Rust only.

References list the definition and every use, including calls through lent library instances, in a panel; click an authored location to open it. Opening another entity needs the editor draft applied first, as for diagnostics. The backend commands are `editor.definition` / `editor.references` (MCP `wirecraft.workbench` operations `definition` / `references`), with the same nodeId/section/text/offset arguments as hover.

C editor Ctrl-Space suggestions show identifiers and signatures. Hover shows clangd function/type declarations, complete emitted struct definitions, and signatures/provenance of injected library functions. The backend/MCP workbench operations `semantic_tokens` and `hover` accept the same nodeId/section/text context (`offset` is UTF-8 bytes for hover). clangd receives the full generated C program, runtime, connected dependencies and companion files, with results remapped to the visible body. Syntax coloring remains available if clangd is missing.

The compiler tests require Go, Node.js and rustc on PATH (`WIRECRAFT_TEST_RUSTC` overrides rustc). Optional external Kotlin/PHP/systemJS tests run with:

```sh
WIRECRAFT_TEST_KOTLINC=/path/to/kotlin-2.2.0/bin/kotlinc \
WIRECRAFT_TEST_PHP=/path/to/php WIRECRAFT_TEST_BUN=/path/to/bun \
cargo test -p wirecraft-runtime --test template_compilers --locked -- --include-ignored
```

The rust-analyzer editor tests are ignored unless requested; they need `rust-analyzer` on PATH (or `WIRECRAFT_RUST_ANALYZER`) and cargo:

```sh
cargo test -p wirecraft-runtime --test rust_editor --locked -- --include-ignored
```

Go, Kotlin, JavaScript and PHP navigation is checked from the model by default. The language-server tests are ignored unless requested; they use the servers on PATH or `WIRECRAFT_TEST_GOPLS`, `WIRECRAFT_TEST_KOTLIN_LANGUAGE_SERVER`, `WIRECRAFT_TEST_JAVASCRIPT_LANGUAGE_SERVER` and `WIRECRAFT_TEST_PHP_LANGUAGE_SERVER`:

```sh
cargo test -p wirecraft-runtime --test template_navigation --locked -- --include-ignored
```

Inspector commit/recovered-draft checks use `python3 scripts/draft_smoke.py` (owned process and temporary state). Add `--draft` to installed package verification.

The corrective native suite can be run with `python3 scripts/followup_smoke.py`; startup and pointer-release/cursor checks use `scripts/startup_smoke.py` and `scripts/gesture_release_smoke.py`. All use owned processes and temporary fixtures/state. Installed verification accepts `python3 scripts/package_smoke.py dist/wirecraft-0.6.3-linux-x86_64.tar.gz --desktop --walkthrough --editor --canvas --startup --gesture --followup`. These native checks require the working X11/Vulkan display. They send real input to their own temporary window but never take the keyboard focus: when another window has it, a smoke pauses (and says so on stderr) until its window is clicked again, and stops after three minutes. A new window may still receive the focus from the window manager when it first opens.
