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

Routing has an 80 ms quiet interval and one background projection at a time. Immediate pointer movement uses inexpensive endpoint feedback. Completed obstacle-avoiding routes retain their geometry at their computed offset; new movement applies a residual delta. Epoch/revision checks reject stale work, and preview positions remain subject to viewport culling.

## Verification

`crates/wirecraft-core/tests/canvas_interactions.rs` checks selection operations, containment/segment crossing and anchored zoom at the clamps. The six desktop scene unit tests cover directional windows, picking, bounds, collapsed projection, moving-object culling and preserving computed detours.

```sh
cargo build --workspace --locked
cargo test --workspace --locked
python3 scripts/canvas_smoke.py
python3 scripts/package_smoke.py dist/wirecraft-0.4.1-linux-x86_64.tar.gz --desktop --walkthrough --editor --canvas
```

The native script creates a temporary graph/state directory and owns its window and backend. It exercises actual wheel/button/modifier input, outside release, box directions, graph/tree highlights, bounds, preview-without-mutation, one-step undo, descendant deduplication, tree reparenting and rejected cycles. It needs the same X11/XWayland/Vulkan, xdotool and ImageMagick environment as the other desktop suites. Other platforms remain unverified.
