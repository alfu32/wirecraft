# Current status

Baseline captured; implementation in progress. No parity completion claim.

- Source clean at 7d392d3a0e30b86445e00e7c03032a5188b26593; old app remains running, read-only access only.
- Destination initially empty; 3-crate Rust workspace established (domain/storage, services/transports/tools, GPUI desktop).
- Live MCP discovery, schemas, resources/prompts, safe reads recorded under baseline/. User identity records redacted.
- GPUI selected after official documentation comparison. Foundation cargo check --workspace passed (GPUI transitive libc pinned to 0.2.189).
- Release policy: regular Conventional Commit commits and pushes; patch/minor tags only. Major set exclusively by user.

Next: implement typed lossless .orch model and graph transactions, then shared application dispatcher and desktop workflow. Follow docs/migration/plan.md. Record actual command results here after each milestone.

Milestone: domain + headless services implemented. `cargo test -p wirecraft-core -p wirecraft-runtime`: 12 integration tests passed (6 core + 6 runtime), including real authenticated HTTP/MCP, real clangd diagnostics after edits, document symbols/completion, restart/cleanup, Unicode positions, external process failure/cancellation/timeout. No test was skipped. Desktop is not connected yet; implementation continues. Core/source fixture is a copy, source checkout unchanged.
