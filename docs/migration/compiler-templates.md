# Compiler and template migration

Baseline: Threadworks 2.6.7, `7d392d3a0e30b86445e00e7c03032a5188b26593`. Its compiler implementations are in `../threadworks/compilers-impl/src/main/kotlin/com/threadwork/compiler/`; the generic role/assembly contract is `generic/CompilerTemplates.kt`, with traversal in `../threadworks/compiler-api/src/main/kotlin/com/threadwork/compiler/CompilerApi.kt`.

The 90 original text resources are retained under `crates/wirecraft-core/resources/compiler-templates/`. `PROVENANCE.json` records each SHA-256. They are embedded at build time; the installed program does not depend on the original checkout. No TCC, DLL or JAR implementation is imported. Python archetypes exist in the baseline, but a dedicated Python graph compiler/template set does not.

| Set | Stored language / technology | Source default | Current implementation |
|---|---|---|---|
| C | `c` / `c-native` | single-file | Existing typed Rust C17 emitter with `src` capability facades and shared library instances; `run` capabilities are rejected as in the baseline; C template overrides still pending |
| Go | `go` / `go` | single-file | Template graph assembly, runner, FIFO transports and source capability templates |
| Kotlin | `kotlin` / `kotlin-jvm` | source-set | Single-file and file-based assembly; Gradle/runtime/entry point project files |
| Node.js | `javascript` / `nodejs` | direct-file-system-homomorphism | Single/file-based CommonJS modules, runtime and package manifest |
| PHP | `php` / `php` | direct-file-system-homomorphism | Embedded templates and graph assembly; representative interpreter execution verified |
| systemJS (formerly QuickJS) | `javascript` / `systemjs` (`quickjs` accepted) | single-file | Engine-neutral scripts from the baseline QuickJS templates; tested on Bun, also verified on QuickJS and Node.js |
| Rust (Wirecraft addition, no baseline) | `rust` / `rust` | single-file | Wirecraft-authored set in `resources/compiler-templates-native/rust/`: one dependency-free source file plus `Cargo.toml`; see [Rust template set](#rust-template-set-wirecraft-addition-0200) |
| Custom | any / `compiler-template-set` | classified-filesystem | Model `@…` role overrides and portable JSON template bundles |

The initial 0.5.0 release had only C17 and literal exports. These additions form the 0.6.0 milestone. Asset availability is not a claim of complete language parity. Cross-composite routing/code interfaces, all collection expressions, capability variants, mixed technologies and language-specific editor intelligence require additional tests. The generic LSP service already accepts explicit external servers, but only C/C++ has the fully connected generated-body editor adapter.

## Library instances (Wirecraft adaptation, 0.10.0)

At the user's request, Wirecraft deliberately diverges from the baseline here. A service library declares **one shared instance** of its top-level functions. Each dependency (`lib`) link lends that instance to its consumer as an argument **named after the link**, so a link `math` gives a body `math->maximum(2, 4)` (C), `$math->maximum(…)` (PHP) or `math.maximum(…)` (others). Several links from one library, even with different names, receive the same instance. The baseline instead used compiler-generated prefixed aliases (`math__maximum`) in C/PHP, package-scope calls in Go and a separate per-link `math1` value in Kotlin/Node/QuickJS.

| Set | Instance (`lib_math` library) | Consumer argument |
|---|---|---|
| C (typed C17) | `typedef struct Lib_mathLibrary { int (*maximum)(int, int); } …; static const Lib_mathLibrary lib_math_library` | `const Lib_mathLibrary *math` |
| Go | `type Lib_mathLibrary struct { maximum func(…) … }`, `var lib_math_library = &Lib_mathLibrary{…}` | `math *Lib_mathLibrary` |
| Kotlin | `object Lib_mathLibrary { val maximum = … }` (function references taken at file scope); with library Instantiation text, `val lib_math_library: lib_math = <instantiation>` as in the baseline | `math: Lib_mathLibrary` |
| Node.js / QuickJS | `const lib_math_library = Object.freeze({ maximum })`, or the library's Instantiation expression; file-based Node modules export it and callers use `require(…).lib_math_library` (one cached module) | `math` |
| PHP | `$lib_math_library = new class { public function maximum(mixed ...$arguments): mixed { return \maximum(...$arguments); } }`, or the Instantiation expression | `mixed $math` |
| Rust | `struct Lib_mathLibrary;` (a `Copy` unit struct) with forwarding methods `fn maximum(self, left: i64, right: i64) -> i64 { maximum(left, right) }`, `static lib_math_library: Lib_mathLibrary`; with Instantiation text, `type Lib_mathLibrary = lib_math` and a `LazyLock` of the expression, as in Kotlin | `math: &Lib_mathLibrary` |

Only top-level functions are exported. Overloaded or generic Kotlin functions, Go methods/generics and nested C statements are skipped. In C, `#include`d `stdio.h`/`time.h`/`uuid/uuid.h` add their supported functions as members, as the former aliases did. Library-to-library links share file scope and inject nothing, and an exporter is emitted only for libraries lent to an executable entity. **Compatibility:** C bodies that still call the former `link__function` aliases keep compiling; those aliases are generated only for bodies that use them (the supplied ticker relies on this). The C editor context lists the instance with its type and member signatures; clangd completes `math->`.

The 90 baseline templates stay byte-identical. The adaptation lives in `crates/wirecraft-core/resources/compiler-templates-wirecraft/`, which takes precedence when a built-in set loads, so exported bundles carry it too. It is generated from the baseline by exact replacements in `scripts/template_overlay.py`, which fails if baseline text is missing: injected library arguments are typed in Go/Kotlin and no longer filtered out in Go/PHP; per-link library values are removed; Kotlin/Node/QuickJS gain a service-library role at file scope, and Kotlin composites stop invoking library children. Instance code comes from `wirecraft_core::libraries`.

## WebServer transcodings (Wirecraft viability test, 0.16.0)

At the user's request, `examples/WebServer.orch` (the single-file C17 HTTP server) is transcoded into each template technology to test the generators on a real network. `scripts/transcode_webserver.py` keeps the C17 topology: the same entities, IDs, links, `routes` composite, layout and IO marks. Only the technology, the bodies (`examples/webserver-transcoded/<language>/<entity>.<section>.<ext>`) and the type field types change. Field types become the neutral `string`/`number`, with nested types kept as references. The script writes `examples/webserver-<language>.orch`.

`tests/transcoded_webservers.rs` runs the same exchange as the C17 example test for each design:
1. Generate the project and check that every authored section maps to its generated lines.
2. Build it with the real toolchain.
3. Send the three routes, 404, 405 with `Allow`, a 431 rejection with unread bytes, and HEAD.
4. Check that the server exits 0 after its fifth resolved request, with the access log.

| Language | Status | Generator changes it required |
|---|---|---|
| Go | Passes (Go 1.25) | Composite boundary channels in single-file layouts; library `import` declarations hoisted into the file's import block (lines stay as comments, source maps shift) |
| Kotlin | Passes (Kotlin 2.4.21, Java 21; `--include-ignored`) | None beyond the boundary channels. Bodies rebuild the immutable data classes with `copy`; `base_lib` mirrors the C platform headers with `typealias` declarations, because imports must precede every declaration |
| Node.js | Passes (Node.js 22.17) | Asynchronous runner that yields to the event loop between ticks, with idle back-off and `threadworkRunner.wake()`; runtime emitted once per single file (nested composites redeclared `ThreadworkRunner`) |
| PHP | Passes (PHP 8.4.26 built locally: `--disable-all --enable-cli --enable-pcntl`; `--include-ignored`, `WIRECRAFT_TEST_PHP`) | Composites declare `global` for every buffer their children use (a child fed by an outer composite's link received an undefined local, a `TypeError`); drain trace opt-in on stderr |
| Bun | Passes (Bun 1.3.5; `--include-ignored`, `WIRECRAFT_TEST_BUN`) | None: Bun runs the Node.js set's CommonJS output unchanged, including the asynchronous runner, SIGTERM and the 1 % idle CPU |
| systemJS | Passes on Bun 1.3.5 (`--include-ignored`, `WIRECRAFT_TEST_BUN`) | Renamed from QuickJS at the user's request (see below); the runner yields between ticks where the host has timers, and handles SIGINT/SIGTERM through `process.on`. The bodies take sockets from `node:net`, which plain QuickJS lacks |
| Rust | Passes (rustc 1.99, edition 2021, `-D warnings`; runs by default) | New set (see [Rust template set](#rust-template-set-wirecraft-addition-0200)). C `int` fields stay integers (`i64`) instead of becoming `number`. `std` has no accept timeout, so a listening thread hands connections to `http_wait` over a channel (`recv_timeout`): 0 % idle CPU, about 0.3 ms per request, SIGINT/SIGTERM exit 0 |

**Composite boundary channels.** A data link whose ends sit in different composites, such as `router` → `routes/routeHomeAbout`, is declared and transported by its owning composite. In a single-file layout its buffers are file-scope symbols, so the nested end receives them like any other link. File-based layouts still return an explicit `unsupported` error for such links, since one file cannot see another's buffers.

**Node.js runner (Wirecraft overlay adaptation 2).** The baseline runner was a synchronous `while` loop, so no socket, timer or signal handler ever ran. SIGTERM could not stop a generated program, and a server could never accept a connection. The runner is now `async`:
- after a tick that moved packets it continues on `setImmediate`;
- consecutive idle ticks wait 1, 2, 4 … up to 25 ms, the C17 example's `poll` timeout;
- `threadworkRunner.wake()` ends such a wait as soon as an event handler has work.

Measured on the Node.js WebServer: idle CPU fell from 12 % (with a fixed 1 ms wait) to 1 %. A request arriving at an idle server was answered in 4.2 ms on average (maximum 36 ms over 20 requests), and SIGTERM exits 0. Bodies that do work without moving packets now run at most 40 ticks per second while idle.

**PHP composite boundaries (overlay adaptation 4).** A PHP function sees a file-scope buffer only after declaring it `global`. Composites declared only the links they own, so a route inside `routes`, fed by a root-owned link, received an undefined local: `TypeError: … must be of type array, null given`. Composites now declare `global` for every buffer their children use.

**Runtime trace (overlay adaptation 5).** The Go and PHP runtimes printed a trace line on every shutdown-drain tick to stdout, which for the WebServer is its access log. As in the C17 runtime, the trace is now opt-in (`WIRECRAFT_TRACE`) and goes to stderr.

**systemJS (overlay adaptation 6, 0.19.0).** At the user's request the QuickJS set is called systemJS. Its output is plain ECMAScript: the only host-specific code feature-detects QuickJS's `os.signal`, so one script runs on QuickJS, Bun and Node.js. Compatibility details:
- Stored projects with technology `quickjs` select it unchanged, and so does `TemplateSet::builtin("quickjs")`.
- The 90 baseline assets keep their `quickjs/` directory and stay byte-identical; `examples/quickjs.orch` is now `examples/systemjs.orch`.
- Where the host has timers (Bun, Node.js, Deno), the runner yields between ticks like the Node.js runner, including the idle back-off and `wake()`. Plain QuickJS has no timers, so it still runs ticks back to back.
- Shutdown signals also use `process.on` where it exists. Before this, Bun ignored SIGTERM because the set only knew QuickJS's `os.signal`.
- The template tests run systemJS on Bun (`WIRECRAFT_TEST_BUN`, default `bun`) instead of `qjs`.
- The systemJS WebServer idles at 1 % CPU on Bun and exits 0 on SIGTERM.

**Regression test.** `template_compilers.rs` adds a producer at the root feeding a consumer inside a nested composite, executed in every language: Go and Node.js by default; Kotlin, PHP and systemJS (on Bun; QuickJS 2025-04-26 built from Debian's verified orig tarball was used before the rename) with `--include-ignored`. It also checks that file-based layouts refuse such a link with an explicit error. With the old PHP composite template it fails with the `TypeError` above.

**Single-file runtime (overlay adaptation 3).** Nested composites render the same single-file assembly as the root, so the Node.js and QuickJS runtime classes were declared once per composite. That is a syntax error in any single-file design with a nested composite. The runtime is now emitted for the compilation root only. PHP was unaffected because its runtime is wrapped in `class_exists` guards.

**Language adaptations in the bodies.**
- C `static` counters become entries of the run context's import map.
- Library functions return values instead of filling C out-parameters, because PHP library instances forward `...$arguments`, which cannot pass references.
- Go has no `poll()`: `http_wait` accepts under a deadline and hands the connection to the next `http_accept`.
- PHP type instances are objects, so `socket_accept` deep-copies a request (`unserialize(serialize(…))`) before sending it down two paths. The library uses core streams only: `stream_socket_server`, `stream_select` as `poll`, and a 2 s read timeout.
- Node.js sockets are event driven: the library reads each connection completely between ticks, or rejects it, and `http_accept` takes the next complete request. Because JS values are references, `socket_accept` copies a request before sending it down two paths, as C does by value.

## Rust template set (Wirecraft addition, 0.20.0)

At the user's request Wirecraft has a Rust generator. Threadworks has no Rust template set, so this one is written by hand rather than adapted. It lives in `crates/wirecraft-core/resources/compiler-templates-native/rust/` (embedded under `native/`, outside the baseline and its generated overlay), uses the same roles and context as the Go set, and the catalog reports no `sourceRevision` for it. Contract of the generated program:
- **One file, no crates.** `<Root>.rs` builds with `rustc --edition 2021 <Root>.rs` (2024 also compiles cleanly), or with the generated `Cargo.toml` (an empty `[workspace]` keeps it out of an enclosing one). The file starts with `#![allow(...)]` for unused, naming and unreachable-code lints, since bodies and link names follow the model.
- **Nodes** are `fn init_<x>(context: &mut ThreadworkContext, <links>) -> ThreadworkResult` and `fn run_<x>(…)`. Bodies are statements that end before the template's `Ok(())`; `?` works (`ThreadworkError` is `Box<dyn std::error::Error>`, so `.ok_or("text")?` and `return Err(….into())` work). `use` declarations are allowed inside bodies.
- **Links.** Data link buffers are file-scope statics, `ThreadworkBuffer` (a `Mutex<VecDeque<Box<dyn Any + Send>>>`), passed as `&ThreadworkBuffer`. `push(buffer, value)` moves a value in (clone it to send it down two paths), `pop::<T>(buffer) -> Option<T>` leaves a packet of another type queued, and `len()`/`is_empty()` inspect the queue. Each tick moves at most one packet per link, as in the other sets.
- **Self-links** give `<link>_in` and `<link>_out`. Link and field names that are Rust keywords become raw identifiers (`r#type`), and `self`/`super`/`crate`/`Self` get a trailing `_`.
- **State that C bodies keep in statics** lives in the run context: `context.state::<T>("key")` returns a `&mut T`, created with `T::default()` on first use.
- **Types** become `#[derive(Clone, Debug, Default)] pub struct`s. Field types: `string`/`char *` → `String`, `number`/`double` → `f64`, `float` → `f32`, `int`/`date` → `i64`, `boolean` → `bool`, `void *` → `Option<Arc<dyn Any + Send + Sync>>`, and references → `Option<Box<T>>`.
- **Runner.** `threadwork_runner` (`shutdown_request`, `is_running`, `get_shutdown_signal`, the bounded shutdown drain) stores only atomics. SIGINT/SIGTERM are installed through the C library's `signal`; the handler stores atomics only. The drain trace is opt-in (`WIRECRAFT_TRACE`) on stderr, as in Go and PHP.
- **Libraries** share the file's module scope. Repeated single-line file-scope `use` declarations stay in the file as `// use …; (duplicate)` comments, so line numbers and source maps are unchanged. Instance methods forward column-0 functions with their generics, `where` clauses, `unsafe` and parameter patterns; `async`/`extern` functions, `main` and `impl` members are not exported.

Library forwarding methods repeat the `///` comments of the functions they forward to. Rust bodies are edited with rust-analyzer (see the README's language intelligence section). Pending: file-based layouts (the set is single-file only, like Go) and `run` capabilities (rejected, as in Go). `tests/template_compilers.rs` runs the producer/consumer, two-link library, nested-composite and library-signature graphs with `rustc` by default (`WIRECRAFT_TEST_RUSTC` overrides it), and checks the raw-identifier and duplicate-import handling.

## Shared entry points

- Desktop **Generate → Generate Project (effective technology)** uses the project's inherited `technologyId`. Select a new/empty destination; output is staged atomically. **Export Compiler from Model Overrides** exports a portable bundle.
- `wirecraft templates` lists the seven sets; `wirecraft templates go` prints its complete roles/project templates.
- `wirecraft compile PROJECT.orch OUTPUT_DIRECTORY` selects by inherited technology. Optional `--template-set go` and `--layout single-file` select explicitly.
- `wirecraft export-compiler PROJECT.orch NEW_DIRECTORY nodejs` exports `compiler.wirecompiler.json`, including model overrides. Consume it with `wirecraft compile PROJECT.orch NEW_OUTPUT --compiler compiler.wirecompiler.json`.
- API `compile.project`, `compiler.templates`, `compiler.export` expose the same services. `wirecraft.workbench` MCP operations are `compile_project`, `compiler_templates`, `export_compiler`, with the service parameters inside `arguments`. Existing `wirecraft.compile_c17` and literal-export contracts stay available.

Generation never invokes downloaded tools. Build/run through the existing supervised external jobs or your shell: `go build`, `node path/to/root.js`, `kotlinc SOURCES -include-runtime -d app.jar` then `java -jar app.jar`, `php root.php`, `bun run root.js` for systemJS (engine-neutral scripts also run on `qjs` or `node`), or `rustc --edition 2021 -O root.rs` / `cargo build` for Rust. Kotlin's retained Gradle template pins Kotlin 2.2.0 and JVM 21. The Kotlin tests use `kotlinc` from PATH or `WIRECRAFT_TEST_KOTLINC`; they pass with Kotlin 2.2.0 and 2.4.21 on Java 21. Kotlin 1.9.0 fails on Java 21 (`ArrayDeque.addLast` resolves to JDK 21's sequenced-collection API) even for baseline output.

## Portable bundle and compatibility

`TemplateSet` is a JSON data contract containing set identity, language/technology, extension, default layout, role sources, project files and static filenames. It replaces the original generated JVM compiler-class/plugin ABI. The Rust application evaluates the data directly; it does not load Java classes or run a generated compiler binary. Model fields and unknown stored data remain unchanged.

MiniJinja is the rendering implementation ([official documentation](https://docs.rs/minijinja/latest/minijinja/)); a compatibility layer maps Pebble `elseif`, its empty test, permissive undefined variables and legacy `${identifier.path}` placeholders. Every original `.peb` parses in tests. Arbitrary Pebble extensions/macros from third-party JVM plugins are not promised compatible. Rendering has an instruction budget and a 16 MiB output writer limit. Generated projects reject unsafe/duplicate paths, excessive output and unresolved external connections on subtree compilation. Exported bundles contain source and should be reviewed like model templates.

Kotlin assembly explicitly includes channel declarations in single-file roots and imports generated runtime symbols in file-based nodes. These are Rust planner corrections around the unchanged reference templates. Source maps record authored source/initialization ranges. Errors from templates and missing build tools remain failures, never hard-coded success.

## Verification

The first connected tests exercise producer → typed channel → consumer → shutdown through external Go, Node.js and Kotlin processes, both Node layouts and both Kotlin layouts. PHP/QuickJS rendering, override/bundle round trips, template errors and all original template syntax have separate checks. The external suite was also run with real PHP 8.4.11 and a locally built QuickJS 2026-06-04. Exact final commands/results are recorded in status.md.

C bundles can be inspected/exported, but execution returns `unsupported` until C template parity is implemented. The default typed C17 path rejects requested non-single layouts and model overrides it cannot apply. Template composite-boundary data channels and foreign nested technologies likewise return explicit unsupported errors. Earlier Wirecraft inspector aliases `direct-filesystem` and `classified` are accepted on input; the UI now writes the original canonical identifiers.


Manual desktop verification: open `examples/nodejs.orch` in an isolated `--state-dir`, choose Generate Project, select a new output path and run `node OUTPUT/hello_nodejs.js`. Then choose Export Compiler from Model Overrides, select another new directory and use its JSON bundle through the CLI example above. Check inspector Technology choices without changing a live project. One native Generate/execution succeeded here; repeated full dialog automation did not pass because GNOME's Wayland picker is outside X11 automation and a private GTK portal was intermittent. Native bundle-picker completion remains unverified; the corresponding real CLI/API/MCP path passed.

The optimized Linux archive was unpacked into a temporary directory and all five `examples/` programs compiled/executed there, followed by Node bundle export/reuse. Installation includes those examples under `share/doc/wirecraft/examples`. At that time the Go/PHP reference runtimes still printed their shutdown-drain trace to stdout. Since 0.18.0 that trace is opt-in, as in C17: set `WIRECRAFT_TRACE` and it goes to stderr (overlay adaptation 5).
