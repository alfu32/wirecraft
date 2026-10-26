# Current migration status

Wirecraft 0.2.0 is a working Linux migration milestone, **not a complete replacement**. The source remains unchanged and its existing desktop/MCP server remains running. Source revision: `7d392d3a0e30b86445e00e7c03032a5188b26593` (Threadworks 2.6.7). Destination origin is `git@github.com:alfu32/wirecraft.git`, branch master. Release policy: regular Conventional Commits and pushes, patch/minor tags only; major versions belong to the user.

## Working

- Rust workspace: independent domain/storage/geometry, application/backend/adapters, GPUI desktop HTTP client.
- Current list and legacy map .orch files, unknown payload fields and binary data. All 45 source projects load. Atomic saves, topology repair, transactional entity/link/hierarchy mutations, typed ports, status history, inherited settings, revision conflicts and undo/redo.
- GPUI canvas with spatial node/edge culling, revision-cached routes, virtual hierarchy, select/multiselect/window selection/drag, node/type/link creation, link hit-testing, focused hierarchy navigation, pan/zoom/Fit, five text sections and JSON entity properties. Clipboard fragments and alignment/distribution. Viewport/selection/section, applied graph and unapplied editor drafts recover across restart. Dirty polling is constant time using undo state identities rather than whole-document serialization.
- Embedded source archetypes with import/save and fresh entity/type/link IDs; typed network graphs/reports; Markdown fiches; real free-size SVG, PNG and vector PDF diagrams.
- Literal file-export/multi-tech generation, source maps, staged output with path/collision checks and refusal to overwrite nonempty directories. This is distinct from pending language graph compilers.
- Original 19 MCP tool names, five resources and two prompts, plus 15 Wirecraft extensions for project/workflow/LSP/build operations. HTTP and stdio use the same application services. Reduced command/technology catalogs report actual availability.
- Real supervised clangd: negotiation, open/change/close, versions, UTF-8/16/32 conversion in both directions, diagnostics, document/workspace requests, cancellation, stale replies, explicit restart and crash handling. Desktop checks standalone selected source; generated graph source/editor navigation UX remains pending.
- External builds with bounded output, exit status, cancellation/timeouts, Unix process-group cleanup and Linux parent-death guards. Backend shutdown closes live SSE streams. Failed saves retain edits and recovery; recovery errors are visible without repeating a committed mutation.
- Explicit Java Preferences XML import into new JSON settings. Indentation/tool configuration active; appearance/panel values archived; identity/AI nodes excluded. Linux package/install scripts, desktop/MIME/icon resources and CI configuration.

## Checks actually executed

- `cargo test --workspace --locked`: **29 integration tests passed**, none skipped (11 core, 18 runtime). Includes real authenticated HTTP MCP and stdio clients, real clangd diagnostics/completion/symbols/restart/crash, deterministic LSP peer tests for server requests/cancellation/stale replies, Unicode positions, failed saves/recovery, live SSE shutdown, all embedded archetype imports, real SVG/PNG/PDF, and generated C → clang → executed binary.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed. Cargo emits a transitive proc-macro-error2 future-incompatibility notice, no project lint failure.
- `cargo fmt --all -- --check`, `git diff --check`: passed for the verified source state.
- `cargo build --workspace --locked`: passed. `scripts/package-linux.sh release`: passed, optimized build took about 14 minutes on first build. Archive at `dist/wirecraft-0.2.0-linux-x86_64.tar.gz` with SHA-256 file.
- `python3 scripts/package_smoke.py dist/wirecraft-0.2.0-linux-x86_64.tar.gz --desktop`: passed. Installed into a temporary prefix; new/validate, HTTP readiness, MCP initialize/34 tools/mutation/save/PDF and installed desktop verified.
- `python3 scripts/desktop_smoke.py`: passed. Own PID/temp state: two processors, drag, directed link, clipboard paste/undo, source draft, quit/restart/restore/apply, UI-triggered clangd errors, external compiler errors, clean backend/LSP exit. Uses available X11/XWayland with Vulkan. Screenshots under `target/desktop-smoke`, selected evidence copied to evidence/.
- Xvfb presentation was blocked by missing DRI3 (PlatformNotSupported). An initial title-only match selected the unrelated IDE; automation was corrected to require the owned PID and visible window. No source/project file changes resulted.
- Unoptimized index benchmark: 100,000 primitives built in ~309ms; 1,000 viewport queries in ~172ms (mean 64 visible). This is a spatial index measurement, not a GPU frame benchmark.
- Original `/health` still returns Threadwork 2.6.7 / status ok. Windows/macOS builds, clean-image Linux dependency installation and remote CI results are not claimed.

## Remaining scope and exact continuation

See parity.md and commands.md. Main gaps: C/Go/Kotlin/JS/PHP/QuickJS graph compiler/template/layout parity and source mapping into editors; full telescopic composites/obstacle-aware routing/resizing; complete menus/palette and advanced editors; ISO/roll technical sheets/title/folding/poster tiling; dedicated project management and analysis actions; OAuth and AI provider workflows; appearance/options fidelity; non-Linux packaging. JVM plugin ABI is deliberately incompatible, plugin data is retained. TCC is deliberately excluded.

Next: port the C17 graph compilation vertical slice. Read `../threadworks/compilers-impl/src/main/resources/C.implementation.md`, `compiler-templates/c/`, `compiler-api/.../CompilerApi.kt`, and C compiler tests. Build on `core::generation::{VirtualFile,SourceMap}` without treating processor bodies as standalone files. Implement type declarations, per-link buffers and thin processor/composite execution, template assembly and precise origin maps; run generated code with external clang through Jobs. Expose one real Generate workflow via backend/MCP/desktop, then connect clangd diagnostics to mapped node sections. Use copied/temporary fixtures, keep the old app untouched, update parity/status, commit and push. After that expand language templates/layouts and sheets/editor fidelity. Do not mark the migration complete while these workflows remain pending.
