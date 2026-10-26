# Threadworks baseline

Source: `../threadworks`, clean revision `7d392d3a0e30b86445e00e7c03032a5188b26593` (2.6.7), inspected 2026-10-26. Destination initially contained only an unborn Git repository, origin `git@github.com:alfu32/wirecraft.git`. No destination or ancestor AGENTS.md exists. Source AGENTS.md and design guide were read as reference; no source changes or old-process lifecycle actions are authorized or needed.

## Purpose and evidence

Topology-first IDE: hierarchical processors, libraries, types, links, source/specification/tests/initialization/AI instructions, technical drawings, generated software and granular ownership. Design principles in `docs/design-guide.md` are advisory: reusable capabilities belong in libraries, processing nodes route values and typed errors.

| Area | Implemented source and baseline | Qualification |
|---|---|---|
| Model | `core/.../model/Model.kt`, `LinkInteraction.kt`, `LinkTransport.kt` | One Node type; six kinds; links are entities. Inherited technology, layout, responsibility, revisions and source languages. Generic type expressions, binary Base64, plugin data, status history and users are persisted. |
| Persistence | `storage-json/.../JsonDocumentStore.kt`, `DocumentRepository.kt` and tests | Current list-shaped nodes, legacy map accepted. Repairs parents/children, ports, link ownership and crossed boundaries. Legacy link endpoints become processors with migration metadata. Repository mutations synchronize indexes. |
| Validation/classification | `core/.../validation/DocumentValidator.kt`, `classification/NodeClassifier.kt` | Structural/type/port checks and inferred stereotypes. Validator does not explicitly detect parent cycles: Rust must reject cycles. |
| Designer | `app-desktop/.../ui/ThreadworkDesktopApp.kt` (canvas starts near line 3000) | Selection/window selection, drag, pan/zoom, composites, creation, routing; 512px memory-only LRU tiles, route cache and overlay. Screenshot proves running empty Designer and modeless MCP management dialog, not all interaction behavior. |
| Shell | same file, `menuBar` and `registerCommands` near lines 694–843 | File/Edit/Graph/Generate/MCP/AI Support/Help; Designer, Editor, Project Management, Archetypes, Analysis tabs. Ctrl N/O/S/Shift-S/P/Q; undo Z, redo Y/Shift-Z, clipboard X/C/V; Delete; Escape; Shift-Ctrl-P palette. Options, About, entity inspector, print preview, authentication and MCP dialogs. |
| Editors | `editor/GridCodeEditorAdapter.kt`, `CodeEditorAdapter.kt`, `ThreadworkEditorSettings.kt` | Active grid editor: multicursor, highlighting, indentation, completion and undo. CodeMirror is an adapter boundary, not the active editor. |
| Project management | `ui/ProjectManagementPanel.kt`, `identity/*`, core project tests | User assignment/status/history; provider OAuth implementation. Network requests and credentials require separate future verification. |
| Sheets | `ui/SheetLayout.kt`, `PreprintDialog.kt`, `FopPdfWriter.kt`, `RasterPdfWriter.kt` | ISO/roll sheets, title block, parts list, SVG/PNG/PDF and poster tiling. Significant feature, must not be silently dropped. |
| Analysis | `core/.../analysis/NetworkAnalysis.kt`, `ui/NetworkAnalysisPanel.kt` and tests | Sparse principal/error/dependency/structural graphs, thresholds, findings and actions. MCP exposes counts only, not this full report. |
| Archetypes | `builtin-archetype/src/threadworks`, `ui/WorkflowArchetypesPanel.kt` | Stored .orch templates across languages, import/save UI. |
| Compilers | `compiler-api/.../CompilerApi.kt`, `VirtualFileStorage.kt`, `compilers-impl/...` | Recursive compilation, four filesystem layouts, source maps, Pebble templates for C/Go/Kotlin/Node/PHP/QuickJS, generic compiler/compiler-compiler, docs and filesystem export. Tests exist for these; not executed in read-only source checkout. |
| Intelligence | `completion-core/.../{CodeAnalysis,CompilerCodeAnalysisProvider,TreeSitterAnalysisProvider,ExternalCodeIndex}.kt`; `ui/{EmbeddedSourceValidator,TinyCcSourceValidator,QuickJsSourceValidator}.kt` | Existing model/syntax/compiler providers. Rust replaces C embedded TCC with clangd + external builds; TCC/JAR/DLL will not be copied. |
| Settings/session | `ui/Threadwork{Ui,Designer}Settings.kt`, `editor/ThreadworkEditorSettings.kt`, `identity/UserIdentity.kt` | Java Preferences under com/threadwork; panel widths/editor settings/identity persistence. Old settings must remain untouched; import is explicit. |
| Background work | `ThreadworkDesktopApp.kt`, `McpModelGateway.kt`, `McpServerController.kt` | Swing EDT ownership; single validation executor, 700ms debounce; autosave 10s, selection refresh 50ms, resource 1s, route debounce 180ms. MCP desktop gateway marshals to EDT; headless gateway synchronized. HTTP daemon executor. |
| Packaging | Gradle scripts, `.github/workflows/release.yml`, `snapcraft.yaml`, `snap/` | Kotlin 2.2/JDK21; FlatLaf, serialization, Pebble, Batik/FOP/PDF and embedded JARs; fat JAR, Snap and Windows release jobs. JAR ServiceLoader plugins are real, not Rust-compatible. Platform support requires actual builds, not assumptions. |

## Live MCP baseline

Connection discovered in user screenshot and confirmed by local `docs/mcp.md` and `McpServerController.kt`: `http://127.0.0.1:8765/mcp`. No connected Threadworks tool or matching Codex config entry was available. HTTP requests succeeded without credentials. `/health` reports Threadwork 2.6.7. Server advertises stateless discovery 2026-07-28 plus legacy initialize (including 2025-11-25). Advertised 19 tools, five resources, two prompts are captured in `baseline/`.

Safe live calls: discovery, initialize, list tools/resources/prompts, get_design/entity(root)/fragment(root), list_entities, validate_design, analyze_network, command and technology catalogs. Live design is empty (one root, one user); validate returns valid=true and no diagnostics. Network counts: one entity, no links/types/composites/ports. Two initial incorrectly named tool probes returned isError=true; retained as error evidence. Further calls used advertised names. No live mutations, saves, commands, project opens, settings changes, stop or restart occurred. Baseline user records are redacted in committed captures.

MCP does not expose screenshots, pointer/keyboard UI automation, dialog inspection or detailed analysis reports. UI behavior beyond the supplied screenshot is source-backed, not runtime-verified.

## Defects, limits and unfinished claims

Documentation is not acceptance evidence. Source has complete working implementations and tests for many broad features, but no complete old test run is claimed. The MCP HTTP server is unauthenticated loopback; arbitrary execute_command can invoke UI operations. Old MCP protocol strings are preserved as source-specific compatibility behavior, not treated as independent proof of an official protocol revision. CodeMirror is inactive. JVM plugins cannot load in Rust. OAuth settings/tokens must not be imported implicitly. Rust parity gaps are tracked explicitly in parity.md; no feature is deleted from scope merely because it is expensive.
