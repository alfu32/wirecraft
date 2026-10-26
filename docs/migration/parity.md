# Feature parity matrix

Status: pending = scoped, not implemented; partial = real subset; verified = listed scope checked. Source paths below are relative to Threadworks and abbreviate Kotlin package roots with `...`.

| Feature/workflow | Source implementation | Evidence | Rust component | Verification | Status / gap |
|---|---|---|---|---|---|
| .orch current and legacy, repair, binary/unknown data | storage-json/.../JsonDocumentStore.kt; core/.../model/Model.kt | source + document.orch/archetypes | core model/store | fixture round trips, malformed data | partial: list/map/unknown/binary round-trip, repair; broader source fixture corpus pending |
| Entity/link CRUD, hierarchy, typed ports | storage-json/.../DocumentRepository.kt | source tests + live read shapes | core graph/service | mutation invariants, cycles, cascade, self-links | verified: CRUD/link/reparent/cascade/self-link tests; broader metadata rules pending |
| inheritance/types/classification/validation | core/.../model,classification,validation | source tests + live validation | core | inheritance/type fixtures | pending |
| Designer selection/drag/create/link/pan/zoom/collapse | app-desktop/.../ui/ThreadworkDesktopApp.kt | source; screenshot of empty designer | desktop canvas | geometry tests + manual smoke | pending |
| Efficient canvas and routing | same file canvas/tile/route implementations | source 512px LRU + cached routes | desktop scene | large graph timing/culling | pending |
| Source/spec/tests/initialization/usage editor | editor/GridCodeEditorAdapter.kt | active source | desktop editor | edit/save/reopen | pending; multicursor/completion/highlight parity |
| File commands, clipboard, align, undo, palette | ui/ThreadworkDesktopApp.kt registerCommands | live catalog + source | app services/desktop | workflow smoke | pending |
| Session/options/autosave | ui/*Settings.kt; editor/*Settings.kt | Java Preferences + 10s timer source | runtime config/desktop | restart fixture | pending |
| MCP 19 tools, five resources, two prompts | mcp-server/.../ThreadworkMcpService.kt | baseline captured live | runtime MCP/service | real HTTP and stdio clients | pending |
| MCP management/logging/lifecycle | McpServerController.kt; McpServerDialog.kt | screenshot + health | runtime/desktop | independent backend lifecycle | pending |
| C/C++ language intelligence | completion-core; TinyCcSourceValidator.kt | source, TCC intentionally excluded | runtime LSP clangd | real diagnostics/sync/restart | partial: real clangd sync, diagnostics, symbols/completion, encoding/restart/cleanup checked; editor connection pending |
| Other language intelligence | completion-core + compiler APIs | source | LSP configurations | explicit capabilities/errors | pending |
| external builds/jobs/cancellation | compiler-api + app Main.kt | source | runtime jobs | tool success/error/cancel | verified on Linux: jobs success/failure/cancel/timeout; generation separate |
| Graph-to-source compilation/layout/source maps | compiler-api; compilers-impl | source + tests | generation services | old/new compiler fixtures | pending |
| C/Go/Kotlin/JS/PHP/QuickJS/generic compiler | compilers-impl templates/classes | live technologies + source | generation adapters | compiled outputs | pending |
| Archetype browse/import/save | builtin-archetype; WorkflowArchetypesPanel.kt | stored .orch fixtures | core/service/desktop | import ID remap | pending |
| Network analysis/findings/actions | core/.../analysis; NetworkAnalysisPanel.kt | source; live MCP counts only | core analysis/desktop | graph fixtures | pending |
| Project status/users/assignee/responsible/history | core model; ProjectManagementPanel.kt | source + live user count | core/service/desktop | history and identity merge | pending |
| OAuth provider identity | identity/DesktopOAuthClient.kt | source only | future identity adapter | isolated auth integration | pending; no credential import |
| Sheets/title/parts/ISO/roll/SVG/PNG/PDF/tiling | SheetLayout, PreprintDialog, FopPdfWriter | source + tests | export service/desktop | golden geometry + exports | pending |
| AI fiches/prompts/source/test assistance | ai/AiSupport.kt | source + live commands | app service/desktop | prompt fixtures/clipboard | pending |
| JVM compiler and desktop plugins | ServiceLoader and desktop plugin APIs | source | explicit incompatibility | preserve pluginData, docs | JVM ABI intentionally unsupported |
| TCC DLL/embed | TinyCcSourceValidator; lib/tinycc-embed.jar | source | clangd + external tool | no TCC deps | excluded by user |
| packaging/install/settings migration | release.yml, snapcraft.yaml, Java Preferences | source only | scripts + CI | temporary install/Linux smoke | pending; other platforms unverified |
