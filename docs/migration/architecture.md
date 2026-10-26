# Architecture decisions

## ADR 001 — Rust services with a GPUI client

Choose GPUI 0.2.2 (pinned) with gpui-component 0.5.1. Official documentation checked 2026-10-26: https://gpui.rs/ and https://docs.rs/gpui/0.2.2/gpui/. GPUI provides GPU paths/quads/text, retained entities, custom canvas and background tasks. Its pre-1.0 API and Linux system dependencies are accepted constraints. Alternatives checked: eframe (https://docs.rs/eframe/latest/eframe/) offers native/web immediate UI and paint callbacks; iced (https://docs.rs/iced/latest/iced/) offers typed update/view and task/subscription APIs. GPUI matches the requested direction and editor/canvas needs; none eliminates the need for custom graph geometry. Windows and macOS builds remain separate acceptance gates.

`wirecraft-core`: transport/framework-free typed graph, rules, repository operations, compatibility codecs, atomic project storage. Unknown document/node/section fields round-trip. Domain mutations operate on a candidate snapshot and commit atomically only after validation. Stable IDs, topology repair, immutable reads, undo and redo.

`wirecraft-runtime`: application service owns a single document/session, serializes commands, revision checks, HTTP backend, event subscriptions, MCP adapter, LSP/tool supervision. Desktop and MCP call the same command dispatcher. File and subprocess work stays off the UI thread. DTOs are JSON envelopes around typed document data; transport errors do not live in domain entities.

`wirecraft-desktop`: GPUI shell, local HTTP client on a worker, immutable snapshots, canvas and editor state. Pointer motion updates local preview; one backend mutation on release. Render primitives are culled in world coordinates; nodes/annotations use uniform cells and immutable route segments use a BVH; projection/routes/indexes cache per document revision. No per-project raster disk cache. Viewport transforms are tested separately. World geometry is f64, screen geometry f32. Hierarchy navigation gives a focused subgraph. Full obstacle-aware routing and GPU frame budgets remain separate parity gates.

## ADR 002 — Local lifecycle and contract

Backend binds IPv4 loopback on an OS-selected port by default, avoiding the running old server. Desktop owns its backend process, passes an ephemeral bearer token, waits for readiness, and shuts down only that child. Standalone backend prints one JSON discovery record with endpoint/token; do not publish it. No network bind, implicit reverse proxy or database service. Reject browser Origin requests. HTTP API `/api` receives `{command, arguments, expectedRevision?}`; response `{ok, result?, error?:{code,message}, revision}`. `/events` carries revision/progress events; reconnect reads a full snapshot. Concurrent stale writes return conflict. Save failures retain dirty state. Health and reads never mutate projects.

MCP `/mcp` preserves threadwork.* names and threadwork:// resources where supported, with explicit errors for unsupported desktop-only commands. Tool responses include textual and structured JSON and isError. stdio supports newline JSON-RPC with protocol-only stdout. Stateless HTTP notifications return 202, absent GET event stream returns 405. Unknown methods -32601, malformed requests -32600/-32700, invalid arguments -32602; service errors remain structured. No transport duplicates business mutations.

## ADR 003 — Language/tool adapters

An LSP adapter owns one subprocess and pipes; Content-Length framing, initialize/initialized, server capability reporting, workspace URI, UTF-16 negotiation by default (support negotiated UTF-8/UTF-32), monotonic document versions, didOpen/full-text didChange/didClose, asynchronous diagnostics, stale version rejection, generic requests and $/cancelRequest. Handle configuration/workspace/progress/registration server requests with explicit replies. On missing executable or crash, return structured unavailable/process errors. Explicit restart creates a new generation; old pending requests fail. Shutdown sends shutdown/exit, times out then kills and waits; Drop is a final kill guard. No shell invocation. clangd discovers compile_commands.json through workspace or explicit --compile-commands-dir. clangd diagnostics are editor feedback, external compiler/build execution is a distinct service. C/C++ first; other server executable/args/language IDs are configuration data, not a plugin system.

Build jobs have IDs, cancellation, timeout, bounded output and progress. They execute argv in explicit working directories; process groups on supported Unix platforms must prevent child leaks. Compilation generation parity is distinct from invoking build tools and remains tracked.

## Compatibility

Keep .orch extension, current list/legacy map, field names source/initialization and Base64 payload. Preserve unknown fields. Reject duplicate IDs and cycles rather than silently lose entities. Import Java Preferences from an explicit exported XML into Wirecraft's own config, never edit old preferences or automatically import provider secrets. JVM plugins, TCC and TinyCC native integration are incompatible by design; graph data is retained. All other gaps remain pending, not declared deliberate removals.

## Release procedure

Commit small verified milestones with Conventional Commits; push configured origin. Tag only verified milestone releases with SemVer. Only patch/minor increments are authorized; major changes require the user to set the version explicitly.

## ADR 004 — Portable exports and explicit configuration import

Diagram SVG is generated from domain geometry; raster/PDF conversion lives in runtime adapters. resvg 0.45.1 and svg2pdf 0.13.0 were checked against official API documentation (https://docs.rs/resvg/0.45.1/resvg/ and https://docs.rs/svg2pdf/0.13.0/svg2pdf/). Free-size export remains available. ISO/roll/poster geometry now lives in core sheets; documentation layout in core documentation. Multipage output uses PDF with real physical page dimensions. Vector text is searchable; explicit raster PDF uses compressed RGB images on white. PNG/raster allocation is limited to 40 megapixels per page, raster PDF to 160 megapixels total. Sheet planning has 256-page/32 MiB SVG limits. Preview rasterization is bounded to 1200 pixels per side, independent of print resolution.

Built-in archetypes are copied from the recorded source revision and embedded in the backend binary. Clipboard and archetype import share ID/type/link remapping. Literal file generation has explicit source mappings, rejects unmigrated graph technologies and unsafe/duplicate paths, stages output atomically and refuses nonempty output directories. Language wrapper/template generation is still a separate pending capability.

Settings import reads a user-supplied Java Preferences XML export into a new Wirecraft JSON file. It activates supported indentation, theme/fonts, designer colors, wheel and composite size settings, and archives other panel values. It excludes identity/AI preference nodes, does not resolve an external DTD, and never edits the old preference store. Runtime executable/argv settings remain explicit and transport-neutral.

## Lifecycle verification refinements

Dirty state uses unique history-state identities so status polling is constant time and undo can return precisely to the saved baseline. Revision publication and recovery also run when an editor commit succeeds but its subsequent save fails. Recovery failures annotate successful results instead of inviting duplicate mutations. SSE streams terminate on backend shutdown; tool shutdown begins before HTTP drain completes. Linux child processes carry parent-death guards. LSP pending guards cancel abandoned requests, and document versions reject stale request results as well as stale diagnostics. Stdio is serialized; use HTTP for concurrent LSP request/cancel exchanges.

## ADR 005 — C17 graph generation and mapped analysis

`core::c17::C17` emits a single portable C17 unit from typed graph operations. The reusable C queue/runner ABI is retained as a generated-language asset with documented transport-count/capacity corrections and opt-in stderr tracing. Assembly phases are explicit: runtime → payload/model types → libraries/function-pointer aliases → buffers/transports → prototypes → processor/composite definitions → entry point and cleanup. Unsupported capabilities fail before output is written. A shared virtual-file/source-map representation records authored lines and generated indentation offsets; templates for other languages can use that same boundary without a general plugin framework.

`runtime::GeneratedAnalysis` owns a separate temporary clangd workspace/session for the generated unit. It synchronizes versions, rejects results when the model revision changes, and maps diagnostics through source ranges. It shares C17 generation logic with CLI/API/MCP, while desktop only presents results and navigation. UTF encoding conversion is explicit because GPUI input cursor columns count Unicode scalars. Raw generated diagnostics remain available for unmapped runtime/assembly lines. Analyzer shutdown removes the owned temporary workspace after stopping clangd.

The authenticated backend initially enables MCP and provides independent Start/Stop/Restart controls. Stopping MCP returns HTTP 503 there while the UI API stays connected. A 256-entry metadata-only access log excludes bodies and credentials. This differs from the original off-by-default control; the token and loopback/Origin boundary protect the unified local service. Wirecraft never manages the old running server.

## ADR 006 — Screenshot-driven workbench and shared drawing

The twenty-two photos are indexed chronologically with original hashes. They establish view structure and visible controls; Kotlin source and runtime evidence establish semantics. The GPUI workbench retains recognizable views and five editor sections, but uses neutral light/dark surfaces, teal interaction accents and a consistent 24-unit stroke icon vocabulary. Font assets retain their SIL OFL. No generated bitmap icon dependency is introduced.

`core::drawing` projects expanded/collapsed topology once for both desktop and exported plans: nodes, ports, link segments, type usage and visible endpoint projection. Segment-level culling avoids large empty link bounding boxes. Corridor trials are bounded; full legacy routing fidelity is not claimed. Matrix adjacency remains sparse, rows virtualized and columns paged. Degree/SCC/components cover the full graph; all-node reachability is explicitly bounded above 4096 nodes to avoid quadratic interactive work.

Editor state is staged independently of snapshots. Only changed top-level properties enter patches; structured tables preserve unknown data and validate duplicate keys before mutation. A correlated commit fingerprint protects text typed while an earlier Apply is in flight. Backend response revision determines whether to advance that baseline; unrelated canvas/status operations cannot do so. Invalid tables persist as drafts rather than being discarded. Tracking is read-only, with root master revision handled as a document field rather than blindly copied into the node.

Options is a staged client form over validated runtime settings and atomic persistence. Network inference adapters remain explicitly unavailable, matching source stubs; clipboard tasks use shared core prompt generation. Printing is explicit: discover CUPS queues, render an owned temporary PDF and launch a supervised `lp` job; retain the temporary file until the job exits. No print is sent merely by previewing or selecting a printer. The Sheet toolbar opens unified export preview rather than the old editable canvas paper overlay; that fidelity gap remains documented.
