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

0.1.0: runnable domain/backend/GPUI vertical slice, compatibility and live MCP baseline. 0.2.0: archetypes, fragments/layout, reports, exports, explicit settings import and verified Linux installation. 0.3.0: typed C17 graph-to-source/native execution and generated clangd diagnostics with editor navigation. These milestones have observable integration and owned-desktop checks recorded in status.md; they do not imply full parity. 0.4.0 implements the screenshot-driven workbench: coherent icons, composite canvas, structured inspector, board/backlog, archetype previews, matrices/actions, technical sheets/documents, options, MCP management and clipboard assistance. See walkthrough.md for each photo and acceptance check. Remaining dependencies are C source capabilities/template overrides/compiler export, other language compilers, broader inline LSP navigation/languages, identity and exact layout/platform fidelity. Network AI and automatic layout optimization were unfinished in the source baseline. The headless services were built before UI polish to enable real contract and process verification throughout.

## Working C example priority (new evidence 023–025)

Reorder the next slices around immutable threadworks/example-c.orch and ticker-print.pdf. The working source example is stronger acceptance evidence than a synthetic graph alone.

1. Compile the unchanged mixed C/file-export ticker with external clang, preserve every text/binary export and source map, and compare real diagnostics. The supplied gen_uuid source ends in an unfinished pu token; assert its mapped error, then use an explicitly corrected temporary copy for execution. Resolve actual type/dependency/layout blockers without changing the fixture. Never run its legacy TCC launcher or embedded foreign binaries.
2. Add shared editor context/completion services: compiler-generated wrapper/runtime/types, current node inputs/outputs and injected library aliases; full draft synchronization without committing, Unicode/range mapping, cancellation/stale response guards. GPUI Ctrl-Space must insert actual usable completions and show signatures. Test real clangd plus native UI. Syntax language follows the effective section language.
3. Preserve vector drawing geometry through Plan PDF and documentation composition. The Rust shared scene emits SVG paths/text directly into vector PDF; preserving the vector contract takes precedence over retaining the old PostScript intermediate. Rasterize only preview/explicit raster output; inspect PDF streams/images. Improve document headings, sections, code and composition against the supplied reference.
4. Re-run the fixture through shared API/MCP and installed desktop, update evidence/parity/status, finish the pending minor release checkpoint, then continue the remaining compiler/template/language/identity parity.

The reference PDF currently embeds a first-page JPEG despite the described vector renderer; retain this observation and test Wirecraft's vector contract directly. Do not equate the PDF container or a preview thumbnail with vector output.

The 0.4.0 checkpoint also includes compiler-context C17 Ctrl-Space completion, compact dropdown/inline-table inspector and real Linux window controls. Source capabilities are the next compiler slice; see status.md for the exact implementation entry point.

0.4.1 corrects canvas/tree gestures at the user's request before the next compiler slice. Acceptance: real mouse input for anchored wheel zoom, middle drag/outside release, directional windows and modifier picks, visible individual/group contours, tree selection/drop reparent, one undoable group/subtree move and no document mutation while previewing. Run scripts/canvas_smoke.py and include --canvas in package verification.

## Evidence-driven corrective slice (0.5.0)

Before the next source-capability/compiler milestone, implement and verify photos 026–042: assembled technical sheets, all-command scrolling context menu and nested archetypes, grammar registration/full-program semantic colors, callable signatures and hover, dependency callouts, separated octilinear routes and composite ports, pointer release/window-edge hitboxes, and warning/error badges plus a clickable issue list. Acceptance includes the unchanged supplied ticker and an isolated WebServer topology, real clangd/MCP, native menu insertion/undo and typed editor interaction, and installed Linux package checks. The currently reachable JVM 2.6.7 MCP was consulted using read-only calls; advertised schemas and results are under baseline/jvm-live-2026-10-27. This supersedes the earlier connection-refused limitation without changing the original project.


## Compiler/template restoration (0.6.0)

The user's missing-language inventory takes precedence over the remaining C source-capability slice. Acceptance: embed and checksum all six original template sets; parse every Pebble asset; connect graph generation, template catalog and portable model-override export to CLI, desktop and MCP; execute isolated Go/Node/Kotlin/PHP/QuickJS producer/channel/consumer programs; verify both relevant file layouts, source maps, custom bundle round trips, unsafe paths and malformed templates. Keep broader parity explicit. Continue with C source capabilities/overrides, template composite boundary ports, aggregate mixed-language dispatch, remaining collections and generated-body editor adapters; do not equate asset import with full migration.

## Router and recovered inspector edits (0.6.1)

Migrate the JVM candidate/Euclidean scoring rules and test reference outputs before routing reservations. Keep strict box/channel constraints, add native diagonal fallback moves and preserve composite boundary ports. Check all quadrants and both desktop/vector output. Separately restore inspector text Enter/blur commits (JVM applyOnCommit), preserve real recovered drafts, and make Apply/Discard visible in Designer. Verify link rename, restart, body-draft Apply and recovered-name Discard through an owned native fixture; do not modify the user's live draft or project.
