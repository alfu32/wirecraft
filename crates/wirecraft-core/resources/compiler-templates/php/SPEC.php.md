# PHP Compiler Template Set

This template set follows the [Threadwork Design Guide](../../../../../../docs/design-guide.md).
Libraries define reusable capabilities, processing functions employ and route
their results/errors, and generated links preserve the visible data,
dependency, and failure topology.

This directory defines the built-in PHP compiler as a Pebble template set.
`compiler.properties` binds compiler roles to `.peb` files and is loaded by
`PhpCompiler`. The generic compiler kernel performs traversal, child-first
compilation, compiler delegation, and filesystem layout; these resources define
the PHP syntax placed in each generated artifact.

## Manifest Behavior

- Generated primary files use `.php`.
- Direct filesystem homomorphism is the default, so composite nesting becomes
  directory nesting.
- Link files are disabled; link declarations and transport calls are embedded in
  the owning composite.
- Nodes named `composer.json` or `.env` are copied as literal static files.
- A project-level `composer.json` is always generated from `composer-json.peb`.

Role selection prefers stereotype/layout-specific templates, then composite,
node-kind, and generic-node roles. Terminal nodes use `processor.peb`; entities
with children resolve through `composite.peb` before an assembly template wraps
the result.

## Node Template Context

All node templates receive a common context:

| Value | Purpose |
| --- | --- |
| `id`, `name` | Raw model identity and name. |
| `symbol`, `safeName` | Sanitized, case-preserving PHP function identifier. |
| `node`, `self` | Full node view with kind/stereotype, relationship IDs, ports, text, effective technology, layout, metadata, derived symbols, and optional link data. |
| `document` | `id`, project `name`, and `rootNodeId` for node-level rendering. |
| `options.projectName` | Normalized compilation project name. |
| `metadata`, `text`, `technology`, `layout` | Current node views; technology values are inherited/effective. |
| `children`, `parent` | Resolved immediate hierarchy views. |
| `childArtifacts`, `linkArtifacts` | Child-first compilation results with declaration, instantiation, paths, symbols, layout, and `isInline`. |
| `inlineChildArtifacts`, `externalChildArtifacts` | Child artifacts split by single-file versus external layout. |
| `childForwardDeclarations`, `linkForwardDeclarations`, `descendantForwardDeclarations` | Recursively collected prototype blocks. PHP currently defines no forward-declaration templates, so these values are empty. |
| `ownForwardDeclaration`, `forwardDeclarations` | Assembly-only prototype text for the current entity and complete subtree; retained for template sets that require prototypes. |
| `incomingDataLinks`, `outgoingDataLinks`, `dependencyInjectionLinks` | Data links are separated from library bindings and expose Type, A/B buffer, transport, and endpoint symbols. |
| `typeFields` | Shared-Type fields with raw/sanitized names, resolved type symbols, and reference semantics. |
| `ports` | Port ID, name, direction, data type, and metadata. |
| `declaration`, `instantiation`, `specification`, `tests`, `usageInstructions` | Current node text fields. |
| `declarationIndent4`, `instantiationIndent4` | User text indented for a PHP function body. |
| `primaryPath`, `layoutStrategy`, `isCompilationRoot` | Output/layout state for this node. |
| `safeProjectName`, `safePackageName`, `projectIdentifier` | Path-, package-, and identifier-safe project forms. |

A link node also receives `link`, `sourceNode`, and `targetNode`. `link` provides
the link identity, type definition, endpoint IDs/ports, transport kind, computed
`sourceReference`/`targetReference`, and escaped single- and double-quoted forms.

## Templates

### `processor.peb` (`processor.declaration`, `node.declaration`)

Creates a PHP function named by `symbol`. Its by-reference `$context` parameter
is the shared runtime state. `instantiationIndent4` performs node setup before
`declarationIndent4`, the normal execution body. This wrapper gives every
terminal processing node a uniform callable contract.

### `composite.peb` (`group.declaration`, `composite.declaration`)

Creates the function for a composite. It emits the composite's own setup and
declaration, invokes every non-note child via `child.symbol`, then executes each
non-empty `linkArtifact.instantiation`. It is necessary to convert hierarchy into
an executable sequence while retaining the same `$context` across children.

### `link-declaration.peb` (`link.declaration`)

Creates two typed FIFO arrays and the link-specific transport function. The
function shifts at most one packet from the A-side array to the B-side array.
Dependency-injection links remain bindings and do not emit transport state.

### `link-instantiation.peb` (`link.instantiation`)

Calls the named transport function with its A and B arrays after child execution.

### `type-declaration.peb` (`type.declaration`)

Generates a PHP value class whose promoted constructor properties correspond to
the shared Type fields. Built-ins map to native PHP types; custom fields use the
referenced Type symbol.

### `note.peb` (`note.declaration`)

Uses `node.declarationBlockComment` to turn a note's declaration, or its
specification fallback, into a non-executable block comment.

### `child-import.peb` (`child.import`)

Receives the parent node context plus a `child` artifact and emits
`require_once` from `child.relativePath`. File-based composites need it before
calling functions declared in child translation files.

### `runtime.peb` (`runtime.support`)

Defines the shared runtime context used by generated processor wrappers. Link
queues and transport functions are emitted per modeled link.

### `assembly-file.peb` (`composite.file-based`)

Orders `childImports`, `runtimeSupport`, `linkDeclarations`, and
`ownDeclaration` after one `<?php` opening tag. These values are pre-rendered
strings supplied only during composite assembly. The template makes a separate
composite file syntactically complete.

### `assembly-single.peb` (`composite.single-file`)

Builds one PHP translation unit. Besides imports, runtime, link declarations,
and the current declaration, it inserts `inlineChildDeclarationsWithoutPhpTag`.
Removing nested opening tags is required when child artifacts are inlined into
an already-open PHP file.

### `static-path.peb` (`static-file.path`)

Receives node context plus `explicitPath`. It honors `metadata.path` or
`metadata.file`; otherwise it returns `safeProjectName/name`. Static nodes bypass
the normal primary-path strategy, so this role determines their destination.

### `composer-json.peb` (project file content)

Receives project context and uses `safePackageName`. Project context consists of
`document` (with all node views), `root`, `options`, `projectName`, normalized
name variants, `layoutStrategy`, `scopeRoots`, and single-file/main-class naming
helpers. The generated manifest establishes a valid Composer project boundary.

## Editing Rules

Pebble output is not auto-escaped and unknown variables render empty. Use the
provided escaped fields inside PHP string literals. Keep a single `<?php` tag per
assembled unit, preserve by-reference `$context` semantics, and use `{# ... #}`
for source-only comments.
