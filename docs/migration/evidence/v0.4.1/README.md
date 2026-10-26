# Wirecraft 0.4.1 canvas evidence

These captures come from the installed optimized package on an owned window with a temporary graph/state directory. No user project or original Threadworks state was changed.

- selection-contours.png: individual selected contours, whole-selection bounds and full-width tree highlights.
- drag-rerouted-preview.png: a group follows the pointer while the backend revision and saved layout remain unchanged; routing has had time to run in the background.
- tree-reparented.png: a tree drag changes the real hierarchy through the shared service.

All 61 Rust tests, formatting, strict Clippy, installed backend/MCP/compiler/clangd checks and all four native suites passed. The canvas suite additionally checks pointer-anchored wheel zoom, middle-button pan and release outside the canvas, both window directions, additive/subtractive/replacement selection, one-step group undo, deduplicated descendant movement, tree reparenting and rejected cycles. Unit tests check that the completed obstacle detour survives the preview path and that translated primitives remain culled.

See verification.json for code and binary hashes, exact suite results and CI observations; the final remote CI run was still in progress when recorded. The [gesture contract](../../canvas-interactions.md) documents the controls and implementation boundaries. Available-platform evidence is Linux only.
