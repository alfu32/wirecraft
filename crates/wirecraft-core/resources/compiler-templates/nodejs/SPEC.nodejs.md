# Node.js Compiler Template Set

This template set follows the [Threadwork Design Guide](../../../../../../docs/design-guide.md).
Libraries define reusable capabilities, processing functions employ and route
their results/errors, and generated links preserve the visible data,
dependency, and failure topology.

This directory defines the built-in CommonJS compiler. `compiler.properties`
maps semantic template roles to Pebble (`.peb`) resources; `JSCompiler` loads
the manifest through `CompilerTemplateSetLoader`. Templates emit text only.
`TemplateSetCompiler` walks the model, selects roles, assembles child artifacts,
and applies the selected filesystem layout.

## Manifest Behavior

- `fileExtension=js` gives generated node files the `.js` suffix.
- `defaultLayoutStrategyId=direct-file-system-homomorphism` mirrors composite
  nesting as directories.
- `emitLinkFiles=false` keeps links as declarations and transport calls inside
  their owning composite instead of creating one JavaScript file per link.
- `staticFiles=package.json,config.json` treats nodes with these names as literal
  files. Their declaration text is copied without compilation.
- `project.1.*` always creates the project `package.json` from project context.

Template lookup is most-specific first: stereotype plus layout, stereotype,
composite plus layout, composite, node kind plus layout, node kind, then generic
node. Consequently `processor.peb` handles terminal `Node` and `Processor`
entities, while child-bearing entities resolve to `composite.peb`.

## Node Template Context

Node templates receive the following Pebble values. Maps are accessed with dot
notation, for example `node.text.declaration`.

| Value | Meaning and generated-code role |
| --- | --- |
| `id`, `name` | Raw model identity and display name. Do not use `name` directly as a JavaScript identifier. |
| `symbol`, `safeName` | Sanitized, case-preserving JavaScript identifier for the current node. |
| `node`, `self` | Full node view: identity, `kind`, stereotype, relationship IDs, ports, metadata, text, effective technology, layout, optional `link`, and derived symbols. |
| `document` | Node-context document summary: `id`, project `name`, and `rootNodeId`. |
| `options.projectName` | Normalized output project name. |
| `metadata`, `text`, `technology`, `layout` | Direct views of the current node. Technology contains effective/inherited language and technology IDs. |
| `children`, `parent` | Child node views and optional parent node view. |
| `childArtifacts`, `linkArtifacts` | Already compiled child and owned-link artifacts. Each exposes `node`, declaration, instantiation, path, relative paths, symbols, layout ID, and `isInline`. |
| `inlineChildArtifacts`, `externalChildArtifacts` | Child artifacts partitioned by whether their effective layout is `single-file`. |
| `childForwardDeclarations`, `linkForwardDeclarations`, `descendantForwardDeclarations` | Recursively collected prototype blocks. The Node.js set defines no forward-declaration templates, so these are currently empty. |
| `ownForwardDeclaration`, `forwardDeclarations` | Assembly-only current-entity and complete-subtree prototype text. These support languages such as C without changing Node.js output. |
| `incomingDataLinks`, `outgoingDataLinks`, `dependencyInjectionLinks` | Data-link descriptors are separated from library bindings and expose Type, buffer, transport, and endpoint symbols. |
| `typeFields` | Shared-Type fields with display and sanitized names, resolved type names, and reference semantics. |
| `ports` | Declared ports with ID, name, direction, data type, and metadata. |
| `language`, `stereotype` | Effective language ID and classified stereotype name. |
| `declaration`, `instantiation`, `specification`, `tests`, `usageInstructions` | Current long-text fields. |
| `declarationIndent2`, `instantiationIndent2` | User declaration or instantiation text indented for a JavaScript function body. |
| `primaryPath`, `layoutStrategy` | Current output path and effective layout (`id`, `displayName`). |
| `isCompilationRoot` | True for the document root or an explicitly compiled scope root. |
| `safeProjectName`, `safePackageName`, `projectIdentifier` | Project name normalized for paths, package metadata, or identifiers. |

For a link node, the context additionally contains `link`, `sourceNode`, and
`targetNode`. `link` includes `name`, `variableName`, `typeName`, `typeSymbol`,
`typeFields`, A/B buffer symbols, the transport symbol, endpoint IDs and port
names, transport kind, and endpoint references.

## Templates

### `processor.peb` (`processor.declaration`, `node.declaration`)

Emits one CommonJS function for a terminal processing node and attaches it to
`module.exports`. `symbol` is both the function and export property name.
`instantiationIndent2` is emitted first for per-node setup; `declarationIndent2`
is the executable body. The wrapper is necessary to give the generic visitor a
uniform callable artifact for every processing box.

### `composite.peb` (`group.declaration`, `composite.declaration`)

Emits the callable function representing a composite. It first inserts the
composite's own instantiation/declaration text, then invokes non-note
`childArtifacts`, and finally inserts each `linkArtifact.instantiation` transport
call. This is the executable orchestration layer that turns hierarchy into run
order and links into packet movement.

### `link-declaration.peb` (`link.declaration`)

Declares `link.aPortSymbol` and `link.bPortSymbol` as FIFO arrays and defines the
named `link.transportSymbol` operation. One call moves at most one packet from A
to B. Dependency-injection links do not produce data buffers.

### `link-instantiation.peb` (`link.instantiation`)

Invokes the link's named transport function with its A and B buffers. Composite
templates place this call after child processing.

### `type-declaration.peb` (`type.declaration`)

Emits one JSDoc structural typedef from the shared Type entity. Link descriptors
refer to its sanitized symbol, and selected-scope compilation pulls the Type in
with its endpoint link.

### `note.peb` (`note.declaration`)

Converts note declaration/specification text, through
`node.declarationBlockComment`, into a JavaScript block comment so documentation
can participate in generated source without becoming executable code.

### `child-import.peb` (`child.import`)

Receives the normal parent context plus one `child` artifact. It emits a
CommonJS `require` using `child.symbol` and `child.relativeModulePath`. File-based
composites need this declaration to call child functions stored in other files.

### `runtime.peb` (`runtime.support`)

Emits the runtime context used by generated processors. Per-link transport is
owned by `link-declaration.peb`, so the runtime no longer infers queues from
string endpoint references.

### `assembly-file.peb` (`composite.file-based`)

Receives assembly-only strings: `childImports`, `runtimeSupport`,
`linkDeclarations`, and `ownDeclaration`. It orders these as imports, runtime,
wire contracts, then the composite function. This ordering ensures every symbol
is declared before its first use in a separate-file artifact.

### `assembly-single.peb` (`composite.single-file`)

Receives the same assembly values plus `inlineChildDeclarations`. It embeds child
functions instead of requiring files, then emits wire contracts and the current
composite. `childImports` remains relevant when a descendant overrides the
single-file strategy and therefore remains external.

### `static-path.peb` (`static-file.path`)

Receives the node context plus `explicitPath`. An explicit `metadata.path` or
`metadata.file` wins; otherwise the path is `safeProjectName/name`. The template
is necessary because literal files bypass normal node primary-path layout.

### `package-json.peb` (project file content)

Project files receive project context rather than node context. This template
uses `safePackageName` and `root.symbol` to produce CommonJS package metadata.
Project context also provides `document` (including all node views), `options`,
`projectName`, `safeProjectName`, `projectIdentifier`, `layoutStrategy`,
`scopeRoots`, `singleFileBaseName`, `singleFileClassName`, and `mainClassName`.

## Editing Rules

Pebble uses `{{ value }}` for output and `{% ... %}` for control flow. Automatic
escaping and strict variables are disabled. Use the pre-escaped link fields in
quoted JavaScript literals, preserve CommonJS exports, and keep template comments
inside `{# ... #}` so they never enter generated source.
