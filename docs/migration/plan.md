# Dependency-ordered plan

1. Baseline + foundation (0.1.0): record revision/live schemas, compile Rust workspace. Acceptance: cargo check and reproducible baseline.
2. Document vertical slice: lossless .orch load/save, repair and mutations, HTTP service, GPUI hierarchy/canvas/create/edit/save, undo/redo. Acceptance: old fixtures round-trip; desktop and HTTP observe same revision; project reopens.
3. MCP parity: advertised 19 tool schemas, resources/prompts, stdio and loopback HTTP, same services. Acceptance: real HTTP/stdio clients, old read response comparisons and isolated mutations.
4. Language/build services: real clangd initialization/sync/diagnostics/navigation/completion/symbols, tool jobs/cancel/shutdown. Acceptance: real C diagnostics change after edit, missing executable/error/restart tests, no owned process left.
5. Canvas/editor expansion: selection, drag, links, reparenting, collapse, navigation, viewport restoration; culling/routing measurements. Acceptance: manual desktop smoke plus geometry tests.
6. Remaining parity: full compiler/templates/layouts/source maps, archetypes, analysis actions, project management, technical sheets/export, richer editing and AI assistance, identity as appropriate. Acceptance: representative old fixtures and per-feature checks, never placeholder success.
7. Configuration migration/install/package: explicit config import, Linux package and CI, available-platform smoke. Acceptance: install into temporary prefix, launch both modes and connect MCP. Cross-platform claims only after platform builds.

Keep each stage runnable. This is a large migration; incomplete features remain in parity.md and the exact next step in status.md. Reorder services before desktop polish when that makes behavior independently testable.

## Milestone checkpoint

0.1.0: runnable domain/backend/GPUI vertical slice, compatibility and live MCP baseline. 0.2.0: archetypes, fragments/layout, reports, exports, explicit settings import and verified Linux installation. 0.3.0: typed C17 graph-to-source/native execution and generated clangd diagnostics with editor navigation. These milestones have observable integration and owned-desktop checks recorded in status.md; they do not imply full parity. 0.4.0 targets the screenshot-driven workbench: coherent icons, composite canvas, structured inspector, board/backlog, archetype previews, matrices/actions, technical sheets/documents, options, MCP management and clipboard assistance. See walkthrough.md for each photo and acceptance check. Remaining dependencies are C source capabilities/template overrides/compiler export, other language compilers, inline LSP editor interactions, identity and exact layout/platform fidelity. Network AI and automatic layout optimization were unfinished in the source baseline. The headless services were built before UI polish to enable real contract and process verification throughout.

## Working C example priority (new evidence 023–025)

Reorder the next slices around immutable threadworks/example-c.orch and ticker-print.pdf. The working source example is stronger acceptance evidence than a synthetic graph alone.

1. Compile the unchanged mixed C/file-export ticker with external clang, preserve every text/binary export and source map, and compare real diagnostics. The supplied gen_uuid source ends in an unfinished pu token; assert its mapped error, then use an explicitly corrected temporary copy for execution. Resolve actual type/dependency/layout blockers without changing the fixture. Never run its legacy TCC launcher or embedded foreign binaries.
2. Add shared editor context/completion services: compiler-generated wrapper/runtime/types, current node inputs/outputs and injected library aliases; full draft synchronization without committing, Unicode/range mapping, cancellation/stale response guards. GPUI Ctrl-Space must insert actual usable completions and show signatures. Test real clangd plus native UI. Syntax language follows the effective section language.
3. Preserve vector drawing geometry through Plan PDF and PostScript export and documentation composition. Rasterize only preview/explicit raster output; inspect PDF streams/images and PostScript operators. Improve document headings, sections, code and composition against the supplied reference.
4. Re-run the fixture through shared API/MCP and installed desktop, update evidence/parity/status, finish the pending minor release checkpoint, then continue the remaining compiler/template/language/identity parity.

The reference PDF currently embeds a first-page JPEG despite the described vector renderer; retain this observation and test Wirecraft's vector contract directly. Do not equate the PDF container or a preview thumbnail with vector output.
