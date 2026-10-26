# Screenshot-driven migration

Reference: [25 chronological photos](threadworks/README.md), original names/times/hashes in [manifest](threadworks/manifest.json). All image bytes are unchanged. Captures establish visible layout; the Kotlin implementation and live baseline establish behavior. Source revision: Threadworks 2.6.7 at `7d392d3a0e30b86445e00e7c03032a5188b26593`.

## Photo-to-implementation map

Source UI paths below are relative to `app-desktop/src/main/kotlin/com/threadwork/app/`.

| Photos / workflow | Source evidence | Connected Rust implementation | Verification / remaining detail |
|---|---|---|---|
| 001,015–017 Designer | ui/ThreadworkDesktopApp.kt canvas/hierarchy/registerCommands | core drawing/viewport; desktop scene: absolute composites, collapse/resize/subtree moves, ports/types, usage boxes, selection, alignment | Core projection/route/BVH tests; native collapse/move/resize/callout selection and original creation/link/drag smoke. Full source routing/boundary typography remains partial |
| 002 Editor | Same file inspector/editor; editor/GridCodeEditorAdapter.kt | Selected list and collapsible hierarchy; five code sections, section-language/C/C++ highlighting; structured scalar/type/metadata inspector, inherited values and tracking; advanced JSON | Native name/source preservation, type/metadata Apply, duplicate-key rejection and invalid draft recovery. Master revision/inherited tracking tests. C17 Ctrl-Space implemented; broader language completion/navigation pending |
| 021–025 Contextual code completion | editor/GridCodeEditorAdapter.kt Ctrl-Space; compiler/c/CCompiler.kt codeIntelligence/generatedFunctionHeader | Shared compiler context and a dedicated real clangd, source-mapped to the draft body; GPUI Ctrl-Space and generated header | Real ticker runtime/buffer/library/member, Unicode, scope, empty-body and stale-revision tests; native insertion/undo/typing dismissal passed. Broader languages/navigation and explicit popup cancellation remain gaps. |
| 003 Project management | ui/ProjectManagementPanel.kt | Six real status lanes, counts/cards, drag/drop, remove-to-backlog, add BUSINESS; shared history/undo | Native lane move/removal/backlog and API status tests. OAuth/avatar providers pending |
| 004 Archetypes | ui/WorkflowArchetypesPanel.kt | Grouped embedded catalog/descriptions; read-only scene preview, explicit Insert; import/save paths | Native asserts preview does not mutate, Insert does; all embedded imports/remapped references tested |
| 005 Analysis | ui/NetworkAnalysisPanel.kt; core analysis | P/E/D/U sparse matrices, paged columns/virtual rows; dependent→dependency direction, coverage/candidates/findings and inspection actions | Fixtures and native matrix endpoint navigation. Reachability explicitly bounded above 4096. Automatic layout/model optimization was future work in source |
| 006–009,012,016 Menus | ui/ThreadworkDesktopApp.kt registerCommands | Categorized menus, searchable keyboard palette, file/clipboard/undo/align/distribute/export/options/About/shared MCP guide | Existing services and native palette/Help checks. Compiler-from-overrides and other graph compilers remain pending, not fake successful commands |
| 010 MCP | ui/McpServerDialog.kt; server controller | Authenticated endpoints/config, independent Start/Stop/Restart, health and metadata-only 256-request log | Real HTTP 503/200 transition while API works; native controls. Leaves old app untouched |
| 011,020 AI | ai/AiSupport.kt | Clipboard tasks with dossiers and C17 context; staged provider profiles/env-var credential references | Prompt fixtures/errors. Audited all four source providers: each returns Unavailable; browser auth explicitly unimplemented. Network inference not claimed |
| 013 Plan | ui/PreprintDialog.kt; SheetLayout.kt | Real backend preview, ISO/landscape/roll/Auto, 1:N, split/overlap, parts/title/folding; vector/searchable or raster PDF, SVG/PNG/Mermaid; PDF All; printer profiles | ISO dimensions/page counts, PNG thumbnail, vector text/raster image/compression, native preview. Physical print and exact legacy typography unverified |
| 014 Documentation | PreprintDialog.kt; FopPdfWriter | Header/footer/margins, real paginated preview and PDF, escaped HTML, Markdown | Output/escaping and combined Plan+Docs tests; native documentation preview. Full rich Markdown/FOP layout fidelity remains partial |
| 018–019 Options | ui/ThreadworkAppearance.kt, ThreadworkDesignerSettings.kt, editor/ThreadworkEditorSettings.kt | Staged general/color/provider forms; theme/fonts/embedded Monaspace, color pickers/grid/wheel/composite size/indent; atomic settings; Java XML import | Native Save/restart theme/indent/panes; validation/import tests. Missing legacy fonts fall back; unknown panel values archived |

## Visual language and interaction decisions

Neutral light/dark surfaces, teal active states, readable pane spacing and a consistent 24-unit/1.5-stroke vector icon set replace mixed Swing artwork. Common icons are embedded as SVG for editor/color-picker widgets as well. Monaspace Neon/Krypton are embedded with their OFL. The source's D-DIN font is not redistributed without a separately located license; imported family names use system fallback if absent.

Transport and business behavior are shared: desktop is a client of the authenticated backend; MCP invokes the same services. Table drafts preserve unknown values and survive invalid intermediate states. Apply sends only changed properties. Correlated response revisions protect newer typing from an earlier commit; unrelated status/canvas updates cannot advance the editor baseline.

The Sheet button toggles unified Export preview instead of the source's editable paper overlay on the canvas. This makes the real export controls reachable, but the original paper-placement interaction remains a fidelity gap. Printing requires explicit selection/submission; discovery/preview never send a physical job. Export profiles belong to Wirecraft's isolated session, not Java Preferences.

## Runnable slices and acceptance

1. Chronological rename/index: initial 20 original hashes retained (`75a0d2f`); five later editor captures appended as 021–025, with original names/times/hashes recorded.
2. Workbench/icon/inspector/board/matrix/archetype preview: connected (`a73f4e7`).
3. Composite drawing, settings and independent MCP controls: connected (`24f610d`).
4. Technical reports, clipboard assistance and inspector collections: connected (`260d016`).
5. Refinements: preserved tracking, table recovery, per-printer PDF All, font migration, bounded reports, segment BVH and native interaction regression; current code/checks in status.md.
6. Linux package/install/native verification and minor release: record actual results in status.md before tagging.

## Verification procedure

Run `cargo fmt --all -- --check`, `cargo test --workspace --locked`, strict workspace Clippy and build. `python3 scripts/desktop_smoke.py` checks basic creation/link/clipboard/recovery plus real clangd/compiler/generated diagnostic reveal and process cleanup. `python3 scripts/walkthrough_smoke.py` checks the mapped views and mutations on a deterministic synthetic topology fixture under runtime tests. It uses only its owned PID/backend/token, temporary project/settings and actual X11/XWayland events; it never prints or edits live projects. Captures are under target/walkthrough-smoke; reviewed release captures belong under evidence/.

Performance examples: `cargo run -p wirecraft-core --release --example drawing_bench` and `--example route_cull_bench`. These measure CPU projection/index/query work, not GPU frame time. Packaging uses scripts/package-linux.sh and scripts/package_smoke.py. Retain all incomplete parity in parity.md; no milestone implies the complete migration is done.

## Follow-up interaction requirements

The user prefers the original compact inspector: label/value rows, dropdowns and inline tables over cards. The new inspector follows that structure, with state history and metadata/type tables, staged Apply, preserved unknown data and invalid-draft recovery. Linux client decorations expose real minimize/maximize/restore/close actions; other platforms retain their native title bars but remain unverified. See scripts/editor_smoke.py for the compiler-context and window-control workflow and scripts/walkthrough_smoke.py for collection/recovery and board/report regression.

The supplied ticker fixture additionally verifies C plus six unchanged file exports, a mapped error from the original unfinished pu token, and real CSV output from an explicitly corrected temporary copy. Documentation's composition page now embeds vector paths/text rather than Mermaid source or a preview bitmap; real PDF image/text inspection is part of the ticker MCP test.
