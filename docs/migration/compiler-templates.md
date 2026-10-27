# Compiler and template migration

Baseline: Threadworks 2.6.7, `7d392d3a0e30b86445e00e7c03032a5188b26593`. Its compiler implementations are in `../threadworks/compilers-impl/src/main/kotlin/com/threadwork/compiler/`; the generic role/assembly contract is `generic/CompilerTemplates.kt`, with traversal in `../threadworks/compiler-api/src/main/kotlin/com/threadwork/compiler/CompilerApi.kt`.

The 90 original text resources are retained under `crates/wirecraft-core/resources/compiler-templates/`. `PROVENANCE.json` records each SHA-256. They are embedded at build time; the installed program does not depend on the original checkout. No TCC, DLL or JAR implementation is imported. Python archetypes exist in the baseline, but a dedicated Python graph compiler/template set does not.

| Set | Stored language / technology | Source default | Current implementation |
|---|---|---|---|
| C | `c` / `c-native` | single-file | Existing typed Rust C17 emitter; source/runnable capabilities and C template overrides still pending |
| Go | `go` / `go` | single-file | Template graph assembly, runner, FIFO transports and source capability templates |
| Kotlin | `kotlin` / `kotlin-jvm` | source-set | Single-file and file-based assembly; Gradle/runtime/entry point project files |
| Node.js | `javascript` / `nodejs` | direct-file-system-homomorphism | Single/file-based CommonJS modules, runtime and package manifest |
| PHP | `php` / `php` | direct-file-system-homomorphism | Embedded templates and graph assembly; representative interpreter execution verified |
| QuickJS | `javascript` / `quickjs` | single-file | Embedded templates and graph assembly; representative interpreter execution verified |
| Custom | any / `compiler-template-set` | classified-filesystem | Model `@…` role overrides and portable JSON template bundles |

The initial 0.5.0 release had only C17 and literal exports. These additions form the 0.6.0 milestone. Asset availability is not a claim of complete language parity. Cross-composite routing/code interfaces, all collection expressions, capability variants, mixed technologies and language-specific editor intelligence require additional tests. The generic LSP service already accepts explicit external servers, but only C/C++ has the fully connected generated-body editor adapter.

## Shared entry points

- Desktop **Generate → Generate Project (effective technology)** uses the project's inherited `technologyId`. Select a new/empty destination; output is staged atomically. **Export Compiler from Model Overrides** exports a portable bundle.
- `wirecraft templates` lists the six sets; `wirecraft templates go` prints its complete roles/project templates.
- `wirecraft compile PROJECT.orch OUTPUT_DIRECTORY` selects by inherited technology. Optional `--template-set go` and `--layout single-file` select explicitly.
- `wirecraft export-compiler PROJECT.orch NEW_DIRECTORY nodejs` exports `compiler.wirecompiler.json`, including model overrides. Consume it with `wirecraft compile PROJECT.orch NEW_OUTPUT --compiler compiler.wirecompiler.json`.
- API `compile.project`, `compiler.templates`, `compiler.export` expose the same services. `wirecraft.workbench` MCP operations are `compile_project`, `compiler_templates`, `export_compiler`, with the service parameters inside `arguments`. Existing `wirecraft.compile_c17` and literal-export contracts stay available.

Generation never invokes downloaded tools. Build/run through the existing supervised external jobs or your shell: `go build`, `node path/to/root.js`, `kotlinc SOURCES -include-runtime -d app.jar` then `java -jar app.jar`, `php root.php`, or `qjs root.js`. Kotlin's retained Gradle template pins Kotlin 2.2.0 and JVM 21. Tests can select a standalone compiler using `WIRECRAFT_TEST_KOTLINC`; this does not change the user's SDK installation.

## Portable bundle and compatibility

`TemplateSet` is a JSON data contract containing set identity, language/technology, extension, default layout, role sources, project files and static filenames. It replaces the original generated JVM compiler-class/plugin ABI. The Rust application evaluates the data directly; it does not load Java classes or run a generated compiler binary. Model fields and unknown stored data remain unchanged.

MiniJinja is the rendering implementation ([official documentation](https://docs.rs/minijinja/latest/minijinja/)); a compatibility layer maps Pebble `elseif`, its empty test, permissive undefined variables and legacy `${identifier.path}` placeholders. Every original `.peb` parses in tests. Arbitrary Pebble extensions/macros from third-party JVM plugins are not promised compatible. Rendering has an instruction budget and a 16 MiB output writer limit. Generated projects reject unsafe/duplicate paths, excessive output and unresolved external connections on subtree compilation. Exported bundles contain source and should be reviewed like model templates.

Kotlin assembly explicitly includes channel declarations in single-file roots and imports generated runtime symbols in file-based nodes. These are Rust planner corrections around the unchanged reference templates. Source maps record authored source/initialization ranges. Errors from templates and missing build tools remain failures, never hard-coded success.

## Verification

The first connected tests exercise producer → typed channel → consumer → shutdown through external Go, Node.js and Kotlin processes, both Node layouts and both Kotlin layouts. PHP/QuickJS rendering, override/bundle round trips, template errors and all original template syntax have separate checks. The external suite was also run with real PHP 8.4.11 and a locally built QuickJS 2026-06-04. Exact final commands/results are recorded in status.md.

C bundles can be inspected/exported, but execution returns `unsupported` until C template parity is implemented. The default typed C17 path rejects requested non-single layouts and model overrides it cannot apply. Template composite-boundary data channels and foreign nested technologies likewise return explicit unsupported errors. Earlier Wirecraft inspector aliases `direct-filesystem` and `classified` are accepted on input; the UI now writes the original canonical identifiers.


Manual desktop verification: open `examples/nodejs.orch` in an isolated `--state-dir`, choose Generate Project, select a new output path and run `node OUTPUT/hello_nodejs.js`. Then choose Export Compiler from Model Overrides, select another new directory and use its JSON bundle through the CLI example above. Check inspector Technology choices without changing a live project. One native Generate/execution succeeded here; repeated full dialog automation did not pass because GNOME's Wayland picker is outside X11 automation and a private GTK portal was intermittent. Native bundle-picker completion remains unverified; the corresponding real CLI/API/MCP path passed.
