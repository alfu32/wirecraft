# Threadworks walkthrough

Chronological capture order on 2026-10-26 (Europe/Brussels). Original image bytes are preserved; `manifest.json` records original names, timestamps and SHA-256 checksums. The screenshots establish visible layout, not proof that every displayed command works. Source baseline: Threadworks 2.6.7, `7d392d3a0e30b86445e00e7c03032a5188b26593`.

| Photo | Feature | Observed behavior / layout |
|---|---|---|
| [001](001-designer-overview.png) | Designer canvas and expanded hierarchy | Composite workers, types, libraries, named typed ports, directed routes, validation markers; grid and status bar. |
| [002](002-editor-inspector.png) | Editor and inspector | Selected-entity list above hierarchy; declaration/specification/usage/tests/instantiation tabs; section language; general/effective/management/tracking/metadata inspector. |
| [003](003-project-kanban.png) | Project management Kanban | Six status columns, counts, entity cards, paths, responsibility/assignee and remove-from-board control; backlog tab. |
| [004](004-archetype-preview.png) | Archetype browser | Language-grouped catalog, descriptions, read-only canvas preview and explicit Insert Archetype into Design. |
| [005](005-network-analysis.png) | Network analysis | Principal/error/dependency/union matrices; selectable relationship cells; overview and qualitative findings with affected-entity selection. |
| [006](006-file-menu.png) | File menu | New/Open/Save/Save As/archetype/export/options/quit and shortcuts. |
| [007](007-edit-menu.png) | Edit menu | Undo/redo, cut/copy/paste, align and distribute submenu. |
| [008](008-graph-menu.png) | Graph menu | Zoom extents, select/node/link modes, deletion and command palette. |
| [009](009-generate-menu.png) | Generate menu | Generate project/selection and generate compiler from model overrides. |
| [010](010-mcp-management.png) | MCP management | Endpoint and health endpoint; start/restart/stop/health; HTTP access log. Sheet tool is visibly active behind this dialog, not evidence it opens MCP. |
| [011](011-ai-support-menu.png) | AI support menu | Copy fiches, generate source, tidy specifications, generate test data. These entries appear disabled in this capture; source behavior must establish availability. |
| [012](012-help-menu.png) | Help menu | About and MCP Agent Guide. |
| [013](013-export-plan.png) | Plan export | Printer list, plan preview, format/scale/split/overlap; rasterized/searchable PDF; PDF/SVG/PNG/Mermaid; PDF All and persisted printer settings. |
| [014](014-export-documentation.png) | Documentation export | Documentation preview, page format/header/footer/margins, PDF/HTML/Markdown. |
| [015](015-canvas-box-selection.png) | Box selection | Translucent selection rectangle over several types, partial-overlap selection and current canvas viewport. |
| [016](016-command-palette-align.png) | Command palette and alignment | Full categorized command list with shortcuts and archetype submenu; Align Left chosen for multi-selection. |
| [017](017-canvas-aligned-result.png) | Aligned canvas result | Type nodes share a left edge after command; rest of graph retained. |
| [018](018-options-general.png) | General options | Theme, designer/editor fonts, composite title target and reference viewport, indent spaces, mouse-wheel rows. |
| [019](019-options-colors.png) | Designer color options | Editable semantic canvas/grid/node/type/compiler/library/error/test and link colors with color pickers. |
| [020](020-options-inference.png) | Inference provider options | Active provider, Claude/Codex/ChatGPT/local Ollama/clipboard tabs, endpoint/model/API token; browser authorization explicitly marked not implemented. |

Implementation and verification tracking: [walkthrough parity](../walkthrough.md).
