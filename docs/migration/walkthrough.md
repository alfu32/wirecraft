# Screenshot-driven migration

Reference: [20 chronological photos](threadworks/README.md), original names and hashes in [manifest](threadworks/manifest.json). Local Kotlin source remains authoritative for behavior. Images 011 and 020 explicitly show disabled AI actions / unimplemented browser authorization; do not turn these into fabricated successes.

| Photos | Workflow | Source evidence (under app-desktop/src/main/kotlin/com/threadwork/app) | Implementation target | Acceptance / current gap |
|---|---|---|---|---|
| 001,015–017 | Designer, hierarchy, composite display, multi-selection and alignment | ui/ThreadworkDesktopApp.kt canvas/registerCommands | desktop scene and shared geometry/services | Existing select/drag/link/cull; expand composite/ports, improve selection, expose all align commands |
| 002 | Selected list, section editor and structured inspector | ui/ThreadworkDesktopApp.kt inspector/editor | desktop editor + typed form patches | Replace JSON-first inspector with editable fields, inherited values, status/history and advanced JSON escape hatch |
| 003 | Kanban and backlog | ui/ProjectManagementPanel.kt | desktop management using update_entity | Six real status columns; move/unassign and backlog; changes persist and undo |
| 004 | Grouped archetypes and preview | ui/WorkflowArchetypesPanel.kt | archetypes services + read-only scene | Select to preview; explicit insert; selection must not mutate design |
| 005 | Matrices and report actions | ui/NetworkAnalysisPanel.kt | core analysis + desktop report | Four sparse relation matrices, actual indicators/findings, navigate affected entities |
| 006–009,012,016 | Menus, palette, shortcuts and guide | ui/ThreadworkDesktopApp.kt registerCommands | shared desktop action dispatcher | Recognizable menus; command search; real existing commands; unsupported compilation overrides explicitly tracked |
| 010 | MCP management | ui/McpServerDialog.kt | runtime MCP controls/log + desktop | Authenticated endpoint, health, bounded access log and independent MCP enable/disable (do not kill UI backend) |
| 011,020 | AI/provider configuration | ai/AiSupport.kt | runtime prompt/provider adapter + desktop | Clipboard prompts without credentials; network provider calls explicit; no fake authorization |
| 013,014 | Technical sheet and documentation export | ui/PreprintDialog.kt, SheetLayout.kt | core sheet geometry/docs; runtime exports; desktop preview | Format/scale/title/parts, PDF/SVG/PNG/Mermaid; HTML/Markdown docs; ISO/poster and native printers tracked separately |
| 018,019 | General and semantic appearance options | ui/ThreadworkAppearance.kt, ThreadworkDesignerSettings.kt, editor/ThreadworkEditorSettings.kt | persisted settings + desktop options | Save/cancel real settings, apply fonts/indent/grid/colors/wheel, restore across restart |

## Dependency order

1. Preserve and index walkthrough (hashes, chronological names, feature evidence).
2. Cohesive vector icon language and shell; real menus/palette and structured inspector; preserve draft/revision safety.
3. Management board/backlog, analysis matrices/actions, archetype preview/explicit insert.
4. Canvas composite/ports/resize/selection fidelity; semantic appearance settings.
5. Sheet/documentation preview/export, MCP management and prompt/provider workflow.
6. Full checks, isolated desktop workflow evidence, updated parity/status, minor release and push.

Each slice must remain runnable. Test meaningful mutations and failure cases through the same services used by UI/MCP. Never label an unverified or pending workflow complete.

## Progress

- Reference indexing complete: 20 images visually inspected, renamed in capture order, exact bytes preserved with SHA-256. Source application remains untouched.
- Implementation slices pending in this branch after v0.3.0.

### Workbench slice

Implemented menus and searchable command palette, native stroke icons, structured general/management/language/link inspector with explicit Apply and advanced JSON, six-lane Kanban/backlog with drag/drop and transactional status changes, read-only archetype preview plus explicit insertion, and virtual-row adjacency matrices with endpoint/findings navigation. Core report now separates structural relationship union from containment. Source column status names and null backlog behavior were checked in ProjectManagementPanel.kt.

Checks: workspace tests passed (38 integration tests); strict clippy passed after collapsing a nested preview conditional; GPUI built and an owned temporary-state window launched/captured/closed successfully (`target/walkthrough-smoke/initial.png`). Full interactive board/editor regression remains the next acceptance gate. Options, sheet/detail fidelity and provider actions are still pending.

### Canvas and local controls slice

Expanded-composite projection now follows absolute child coordinates from the source data, hides collapsed descendants, and projects external links onto the visible boundary. The GPU canvas shows semantic roles, fields, port names/types and diagnostic markers; supports collapse toggles, resize commits, composite subtree moves and a visible selection rectangle. Spatial indexing remains in use. Routing tries bounded alternative corridors around terminal obstacles; this is still less capable than the source's full boundary/routing heuristics.

Settings now have validated appearance/font/grid/color/wheel/provider values and an application-owned atomic settings store, shared HTTP service, staged Save/Cancel/defaults UI. MCP can be disabled independently of the UI API and has a bounded metadata-only access log. Tests verify actual HTTP 503/200 transitions, continued UI service operation, settings persistence/rejection, and composite projection.

Checks: workspace tests passed (40 integration tests), strict clippy passed. Interactive resize/collapse/settings verification and export/provider work still pending.

### Export, assistance and inspector slice

Technical-sheet service now exports Auto/A0–A4, landscape and roll formats, 1:N scales, overlap and fixed-sheet pagination. Multipage PDFs keep ISO dimensions; selectable vector/searchable or raster PDF modes are real. Single-image formats reject multiple pages rather than losing content. Documentation exports HTML, Markdown and paginated SVG-backed PDF with margins/header/footer. Preview thumbnails are rasterized in the backend at at most 1200 pixels per side. CUPS discovery and explicit supervised print submission are connected; no physical print has been submitted during verification.

Clipboard task prompts now include source/specification/tests instructions, selected dossiers and available C17 wrapper context. Source audit of ai/AiSupport.kt confirms all four network providers return Unavailable; Wirecraft reports those configurations as unsupported rather than claiming inference. A new MCP workbench tool exposes the same preview/export/settings/prompt/printing services (37 total tools).

Inspector collections now edit metadata and type fields (references/array/list/map), preserve unknown fields and recover even invalid staged table edits. The editor adds a selected-entity pane, collapsible hierarchy and persisted resizable pane widths. Light/dark chrome, color pickers and semantic canvas options are connected. Analysis matrices page columns and virtualize rows; dependency direction now follows the source's dependent → dependency convention, excludes source/runnable capabilities from P/E/D, and adds candidate/error-coverage indicators and findings.

Checks to date: full tests passed before the last indicator/pane refinements (43 integration tests); export tests check real vector/raster PDF content, ISO sizes/page counts, bounded preview PNG, HTML escaping and unsupported multi-image output. Updated native desktop smoke passed creation, link, clipboard/undo, recovered source draft, real clangd/build/generated diagnostics, click-to-reveal and process cleanup. That smoke found and corrected a restored-selection lookup before the backend's first snapshot. Further screenshot-driven interaction checks are in progress; no release tag yet.
