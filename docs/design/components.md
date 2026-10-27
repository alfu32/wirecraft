# Pluggable components: Exterior and Reference

Status: specification, not implemented. This is a Wirecraft extension beyond the Threadworks 2.6.7 baseline.

A design becomes a reusable component by linking some of its entities to the **Exterior**, the outside world. Another design uses it by placing a **Reference** to it and linking to the ports the Exterior links publish. Links keep encoding their ports. Nothing is a port unless a link makes it one, and every link keeps both endpoints.

## Exterior

An Exterior is a node of kind `Node` with `stereotype: "Exterior"`, placed directly under the design root.

- **One logical entity, many placements.** Every Exterior node in a design stands for the same outside world. Several placements let each port's link stay short, like off-page connectors in a schematic. Each placement has its own layout. No identifier links the placements: in this version a design has exactly one exterior.
- **Published ports.** A link from an entity to an Exterior publishes an **output port**. A link from an Exterior to an entity publishes an **input port**. The port's name is the link's name, and its type is the link's type.
- **Rules** (each is a validation error):
  - An Exterior has no source, initialization, tests or technology, and is never emitted or initialized.
  - It cannot be the endpoint of a library or dependency link, of a self-link, or of a link whose other end is also an Exterior.
  - Port identity is name plus direction. It must be unique across all placements, and every Exterior link needs an explicit type.
  - An Exterior may appear only directly under the root. Composite boundaries keep their existing boundary ports.
- **Drawing.** A compact glyph labelled `exterior`, with one routed port per link, using ordinary routing.

A design containing Exterior links is a **component**. It validates on its own, but it compiles to a program only through a Reference, because its output ports have nowhere to go. A future harness may bind Exterior ports to stdin/stdout or files so a component can run standalone. That harness is not part of this version.

## Reference

A Reference is a node of kind `Node` with `stereotype: "Reference"` and a `reference` object:

```json
"reference": { "path": "webserver-component.orch", "sha256": "…", "name": "WebServer", "date": "2026-10-28T…Z" }
```

- **`path`** is relative to the referencing file. It is rejected if it escapes the project directory, using the generated-path safety rules.
- **`sha256`** pins the exact referenced file. `name` and `date` copy its master revision for display.
- **Missing or changed file.** If the file is missing or its hash differs, that is a diagnostic, and nothing changes silently. **Update reference** is an explicit command. It re-pins the hash and reports added, removed or retyped ports, and any link attached to a removed or retyped port.
- **Read-only.** The Reference is drawn as a box showing the component's published ports. Its content is not editable in place; **Open component** opens the referenced file.
- **Types.** The component's types are visible to the referencing design under the Reference, so links can use `ReqRep` from the component without copying it.

### Linking to a Reference

A link to or from a Reference names a published port with the existing link fields. A link leaving a Reference uses `sourcePortName` = output port name. A link entering it uses `targetPortName` = input port name.

- **Checks:** the port must exist, its direction must match, and the link type must equal the port type.
- **Choosing the port.** When a link is dropped on a Reference, the editor proposes the ports of matching direction and type, so drawing stays a single gesture.
- **Fan-in.** An input port accepts any number of consumer links. All of them deliver into the component's single internal input buffer.
- **No fan-out.** An output port connects to exactly one consumer link, because links are point-to-point buffers. Distributing to several consumers needs an explicit splitter entity.

## Compilation

References are expanded into an ordinary graph before any compiler runs, so the C17 emitter and the template compilers share the same pure transformation in `wirecraft-core`.

1. **Load and verify.** Load the referenced document, check its hash, and reject cycles between components.
2. **Copy.** Copy the component's entities into a synthetic composite in place of the Reference, with fresh IDs (the archetype import remapping). Symbols are prefixed by the Reference name. The component's types are emitted once per program, even with several References to the same file.
3. **Join each published port.** The internal Exterior link and its consumer links become one channel:
   - **Output port:** the internal link's source buffer transports into the consumer link's target buffer.
   - **Input port:** every consumer link transports into the internal link's target buffer.

   Joining adds no transport hop, so a component boundary adds no tick.
4. **Libraries.** Libraries linked inside the component stay internal. Their instances are lent only to the component's own entities.

**Per-instance state.** Bodies and libraries may keep state in `static` or global variables; the WebServer uses `sequence`, `limit` and `http_listener`. Two References to the same component in one program would silently share that state. Until per-instance state exists, a program with more than one Reference to the same component is rejected with a diagnostic. The intended follow-up gives each Reference its own instance of the component's libraries, and moves body state into an instance-scoped structure lent like a library instance.

**Unknown fields and compatibility.** `stereotype` and `reference` are additive. Files that use neither are unchanged. Files using them still parse as ordinary nodes and links. The Threadworks JVM app cannot expand References, so such designs are Wirecraft-only for compilation; whether the JVM app opens them cleanly is to be verified against the baseline.

## WebServer split

The WebServer component contains only the HTTP transport:

| Entity | Role |
| --- | --- |
| `socket_accept` | Accepts connections, keeps pending requests on `awaitResponseLoop`, resolves responses |
| `socket_respond` | Renders response headers through `libhttp` |
| `base_lib`, `lib_http_server` | Platform headers and the HTTP library, lent as `platform` and `libhttp` |
| `Request`, `Response`, `ReqRep` | The packet types |

Links inside the component:

| Link | From → To | Role |
| --- | --- | --- |
| `request` | `socket_accept` → Exterior | **Output port** (ReqRep): accepted requests |
| `response` | Exterior → `socket_respond` | **Input port** (ReqRep): completed responses, fan-in |
| `rendered` | `socket_respond` → `socket_accept` | Responses with rendered headers (renamed from `response`) |
| `awaitResponseLoop` | `socket_accept` → `socket_accept` | Pending requests, unchanged |
| `platform`, `libhttp` | library links | Unchanged |

`socket_respond` then reads one input buffer instead of iterating one link per route.

The consumer design, which is the current example minus the component, contains:
- a `WebServer` Reference;
- `router` and the `routes` composite with its four routes;
- `request` from the Reference's `request` port to `router`, plus the four `route*Request` links;
- the four `route*Response` links into the Reference's `response` port.

The path is the same 5 hops as today: `socket_accept` → `router` → route → `socket_respond` → `socket_accept`.

The standard WebServer design, offered as an archetype, is the component alone: `socket_accept`, `socket_respond`, their types and libraries, and two Exterior placements.

## First implementation slice

1. Model and validation for Exterior and Reference nodes, Reference link port checks, and diagnostics for missing or changed files.
2. Drawing: the Exterior glyph, Reference boxes with projected ports, and port choice when a link is dropped.
3. Expansion in `wirecraft-core`, used by the C17 emitter first. Template compilers follow once their composite boundary channels are migrated.
4. The WebServer split into `examples/webserver-component.orch` and `examples/webserver.orch` (consumer). The existing end-to-end HTTP test runs against the consumer.
5. Pending after the slice: per-instance state, the standalone harness, a splitter archetype, and Exterior inside composites.
