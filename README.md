# Wirecraft

Rust + GPUI migration of [Threadworks](https://github.com/alfu32/threadworks), based on revision `7d392d3a0e30b86445e00e7c03032a5188b26593` (2.6.7). This is an early working migration, **not feature-complete**. See [parity](docs/migration/parity.md), [architecture](docs/migration/architecture.md), and [current status](docs/migration/status.md).

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

The desktop supports node/type/link creation, selection, shift selection, drag and window selection, middle/right-button pan, wheel pan, Ctrl-wheel cursor zoom, Fit, hierarchy navigation, double-click entry/edit, five entity text sections, JSON property editing, Apply, undo/redo, load/save and recovery. Ctrl N/O/S/Shift-S/Z/Y/Shift-Z/Enter/Q and Escape retain familiar workflows. Apply drafts before changing entities or text sections; Reload entity explicitly discards a draft. Unsaved graph changes block New/Open until saved. Project Management currently uses the JSON entity property editor; it is not a port of the original dashboard.

Session and draft data live in `$XDG_STATE_HOME/wirecraft` (fallback `~/.local/state/wirecraft`). `--state-dir` isolates them. Original Java Preferences are never changed. Closing the desktop stops only its owned backend; the Threadworks process is independent. A crash recovery snapshot preserves applied changes; UI session data preserves editor drafts. Independent sessions need separate state directories.

## MCP

For a desktop session, open **MCP → Copy MCP client configuration**. It includes the ephemeral local endpoint and bearer header. Treat this configuration as access to the open design. The old `threadwork.*` tool names and `threadwork://` resource URIs are retained.

For headless stdio clients, use the built executable (stdout contains protocol only):

```json
{"mcpServers":{"wirecraft":{"command":"/absolute/path/target/debug/wirecraft","args":["mcp-stdio","--project","/absolute/path/project.orch"]}}}
```

For HTTP, `wirecraft serve` prints one JSON readiness record containing `endpoint`, `mcp`, and `token`; send `Authorization: Bearer <token>` to `/mcp`, `/api`, or `/events`. The backend binds only `127.0.0.1`, defaults to an OS-selected port, rejects browser Origin requests, and leaves the old server alone. `/health` is a non-mutating liveness route. Explicit `--port` and `--token` are available; `--recovery FILE` opts into recovery storage.

An API request is `{"command":"threadwork.get_design","arguments":{}}`. Mutations can add `"expectedRevision": 3` to reject stale updates. `/events` is an authenticated SSE stream; reconnecting clients must reload a snapshot after a resync event. `file.open`, `file.save`, `edit.undo`, `edit.redo` and document commands share this contract. Errors carry a stable code and message. MCP retains the original 19 tool schemas and adds 17 Wirecraft tools for project files, graph fragments, archetypes, reports, exports, literal/C17 generation, mapped graph diagnostics, LSP and build jobs; unavailable desktop commands and graph compilers return explicit limitations rather than fake success.

## Reusable workflows, reports and exports

Designer Copy/Cut/Paste retains subtrees and internal links, remapping IDs and typed references. Archetypes embeds the original reusable workflows; import a bundled design or a .orch file into the focused composite. Export can save a selected subtree as an archetype, copy Markdown component fiches, or write a focused diagram as SVG, PNG or vector PDF. These drawings are free-size; the original ISO sheets, rolls and poster assembly are still pending. Analysis reports principal/error/dependency/structural graph degrees, cycles, components, classifications and findings; actionable layout optimization remains pending.

Literal filesystem generation supports `file-export` / `multi-tech` entities, text/binary files, relative `metadata.path` or `metadata.file`, hierarchy directories and origin maps. It rejects unmigrated language graph technologies and unsafe/colliding paths. The output directory must be new or empty; generation never merges into a nonempty user directory. `generate.files` optionally takes `nodeId` and `outputDir`; omitting the output returns the virtual files for inspection.

## C17 topology generation

`Export → Generate C17 topology`, `compile.c17` through the application API, and MCP `wirecraft.compile_c17` invoke the same compiler. The existing CLI workflow is also available:

```sh
wirecraft compile project.orch /tmp/new-generated-directory
wirecraft desktop project.orch
```

A C-owned graph uses `technologyId: c-native`, `languageId: c`, `single-file` layout. Nodes contain recurring C statements in Source and setup statements in Initialization. Service-library declarations contain complete translation-unit declarations/includes. Links supply typed buffers named from their link name; processing code uses `push(packet, &item)`, `pop(packet, &item)` and `threadwork_buffer_count(packet)`. Type entities emit structs. Composite execution initializes children, runs children in hierarchy order, then transports one packet per owned data/error link. Library links expose prefixed function pointers. Modeled code requests shutdown with `threadwork_runner__shutdown_request(&threadwork_runner)`; generated code drains residual traffic and frees remaining buffers.

The compiler emits one C17 translation unit and origin mappings. Tests compile and execute typed producer/consumer, nested composite/library and self-feedback fixtures with clang. Source capabilities, graph template overrides, parameterized collection types, aggregate mixed-language delegation and the other language graph compilers remain pending. C runnable capabilities remain unsupported, matching the original C17 restriction; no TCC integration is present.

Two deliberate runtime corrections: single-packet transport updates counts per packet and checks destination capacity for that packet; shutdown debug traces are opt-in on stderr with `-DWIRECRAFT_TRACE`. Self-links use explicit `<link>_in` / `<link>_out` body arguments, fixing the old duplicate generated parameter shape. These changes leave stored graph data intact.

## Language intelligence and builds

Install `clangd` and `clang`/`clang++` on PATH. Desktop **Tools** analyzes the selected Source section as a standalone C/C++ translation unit in an isolated intelligence workspace. It can also run an external C syntax check; refresh Build jobs to see authoritative stdout/stderr and exit status. `WIRECRAFT_CLANGD` and `WIRECRAFT_CC` select executable paths. **Analyze C17 graph** analyzes the generated translation unit and maps errors to node text sections. Click a mapped diagnostic to reveal its entity, section and position. The generated analyzer has an independent temporary workspace and stop control.

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

Wirecraft reads `settings.json` from its state directory. Supported settings include `schemaVersion: 1`, `editorIndentSpaces` (1–16; default 4), `clangd: {executable, args}`, `cCompiler` and `cppCompiler`. Environment executable overrides take precedence.

Import an explicitly exported Java Preferences XML into a new JSON file:

```sh
wirecraft migrate-settings threadworks-preferences.xml /tmp/imported-settings.json
```

Review the output and place it at your chosen state directory's `settings.json`. Indentation is active; UI/designer/appearance preferences are archived under `legacyPreferences` until their controls are migrated. Identity/AI credentials are excluded. The importer never reads or writes the live preference store and refuses to overwrite an existing settings file.

```sh
scripts/package-linux.sh release
# Extract dist/wirecraft-0.3.0-linux-<architecture>.tar.gz, then:
./install.sh /your/prefix
```

The package includes both binaries, embedded archetypes, a desktop entry, MIME description, icon and migration documentation. Ensure the prefix's bin directory is in PATH. System graphics/audio libraries and Vulkan drivers remain prerequisites. Native file associations are not changed automatically. `scripts/package-linux.sh dev` can package a development build for install smoke checks.

## Verification and limits

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
python3 scripts/desktop_smoke.py
```

The desktop smoke requires a real X11/XWayland display with Vulkan/DRI3, xdotool and ImageMagick. It launches its own PID-scoped window with temporary state. Xvfb on this machine lacks DRI3 and cannot present GPUI. Available-platform evidence is Linux only. macOS/Windows packaging and interaction have not been verified.

Pending capabilities include remaining C17 capability/template features and Go/Kotlin/JS/PHP/QuickJS graph compiler parity, ISO/roll technical sheets and poster tiling, advanced editor features, dedicated project management, OAuth and AI provider workflows, and full editor/analysis interaction fidelity. JVM plugin ABI is incompatible; plugin data survives. See the parity matrix for exact scope and continuation.
