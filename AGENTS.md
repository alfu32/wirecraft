# Wirecraft development

Read `docs/migration/status.md`, `parity.md` and `architecture.md` before continuing the migration. Source baseline is Threadworks 2.6.7 at `7d392d3a0e30b86445e00e7c03032a5188b26593` in `../threadworks`. Keep that checkout unchanged and do not stop/restart its running application.

- `wirecraft-core`: graph/model/rules, codecs, geometry and pure reports/generation. Keep UI and network dependencies out.
- `wirecraft-runtime`: application transactions, storage orchestration, HTTP/MCP, configuration, language servers and subprocess supervision. Desktop and MCP share these services.
- `wirecraft-desktop`: GPUI client, immutable snapshots, canvas and editors. Blocking work stays on its client worker/backend. Maintain spatial culling and virtual hierarchy; measure before replacing them.
- Preserve `.orch` data and unknown fields. Reject corrupt topology and unsafe generated paths. Use temporary fixtures for mutation checks. Do not reintroduce TCC, its DLLs or JARs.
- GPUI and libc versions are pinned for a known transitive xattr compatibility issue. Use Cargo.lock.
- Implement and verify real workflows; keep pending parity explicit. Do not advertise unsupported compiler/editor/export behavior as complete.

Run formatting, relevant integration tests and strict lint checks for Rust changes. Available commands: `cargo fmt --all -- --check`, `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`. Desktop smoke uses an owned PID and temporary state on a working X11/XWayland/Vulkan display; Xvfb lacks DRI3 here. Packaging/install smoke instructions are in README.md.

Commit regularly using Conventional Commits/Commitizen and push the configured origin. Tag verified milestones with SemVer. Only patch and minor increments are authorized; the user sets major versions explicitly.
