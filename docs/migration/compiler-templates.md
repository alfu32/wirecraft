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
| QuickJS | `javascript` / `quickjs` | single-file | Embedded templates and graph assembly; representative interpreter execution verified |
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

Only top-level functions are exported. Overloaded or generic Kotlin functions, Go methods/generics and nested C statements are skipped. In C, `#include`d `stdio.h`/`time.h`/`uuid/uuid.h` add their supported functions as members, as the former aliases did. Library-to-library links share file scope and inject nothing, and an exporter is emitted only for libraries lent to an executable entity. **Compatibility:** C bodies that still call the former `link__function` aliases keep compiling; those aliases are generated only for bodies that use them (the supplied ticker relies on this). The C editor context lists the instance with its type and member signatures; clangd completes `math->`.

The 90 baseline templates stay byte-identical. The adaptation lives in `crates/wirecraft-core/resources/compiler-templates-wirecraft/`, which takes precedence when a built-in set loads, so exported bundles carry it too. It is generated from the baseline by exact replacements in `scripts/template_overlay.py`, which fails if baseline text is missing: injected library arguments are typed in Go/Kotlin and no longer filtered out in Go/PHP; per-link library values are removed; Kotlin/Node/QuickJS gain a service-library role at file scope, and Kotlin composites stop invoking library children. Instance code comes from `wirecraft_core::libraries`.

## Shared entry points

- Desktop **Generate → Generate Project (effective technology)** uses the project's inherited `technologyId`. Select a new/empty destination; output is staged atomically. **Export Compiler from Model Overrides** exports a portable bundle.
- `wirecraft templates` lists the six sets; `wirecraft templates go` prints its complete roles/project templates.
- `wirecraft compile PROJECT.orch OUTPUT_DIRECTORY` selects by inherited technology. Optional `--template-set go` and `--layout single-file` select explicitly.
- `wirecraft export-compiler PROJECT.orch NEW_DIRECTORY nodejs` exports `compiler.wirecompiler.json`, including model overrides. Consume it with `wirecraft compile PROJECT.orch NEW_OUTPUT --compiler compiler.wirecompiler.json`.
- API `compile.project`, `compiler.templates`, `compiler.export` expose the same services. `wirecraft.workbench` MCP operations are `compile_project`, `compiler_templates`, `export_compiler`, with the service parameters inside `arguments`. Existing `wirecraft.compile_c17` and literal-export contracts stay available.

Generation never invokes downloaded tools. Build/run through the existing supervised external jobs or your shell: `go build`, `node path/to/root.js`, `kotlinc SOURCES -include-runtime -d app.jar` then `java -jar app.jar`, `php root.php`, or `qjs root.js`. Kotlin's retained Gradle template pins Kotlin 2.2.0 and JVM 21. The Kotlin tests use `kotlinc` from PATH or `WIRECRAFT_TEST_KOTLINC`; they pass with Kotlin 2.2.0 and 2.4.21 on Java 21. Kotlin 1.9.0 fails on Java 21 (`ArrayDeque.addLast` resolves to JDK 21's sequenced-collection API) even for baseline output.

## Portable bundle and compatibility

`TemplateSet` is a JSON data contract containing set identity, language/technology, extension, default layout, role sources, project files and static filenames. It replaces the original generated JVM compiler-class/plugin ABI. The Rust application evaluates the data directly; it does not load Java classes or run a generated compiler binary. Model fields and unknown stored data remain unchanged.

MiniJinja is the rendering implementation ([official documentation](https://docs.rs/minijinja/latest/minijinja/)); a compatibility layer maps Pebble `elseif`, its empty test, permissive undefined variables and legacy `${identifier.path}` placeholders. Every original `.peb` parses in tests. Arbitrary Pebble extensions/macros from third-party JVM plugins are not promised compatible. Rendering has an instruction budget and a 16 MiB output writer limit. Generated projects reject unsafe/duplicate paths, excessive output and unresolved external connections on subtree compilation. Exported bundles contain source and should be reviewed like model templates.

Kotlin assembly explicitly includes channel declarations in single-file roots and imports generated runtime symbols in file-based nodes. These are Rust planner corrections around the unchanged reference templates. Source maps record authored source/initialization ranges. Errors from templates and missing build tools remain failures, never hard-coded success.

## Verification

The first connected tests exercise producer → typed channel → consumer → shutdown through external Go, Node.js and Kotlin processes, both Node layouts and both Kotlin layouts. PHP/QuickJS rendering, override/bundle round trips, template errors and all original template syntax have separate checks. The external suite was also run with real PHP 8.4.11 and a locally built QuickJS 2026-06-04. Exact final commands/results are recorded in status.md.

C bundles can be inspected/exported, but execution returns `unsupported` until C template parity is implemented. The default typed C17 path rejects requested non-single layouts and model overrides it cannot apply. Template composite-boundary data channels and foreign nested technologies likewise return explicit unsupported errors. Earlier Wirecraft inspector aliases `direct-filesystem` and `classified` are accepted on input; the UI now writes the original canonical identifiers.


Manual desktop verification: open `examples/nodejs.orch` in an isolated `--state-dir`, choose Generate Project, select a new output path and run `node OUTPUT/hello_nodejs.js`. Then choose Export Compiler from Model Overrides, select another new directory and use its JSON bundle through the CLI example above. Check inspector Technology choices without changing a live project. One native Generate/execution succeeded here; repeated full dialog automation did not pass because GNOME's Wayland picker is outside X11 automation and a private GTK portal was intermittent. Native bundle-picker completion remains unverified; the corresponding real CLI/API/MCP path passed.

The optimized Linux archive was unpacked into a temporary directory and all five `examples/` programs compiled/executed there, followed by Node bundle export/reuse. Installation includes those examples under `share/doc/wirecraft/examples`. The unchanged Go/PHP reference runtimes still print their shutdown-drain trace messages; these are retained behavior, unlike the earlier C17 opt-in trace correction.
