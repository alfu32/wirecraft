# Command-by-command parity

Source command catalog was read from the running 2.6.7 app. Implementations originate in `app-desktop/.../ui/ThreadworkDesktopApp.kt` (`registerCommands`, `registerAiCommands`). This table distinguishes desktop UI availability from the smaller enabled MCP execute_command catalog; MCP additionally exposes equivalent model/workflow tools.

| Source ID | Source title | Wirecraft behavior | Status |
|---|---|---|---|
| `ai.copyComponentFiches` | AI Support: Copy Selected Component Fiches as Markdown | Export → Copy component fiches (Markdown); richer dossier/compiler context pending | partial |
| `ai.generateSource` | AI Support: Generate Source Code for Selected Components | Retained in migration scope; no successful placeholder or enabled MCP command | pending |
| `ai.generateTestData` | AI Support: Generate Test Data for Selected Components | Retained in migration scope; no successful placeholder or enabled MCP command | pending |
| `ai.tidySpecification` | AI Support: Tidy and Format Selected Specifications | Retained in migration scope; no successful placeholder or enabled MCP command | pending |
| `app.options` | Application: Options... | settings.json + explicit XML import; full options UI pending | partial |
| `commands.palette` | Commands: Open Palette | Retained in migration scope; no successful placeholder or enabled MCP command | pending |
| `compile.compiler` | Generate: Compiler from Overrides | Retained in migration scope; no successful placeholder or enabled MCP command | pending |
| `compile.project` | Generate: Project or Selection | literal file-export/multi-tech and native C17 topology + mapped diagnostics + external tools; remaining capabilities/templates/languages pending | partial |
| `edit.align.bottom` | Edit: Align Bottom | entities.align service supports all four edges and both distribution axes; Designer exposes left/horizontal controls | partial UI |
| `edit.align.left` | Edit: Align Left | entities.align service supports all four edges and both distribution axes; Designer exposes left/horizontal controls | partial UI |
| `edit.align.right` | Edit: Align Right | entities.align service supports all four edges and both distribution axes; Designer exposes left/horizontal controls | partial UI |
| `edit.align.top` | Edit: Align Top | entities.align service supports all four edges and both distribution axes; Designer exposes left/horizontal controls | partial UI |
| `edit.copy` | Edit: Copy | entities.copy; typed fragment clipboard | implemented |
| `edit.cut` | Edit: Cut | entities.cut; atomic fragment + delete | implemented |
| `edit.distribute.horizontal` | Edit: Distribute Evenly Horizontally | entities.align service supports all four edges and both distribution axes; Designer exposes left/horizontal controls | partial UI |
| `edit.distribute.vertical` | Edit: Distribute Evenly Vertically | entities.align service supports all four edges and both distribution axes; Designer exposes left/horizontal controls | partial UI |
| `edit.paste` | Edit: Paste | entities.paste; ID/type/link remapping | implemented |
| `edit.redo` | Edit: Redo | edit.redo | implemented |
| `edit.redoAlt` | Edit: Redo | Ctrl-Shift-Z → edit.redo | implemented |
| `edit.undo` | Edit: Undo | edit.undo; state identity restores dirty baseline | implemented |
| `file.new` | File: New | Desktop New + file.new; unsaved-change guard | implemented |
| `file.open` | File: Open... | Desktop Open + file.open | implemented |
| `file.printPreview` | File: Export... | SVG/PNG/vector PDF free-size focused graph; ISO/roll/poster sheets pending | partial |
| `file.quit` | File: Quit | Ctrl-Q; owned backend/LSP/jobs shutdown | implemented |
| `file.save` | File: Save | Desktop Save + file.commit_and_save / file.save | implemented |
| `file.saveAs` | File: Save As... | Desktop Save as + file.commit_and_save | implemented |
| `file.saveAsArchetype` | File: Save Model as Archetype... | Export → Save selected subtree + archetypes.save | implemented |
| `graph.deleteSelection` | Graph: Delete Selection | Delete; repository cascade | implemented |
| `graph.mode.link` | Graph: Link Mode | Designer Link | implemented |
| `graph.mode.node` | Graph: Node Mode | Designer Node | implemented |
| `graph.mode.select` | Graph: Select Mode | Designer Select / Escape | implemented |
| `graph.mode.type` | Graph: Type Mode | Designer Type | implemented |
| `graph.zoomExtents` | Graph: Zoom Extents | Designer Fit | implemented |
| `help.about` | Help: About | CLI help/version string; dedicated About dialog pending | partial |
| `help.mcpAgentGuide` | Help: MCP Agent Guide... | threadwork://agent-guide resource; dedicated guide dialog pending | partial |
| `mcp.manage` | MCP: Manage Server... | owned authenticated backend + MCP connection config; old management/log dialog parity pending | partial |
| `sheet.toggle` | Sheet: Toggle Preview | Retained in migration scope; no successful placeholder or enabled MCP command | pending |
