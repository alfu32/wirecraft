# Routing, ports and PCB-style crossings

Reference: Threadworks 2.6.7, `7d392d3a0e30b86445e00e7c03032a5188b26593`, `app-desktop/src/main/kotlin/com/threadwork/app/ui/ThreadworkDesktopApp.kt`: `bestRouteBetween`, `routeCandidates`, `chamferOrthogonalTurns`, `routeCost`, `portCapPoints`, `applyOnCommit`. The previous Wirecraft router searched H/V lanes and added short diagonal cuts afterwards. That was an approximation and did not migrate the original route-selection behavior.

## Current routing rules

- **Self-links have priority.** Choose the less occupied horizontal side before adding either feedback port, preferring right on ties. Count ordinary/boundary ports and right-side provider boxes; top flagpoles do not count. Place an adjacent input-above-output pair on that side; each subsequent loop includes already placed pairs in its occupancy count. Allocate their ports first on each side, then order other ports by connection angle. Route feedback before ordinary channels, regardless of UUID or stored link position. Reserve a compact external U between the two port heights; it no longer travels around the top or bottom of the box. Multiple feedback links use separate local envelopes. Only physical obstructions grow those envelopes in small rings; other channels cannot force a self-link across the drawing.
- Restore the JVM's **46-world-unit horizontal stubs**, including both sides of composite crossings. An output connection is **20 units outside its box**; an input connection is at the box boundary. Dependency/provider boxes and consumer flagpoles retain their distinct representation.
- Migrate the JVM candidate family: direct snapped paths, midpoint horizontal/vertical lanes in 40-unit steps (span seven), and both long-diagonal alternatives. Candidate orthogonal turns use a maximum 22-unit chamfer **before scoring**. Score Euclidean length, 18 per intermediate bend and 2500 per crossing. Reserve final geometry, including real diagonals, before later links route. There is no post-routing corner-cut pass changing those reservations.
- Prefer routes without box penetration or coincident channel runs; near parallel lanes retain a 50-unit spacing cost within ten units. If candidates cannot fit, bounded A* visibility search supports horizontal, vertical and both diagonal axes. Endpoint guide projections enable exact 45° moves on a nonuniform grid, with fractional ports; exact anchors survive floating-point guide deduplication. The 250000-vertex limit remains.
- **When no complete clear solution is found, draw best effort.** This supersedes the earlier visible-routing-error policy at the user's request. Retain the route and its horizontal ends, use soft obstacle/channel penalties to choose a snapped fallback, and keep feedback local. Do not hide the link or display a routing-error label. Best effort may cross an obstruction in an impossible/crowded layout; clear routes remain preferred.

## Original endpoint symbols

`core::drawing::port_glyphs` uses the original five-point `portCapPoints` templates at scale 0.6: hollow pentagonal outputs and filled, notched input caps. They mirror with the actual side, rather than using a generic diamond/arrowhead. The same geometry feeds desktop and SVG/PDF. Output caps remain pickable and included in selection bounds after moving the connection outside the box. Composite pierce circles use the reference five-unit radius. Flow arrows follow the route at three quarters of visible foreground length and stay clear of via gaps.

## Crossings with vias

At an unconnected crossing, one conductor remains uninterrupted. The other is represented by **two hollow circles and a muted gray backside connector**:

```text
        |
  ----o | o----
        |
```

The gray line runs between the circles behind the foreground conductor, including around a bend when needed. Foreground fragments stop at the circles' outer rims; the backside segment meets their inner rims. No background erase mask destroys the crossing conductor, and there is no curved bridge arch or filled junction dot. Standard circle radius is 2.5 units and center offset is 12 units; nearby crossings/ends reduce these distances to keep pairs separate. Desktop and vector exports share trace fragments, via positions and backside polylines. Self-links are reserved first and keep foreground priority.

These are drawing conventions, not a PCB electrical/layer model.

## Verification and deliberate differences

`WIRECRAFT_TEST_KOTLINC=/path/to/kotlinc python3 scripts/reference-router-fixtures.py` extracts the pure functions from the **recorded git object**, compiles them in an isolated temporary JVM and regenerates `core/tests/fixtures/router-reference.json`. The running JVM UI and source checkout are untouched. The fixture records source revision/path and extracted-method SHA-256.

Tests compare **ten original route outputs and four original port templates**, including a chamfered obstacle lane. Additional checks cover all-quadrant optimal diagonal displacement, fractional/translated visibility search, diagonal overlap/crossing costs, feedback envelope/port priority, 46-unit leads, original cap coordinates, PCB via/backside geometry, best-effort output without error labels, and the WebServer/ticker fixtures. Composite crossings retain six separate symbolic ports and aligned horizontal stubs. SVG uses the shared route and crossing geometry.

The original uses integer java.awt.Point coordinates; Wirecraft retains f64 fractions for stable translation and composite ports. Strict clearance/channel checks before best effort, local feedback contour selection and bounded visibility search are maintained Rust behavior. Matching the recorded source cases does not claim every crowded drawing is pixel-identical. Final commands, native captures and package checks are in status.md and evidence/v0.6.1/.
