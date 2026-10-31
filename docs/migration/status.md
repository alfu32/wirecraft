# Current migration status

Wirecraft **0.6.3** ignores selected links during alignment/distribution and retains compact same-side feedback loops on the less crowded side (right on ties), preserves the 0.6.1 native diagonal routing and recovered inspector fixes, and retains the compiler/template sets and existing canvas, export and editor work. **The complete migration remains unfinished.** The authoritative source baseline is Threadworks 2.6.7 at `7d392d3a0e30b86445e00e7c03032a5188b26593`. The source checkout, its running application, live Wirecraft project/settings, and user changes to root `example.orch` / `example.copy.orch` were preserved. Changes use Conventional Commits; only patch/minor versions are authorized.

## Release matrix, licenses and Store listing (0.27.0)

At the user's request, Wirecraft has Meshcraft's release workflow and six-target artifact matrix (`.github/workflows/distribution.yaml`, `scripts/release/`, `packaging/`; details in [packaging/README.md](../../packaging/README.md)). A manual **Release** run at an existing `vX.Y.Z` tag produces 28 verified artifacts:
- Linux x86_64/aarch64: ZIP, tar.gz, AppImage, Snap, DEB, RPM, built in Ubuntu 18.04 against glibc 2.27 with every non-driver library bundled.
- Windows x64/ARM64: ZIP, tar.gz, MSI, and an MSIX with its public CER.
- macOS Intel/Apple Silicon: ZIP, tar.gz, DMG.

Adaptations from Meshcraft:
- **Executables:** each payload carries both `wirecraft-desktop` and the `wirecraft` command line, and both gained `--version` for the install checks.
- **Tags and toolchain:** tags keep the `v` prefix. The toolchain is pinned in the workflow, because a `rust-toolchain.toml` would also switch local rust-analyzer.
- **Validation runner:** validation runs on the current runner, since the tests need clang, clangd, Go and Node.js.
- **MSIX signing:** the MSIX carries the reserved Store identity (since 0.27.3) and is signed with a self-signed certificate for that publisher, created per run. There are no repository secrets.
- **Snap:** strictly confined; it cannot reach host compilers or language servers.

The workspace license is now `MIT OR Apache-2.0`, with `LICENSE-MIT` and `LICENSE-APACHE`. The icon PNG and ICO are drawn from `assets/wirecraft.svg`.

**Microsoft Store listing:** `packaging/microsoft-store` prepares an import for Store ID 9NDMQVXLGBQJ ([README](../../packaging/microsoft-store/README.md)):
- 32 locales, filled into the supplied Partner Center export.
- Ten 1920×1080 screenshots of the real 0.27.0 release build on the working WebServer examples. The design features come first, then project management, plan export and the MCP server.
- Nine artwork images composed from screenshot crops.
- `scripts/capture_store_features.py` runs every scene on a private Xvfb with software Vulkan, so its clicks and typing never reach the desktop session.

Verification:
- The release unit tests passed (20/20, including real DEB/RPM builds and the 28-artifact verifier, in a container with rpm tools).
- In the Ubuntu 18.04 container, a real x86_64 build produced both executables, a 23-file ELF bundle needing glibc ≤ 2.27, and portable archives. On this machine, the payload launchers ran both commands; a DEB from that archive installed, ran both commands and removed cleanly.
- The Store builder tests passed (12/12), as did `microsoft_store.py --check` and a headless render of `review.html`.

Pending:
- The Release workflow has not run on GitHub. AppImage, Snap, Windows and macOS packaging, and the ARM builds, are unverified.
- The Windows application itself has never been run.
- The Store needs confirmation of the product name and a privacy policy page.

0.27.1: the first Release run (v0.27.0) passed on Linux x86_64/aarch64, including the AppImage startup smoke, and on macOS Apple Silicon. On Windows ARM64, the installed MSIX refused to start `wirecraft.exe` (`Access is denied`), because only `wirecraft-desktop.exe` was declared in its manifest. The command line is now a second, unlisted console application with a `wirecraft.exe` alias.

0.27.2: the v0.27.1 run passed on Linux and both macOS targets. On both Windows targets MakeAppx rejected the manifest, because a console-subsystem application must also declare `desktop4:SupportsMultipleInstances="true"`; the command-line application now declares it.

0.27.3: Partner Center refused the 0.27.2 MSIX: the unlisted command-line application (`AppListEntry="none"`) is a headless app, which needs a waiver, and its PublisherDisplayName `Wirecraft contributors` was not the account's `K3D0`. The manifest now has one application, `wirecraft-desktop.exe`, with one alias extension holding the `wirecraft-desktop.exe` and `wirecraft.exe` aliases (the 0.27.3 attempt with a second alias extension for `wirecraft.exe` failed MakeAppx: an application declares `windows.appExecutionAlias` once). `wirecraft-desktop` runs the command line when started under the name `wirecraft`. The package carries Partner Center's Product identity: `K3D0.wirecraft`, publisher `CN=7F0CBC95-8035-4AF2-9B17-C38D58351CE5` and display name `K3D0` (family `K3D0.wirecraft_tfmtd4yrk952g`). Release CI now also runs both commands through their aliases. The v0.27.4 Release run passed on all six targets: on Windows x64 and ARM64 the installed `K3D0.wirecraft` package started from its install location and through both aliases, and `wirecraft help` printed the command-line usage. Partner Center acceptance of the 0.27.4 MSIX has not been checked here.

0.26.1 fixed the committed WebServer example, which broke CI: the component's `socket_accept` body lacked a semicolon after `http_resolve`, and `examples/webserver.orch` still pinned the component before its `listenerConfig` port.

## Identifier default names (0.26.0)

At the user's request, entities and links created without a name are named with valid, unique identifiers: the kind's base followed by the first counter no entity or link name uses (`processor1`, `Type1`, `packet1`, `group1`, `note1`, `node1`). Before, the canvas named them `New processor`, `New type` and `packet`, so names had spaces and repeated. Core `Document::default_name` picks the name; the service uses it when `create_entity`/`create_link` get no name (the MCP schemas no longer require one), and the canvas preview shows the same name before the click. Names the user types are kept as they are, and port names still come from their link. Verification: `tests/services.rs` `unnamed_entities_and_links_get_unique_identifier_names`. Pending: names typed by the user are not checked to be identifiers.

## Pinned generated header and component bodies (0.25.0)

At the user's request, the generated code now sits inside the editor as its unnumbered first rows, like the baseline's pinned header (`GridCodeEditorAdapter.setPinnedHeader`), instead of a separate labelled panel. It starts at the body's text column, past the line-number gutter, shares the editor's frame, background and focus border, and stays pinned while the body scrolls.
- **Body headers** are shown as the generator emits them: one line ending with the body's opening brace (C17 and PHP emit the brace on the next line, so ` {` is appended, as the baseline shows it). When that line is wider than the editor, `editor.signature`'s `wrapped` form puts one parameter per line and closes with `) {`. The choice follows the editor's width.
- **Component designs:** a design with Exterior ports now gets headers, Type declarations and link code. Editor generation (`components::stub_exteriors`) treats its Exterior placements as plain processors, so a published port is the buffer parameter it is in a program that references the component. The component itself still never generates on its own. The same stubbed design feeds the body context of completion, hover, colors and navigation in component designs; only the header path was tested.

Verification: `tests/signatures.rs` checks the generated lines and wrapped forms for C17 and the six template WebServers, and the header of a component body with its port buffer. `scripts/signature_smoke.py processor component` screenshots showed the Rust and C17 component headers pinned above line 1, aligned with the body and highlighted; the single-line form was not exercised in the window. Pending: signatures for Exterior and Reference entities themselves, function suffixes of component bodies (`_4`) that differ in the referencing program, and links whose consumer has no generated run function other than a library.

## Generated signatures, dropdowns and title bar (0.24.0)

At the user's request, the editor shows the generated code of the selected entity read-only, highlighted, above the body, for C17 and every template technology. Core `editor::signature` (backend `editor.signature`, MCP workbench `signature`) returns:
- **Processor and composite bodies:** the full generated `run_…`/`init_…` header with one parameter per line, without the body's brace; service libraries keep their file-scope comment. Before, a one-line header was shown for C17 and Rust only.
- **Types:** the generated declaration (the C17 `typedef struct`, or the block after a template's `// Name [id]` comment).
- **Links:** a data link's two port buffers and its transport function (the declaration block, not the scheduler's call); a library, source or runnable link shows the argument it gives its consumer, or says that library items share the file scope. Link code is regenerated on every compile, so it is shown read-only, and none of a link's sections feed the generators.

The link allocation names come from the generators' own helpers (`c17::wire_allocation`, `compiler::link_allocation`), so the shown block is the link's even when names repeat. A design that cannot generate (a component with Exterior ports, for instance) shows why in place. Snapshots of the same entity keep the shown code and its highlighter; a replaced highlighter used to leave identical text uncolored, which also affected the main editor after reloads and is fixed there.

Dropdowns (inspector, tables, editor language) now show their value left-aligned with the caret at the right, and their menu opens under the trigger exactly as wide as it, items aligned left. The toolbar shows the Wirecraft version and the project file name beside the root entity name, and the window title is `<root> — <file> — Wirecraft <version>`.

Verification: `tests/signatures.rs` checks headers, Types, data links, lent libraries and library-to-library links on the six template WebServers, C17 prototypes, typedefs and transports on the walkthrough fixture, the component error and the backend command. `scripts/signature_smoke.py` (screenshot-only; its `dropdown` case clicks once) showed the highlighted Rust header, Type and link code and a trigger-wide menu. Pending: signatures for Exterior and Reference entities themselves, and links whose consumer has no generated run function other than a library.

## Navigation in Go, Kotlin, JavaScript and PHP bodies (0.23.0)

At the user's request, Go to Definition and Find References also serve Go, Kotlin, JavaScript (Node.js, systemJS) and PHP bodies. `editor::body_language` names the language of every body with a context; `compiler_language` stays C17/Rust and still gates completion, hover, semantic colors and compiler checks. The Rust context became the template body context: it locates the body in the single-file program of any template set by the `wirecraft-source` markers, with per-language header, parameter, declaration, function-definition and instance-member rules. The Kotlin context drops the Gradle scripts, so the server does not resolve a classpath with Gradle; the Go context adds a `go.mod` for gopls. Template-language servers are ready when their work-done progress has ended. Each language has a configurable server (`gopls`, `kotlinLanguageServer`, `javascriptLanguageServer`, `phpLanguageServer`, with `WIRECRAFT_*` overrides and Options fields); gopls is also found in `$GOPATH/bin` or `~/go/bin`.

JavaScript and PHP library arguments are untyped, so their servers cannot follow calls through them. Core `BodyContext::model_definitions` / `model_references` resolve a call through a lent library argument (`.`, `->`, `::`, `?.`), a model argument and a modeled type or library instance, and list a library function's definition, its calls inside the library and every call through an argument lent by a dependency link in all consumers. They are merged with the server's locations (for definitions, only when the server found no authored or model location), and answer alone when the server cannot start (`serverError`; the desktop status bar says so).

Verification: `tests/template_navigation.rs` checks the contexts and the model navigation on the five WebServer examples by default, and the backend's answer without a server. With the real servers (opt-in), each language passes a local binding, the library call, the library argument and references across the library and `socket_respond`: gopls (from `~/go/bin`), kotlin-language-server 1.3.3, typescript-language-server 4 for Node.js and systemJS, and intelephense (installed into a temporary directory for the run). C17 clangd and Rust rust-analyzer navigation tests still pass. The desktop only changed its gate and status text; no native smoke was run for these languages. Pending: workspace symbol search, rename, and completion, hover and checks for these languages.

The native smokes no longer take the keyboard focus (0.22.1). They used to re-activate their window before every input, fighting the user's typing in other windows; that is how the unexplained `lib_http_server` draft of the 0.22.0 smoke appeared. They now pause until their window has the focus again and stop after three minutes (`scripts/smoke_focus.py`).

## Go to Definition and Find References (0.22.0)

At the user's request, C17 and Rust bodies support Go to Definition (F12, Ctrl-click, palette) and Find References (Shift-F12). `GeneratedAnalysis::locations` sends `textDocument/definition` / `references` to clangd or rust-analyzer and maps every reply through the pure `BodyContext::locate` in core:
- authored bodies go through the source maps;
- the edited body's generated header maps a model argument to its link or library;
- generated type and library-instance structs map to their entity;
- library members reached through the instance (Rust forwarding methods, C function-pointer fields) continue to the function in the lent library's text;
- everything else is reported as generated code, or external (std, system headers).

References are also requested at `BodyContext::related_positions`: the instance members and the library function's definition. So a library function and the calls through every lent instance are found together. The desktop moves the cursor within the edited body, reveals other entities as diagnostics do (the draft must be applied first), and lists references in a panel. Fixed on the way: a snapshot reloading a clean editor reset the cursor to the start even when the text was unchanged, losing any revealed position (diagnostic reveal included).

Verification: core mapping tests run by default (`tests/rust_editor.rs`). A real clangd test (`c17_workflows`, by default) covers a library member through the function pointer, the instance argument, `printf` as external, and references in the library and both consumers. A real rust-analyzer test (opt-in) covers the same for Rust, plus a local binding, runtime methods and std. `scripts/rust_editor_smoke.py` passed in the native window on the Rust WebServer:
- F12 on a local;
- Shift-F12 there;
- F12 from `socket_respond` into `lib_http_server` at line 208;
- the references panel listing the definition, the call in `socket_respond` (12:40) and the generated forwarders;
- Ctrl-click on `ReqRep` revealing the Type.

The panel capture was inspected. Some other window captures repeated a stale frame, so those steps were checked through the UI session and request traces instead. One earlier smoke run saw an unexplained draft appear in `lib_http_server` around Shift-F12/Escape: the smoke had taken the focus while the user was typing elsewhere (fixed in 0.22.1). Later runs assert that no draft exists at those steps. Navigation in other languages followed in 0.23.0.

## Rust body editing and rust-analyzer (0.21.0)

At the user's request, Rust bodies get the editor intelligence that C17 bodies have, through rust-analyzer. `editor::body_context` now also builds the Rust context: the single-file program from the Rust template set, located by the compiler's `wirecraft-source` markers, with the generated function header and the model symbols (`context`, link buffers with their direction and payload, lent library instances with their library). `editor::compiler_language` decides which bodies have a compiler language, for both the backend and the desktop. The generated-source analysis is language-aware: it stages the program and its `Cargo.toml` and starts rust-analyzer with its server-status, progress and Markdown-hover capabilities, then waits until the server is quiescent. Completion, hover and semantic tokens are remapped exactly as for clangd. Completion items are narrowed to the typed prefix, because rust-analyzer leaves filtering to the client. Inline diagnostics use rust-analyzer's pull diagnostics. The graph check (`editor.analyze_generated`, the desktop's background compiler check) saves the program, waits for rust-analyzer's `cargo check` to finish (including restarted checks) and maps the rustc diagnostics to bodies. Unchanged staged files are no longer rewritten; a rewritten manifest made rust-analyzer reload the workspace and cancel the check. Library forwarding methods now repeat the functions' `///` comments, so hovering `math.maximum` shows the documentation. The desktop registers tree-sitter-rust, colors rust-analyzer's type kinds and adds a rust-analyzer executable option (`rustAnalyzer` in settings, `WIRECRAFT_RUST_ANALYZER`).

Verification: `tests/rust_editor.rs` checks the Rust body context by default. With real rust-analyzer 1.99.1 (opt-in, `--include-ignored`), it checks completion of library members, runtime methods and buffers, hover documentation and provenance, semantic tokens, native diagnostics, and `cargo check` errors mapped to body line and column; five consecutive runs passed. The C ticker editor tests still pass. `scripts/rust_editor_smoke.py` passed in the native window, and its screenshots were inspected. It covers Rust syntax and semantic colors, the generated header, Ctrl-Space insertion of `is_running`, a cargo-check issue after Apply, its clearing after Undo, and the owned rust-analyzer processes stopping on close. Hover was verified through the backend only; the native hover popup was not exercised. Pending: navigation requests (definition/references), Rust file-based layouts, and components in Rust designs.

## Rust generator (0.20.0)

At the user's request, Wirecraft has a Rust code generator: technology `rust`, single-file. Threadworks has no Rust template set, so it is a hand-written set in `resources/compiler-templates-native/rust/`, outside the byte-identical baseline. The output is one dependency-free source file plus `Cargo.toml`; library instances are unit structs whose methods forward to the library's functions. `examples/rust.orch` is the hello example. The WebServer is transcoded into Rust and passes the same HTTP exchange; it idles at 0 % CPU, and SIGINT/SIGTERM exit 0. The tests use `rustc` by default, so they are not ignored. Contract and pending scope are in [compiler-templates.md](compiler-templates.md#rust-template-set-wirecraft-addition-0200).

## systemJS (0.19.0)

At the user's request, the QuickJS template set is renamed systemJS, because it emits engine-neutral scripts, and it is tested on Bun. Stored `quickjs` technologies still select it. Its runner now yields to host timers and handles `process` signals. The WebServer is transcoded into systemJS too and passes on Bun. Details are in [compiler-templates.md](compiler-templates.md#webserver-transcodings-wirecraft-viability-test-0160).

## WebServer transcodings and template generator fixes (0.16.0–0.18.0)

At the user's request, `examples/WebServer.orch` (C17) was transcoded into the template technologies to test the generators on a real network server. Go, Kotlin, Node.js, Node.js on Bun, and PHP all pass the same HTTP exchange as the C17 example (`tests/transcoded_webservers.rs`). QuickJS has no socket API; at the user's direction the JavaScript server runs on Bun instead. PHP 8.4.26 and QuickJS 2025-04-26 were built locally, from checksum-verified source, for the optional tests.

Generator defects found and fixed:
- template composite boundary channels (single-file layouts);
- Go library imports;
- the synchronous Node.js runner, under which sockets, timers and SIGTERM never ran;
- the Node.js and QuickJS runtime being redeclared by nested composites;
- missing PHP `global`s for outer-composite buffers;
- the Go/PHP drain trace on stdout.

Details are in [compiler-templates.md](compiler-templates.md#webserver-transcodings-wirecraft-viability-test-0160).

## Sheet preview blocks (0.15.0)

At the user's request, the canvas Sheet preview also outlines the title block and the parts list, labelled, where the technical plan places them on the assembled sheet. `sheets::plan` returns both blocks in world units (`world_blocks`) from the same layout that draws them, so the canvas and the export cannot drift apart. Page contours now use explicit dash lengths like the other dashed contours. Verified by `graph_workflows.rs` (title block in the bottom-right corner and parts list top right, 5 mm inside the page) and `scripts/sheet_blocks_smoke.py` (native screenshot inspected).

Correction: v0.14.0 was pushed with one strict-Clippy error in `tests/components.rs` (`cloned_ref_to_slice_refs`); the check output had been truncated. It is fixed here.

## Placement previews, routed link construction and link rewiring (0.14.0)

At the user's request:
- Node, Type and Port modes draw the future contour under the pointer.
- A link under construction is routed together with the other links. It shows its would-be target, or the port that releasing on empty canvas would create.
- A selected link's ends can be picked up and dropped on another entity, or on empty canvas to make a new port (`links.reconnect`, one undo step).

Details and verification are in [canvas-interactions.md](canvas-interactions.md#placement-previews-routed-links-under-construction-and-link-rewiring-0140). The WebServer end-to-end test still fails only because of the pending user edit to `examples/webserver-component.orch` described below.

## Ports in composites, Port mode and IO marking (0.13.0)

User-requested follow-ups to components:
- **Port mode** (toolbar **Port**, palette **Port Mode**) places an unlinked Exterior that a later link names and publishes. An unlinked port is a warning, not an error.
- **Ports inside composites.** An Exterior may sit in any composite, and the Link-mode gesture works in the focused composite. While the composite is collapsed, its ports are drawn beside it with their links, instead of disappearing.
- **IO marking.** **IO role** in the inspector, or **Mark Selection as IO-in / IO-out / IO-in/out** and **Clear IO Marking** in the Graph commands, stores `io: "in" | "out" | "inout"` as an extension field. It is shown as an amber badge and a dashed amber outline, on the canvas and in SVG/PDF plans. Generation ignores it. A `null` patch value now removes an extension field instead of storing `null`.
- **Dashed contours.** GPUI's dashed border scales its dashes with the 1px stroke and read as a solid line. Expanded composites, IO entities and the selection bounds now stroke explicit dash and gap lengths that continue around corners.
- Plans now draw Exteriors as circles.

Verification: `tests/components.rs` adds a nested-port component program (`got 42`), the collapsed placement, and IO marking with its SVG badge. Native screenshots of the IO badges, the expanded and collapsed composite, and Port mode were inspected. The WebServer end-to-end test currently fails at HEAD. This comes from the user's in-progress `examples/webserver-component.orch` edit (`ec49850`), which changes the pinned hash and adds a `listenerConfig` port, not from this change.

## Pluggable components (0.12.0)

A Wirecraft extension specified with the user in [docs/design/components.md](../design/components.md). A design publishes ports by linking entities to **Exterior** placements, drawn as 73-unit circles. Each placement has one link, takes its name from it (`<link>Out`/`<link>In`) and shows a chevron along the flow. Another design places a **Reference** pinned by SHA-256; its links name the published ports, and inputs accept fan-in. `components::expand` copies the component into the consumer and joins every port with its consumer links into one channel (no extra tick) inside `C17::generate`. Template compilers reject both explicitly.

The WebServer example is split into `examples/webserver-component.orch` (accept/respond transport) and `examples/webserver.orch` (router and routes). The end-to-end HTTP test runs against the consumer, and 5000 simultaneous clients each received 200. New: MCP `wirecraft.components` (`publish`, `reference`, `update_reference`), **Graph → Insert/Update Component Reference**, the Link-mode publish gesture and `wirecraft update-references FILE`. Verified by `tests/components.rs` (validation, pinning, path containment, a fan-in program printing `sum 42`) and `scripts/components_smoke.py` (native screenshots inspected). Pending items are listed in the specification.

## WebServer example (0.11.0)

At the user's request, the open WebServer project was finalised through the live Wirecraft MCP (`127.0.0.1:39589`). The root entity specifies configuration, routes, request flow, string ownership, limits, execution model, shutdown and access log; every entity and the new links carry their own specification, and the Tests sections list acceptance checks. The model was completed: typed `response`/`awaitResponseLoop` links, `Response` status/content type, the `ReqRep` connection, explicit `lib` links, a `platform` library link and a `routeNotFound` fallback. All bodies are implemented; the project was saved by `wirecraft.save_project` (a pre-edit backup was kept), and a copy without the local user entry is `examples/webserver.orch`.

Through the MCP, `compile_c17` and a `wirecraft.build` clang job produced a binary with no warnings under `-std=c17 -Wall -Wextra -pedantic`. It served the three routes and the 404/405/400 cases, and HEAD returned headers only. Twenty concurrent clients each got their own responses. A request split across segments was assembled, a 9 KiB header received 431, an early disconnect was survived, and an occupied port failed startup. `WEBSERVER_MAX_REQUESTS` ended the run with exit 0. `wirecraft.analyze_c17` timed out waiting for clangd diagnostics on this unit, so no editor-diagnostic result is claimed.

Defects found and fixed: C17 header-derived library members matched substrings (`<sys/time.h>` implied `<time.h>`), and service libraries were emitted in ID order instead of dependency order. The MCP agent guide still described prefixed library aliases, and `wirecraft.build` advertised `argv` instead of `args`. A new `webserver_example_…` integration test compiles the example with `-Werror` and drives real HTTP requests.

## Library instances (0.10.0)

At the user's request, a dependency link's name is now the name of an injected library instance rather than a function prefix. Every library declares one shared instance of its top-level functions. This is a C struct of function pointers, a Go struct, a Kotlin object, a frozen JS object or a PHP anonymous class. Each consumer receives it as an argument named after its link. The rules, per-language table, overlay mechanism and C compatibility aliases are in [compiler-templates.md](compiler-templates.md#library-instances-wirecraft-adaptation-0100). The baseline templates remain unchanged; adaptations are generated by `scripts/template_overlay.py`.

Verification: the workspace suite passed (**98 Rust tests, 4 toolchain tests ignored by default**). All 11 template-compiler tests then passed with real Go, Node.js 22, Kotlin 2.2.0/Java 21 (both Kotlin tests later also passed with 2.4.21), PHP 8.4.11 and QuickJS 2026-06-04, covering eight language/layout cases of one library lent under two link names. Go and Kotlin also cover file layouts (Node direct file system, Kotlin source set, PHP direct file system). A strict clang C17 build runs the shared instance from a nested composite, and clangd completes `math->`. The supplied ticker still builds through the compatibility aliases. Formatting and strict Clippy passed. Native UI, packaging and installed smoke were not rerun.

## Desktop fixes, auto-sized boxes and per-side padding (0.7.0–0.9.0)

User-reported fixes: saving without a selection no longer panics (`file.commit_and_save` compared two missing ids as equal); the active mode is highlighted; Sheet toggles page contours on the canvas; menubar dropdowns open on press under their label, left-aligned and without a backdrop, and toggle on a second press; the entity tree uses boxed `+`/`−`. 0.7.0 itself failed to start (duplicate GPUI hover style) and was corrected in 0.7.1. 0.8.0 adds auto-sized boxes and composite padding, with align/distribute on the projected boxes; 0.9.0 makes padding per side, with one handle per corner; rules are in [canvas-interactions.md](canvas-interactions.md). Workspace tests, formatting and strict Clippy pass. `scripts/menu_smoke.py` passed natively on the ticker and WebServer fixtures and `scripts/padding_smoke.py` passed three consecutive runs, with screenshots inspected. Packaging/installed smoke and the older native suites were not rerun.

## C source capabilities (0.7.0)

`src` links in C17 now generate the baseline `link-declaration.peb` facade: a `<Link><n>Capability` struct holding the provider declaration as an escaped string literal and a `getSource` function that calls the runtime `threadwork_materialize_source` to replace `${name}` markers with `threadwork_build_parameter` values. The symbol index follows `dependencyLinkIndex` (position among the provider's lib/src/run links). As in `CCompiler.generatedFunctionHeader`, only the consumer receives `<Type> *<link>`; the provider signature and data-link buffers are unchanged and no transport buffers are allocated. Composites pass `&<dependency>` to the consuming child. Literals are emitted line by line, escaping quotes, backslashes, controls and `?` (so C17 trigraphs cannot alter the product). Origin maps for authored bodies are preserved; the literal itself is not a mapped editable range. `run` links fail with the baseline type-safety reason, and source links must originate in a declaring entity and target an executable C entity.

The consumer's editor context lists the capability with its generated type; real clangd completes `->getSource`, hover reports the facade, and a wrong `getSource` call maps back to the consumer body line. An isolated fixture compiles with clang `-std=c17 -Wall -Wextra -Werror` and prints the materialized provider text (Unicode, quotes, backslash, `??`, tab). Workspace: **92 Rust tests passed, 2 optional toolchain tests ignored**; formatting and strict Clippy passed. Packaging and installed smoke were not rerun for this change.

## Box-only align/distribute (0.6.3)

All four edge alignments and both distribution operations filter links in the shared application service before counting boxes, checking parents or calculating bounds. Mixed selections work even when a selected link belongs to another parent or has extreme stored coordinates. Responses list only operated-on boxes; link data remains untouched. Alignment requires two boxes, distribution three; insufficient box selections fail atomically with explicit box-count messages. The operation remains one undo transaction. Desktop and API use the same service, without a project format change.

Seven relevant workflow integration tests passed, covering all six layout operations and undo. Formatting, workspace strict Clippy, optimized packaging and installed backend/MCP/PDF/C17/clangd smoke passed. An isolated real HTTP fixture also passed all six mixed-selection commands, unchanged links, box-only response IDs and undo. Native UI and the full workspace suite were not rerun in this patch. [0.6.3 verification](evidence/v0.6.3/verification.json).

## Same-side feedback loops (0.6.2)

Both self-link ports now share the side with fewer existing side ports, measured before adding either port; right wins ties. Provider/type callouts count on the right, while top dependency flags do not count. Subsequent feedback pairs include earlier pairs in their occupancy counts. Each pair stays adjacent, with input above output, and routes locally between those heights instead of around the box. Original endpoint symbols, 46-unit leads and priority over ordinary channels remain. Desktop and SVG/PDF use this shared geometry without changing project data.

Verification: **90 Rust tests passed, zero failed or ignored**, including optional real toolchains; formatting and strict Clippy passed. The optimized installed archive passes backend/MCP/PDF/C17/clangd and native canvas/follow-up checks. An initial X11 menu-text capture failure passed on retry; the final exact-viewport native captures were visually checked. Commands, logs and binary hashes: [0.6.2 evidence](evidence/v0.6.2/verification.json).

## Router and recovered-edit correction (0.6.1)

The earlier router was H/V search with short diagonal cuts, not a faithful migration of the JVM route selection. Native diagonal candidates now use the original Euclidean length/bend/crossing rules; ten pure Kotlin route outputs and four original port templates match. A bounded eight-direction visibility search prefers clear routes; when none fits, best-effort geometry is drawn without routing-error labels, as requested. Self-links reserve compact local envelopes and first ports before other channels. Original hollow output/filled notched input caps, 46-unit horizontal stubs, and PCB-style hollow via pairs with gray backside connectors share desktop/vector geometry. Composite ports and dependency callouts remain intact. See [routing.md](routing.md) for deliberate differences, fixture provenance and checks.

The newly reported locked-content incident was inspected read-only: the live graph was at revision 0 with no committed rename, while the saved UI session contained a name-only draft for the selected link. The JVM's `applyOnCommit` handles inspector Enter/focus loss; the Rust name input was missing both. Name edits now commit on Enter/blur; other inspector text fields support Enter. A persistent draft strip in every view exposes Apply, Discard and Show editor, and Ctrl-Enter works from Designer. Real drafts still restore and concurrent-edit protections remain. The old live instance and its draft were not changed. This evidence explains this specific guard lock, not every previously reported intermittent pointer/startup incident.

Native isolated checks pass for name Enter/blur, restart with a real body draft, Apply from Designer, recovered-name Discard and subsequent tree selection. The final workspace run passed **88 Rust tests, zero failed or ignored**, including the optional real compiler/toolchain tests. Formatting and strict Clippy passed; upstream widget dependency warnings remain. Ten routing outputs and four port templates were regenerated from the recorded Kotlin source without changes to the fixture. Native canvas, five startup scenarios, three repeated editor-to-designer release cycles and follow-up editor/menu/export/routing checks passed. The optimized archive passes installed backend/MCP (37 tools), PDF/C17/clangd and native editor/canvas/startup/release/follow-up/draft checks. Results and binary hashes are recorded in [verification](evidence/v0.6.1/verification.json); historical release checks below retain their original scope.

## Compiler/template milestone

The six original template sets (C, Go, Kotlin, Node.js, PHP, QuickJS; 90 hashed resources) are embedded. Shared `compile.project`, `compiler.templates`, `compiler.export` services are connected to desktop Generate, CLI and MCP workbench. Inspector technology choices and canonical filesystem layout IDs are restored. Portable compiler bundles replace generated JVM compiler classes; model role overrides, project files and literal static companions are supported outside the still-pending C template path.

Real Go/Node/Kotlin/PHP/QuickJS producer → typed FIFO → consumer → shutdown fixtures execute successfully. Node/PHP single and filesystem layouts and Kotlin single/source-set layouts are covered. CLI custom-template export/reuse and real HTTP MCP catalog/generation/export pass. Rendering rejects malformed templates, bounds instructions/output, and generation rejects unsafe/duplicate paths and unresolved scope connections. C bundle execution/unsupported overrides, generic composite boundary channels and mixed-language delegation return explicit errors. See [compiler-templates.md](compiler-templates.md) for the exact retained contract and examples. Language generation does not imply broader inline LSP/coloring parity.

0.6.0 checks: **81 Rust tests passed, zero failed or ignored**, with the optional real toolchains explicitly enabled; formatting, workspace build and strict Clippy passed. The default test invocation leaves two optional toolchain tests ignored unless requested. All 90 assets match the exact baseline git objects. CLI/API/MCP compiler workflows pass. Native Generate produced and executed the Node.js example once; the full new native suite did not pass because Wayland/isolated GTK picker observation was inconsistent, so native compiler-export picker completion remains unverified. The previous 0.5.0 native suites below are historical, not a claim about this new picker flow. The optimized Linux archive passes installed backend/MCP/PDF/C17/clangd checks. Its standalone binary also compiles and executes all five new examples outside the checkout and exports/reuses a portable bundle. Details: [verification](evidence/v0.6.0/verification.json).

## Corrective milestone

- Window resize hitboxes occupy actual gutters, with current viewport dimensions and tiled-edge handling. Window-level move/release capture finishes gestures despite child hitboxes; unheld-pointer movement, focus loss and Escape release stale gestures. Real drafts retain their protections. The exact original intermittent startup freeze has not been reproduced; five startup scenarios and repeated editor-to-designer dragging are verified, without claiming a single proven cause for that incident.
- Pointer-centered wheel zoom, middle drag pan, directional windows, Shift-add / Control-remove / replace, tree selection/reparenting, shared selection contours/bounds and one-undo group/subtree movement remain connected. Right-click opens all application commands in a scrollable, occluding menu with nested archetype insertion. Wheel events no longer pass through that menu to the canvas.
- Data/error routing allocates ports by straight-connection angle, chooses the nearer box side, reserves distinct channels, avoids boxes and uses explicit aligned composite boundary ports. Native 45° candidates participate in Euclidean route selection; hollow via pairs and muted gray backside segments distinguish unconnected crossings. Right-to-left links are dashed. Shared output outlines, filled input arrows, channel/type labels and sharp boxes render in canvas and vector exports. Library/source/runnable/type capability links retain right-side provider boxes and top consumer flagpoles; `includes_lib` has four distinct dependant boxes and `write_append_file` two incoming flags.
- The C/C++ grammars are registered explicitly. C17 semantic colors, completion signatures and function/struct hover use a full generated translation unit, runtime, model types, connected libraries and every companion file. Real clangd positions are remapped to the authored body with Unicode, exact-text and revision guards. A real staged header supplies a type/function in an integration test. Hover shows emitted struct declarations and injected-library signature/provenance. Automatic intelligence runs outside the client command queue; semantic responses do not wait for separate diagnostics while holding the editor session.
- Compiler errors **and warnings** produce red exclamation badges. Info is excluded. Independent background generated-source checking updates a clickable status issue count; the full list includes messages and source locations, and mapped rows reveal the failing body/line. Existing compile errors in supplied projects are shown rather than treated as migration failures.
- Plan preview defaults to the entire assembled drawing with page contours, overlap alignment crosses, folding marks, one global parts list and one title block. Individual sheets clip the same vector composition. Positive world coordinates no longer create blank origin pages. External `export.preview` clients retain the tile default; `overview:true` requests the assembly. Typography and every roll-paper detail are not claimed pixel-identical to Swing.
- All **42** supplied screenshots have numbered descriptive names, feature mappings and preserved SHA-256 hashes. Native evidence is under `evidence/v0.5.0/`.

## Established application behavior

The framework-independent core, shared authenticated loopback API/events/MCP, headless executable and GPUI desktop are connected. Current/legacy `.orch`, unknown/binary data, atomic persistence, repair, revision conflicts, undo/redo, recovery and draft/session restoration work. MCP retains 19 legacy tools, five resources and two prompts, plus 18 extensions. The UI includes compact inspector dropdowns/inline tables, Kanban/backlog, archetype preview/insertion, sparse P/E/D/U matrices, options/configuration import, CUPS discovery/explicit jobs and vector plan/document export. Linux minimize/maximize/restore/close controls are verified.

Typed C17 generation includes nested composites, runtime buffers, modeled structs, library aliases, self-feedback, lifecycle, origin maps and literal companions. Array expressions use the existing erased `ThreadworkArray` runtime value. Non-emitting links' legacy filesystem-layout metadata no longer prevents C body generation; stored metadata is unchanged. External clang builds/runs isolated programs. Generic LSP services retain capability negotiation, synchronization, coordinate conversion, diagnostics, navigation/completion/symbol requests, cancellation, stale-result/crash/restart handling and owned-process cleanup. TCC, its DLL integration and JAR execution are excluded.

## Reference and compatibility evidence

The JVM MCP is reachable again at the user-supplied `http://127.0.0.1:8765/mcp`. Read-only initialization/listing/entity/validation/analysis/catalog calls recorded 19 tools, 37 commands, a structurally valid 22-entity WebServer, and a missing-entity failure. The Rust fixture matches network counts and link interaction/transport/layout metadata. Evidence: `baseline/jvm-live-2026-10-27/`. No JVM command execution, project mutation, Start/Stop/Restart, or settings change was sent. MCP exposes model contracts, not popup/hover rendering, routing geometry, pointer state or the compiler UI overlay; source and photographs establish those behaviors.

The immutable supplied ticker has 30 entities/seven generated files. Its `gen_uuid` body ends in unfinished `pu`: tests assert that mapped error, then correct only a temporary copy to execute and produce CSV. Embedded foreign binaries/legacy launchers are never executed. The old ticker PDF embeds a JPEG on its first page despite the intended vector/PostScript drawing contract. Wirecraft preserves vectors directly through SVG into PDF, rather than retaining PostScript as an intermediate.

## 0.5.0 verification (historical)

- `cargo fmt --all -- --check`: passed.
- `cargo test --workspace --locked`: **71 passed**, none skipped: 22 core integration, six desktop geometry, 43 runtime integration tests. Includes actual HTTP/stdio MCP, real clangd/clang processes, companion-header AST resolution, Unicode/range mapping, hover/signatures, conflicts/failures/cancellation/restart/cleanup, shared plan assembly and no-overlap/no-box-entry WebServer routing.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed. The vendored upstream widget still emits its existing dependency deprecation warnings; application lint checks are clean.
- Native development checks passed: five startup cases; three editor-to-designer release cycles; full canvas gestures/draft preservation; completion insertion/undo/stale dismissal/Apply; inspector fields, table recovery, panes/theme, board, archetypes, analysis, export, MCP controls and window controls. Compiler colors, complete struct/injected-function hover, issue-list navigation, context scrolling/nested insertion and assembled multipage/WebServer rendering were exercised and visually inspected.
- Optimized archive creation and installed headless API/MCP/PDF/C17/native-program/clangd checks passed. Final installed native suite results and binary hashes are recorded in `evidence/v0.5.0/verification.json`.
- X11/XWayland/Vulkan on Linux was exercised. Xvfb/Xephyr lack DRI3 here. Windows/macOS, clean-image dependency installation and physical printer output remain unverified. Projection/culling measurements in older evidence are not new-router GPU frame-rate claims.

Build/run, isolated state directories, clangd/toolchain settings and MCP client configuration are in [README](../../README.md). The current live instance keeps its old executable; launch the new binary with a distinct `--state-dir` to try it alongside that session.

## Remaining migration and exact continuation

Main gaps: C template overrides and C bundle execution; remaining parameterized collection constructors; full Go/Kotlin/JavaScript/PHP/QuickJS parity including template composite boundary channels, capability variants and aggregate mixed-language delegation; broader-language inline navigation/completion and multicursor; OAuth identity; editable canvas paper overlays; exact legacy typography/Markdown print fidelity. The JVM plugin ABI is deliberately incompatible, while stored plugin data survives. Network AI/browser authorization were unavailable stubs in the baseline; clipboard assistance works. Already-issued hover/completion requests may finish after dismissal, with stale results rejected; generic LSP request cancellation is available. Extremely crowded/overlapping layouts can exceed the bounded routing search and receive visible best-effort geometry rather than a hidden link or routing-error label.

Next functional slice: **C template overrides / bundle execution**, using `../threadworks/compilers-impl/src/main/kotlin/com/threadwork/compiler/c/CCompiler.kt` with the retained `compiler-templates/c/*.peb` roles, so model `@…` overrides and portable C bundles render through the same typed C17 contract instead of returning `unsupported`. Then expand template boundary channels, capability variants, collection types and mixed-language dispatch. Keep these requirements open in parity.md; Neither release claims full migration completion.
