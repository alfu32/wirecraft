# Dependency-ordered plan

1. Baseline + foundation (0.1.0): record revision/live schemas, compile Rust workspace. Acceptance: cargo check and reproducible baseline.
2. Document vertical slice: lossless .orch load/save, repair and mutations, HTTP service, GPUI hierarchy/canvas/create/edit/save, undo/redo. Acceptance: old fixtures round-trip; desktop and HTTP observe same revision; project reopens.
3. MCP parity: advertised 19 tool schemas, resources/prompts, stdio and loopback HTTP, same services. Acceptance: real HTTP/stdio clients, old read response comparisons and isolated mutations.
4. Language/build services: real clangd initialization/sync/diagnostics/navigation/completion/symbols, tool jobs/cancel/shutdown. Acceptance: real C diagnostics change after edit, missing executable/error/restart tests, no owned process left.
5. Canvas/editor expansion: selection, drag, links, reparenting, collapse, navigation, viewport restoration; culling/routing measurements. Acceptance: manual desktop smoke plus geometry tests.
6. Remaining parity: full compiler/templates/layouts/source maps, archetypes, analysis actions, project management, technical sheets/export, richer editing and AI assistance, identity as appropriate. Acceptance: representative old fixtures and per-feature checks, never placeholder success.
7. Configuration migration/install/package: explicit config import, Linux package and CI, available-platform smoke. Acceptance: install into temporary prefix, launch both modes and connect MCP. Cross-platform claims only after platform builds.

Keep each stage runnable. This is a large migration; incomplete features remain in parity.md and the exact next step in status.md. Reorder services before desktop polish when that makes behavior independently testable.
