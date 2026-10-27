# Local GPUI input extension

Upstream: gpui-component 0.5.1, Apache-2.0, crates.io checksum d021d46b4088d3d93a57ccdf443da85695a77272108caca2f6fe5369f584966a. Original sources and license are retained. Upstream tests and the package lockfile are omitted; the workspace lockfile controls dependencies.

Two input files add a narrow API for semantic highlight overlays: state.rs stores validated, sorted UTF-8 ranges tied to exact input text and clears them on edits; element.rs combines those styles with syntax coloring. No application services or language intelligence live in the widget. This extension can be removed when the pinned upstream input exposes an equivalent public API. See the workspace tests and native editor smoke for actual compiler classification/rendering verification.

window_border.rs restricts resize hitboxes/cursors to actual gutters, uses current viewport bounds, and respects tiled edges. Upstream assigned an edge cursor to a full-window hitbox based on a paint-time pointer sample, leaving resize cursors over application content.
