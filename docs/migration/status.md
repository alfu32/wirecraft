# Current status

Baseline captured; implementation in progress. No parity completion claim.

- Source clean at 7d392d3a0e30b86445e00e7c03032a5188b26593; old app remains running, read-only access only.
- Destination initially empty; 3-crate Rust workspace established (domain/storage, services/transports/tools, GPUI desktop).
- Live MCP discovery, schemas, resources/prompts, safe reads recorded under baseline/. User identity records redacted.
- GPUI selected after official documentation comparison. Foundation cargo check --workspace passed (GPUI transitive libc pinned to 0.2.189).
- Release policy: regular Conventional Commit commits and pushes; patch/minor tags only. Major set exclusively by user.

Next: implement typed lossless .orch model and graph transactions, then shared application dispatcher and desktop workflow. Follow docs/migration/plan.md. Record actual command results here after each milestone.

Milestone: domain + headless services implemented. `cargo test -p wirecraft-core -p wirecraft-runtime`: 12 integration tests passed (6 core + 6 runtime), including real authenticated HTTP/MCP, real clangd diagnostics after edits, document symbols/completion, restart/cleanup, Unicode positions, external process failure/cancellation/timeout. No test was skipped. Desktop is not connected yet; implementation continues. Core/source fixture is a copy, source checkout unchanged.

## Runnable 0.1.0 milestone

- `cargo build --workspace --locked`: passed on Linux with GPUI/Vulkan.
- `cargo test --workspace --locked`: 14 integration tests passed (6 core, 8 runtime); includes real HTTP MCP and stdio client exchanges, real clangd sync/diagnostics/completion/symbols/restart/cleanup, process failures/cancellation/timeouts, save/recovery and transactional failures.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed. Transitive proc-macro-error2 emits Cargo's future-incompatibility notice; no project lint failures.
- `cargo fmt --all`: applied; final format check included in release.
- All 45 source .orch files load in Rust; per-file output in baseline/source-fixture-validation.json. No source file modified.
- `python3 scripts/desktop_smoke.py`: passed on available X11/XWayland display. Creates two processors in temporary state, drags, edits an unapplied C source draft, exits, restarts, restores and applies that draft, and exits cleanly. Evidence images in evidence/. This does not automate the native file chooser or all canvas modes.
- Xvfb smoke blocked by Vulkan presentation: no DRI3, PlatformNotSupported. No successful Xvfb claim. Initial title-only matching selected the unrelated IDE; test corrected to require the owned process PID and visible window. No source/project file changes resulted.
- Desktop is a real HTTP client to its owned backend subprocess. UI handles Designer, five text sections, JSON entity properties, validation/counts, C/C++ standalone clangd/external syntax checking and MCP connection configuration. Backend supports all original MCP model tool names, resources and prompts; command/technology catalogs report implemented availability.
- Recovery stores applied graph state and original save path/dirty baseline; UI state stores unapplied drafts, selected entity, text section and viewport. Filesystem failures preserve dirty state. Linux owned-backend parent-death signaling plus graceful SIGTERM protect abnormal desktop exit.

Still incomplete: full old compiler/templates/layout/source maps, technical sheets and exports, rich graph analysis, archetype UX, clipboard/align/distribute, dedicated project management, OAuth/AI workflows, Java settings import, packaging and non-Linux verification. No complete migration claim.

Next implementation slice: expand graph operations/analysis and portable exports/archetypes, then packaging/config migration. Keep parity.md synchronized. Commits and tags are pushed to origin/master; use patch/minor releases only.

## 0.2.0 expansion (release verification in progress)

Implemented graph fragments/clipboard with internal link and type remapping; embedded source archetypes and import/save; alignment/distribution; sparse typed network reports; free-size SVG/PNG/vector PDF; Markdown component fiches; literal filesystem virtual files, source maps and staged output; explicit Java Preferences import and active indentation/tool configuration; Linux packaging/install scripts and CI checks. The original 19 MCP tools remain, with 15 extensions using the same services.

`cargo test --workspace --locked`: 23 integration tests passed (10 core, 13 runtime). All bundled archetypes import with fresh IDs. Export tests check real SVG/PNG/PDF output. The generation workflow produces a C source file, runs a real clang build, and executes the resulting binary. Unsafe paths/nonempty output and invalid settings are tested. `cargo clippy --workspace --all-targets --locked -- -D warnings` passes. Canvas benchmark in unoptimized build: 100,000 primitives indexed in 309ms; 1,000 1400×900 viewport queries in 172ms (mean 64 visible). This is an index measurement, not a full GPU frame benchmark.

Release package is building with two jobs. Pending verification before tagging: release build/install smoke, updated desktop smoke, final formatting/tests/lints and source-clean check. Next unresolved functionality remains language graph compiler/template parity and complete sheets/editor/project management/identity/AI behavior.
