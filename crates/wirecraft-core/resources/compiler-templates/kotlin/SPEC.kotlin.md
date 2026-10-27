# Kotlin/JVM Compiler Template Set

This template set follows the [Threadwork Design Guide](../../../../../../docs/design-guide.md).
Reusable algorithms and class/method implementations belong in libraries;
generated processing functions employ those capabilities and preserve their
principal result and operation-error contracts in the generated topology.

This directory defines the built-in naive Kotlin/JVM compiler. The manifest maps
semantic roles to Pebble resources. `NaiveKotlinCompiler` loads the set, while
`TemplateSetCompiler` traverses nodes child-first, selects templates, assembles
composites, and applies each node's effective layout strategy.

## Manifest Behavior

- Primary node files use `.kt` and default to the `source-set` layout.
- Link files are disabled. Link declarations remain artifact text and link
  instantiations are inserted into their owning composite's run function.
- Compiler-template nodes are skipped.
- `build.gradle.kts`, `settings.gradle.kts`, and `gradle.properties` are literal
  static-file names.
- Gradle settings/build files are always generated. `Runtime.kt` and `Main.kt`
  are generated for file-based layouts, but not for `single-file`.
- `source-path.peb` overrides source-set primary paths.

Kotlin differs from the Node.js/PHP sets: `processor.peb` itself handles both
terminals and composites in file-based layouts. `assembly-single.peb` is used
only to combine inline declarations, runtime support, and the entry point.

## Node Template Context

| Value | Purpose in generated Kotlin |
| --- | --- |
| `id`, `name` | Raw model identity and name. |
| `symbol`, `safeName` | General sanitized identifier. |
| `runSymbol`, `initializerSymbol`, `classFileSymbol` | Stable indexed Kotlin symbols used for functions, calls, and filenames. |
| `node`, `self` | Full node view: kind/stereotype, relationship IDs, ports, metadata, text, effective technology, layout, optional link, and derived symbols. |
| `document` | Node-context summary (`id`, project `name`, `rootNodeId`). |
| `options.projectName` | Normalized output project name. |
| `metadata`, `text`, `technology`, `layout` | Current node views. Effective language and technology are already inherited. |
| `children`, `parent` | Resolved immediate hierarchy views. |
| `childArtifacts`, `linkArtifacts` | Child-first compilation outputs with symbols, declarations, instantiations, paths, layout ID, and `isInline`. |
| `inlineChildArtifacts`, `externalChildArtifacts` | Artifacts partitioned by single-file layout. |
| `childForwardDeclarations`, `linkForwardDeclarations`, `descendantForwardDeclarations` | Recursively collected prototype blocks. Kotlin does not require or define them, so they are currently empty. |
| `ownForwardDeclaration`, `forwardDeclarations` | Assembly-only current-entity and complete-subtree prototype text, available for template languages that require declarations before definitions. |
| `incomingDataLinks`, `outgoingDataLinks`, `dependencyInjectionLinks` | Data links are separated from library bindings and expose resolved Type, A/B queue, transport, and endpoint symbols. |
| `typeFields` | Shared-Type fields with raw/sanitized names, resolved type symbols, and reference semantics. |
| `ports` | Port identity, direction, modeled data type, and metadata. |
| `declaration`, `instantiation`, `specification`, `tests`, `usageInstructions` | Node long-text fields. |
| `declarationIndent4`, `instantiationIndent4` | User code indented inside generated Kotlin functions. |
| `primaryPath`, `layoutStrategy`, `isCompilationRoot` | Current artifact and compilation-scope state. |
| `safeProjectName`, `safePackageName`, `projectIdentifier` | Normalized project naming forms. |

Link rendering adds `link`, `sourceNode`, and `targetNode`. `link` contains type
and endpoint data plus pre-escaped reference values. Composite assembly adds
pre-rendered `ownDeclaration`, `runtimeSupport`, `childImports`,
`inlineChildDeclarations`, `childDeclarations`, `childInstantiations`,
`linkDeclarations`, and `linkInstantiations`.

## Templates

### `processor.peb` (file-based node/composite declaration)

Mapped from processor, node, group, and composite declaration roles. It emits a
`generated.nodes` file with `init_*` and `run_*` functions. For a terminal, the
initializer uses `instantiationIndent4` and the runner uses
`declarationIndent4`. For a composite, those functions invoke child initializer
and runner symbols and then each link's named transport function. Incoming links
pass their B queue and outgoing links pass their A queue to processors.

### `processor-single.peb` (single-file node/composite declaration)

Provides the same two-function contract without package or import statements.
Inline child calls use local symbols; externally laid-out children are qualified
as `generated.nodes.*`. This distinction lets child layout overrides survive
inside a mostly single-file project.

### `link-declaration.peb` (`link.declaration`)

Declares two typed `ArrayDeque` values and a named function that moves at most
one value from the A queue to the B queue. Dependency-injection links are skipped.

### `link-instantiation.peb` (`link.instantiation`)

Invokes the named transport function with the link's A and B queues after child
execution in a composite.

### `type-declaration.peb` (`type.declaration`)

Emits a Kotlin `data class` for the shared Type entity. Built-ins map to Kotlin
types and custom fields use the sanitized referenced Type symbol.

### `note.peb` (`note.declaration`)

Emits `node.metadataComment`. It carries model metadata through the compilation
pipeline. If notes are emitted as `.kt` files by a chosen layout, this template
must produce valid Kotlin (for example, by changing it to a comment).

### `assembly-single.peb` (`composite.single-file`)

At the compilation root it emits package/runtime definitions and a `main`
function. It inserts `inlineChildDeclarations` before `ownDeclaration`, then
initializes and runs the root symbol. `isCompilationRoot`, `initializerSymbol`,
and `runSymbol` identify where the process entry point belongs. Nested composite
assemblies omit the root-only runtime and `main` blocks.

### `runtime-file.peb` (project `Runtime.kt`)

Project-level template defining `RuntimeContext`. Per-link queues and transport
functions are generated from link artifacts. It is omitted in single-file mode
because `assembly-single.peb` contains equivalent definitions.

### `main.peb` (project `Main.kt`)

Project-level entry point for file-based layouts. `scopeRoots` is the list of
compiled top-level node views. The template initializes every root first, then
runs every root, preserving a distinct setup phase.

### `build.peb` (project `build.gradle.kts`)

Project-level Gradle build using `mainClassName`. That value is
`generated.MainKt` for file-based output and a derived class name for single-file
output. The build declares Kotlin/JVM, Java 21, repositories, and application
entry-point metadata.

### `settings.peb` (project `settings.gradle.kts`)

Uses project-context `projectName` as `rootProject.name`. Unlike node templates,
project templates receive `document` (including all node views), `root`,
`options`, normalized project names, `layoutStrategy`, `scopeRoots`,
`singleFileBaseName`, `singleFileClassName`, and `mainClassName`.

### `project-name.peb` (`project.name`)

Runs before normal node/project rendering with a deliberately small context:
`projectName`, `safeProjectName`, `safePackageName`, and `safeIdentifier`. Its
result becomes the compiler's normalized project name and therefore influences
paths and project metadata. A blank result falls back to the original name.

### `source-path.peb` (`primary-file.path.source-set`)

Receives node context plus `defaultPath` and places the current node at
`src/main/kotlin/generated/nodes/classFileSymbol.kt`. `classFileSymbol` is
indexed to prevent collisions between equal node names.

### `static-path.peb` (`static-file.path`)

Receives node context plus `explicitPath`. It honors node `metadata.path` or
`metadata.file`; otherwise the literal node name is used. This role is needed
because static files bypass generated primary-file paths.

## Editing Rules

Pebble does no automatic escaping and strict variables are disabled. User code
is inserted verbatim, so templates must provide syntactically correct wrappers
and indentation. Keep source-only explanations in `{# ... #}` comments. When
adding a project file, define matching `project.N.path`, `.content`, and optional
`.reason`, `.layouts`, and `.elementKind` properties.
