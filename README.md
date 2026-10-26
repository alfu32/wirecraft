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

An API request is `{"command":"threadwork.get_design","arguments":{}}`. Mutations can add `"expectedRevision": 3` to reject stale updates. `/events` is an authenticated SSE stream; reconnecting clients must reload a snapshot after a resync event. `file.open`, `file.save`, `edit.undo`, `edit.redo` and document commands share this contract. Errors carry a stable code and message. MCP advertises the original 19 tool schemas; unavailable desktop commands and graph compilers return explicit limitations rather than fake success.

## Language intelligence and builds

Install `clangd` and `clang`/`clang++` on PATH. Desktop **Tools** analyzes the selected Source section as a standalone C/C++ translation unit in an isolated intelligence workspace. It can also run an external C syntax check; refresh Build jobs to see authoritative stdout/stderr and exit status. `WIRECRAFT_CLANGD` and `WIRECRAFT_CC` select executable paths. The generated graph translation-unit/source-map workflow is still pending.

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

## Verification and limits

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
python3 scripts/desktop_smoke.py
```

The desktop smoke requires a real X11/XWayland display with Vulkan/DRI3, xdotool and ImageMagick. It launches its own PID-scoped window with temporary state. Xvfb on this machine lacks DRI3 and cannot present GPUI. Available-platform evidence is Linux only. macOS/Windows packaging and interaction have not been verified.

Pending capabilities include graph-to-source compiler/template parity, full technical sheets/PDF/PNG export, detailed network analysis, archetypes, advanced editor features, dedicated project management, OAuth, AI workflows, clipboard/layout commands and settings migration. JVM plugin ABI is incompatible; plugin data survives. See the parity matrix for exact scope and continuation.
