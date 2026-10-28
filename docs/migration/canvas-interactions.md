# Canvas and hierarchy interactions

Corrected in 0.4.1 according to the user's interaction specification. The original Threadworks checkout and running process are independent and unchanged.

| Gesture | Result |
|---|---|
| Wheel over drawing | Zoom about the world point under the pointer, including zoom limits |
| Hold middle button / wheel and drag | Pan; releasing outside the drawing also ends the gesture |
| Unmodified click | Replace selection, including when clicking an existing member of a multi-selection |
| Shift-click | Add the hit entity; adding it again keeps it selected |
| Control-click | Remove the hit entity; Control takes precedence when both modifiers are held |
| Drag empty space left to right | Select completely enclosed objects; solid window outline |
| Drag empty space right to left | Select enclosed and intersecting objects; dashed crossing-window outline |
| Shift / Control with a selection window | Add / subtract hits from the selection captured at the start of the window |
| Drag a selected node | Move the selection and composite descendants; one undo step |
| Press a tree row | Select that entity, using the same modifiers as the drawing |
| Drag a tree row onto another processing entity/composite | Reparent the picked entity; root moves and descendant cycles are rejected |

Rows have full-width hit targets and highlights. Individual selected objects have contour highlights, and the entire selection has a separate bounds outline. Selected descendants hidden by collapsed composites project their contour to the visible ancestor. Selecting one hidden entity from the tree reveals its containing view without changing the document. Reparenting preserves absolute drawing coordinates.

An unmodified press on an already-selected object defers replacement until release, distinguishing a plain click from a group drag. Child IDs are deduplicated when both a composite and its child are selected. Drag previews do not mutate the backend; release submits one revision-checked batch. The preview remains visible until the authoritative snapshot arrives.

Routing has an 80 ms quiet interval and one background projection at a time. Immediate pointer movement uses inexpensive endpoint feedback. Completed obstacle-avoiding routes retain their geometry at their computed offset; new movement applies a residual delta. Epoch/revision checks reject stale work, and preview positions remain subject to viewport culling. Panning leaves staged editor drafts intact; it never reloads the editor.

## Box sizing and menus (0.8.0–0.9.0, user specification)

| Element | Result |
|---|---|
| Simple box (processor, type, collapsed composite) | Width follows its title (name + name detail), role and language · technology lines, and type fields; height follows the busier lateral side at 24 units per port row plus a 12-unit inset at both ends, right-side callouts, and the text block. Minimum 120 × 70 |
| Expanded composite | Encloses its visible children plus per-side `layout.padding` `{top,right,bottom,left}` (default 64 each, wider than the 46-unit boundary stub) and a 64-unit header above the top padding |
| Corner handles | A selected expanded composite with children has one handle per corner, tested before picking so it can straddle the border. Top-left edits top and left padding, top-right top and right, bottom-right bottom and right, bottom-left bottom and left; each side follows its axis delta (outward grows, minimum 0) as one undoable patch. Simple boxes have no handles |
| Note | Keeps its stored size |
| Align/distribute | Use these projected boxes; a composite moves with its whole subtree |
| Menubar | Opens on press, left-aligned under its label, no backdrop; pressing the label again or outside closes it; hover switches menus while one is open |
| Mode buttons / Sheet | The active mode is highlighted; Sheet toggles dashed page contours from the current export format, scale, split and overlap |
| Entity tree | Composites show boxed `+` (collapsed) and `−` (expanded) |

Sizing is a projection shared by canvas, hit-testing, routing and SVG/PDF export: stored `width`/`height`/`closed*`/`open*` values are left unchanged in `.orch` files. `padding` is stored as an extra layout key, which the JVM store ignores; a plain number (written by 0.8.0) still means uniform padding, and missing sides use the default. Text widths are estimated at 0.6 × font size per character. When zoomed out, the canvas's 7px minimum label size can still make text extend past a box edge. `scripts/menu_smoke.py` (optionally `WIRECRAFT_SMOKE_FIXTURE=…`) checks menus, mode highlight, sheet contours, Fit and save without a selection natively; `scripts/padding_smoke.py` drags a real top-left handle and checks the two edited sides, the unchanged sides and one-step undo.

## Placement previews, routed links under construction and link rewiring (0.14.0)

Wirecraft extensions requested by the user:
- **Future contours.** In Node, Type and Port modes, the canvas shows what a click would create at the pointer.
- **Links under construction.** In Link mode, once a source is chosen, the link follows the pointer. Over an entity, that entity is outlined. Over empty canvas, the port circle the click would publish is shown.
- **One preview mechanism.** The desktop clones the snapshot, adds the would-be entity, link or port, and projects it on the background executor. The whole projection is drawn: the pending link is routed together with every other link, which may shift to make room. Pending items are highlighted and outlined but excluded from the selection bounds.
- **Debouncing.** Previews run 50 ms after the pointer settles, with at most one in flight. Until one arrives, a default-size outline or a straight dashed line with an arrowhead stands in. Measured on the WebServer component: 4–150 ms per preview in a debug build. Previews never touch the design.
- **Rewiring a link.** A selected link shows a grab handle at each end. Pressing one picks that end up; it follows the pointer with the same routed preview. Clicking an entity reattaches it there; clicking empty canvas ends it at a new Exterior port in the focused composite. Escape cancels.
  - The backend command `links.reconnect` (`nodeId`, `end: source|target`, `entityId`, or `parentId`/`x`/`y` for a new port) makes the change in one undo step.
  - The moved end takes the default port name, and the previous entity loses the port no other link uses. `sync_links` moves the link to the new common parent and recomputes its boundaries.
- **Verification.** `scripts/rubber_band_smoke.py` checks all of the above natively, including handle pick-up, a cancelled rewire (no change) and a real drop. `tests/components.rs` covers reconnection, re-parenting, port pruning, new ports and undo. X11 captures can lag behind the newest frame, so the script waits after each hover.

## Verification

`crates/wirecraft-core/tests/canvas_interactions.rs` checks selection operations, containment/segment crossing and anchored zoom at the clamps. The six desktop scene unit tests cover directional windows, picking, bounds, collapsed projection, moving-object culling and preserving computed detours.

```sh
cargo build --workspace --locked
cargo test --workspace --locked
python3 scripts/canvas_smoke.py
python3 scripts/package_smoke.py dist/wirecraft-0.4.1-linux-x86_64.tar.gz --desktop --walkthrough --editor --canvas
```

The native script creates a temporary graph/state directory and owns its window and backend. It exercises actual wheel/button/modifier input, outside release, box directions, graph/tree highlights, bounds, preview-without-mutation, one-step undo, descendant deduplication, tree reparenting, rejected cycles and panning with a staged draft. It needs the same X11/XWayland/Vulkan, xdotool and ImageMagick environment as the other desktop suites. Other platforms remain unverified.
