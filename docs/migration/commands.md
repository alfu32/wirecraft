# Command parity

Source command catalog was captured from the running 2.6.7 application; implementations are in `app-desktop/.../ui/ThreadworkDesktopApp.kt` (`registerCommands`, `registerAiCommands`). The desktop dispatcher is broader than the headless `execute_command` catalog. Headless clients use the corresponding named MCP tools or shared API commands; UI-only commands return an explicit limitation.

| Source IDs | Wirecraft behavior | Status |
|---|---|---|
| `ai.copyComponentFiches` | AI Support: Copy Selected Component Fiches; shared `export.markdown` | Working Markdown dossiers |
| `ai.generateSource`, `ai.generateTestData`, `ai.tidySpecification` | AI Support copies the corresponding task prompt with dossiers and available compiler context; configured network providers report unsupported | Source clipboard behavior migrated; source network implementations were stubs |
| `app.options` | Staged General/Designer Colors/Inference options, Save/Cancel/defaults; settings.json and explicit XML import | Working supported preferences |
| `commands.palette` | Ctrl-Shift-P, categorized search, Up/Down/Enter/Escape | Working |
| `compile.compiler` | Compiler from model overrides | Pending; no successful placeholder |
| `compile.project` | Generate C17 topology or literal files; shared API/MCP, real external compilation | Partial: other graph compilers and remaining C/templates pending |
| `edit.align.bottom/left/right/top` | Edit menu and palette; shared `entities.align` | All four operations connected |
| `edit.distribute.horizontal/vertical` | Edit menu and palette; shared `entities.align` | Both axes connected |
| `edit.copy/cut/paste` | Menu/shortcuts and ID-safe graph fragments | Working |
| `edit.redo`, `edit.redoAlt`, `edit.undo` | Menu, Ctrl-Y/Shift-Z/Z; shared undo history | Working |
| `file.new/open/save/saveAs` | Native path workflows, shared services, unsaved-change/revision guards | Working |
| `file.printPreview` | Export Plan/Documentation with real preview, ISO/roll/tiling/PDF All and explicit CUPS printing | Working listed scope; exact legacy print fidelity unverified |
| `file.quit` | Ctrl-Q; owned backend/LSP/jobs shutdown | Working |
| `file.saveAsArchetype` | Save subtree with stable file format and fresh IDs on import | Working |
| `graph.deleteSelection` | Delete/menu, shared cascade | Working |
| `graph.mode.link/node/select/type` | Toolbar/menu/palette, Escape selects | Working |
| `graph.zoomExtents` | Fit / palette | Working |
| `help.about` | About dialog with version and baseline | Working |
| `help.mcpAgentGuide` | Scrollable/copyable guide identical to MCP resource | Working |
| `mcp.manage` | Dedicated view, config, independent Start/Stop/Restart/health/access log | Working; loopback bearer authentication added |
| `sheet.toggle` | Sheet toolbar/palette toggles unified Export preview | Deliberate UI redesign; editable on-canvas paper overlay remains pending |

The model, file, generation, report, language-server and job services remain accessible headlessly even when their desktop presentation differs. Do not interpret this table as full compiler or platform parity.
