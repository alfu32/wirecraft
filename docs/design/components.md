# Pluggable components: Exterior and Reference

Status: first slice implemented in 0.12.0 (C17 only); [Implemented scope](#implemented-scope) lists what is pending. This is a Wirecraft extension beyond the Threadworks 2.6.7 baseline.

A design becomes a reusable component by linking some of its entities to the **Exterior**, the outside world, drawn as circles. Another design uses it by placing a **Reference** to it and linking to the ports the Exterior links publish. Links keep encoding their ports. Nothing is a port unless a link makes it one, and every link keeps both endpoints.

## Exterior

An Exterior is a node of kind `Node` with `stereotype: "Exterior"`. It may sit at the design root or inside any composite (0.13.0); wherever it is, it publishes a port of the whole design.

- **One logical entity, one placement per port.** Every Exterior node in a design stands for the same outside world, like an off-page connector in a schematic. No identifier links the placements: in this version a design has exactly one exterior.
- **Exactly one link per placement.** Each placement is the endpoint of exactly one link, and that link is the published port:
  - A link from an entity to an Exterior publishes an **output port**.
  - A link from an Exterior to an entity publishes an **input port**.

  The port's name is the link's name, and its type is the link's type.
- **Derived name.** An Exterior has no name of its own. Its name is derived from its link: `<link-name>In` for an input port (the Exterior is the link's source) and `<link-name>Out` for an output port (the Exterior is the link's target).
  - The name is stored so that other tools and the Threadworks app see a meaningful name. Wirecraft rewrites it whenever the link is renamed or reversed.
  - Renaming is done on the link: an edit to the Exterior's own name is replaced by the derived name in the same transaction.
  - A placement without a link keeps the provisional name `port` and publishes nothing; it is a validation warning until linked. Two ways to create one:
    - **Port mode** (toolbar **Port**, or the command palette **Port Mode**): click to place an unlinked circle, then link it in Link mode like any entity. Linking from it makes an input, linking to it an output.
    - **Link mode**: click an entity and then empty canvas to publish an output, or empty canvas and then an entity to publish an input. The circle and its link are created together, in the focused composite. MCP `wirecraft.components` `publish` (with an optional `parentId`) does the same.
- **Rules** (each is a validation error):
  - An Exterior has no source, initialization, tests or technology, and is never emitted or initialized.
  - It cannot be the endpoint of a library or dependency link, of a self-link, or of a link whose other end is also an Exterior.
  - Derived names must be unique, so a design cannot publish two input ports (or two output ports) with the same name. Every Exterior link needs an explicit type.
  - An Exterior with two or more links is an error.
- **Inside composites.** A port nested in a composite stays visible while the composite is collapsed: the circle is drawn beside the collapsed box (inputs left, outputs right, in their stored vertical order) and its link is drawn from the box. There it cannot be dragged; expand the composite to move it. Expansion and C17 generation follow the link through the nesting.
- **Drawing.** An Exterior is a circle, not a box, so it reads as the open connection to the outside:
  - **Size and stroke.** Diameter 73 world units, the height of a single-port processing node (its three text rows; 2.9 squares of the 25-unit grid), so a circle sits level with the entity it connects to. The stroke is heavier than entity boxes. At that size it cannot be mistaken for the small hollow via circles that mark unconnected crossings. Its link attaches at the circle's vertical center, where a single-port node places its port.
  - **Cap.** The circle carries the port cap of the link end it hosts, on the rim facing its link, mirrored by side like any other port (see `core::drawing::port_glyphs`). An input Exterior shows the hollow output cap, because the link starts there. An output Exterior shows the filled, notched input cap, because the link ends there.
  - **Direction mark.** Inside the circle, a chevron points along the flow: away from the link side for an output, toward it for an input. This keeps the direction readable when zoomed out, below the cap's legible size.
  - **Label.** The derived name is drawn beside the circle, opposite the link side, together with the port type.
  - **Routing.** The single link leaves the side facing its counterpart and is routed normally. Moving the circle around the counterpart flips the side and mirrors the cap.

A design containing Exterior links is a **component**. It validates on its own, but it compiles to a program only through a Reference, because its output ports have nowhere to go. A future harness may bind Exterior ports to stdin/stdout or files so a component can run standalone. That harness is not part of this version.

## Reference

A Reference is a node of kind `Node` with `stereotype: "Reference"` and a `reference` object:

```json
"reference": { "path": "webserver-component.orch", "sha256": "…", "name": "WebServer", "date": "2026-10-28T…Z" }
```

- **`path`** is relative to the referencing file. It is rejected if it escapes the project directory, using the generated-path safety rules.
- **`sha256`** pins the exact referenced file. `name` and `date` copy its master revision for display.
- **Missing or changed file.** If the file is missing or its hash differs, that is a diagnostic, and nothing changes silently. **Update reference** is an explicit command. It re-pins the hash and reports added, removed or retyped ports, and any link attached to a removed or retyped port.
- **Ports.** `reference.ports` stores the published ports (`name`, `direction`, `typeName`) when the Reference is pinned, so the design draws and validates without loading the component. The Reference box lists them as `out request: ReqRep` and `in response: ReqRep`.
- **Read-only.** A Reference has no children or body; edit the referenced file instead.
- **Types.** A consumer link uses a component type through `typeDefinitionId` = `<reference id>::<type id>`, which exists after expansion, plus `typeName` for display. Links into or out of the Reference adopt the port's type during expansion.
- **Creating and updating.** **Graph → Insert Component Reference…** (or MCP `reference`) picks a saved design inside this design's directory, which must itself be saved. **Graph → Update Component Reference** (MCP `update_reference`) re-pins the selected Reference and reports added and removed ports. `wirecraft update-references FILE` re-pins every Reference of a file. Loading a file resolves its References relative to it.

### Linking to a Reference

A link to or from a Reference names a published port with the existing link fields. A link leaving a Reference uses `sourcePortName` = output port name. A link entering it uses `targetPortName` = input port name.

- **Checks:** the port must exist, its direction must match, and the link type must equal the port type.
- **Choosing the port.** A link drawn to or from a Reference gets a port automatically: the port named like the link, otherwise the only port of matching direction and type, otherwise the only port of that direction. Otherwise the default port stays and validation reports it; set `sourcePortName`/`targetPortName` on the link.
- **Fan-in.** An input port accepts any number of consumer links. All of them deliver into the component's single internal input buffer.
- **No fan-out.** An output port connects to exactly one consumer link, because links are point-to-point buffers. Distributing to several consumers needs an explicit splitter entity.

## Compilation

`components::expand` in `wirecraft-core` turns References into an ordinary graph inside `C17::generate`, so generation, generated-code analysis and the editor share it. The template compilers reject Exteriors and References explicitly until they adopt the same expansion.

1. **Load and verify.** The loader reads each referenced file, records its hash and resolves nested References; a component that references itself is reported. Expansion refuses any component diagnostic error, including a changed hash.
2. **Copy.** The Reference becomes a composite holding the component's entities, with IDs prefixed `<reference id>::`. Type references inside type definitions and link type expressions are rewritten the same way. Exteriors and their links are not copied.
3. **Join each published port.** The internal Exterior link and its consumer links become one channel:
   - **Output port:** the internal link's source buffer transports into the consumer link's target buffer.
   - **Input port:** every consumer link transports into the internal link's target buffer.

   Joining adds no transport hop, so a component boundary adds no tick. Each end keeps its own argument name (the internal link's name inside the component, the consumer link's name outside). An input port with no consumer link keeps its internal link, fed by nothing. An output port must have exactly one consumer link.
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
| `request` | `socket_accept` → Exterior `requestOut` | **Output port** (ReqRep): accepted requests |
| `response` | Exterior `responseIn` → `socket_respond` | **Input port** (ReqRep): completed responses, fan-in |
| `rendered` | `socket_respond` → `socket_accept` | Responses with rendered headers (renamed from `response`) |
| `awaitResponseLoop` | `socket_accept` → `socket_accept` | Pending requests, unchanged |
| `platform`, `libhttp` | library links | Unchanged |

`socket_respond` then reads one input buffer instead of iterating one link per route.

The consumer design `examples/webserver.orch` (`WebServerApp`), which is the earlier example minus the component, contains:
- the Reference `http` to `webserver-component.orch`;
- `router` and the `routes` composite with its four routes;
- `request` from the Reference's `request` port to `router`, plus the four `route*Request` links;
- the four `route*Response` links into the Reference's `response` port.

The path is the same 5 hops as today: `socket_accept` → `router` → route → `socket_respond` → `socket_accept`.

The standard WebServer design is the component alone, `examples/webserver-component.orch`: `socket_accept`, `socket_respond`, their types and libraries, and the two Exterior circles `requestOut` and `responseIn`.

## Implemented scope

Verified in 0.12.0:
- Exterior and Reference validation, derived names, automatic port choice, pinning and changed-file diagnostics (`crates/wirecraft-runtime/tests/components.rs`).
- C17 expansion with fan-in into one buffer: a two-producer test program prints `sum 42`.
- The WebServer split: the end-to-end HTTP test runs against the consumer; 5000 simultaneous clients each received 200.
- The circle, chevron and label, the Link-mode publish gesture and the Reference box (`scripts/components_smoke.py`).

Verified in 0.13.0: Port mode, ports inside composites (a nested component program prints `got 42`; collapsed placement is asserted on the shared drawing) and native screenshots of the expanded and collapsed composite.

Pending:
- Template compilers (Go, Kotlin, Node.js, PHP, QuickJS).
- Per-instance state, so a component can be referenced more than once per program.
- Clangd analysis inside a component's bodies: a component generates only through a consumer, so the background check skips it.
- **Open component** from a Reference, and a port picker when the automatic choice is ambiguous.
- The standalone harness, a splitter archetype, and the component as a built-in archetype.
- Checking that the Threadworks JVM app opens files that use these fields.
