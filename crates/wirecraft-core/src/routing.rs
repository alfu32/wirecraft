//! Deterministic octilinear routing. Port placement precedes routing so neighboring
//! channels never share a port or a composite boundary crossing.
use crate::{
    Document,
    drawing::{BoundaryPort, Drawing},
    viewport::Rect,
};
use std::collections::{BTreeMap, BinaryHeap};

const GAP: f64 = 10.;
const STUB: f64 = 46.;
const OUTPUT_OFFSET: f64 = 20.;
type Point = [f64; 2];
#[derive(Clone)]
struct Port {
    owner: String,
    edge: String,
    right: bool,
    boundary: bool,
    toward_target: bool,
    other: Point,
    point: Point,
}
fn center(r: Rect) -> Point {
    [r.x + r.width / 2., r.y + r.height / 2.]
}
fn expand(r: Rect, n: f64) -> Rect {
    Rect {
        x: r.x - n,
        y: r.y - n,
        width: r.width + 2. * n,
        height: r.height + 2. * n,
    }
}
fn ancestors(d: &Document, drawing: &Drawing, node: &str) -> Vec<String> {
    let mut result = vec![];
    let mut id = d.nodes.get(node).and_then(|n| n.parent_id.as_deref());
    while let Some(p) = id {
        if drawing
            .shapes
            .get(p)
            .is_some_and(|s| s.composite && s.expanded)
        {
            result.push(p.into());
        }
        id = d.nodes.get(p).and_then(|n| n.parent_id.as_deref());
    }
    result
}
pub(crate) fn route_drawing(d: &Document, drawing: &mut Drawing) {
    let mut ports: Vec<Port> = vec![];
    let mut paths = BTreeMap::new();
    for (id, edge) in &drawing.edges {
        if edge.annotation_only {
            continue;
        }
        let a = drawing.shapes[&edge.source].rect;
        let b = drawing.shapes[&edge.target].rect;
        let ac = center(a);
        let bc = center(b);
        let left = ancestors(d, drawing, &edge.source);
        let right = ancestors(d, drawing, &edge.target);
        let common = left.iter().find(|p| right.contains(p)).cloned();
        let mut path = vec![];
        let mut add =
            |owner: &str, boundary: bool, other: Point, side: bool, toward_target: bool| {
                path.push(ports.len());
                ports.push(Port {
                    owner: owner.into(),
                    edge: id.clone(),
                    right: side,
                    boundary,
                    toward_target,
                    other,
                    point: [0.; 2],
                });
            };
        // For data/error links both ends may use either side. Dependencies are
        // annotations, deliberately excluded from this relaxation.
        add(
            &edge.source,
            false,
            bc,
            edge.source == edge.target || bc[0] >= ac[0],
            true,
        );
        for p in left.iter().take_while(|p| Some(*p) != common.as_ref()) {
            add(
                p,
                true,
                ac,
                bc[0] >= center(drawing.shapes[p].rect)[0],
                false,
            );
        }
        let enter = right
            .iter()
            .take_while(|p| Some(*p) != common.as_ref())
            .collect::<Vec<_>>();
        for p in enter.into_iter().rev() {
            add(
                p,
                true,
                bc,
                ac[0] >= center(drawing.shapes[p].rect)[0],
                true,
            );
        }
        add(
            &edge.target,
            false,
            ac,
            edge.source != edge.target && ac[0] >= bc[0],
            false,
        );
        paths.insert(id.clone(), (path, common));
    }
    let mut sides: BTreeMap<(String, bool), Vec<usize>> = BTreeMap::new();
    for (i, p) in ports.iter().enumerate() {
        sides.entry((p.owner.clone(), p.right)).or_default().push(i);
    }
    for _ in 0..3 {
        for ((owner, right), side_ids) in &sides {
            let mut ids = side_ids.clone();
            let r = drawing.shapes[owner].rect;
            let c = center(r);
            // atan2 against the outward side normal gives a stable top-to-bottom
            // angular fan, rather than alphabetic or link creation order.
            let angle = |p: &Port| (p.other[1] - c[1]).atan2((p.other[0] - c[0]).abs());
            ids.sort_by(|a, b| {
                let is_self =
                    |p: &Port| drawing.edges[&p.edge].source == drawing.edges[&p.edge].target;
                is_self(&ports[*b])
                    .cmp(&is_self(&ports[*a]))
                    .then_with(|| angle(&ports[*a]).total_cmp(&angle(&ports[*b])))
                    .then_with(|| ports[*a].edge.cmp(&ports[*b].edge))
            });
            let inset = if drawing.shapes[owner].composite {
                64_f64.min(r.height / 3.)
            } else {
                12_f64.min(r.height / 4.)
            };
            let count = ids.len();
            for (i, id) in ids.into_iter().enumerate() {
                ports[id].point = [
                    if *right { r.x + r.width } else { r.x }
                        + if !ports[id].boundary && ports[id].toward_target {
                            if *right {
                                OUTPUT_OFFSET
                            } else {
                                -OUTPUT_OFFSET
                            }
                        } else {
                            0.
                        },
                    r.y + inset + (r.height - 2. * inset) * (i + 1) as f64 / (count + 1) as f64,
                ];
            }
        }
        // Refine the fan against the allocated counterpart points, not just box
        // centers. Fixed passes preserve deterministic behavior for symmetric fans.
        for (path, _) in paths.values() {
            let source = ports[*path.first().unwrap()].point;
            let target = ports[*path.last().unwrap()].point;
            for index in path {
                // A composite's symbolic ports fan toward the internal endpoint,
                // avoiding arbitrary UUID ties between links with one external peer.
                ports[*index].other = if ports[*index].toward_target {
                    target
                } else {
                    source
                };
            }
        }
    }
    let mut occupied: Vec<(Point, Point)> = vec![];
    let mut paths = paths.into_iter().collect::<Vec<_>>();
    paths.sort_by(|(a, _), (b, _)| {
        let ea = &drawing.edges[a];
        let eb = &drawing.edges[b];
        (eb.source == eb.target)
            .cmp(&(ea.source == ea.target))
            .then_with(|| d.nodes[a].layout.y.total_cmp(&d.nodes[b].layout.y))
            .then_with(|| d.nodes[a].layout.x.total_cmp(&d.nodes[b].layout.x))
            .then_with(|| a.cmp(b))
    });
    let route_order = paths.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>();
    let mut self_counts: BTreeMap<String, usize> = BTreeMap::new();
    for (id, (path, common)) in paths {
        let self_link = drawing.edges[&id].source == drawing.edges[&id].target;
        let self_rank = if self_link {
            let count = self_counts
                .entry(drawing.edges[&id].source.clone())
                .or_default();
            let rank = *count;
            *count += 1;
            rank
        } else {
            0
        };
        let mut points = vec![];
        let mut region = ancestors(d, drawing, &drawing.edges[&id].source)
            .first()
            .cloned();
        for pair in path.windows(2) {
            let a = &ports[pair[0]];
            let b = &ports[pair[1]];
            let dir_a = if a.right { 1. } else { -1. };
            let dir_b = if b.right { 1. } else { -1. };
            let a_exit = !a.boundary || region.as_deref() != Some(&a.owner);
            let b_enter = !b.boundary || region.as_deref() != Some(&b.owner);
            let start = [
                a.point[0] + dir_a * STUB * if a_exit { 1. } else { -1. },
                a.point[1],
            ];
            let end = [
                b.point[0] + dir_b * STUB * if b_enter { 1. } else { -1. },
                b.point[1],
            ];
            let bounds = region
                .as_ref()
                .and_then(|p| drawing.shapes.get(p))
                .map(|s| expand(s.rect, -1.));
            let obstacles = drawing
                .shapes
                .iter()
                .filter(|(key, s)| {
                    if Some(key.as_str()) == region.as_deref() {
                        return false;
                    }
                    if s.composite
                        && s.expanded
                        && (ancestors(d, drawing, &a.owner).contains(key)
                            || ancestors(d, drawing, &b.owner).contains(key))
                    {
                        return false;
                    }
                    true
                })
                .map(|(_, s)| expand(s.rect, 8.))
                .chain(
                    drawing
                        .annotations
                        .values()
                        .filter(|v| !v.flag || self_link)
                        .map(|v| expand(v.rect, 4.)),
                )
                .collect::<Vec<_>>();
            let mut reserved = occupied.clone();
            reserved.extend([(a.point, start), (end, b.point)]);
            let mut segment = if self_link {
                self_loop(
                    a.point,
                    b.point,
                    drawing.shapes[&a.owner].rect,
                    self_rank,
                    &obstacles,
                    &occupied,
                    bounds,
                )
            } else {
                search(start, end, &obstacles, &reserved, bounds)
            };
            if segment.is_empty() && !self_link {
                // A second search includes the complete obstacle extent; it is
                // also useful for tightly packed imported layouts.
                segment =
                    search_with_margin(start, end, &obstacles, &reserved, bounds, f64::INFINITY);
            }
            if segment.is_empty() {
                segment = best_effort(start, end, &obstacles, &reserved, bounds);
            }
            if points.is_empty() {
                points.push(a.point);
            }
            points.extend(segment);
            points.push(b.point);
            if b.boundary {
                drawing.boundary_ports.push(BoundaryPort {
                    owner: b.owner.clone(),
                    edge: id.clone(),
                    point: b.point,
                    label: drawing.edges[&id].source_label.clone(),
                });
                if region.as_deref() == Some(&b.owner) {
                    region = ancestors(d, drawing, &b.owner)
                        .first()
                        .cloned()
                        .or(common.clone());
                } else {
                    region = Some(b.owner.clone());
                }
            }
        }
        simplify(&mut points);
        occupied.extend(points.windows(2).map(|p| (p[0], p[1])));
        let edge = drawing.edges.get_mut(&id).unwrap();
        edge.route_error = None;
        edge.points = points;
    }
    let routed = route_order
        .iter()
        .map(|id| (id.clone(), drawing.edges[id].points.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut previous: Vec<(String, Point, Point)> = vec![];
    for id in route_order {
        let edge = drawing.edges.get_mut(&id).unwrap();
        for s in edge.points.windows(2) {
            for (other, a, b) in &previous {
                if let Some(p) = intersection(s[0], s[1], *a, *b)
                    && [
                        edge.points.first(),
                        edge.points.last(),
                        routed[other].first(),
                        routed[other].last(),
                    ]
                    .into_iter()
                    .flatten()
                    .all(|q| length(p, *q) > 6.)
                    && !edge
                        .crossings
                        .iter()
                        .any(|q| (p[0] - q[0]).abs() + (p[1] - q[1]).abs() < 1.)
                {
                    edge.crossings.push(p);
                }
            }
        }
        previous.extend(edge.points.windows(2).map(|s| (id.clone(), s[0], s[1])));
    }
}
fn intersection(a: Point, b: Point, c: Point, d: Point) -> Option<Point> {
    let u = [b[0] - a[0], b[1] - a[1]];
    let v = [d[0] - c[0], d[1] - c[1]];
    let cross = |a: Point, b: Point| a[0] * b[1] - a[1] * b[0];
    let den = cross(u, v);
    if den.abs() < 1e-6 {
        return None;
    }
    let delta = [c[0] - a[0], c[1] - a[1]];
    let t = cross(delta, v) / den;
    let s = cross(delta, u) / den;
    if (-1e-9..=1. + 1e-9).contains(&t) && (-1e-9..=1. + 1e-9).contains(&s) {
        Some([a[0] + t * u[0], a[1] + t * u[1]])
    } else {
        None
    }
}
fn simplify(points: &mut Vec<Point>) {
    let mut result: Vec<Point> = vec![];
    for &p in points.iter() {
        if result.last() == Some(&p) {
            continue;
        }
        while result.len() > 1 {
            let a = result[result.len() - 2];
            let b = result[result.len() - 1];
            let u = [b[0] - a[0], b[1] - a[1]];
            let v = [p[0] - b[0], p[1] - b[1]];
            if (u[0] * v[1] - u[1] * v[0]).abs() < 1e-6 && u[0] * v[0] + u[1] * v[1] >= 0. {
                result.pop();
            } else {
                break;
            }
        }
        result.push(p);
    }
    *points = result;
}
// Reference candidate/scoring contract: Threadworks 2.6.7 GraphCanvas
// bestRouteBetween/routeCandidates/chamferOrthogonalTurns/routeCost.
const ROUTING_STEP: f64 = 40.;
const CHAMFER: f64 = 22.;
const BEND_COST: f64 = 18.;
const CROSSING_COST: f64 = 2500.;
const SCALE: f64 = 1000.;
const EPS: f64 = 1e-6;
fn length(a: Point, b: Point) -> f64 {
    (b[0] - a[0]).hypot(b[1] - a[1])
}
fn path_length(path: &[Point]) -> f64 {
    path.windows(2).map(|p| length(p[0], p[1])).sum()
}
fn octilinear(a: Point, b: Point) -> bool {
    let dx = (b[0] - a[0]).abs();
    let dy = (b[1] - a[1]).abs();
    dx < EPS || dy < EPS || (dx - dy).abs() < EPS
}
fn candidates(a: Point, b: Point) -> Vec<Vec<Point>> {
    let mut result = vec![vec![a, b]];
    let lanes = |start: f64, end: f64| {
        // Keep fractional world coordinates: unlike java.awt.Point, the canvas
        // need not round ports or move a route when the graph is translated.
        let mid = (start + end) / 2.;
        let mut lanes = (-7..=7)
            .map(|i| mid + f64::from(i) * ROUTING_STEP)
            .collect::<Vec<_>>();
        lanes.extend([
            start + ROUTING_STEP,
            start - ROUTING_STEP,
            end + ROUTING_STEP,
            end - ROUTING_STEP,
        ]);
        lanes.sort_by(|a, b| (a - mid).abs().total_cmp(&(b - mid).abs()));
        lanes.dedup();
        lanes
    };
    for x in lanes(a[0], b[0]) {
        result.push(vec![a, [x, a[1]], [x, b[1]], b]);
    }
    for y in lanes(a[1], b[1]) {
        result.push(vec![a, [a[0], y], [b[0], y], b]);
    }
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let diagonal = dx.abs().min(dy.abs());
    if diagonal > ROUTING_STEP {
        result.push(vec![
            a,
            [a[0] + dx.signum() * diagonal, a[1] + dy.signum() * diagonal],
            b,
        ]);
        result.push(vec![
            a,
            [b[0] - dx.signum() * diagonal, b[1] - dy.signum() * diagonal],
            b,
        ]);
    }
    result.into_iter().map(chamfer).collect()
}
fn chamfer(path: Vec<Point>) -> Vec<Point> {
    let sign = |v: f64| if v.abs() < EPS { 0. } else { v.signum() };
    let mut result = vec![path[0]];
    for p in path.windows(3) {
        let u = [p[1][0] - p[0][0], p[1][1] - p[0][1]];
        let v = [p[2][0] - p[1][0], p[2][1] - p[1][1]];
        if (u[0].abs() < EPS && v[1].abs() < EPS) || (u[1].abs() < EPS && v[0].abs() < EPS) {
            let cut = CHAMFER
                .min(length(p[0], p[1]) / 2.)
                .min(length(p[1], p[2]) / 2.);
            if cut >= 4. {
                result.push([p[1][0] - sign(u[0]) * cut, p[1][1] - sign(u[1]) * cut]);
                result.push([p[1][0] + sign(v[0]) * cut, p[1][1] + sign(v[1]) * cut]);
                continue;
            }
        }
        result.push(p[1]);
    }
    result.push(*path.last().unwrap());
    simplify(&mut result);
    result
}
fn self_loop(
    a: Point,
    b: Point,
    node: Rect,
    rank: usize,
    obstacles: &[Rect],
    occupied: &[(Point, Point)],
    bounds: Option<Rect>,
) -> Vec<Point> {
    // Feedback owns the nearest local envelope before ordinary channels route.
    // Grow only in small rings when a physical obstacle prevents that envelope.
    let base = STUB + rank as f64 * GAP * std::f64::consts::SQRT_2;
    let mut fallback = None;
    for ring in 0..=8 {
        let clearance = base + f64::from(ring) * GAP;
        let start = [a[0] + clearance, a[1]];
        let end = [b[0] - clearance, b[1]];
        let mut best = None;
        for y in [node.y - clearance, node.y + node.height + clearance] {
            let inner = chamfer(vec![start, [start[0], y], [end[0], y], end]);
            let own = expand(node, 8.);
            let others = obstacles
                .iter()
                .copied()
                .filter(|r| {
                    (r.x - own.x).abs()
                        + (r.y - own.y).abs()
                        + (r.width - own.width).abs()
                        + (r.height - own.height).abs()
                        > EPS
                })
                .collect::<Vec<_>>();
            let mut path = vec![a];
            path.extend_from_slice(&inner);
            path.push(b);
            simplify(&mut path);
            let soft = soft_cost(&path, &others, occupied, bounds);
            if fallback.as_ref().is_none_or(|(old, _)| soft < *old) {
                fallback = Some((soft, path.clone()));
            }
            if route_cost(&inner, obstacles, occupied, bounds).is_none() {
                continue;
            }
            if let Some(cost) = route_cost(&path, &others, occupied, bounds)
                && best.as_ref().is_none_or(|(old, _)| cost < *old)
            {
                best = Some((cost, path));
            }
        }
        if let Some((_, path)) = best {
            return path;
        }
    }
    fallback.map(|(_, path)| path).unwrap_or_default()
}

// Return None for coincident channel runs in *any* direction, including 45°.
// Near-parallel lanes retain the old spacing preference; crossings use the JVM
// penalty. These checks apply to final candidate geometry before it is reserved.
fn segment_penalty(a: Point, b: Point, occupied: &[(Point, Point)]) -> Option<f64> {
    let u = [b[0] - a[0], b[1] - a[1]];
    let len = length(a, b);
    if len < EPS {
        return Some(0.);
    }
    let mut penalty = 0.;
    for &(c, d) in occupied {
        let v = [d[0] - c[0], d[1] - c[1]];
        let other_len = length(c, d);
        if other_len < EPS {
            continue;
        }
        let w = [c[0] - a[0], c[1] - a[1]];
        let cross = |a: Point, b: Point| a[0] * b[1] - a[1] * b[0];
        let den = cross(u, v);
        if den.abs() / (len * other_len) < 1e-9 {
            let project = |p: Point| ((p[0] - a[0]) * u[0] + (p[1] - a[1]) * u[1]) / len;
            let p = project(c);
            let q = project(d);
            if p.max(q).min(len) - p.min(q).max(0.) > EPS {
                let distance = cross(w, u).abs() / len;
                if distance < EPS {
                    return None;
                }
                if distance < GAP - EPS {
                    penalty += 50.;
                }
            }
        } else {
            let t = cross(w, v) / den;
            let s = cross(w, u) / den;
            if (-1e-9..=1. + 1e-9).contains(&t)
                && (-1e-9..=1. + 1e-9).contains(&s)
                && ![c, d]
                    .into_iter()
                    .any(|p| length(a, p) < EPS || length(b, p) < EPS)
            {
                penalty += CROSSING_COST;
            }
        }
    }
    Some(penalty)
}
fn route_cost(
    path: &[Point],
    obstacles: &[Rect],
    occupied: &[(Point, Point)],
    bounds: Option<Rect>,
) -> Option<f64> {
    if path.iter().any(|p| bounds.is_some_and(|r| !r.contains(*p))) {
        return None;
    }
    let mut cost = path_length(path) + path.len().saturating_sub(2) as f64 * BEND_COST;
    for p in path.windows(2) {
        if !octilinear(p[0], p[1]) || obstacles.iter().any(|r| r.intersects_segment(p[0], p[1])) {
            return None;
        }
        cost += segment_penalty(p[0], p[1], occupied)?;
    }
    Some(cost)
}
fn soft_cost(
    path: &[Point],
    obstacles: &[Rect],
    occupied: &[(Point, Point)],
    bounds: Option<Rect>,
) -> f64 {
    let mut cost = path_length(path) + path.len().saturating_sub(2) as f64 * BEND_COST;
    for p in path.windows(2) {
        cost += obstacles
            .iter()
            .filter(|r| r.intersects_segment(p[0], p[1]))
            .count() as f64
            * 75000.;
        cost += segment_penalty(p[0], p[1], occupied).unwrap_or(10000.);
    }
    cost + path
        .iter()
        .filter(|p| bounds.is_some_and(|r| !r.contains(**p)))
        .count() as f64
        * 150000.
}
fn best_effort(
    a: Point,
    b: Point,
    obstacles: &[Rect],
    occupied: &[(Point, Point)],
    bounds: Option<Rect>,
) -> Vec<Point> {
    let mut paths = candidates(a, b);
    // Always include the two snapped direct alternatives, even below one lane step.
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let diagonal = dx.abs().min(dy.abs());
    paths.push(vec![
        a,
        [a[0] + dx.signum() * diagonal, a[1] + dy.signum() * diagonal],
        b,
    ]);
    paths.push(vec![
        a,
        [b[0] - dx.signum() * diagonal, b[1] - dy.signum() * diagonal],
        b,
    ]);
    let mut best = None;
    for mut path in paths {
        simplify(&mut path);
        if !path.windows(2).all(|p| octilinear(p[0], p[1])) {
            continue;
        }
        let cost = soft_cost(&path, obstacles, occupied, bounds);
        if best.as_ref().is_none_or(|(old, _)| cost < *old) {
            best = Some((cost, path));
        }
    }
    best.map(|(_, path)| path)
        .unwrap_or_else(|| vec![a, [b[0], a[1]], b])
}
fn search(
    a: Point,
    b: Point,
    obstacles: &[Rect],
    occupied: &[(Point, Point)],
    bounds: Option<Rect>,
) -> Vec<Point> {
    let mut best = None;
    for path in candidates(a, b) {
        if let Some(cost) = route_cost(&path, obstacles, occupied, bounds) {
            let len = path_length(&path);
            if best
                .as_ref()
                .is_none_or(|(c, l, _)| cost < *c || (cost == *c && len < *l))
            {
                best = Some((cost, len, path));
            }
        }
    }
    // Preserve the reference candidate choice when clear. The bounded visibility
    // search handles layouts for which none of those candidates is feasible.
    if let Some((_, _, path)) = best {
        return path;
    }
    search_with_margin(a, b, obstacles, occupied, bounds, 240.)
}
fn search_with_margin(
    a: Point,
    b: Point,
    obstacles: &[Rect],
    occupied: &[(Point, Point)],
    bounds: Option<Rect>,
    margin: f64,
) -> Vec<Point> {
    let corridor = expand(Rect::from_points(a, b), margin);
    let obstacles = obstacles
        .iter()
        .copied()
        .filter(|r| !margin.is_finite() || r.intersects(corridor))
        .collect::<Vec<_>>();
    let occupied = occupied
        .iter()
        .copied()
        .filter(|(p, q)| !margin.is_finite() || corridor.intersects_segment(*p, *q))
        .collect::<Vec<_>>();
    let mut xs = vec![a[0], b[0]];
    let mut ys = vec![a[1], b[1]];
    for r in &obstacles {
        xs.extend([r.x - GAP, r.x + r.width + GAP]);
        ys.extend([r.y - GAP, r.y + r.height + GAP]);
    }
    for (p, q) in &occupied {
        for p in [p, q] {
            xs.extend([p[0] - GAP, p[0] + GAP]);
            ys.extend([p[1] - GAP, p[1] + GAP]);
        }
    }
    if let Some(r) = bounds {
        xs.extend([r.x + GAP, r.x + r.width - GAP]);
        ys.extend([r.y + GAP, r.y + r.height - GAP]);
    }
    // Diagonal guide intersections from both endpoints make exact 45° paths
    // reachable even on the nonuniform obstacle grid (ports can be fractional).
    let base_x = xs.clone();
    let base_y = ys.clone();
    for p in [a, b] {
        for x in &base_x {
            let delta = (x - p[0]).abs();
            ys.extend([p[1] - delta, p[1] + delta]);
        }
        for y in &base_y {
            let delta = (y - p[1]).abs();
            xs.extend([p[0] - delta, p[0] + delta]);
        }
    }
    for (coords, anchors) in [(&mut xs, [a[0], b[0]]), (&mut ys, [a[1], b[1]])] {
        coords.retain(|v| v.is_finite());
        coords.sort_by(f64::total_cmp);
        coords.dedup_by(|a, b| (*a - *b).abs() < 1e-7);
        // Projection arithmetic can round a guide infinitesimally below a port.
        // Keep the exact endpoints even when approximate guide deduplication won.
        for anchor in anchors {
            if coords.binary_search_by(|v| v.total_cmp(&anchor)).is_err() {
                coords.push(anchor);
            }
        }
        coords.sort_by(f64::total_cmp);
        coords.dedup_by(|a, b| a.total_cmp(b).is_eq());
    }
    let count = xs.len().saturating_mul(ys.len());
    if count > 250_000 {
        return vec![];
    }
    let locate = |p: Point| {
        ys.binary_search_by(|v| v.total_cmp(&p[1])).unwrap() * xs.len()
            + xs.binary_search_by(|v| v.total_cmp(&p[0])).unwrap()
    };
    let point = |i: usize| [xs[i % xs.len()], ys[i / xs.len()]];
    // Consecutive visibility vertices on x-y and x+y lines add four real
    // diagonal moves. No post-search corner replacement changes reservations.
    let mut diagonal = vec![[None; 4]; count];
    for axis in 0..2 {
        let mut lines: BTreeMap<i64, Vec<usize>> = BTreeMap::new();
        for i in 0..count {
            let p = point(i);
            let key = if axis == 0 { p[0] - p[1] } else { p[0] + p[1] };
            lines.entry((key * 1e6).round() as i64).or_default().push(i);
        }
        for line in lines.values_mut() {
            line.sort_by(|a, b| point(*a)[0].total_cmp(&point(*b)[0]));
            for pair in line.windows(2) {
                if octilinear(point(pair[0]), point(pair[1])) {
                    diagonal[pair[0]][axis * 2] = Some(pair[1]);
                    diagonal[pair[1]][axis * 2 + 1] = Some(pair[0]);
                }
            }
        }
    }
    let start = locate(a);
    let goal = locate(b);
    // Four unoriented segment axes plus the initial no-direction state.
    let mut costs = vec![u64::MAX; count * 5];
    let mut prev = vec![usize::MAX; costs.len()];
    let mut heap = BinaryHeap::new();
    costs[start * 5 + 4] = 0;
    heap.push(std::cmp::Reverse((0u64, 0u64, start * 5 + 4)));
    let mut found = None;
    while let Some(std::cmp::Reverse((_, cost, state))) = heap.pop() {
        if costs[state] != cost {
            continue;
        }
        let i = state / 5;
        let direction = state % 5;
        if i == goal {
            found = Some(state);
            break;
        }
        let p = point(i);
        let x = i % xs.len();
        let y = i / xs.len();
        let neighbors = [
            x.checked_sub(1).map(|v| y * xs.len() + v),
            (x + 1 < xs.len()).then_some(i + 1),
            y.checked_sub(1).map(|v| v * xs.len() + x),
            (y + 1 < ys.len()).then_some(i + xs.len()),
            diagonal[i][0],
            diagonal[i][1],
            diagonal[i][2],
            diagonal[i][3],
        ];
        for next in neighbors.into_iter().flatten() {
            let q = point(next);
            let dir = if (p[0] - q[0]).abs() < EPS {
                1
            } else if (p[1] - q[1]).abs() < EPS {
                0
            } else if (q[0] - p[0]) * (q[1] - p[1]) > 0. {
                2
            } else {
                3
            };
            if bounds.is_some_and(|r| !r.contains(q))
                || obstacles.iter().any(|r| r.intersects_segment(p, q))
            {
                continue;
            }
            let Some(penalty) = segment_penalty(p, q, &occupied) else {
                continue;
            };
            let total = cost
                + ((length(p, q)
                    + penalty
                    + if direction != 4 && direction != dir {
                        BEND_COST
                    } else {
                        0.
                    })
                    * SCALE)
                    .ceil() as u64;
            let next_state = next * 5 + dir;
            if total < costs[next_state] {
                costs[next_state] = total;
                prev[next_state] = state;
                heap.push(std::cmp::Reverse((
                    total + (length(q, b) * SCALE).floor() as u64,
                    total,
                    next_state,
                )));
            }
        }
    }
    let Some(mut state) = found else {
        return vec![];
    };
    let mut path = vec![];
    loop {
        path.push(point(state / 5));
        if prev[state] == usize::MAX {
            break;
        }
        state = prev[state];
    }
    path.reverse();
    simplify(&mut path);
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn jvm_candidate_and_scoring_goldens_match() {
        let reference: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/router-reference.json")).unwrap();
        for cap in reference["portCaps"].as_array().unwrap() {
            let side = cap["side"].as_f64().unwrap();
            let outgoing = cap["outgoing"].as_bool().unwrap();
            let points = if outgoing {
                [[100., 100.], [100. + side * 100., 100.]]
            } else {
                [[100. + side * 100., 100.], [100., 100.]]
            };
            let (output, input) = crate::drawing::port_glyphs(&points);
            let actual = if outgoing { output } else { input };
            let expected: Vec<Point> = serde_json::from_value(cap["points"].clone()).unwrap();
            assert_eq!(cap["filled"], !outgoing);
            assert_eq!(actual.len(), expected.len());
            for (a, b) in actual.into_iter().zip(expected) {
                assert!(length(a, b) < 1e-9, "Port cap differs from JVM: {cap}");
            }
        }
        for case in reference["cases"].as_array().unwrap() {
            let a: Point = serde_json::from_value(case["start"].clone()).unwrap();
            let b: Point = serde_json::from_value(case["end"].clone()).unwrap();
            let obstacles = case["obstacles"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| Rect {
                    x: r[0].as_f64().unwrap(),
                    y: r[1].as_f64().unwrap(),
                    width: r[2].as_f64().unwrap(),
                    height: r[3].as_f64().unwrap(),
                })
                .collect::<Vec<_>>();
            let occupied: Vec<(Point, Point)> =
                serde_json::from_value(case["occupied"].clone()).unwrap();
            let expected: Vec<Point> =
                serde_json::from_value(case["expectedRoute"].clone()).unwrap();
            let route = search(a, b, &obstacles, &occupied, None);
            assert_eq!(route, expected, "Reference case {}", case["name"]);
        }
    }
    #[test]
    fn visibility_search_uses_long_diagonals_around_boxes_and_handles_fractional_ports() {
        let a = [0.1, 0.2];
        let b = [400.1, 240.2];
        let obstacle = Rect {
            x: 170.1,
            y: 60.2,
            width: 100.,
            height: 160.,
        };
        let exact = search_with_margin(a, b, &[obstacle], &[], None, 240.);
        assert_eq!(exact.first(), Some(&a));
        assert_eq!(exact.last(), Some(&b));
        for offset in (0..16)
            .map(|i| [f64::from(i) / 7., -f64::from(i) / 11.])
            .chain([[0., 0.], [0.25, -0.125], [10000.25, 20000.75]])
        {
            let a = offset;
            let b = [400. + offset[0], 240. + offset[1]];
            let obstacles = [Rect {
                x: 150. + offset[0],
                y: 60. + offset[1],
                width: 100.,
                height: 160.,
            }];
            let route = search_with_margin(a, b, &obstacles, &[], None, 240.);
            assert_eq!(route.first(), Some(&a));
            assert_eq!(route.last(), Some(&b));
            assert!(
                route_cost(&route, &obstacles, &[], None).is_some(),
                "{route:?}"
            );
            assert!(
                route
                    .windows(2)
                    .any(|p| (p[1][0] - p[0][0]).abs() > 40. && (p[1][1] - p[0][1]).abs() > 40.),
                "No substantial diagonal: {route:?}"
            );
        }
    }
    #[test]
    fn diagonal_lanes_cannot_overlap_and_crossings_have_the_same_cost_at_every_angle() {
        for (a, b, c, d) in [
            ([0., 0.], [200., 200.], [50., 50.], [150., 150.]),
            ([0., 200.], [200., 0.], [50., 150.], [150., 50.]),
            ([0., 0.], [200., 0.], [50., 0.], [150., 0.]),
            ([0., 0.], [0., 200.], [0., 50.], [0., 150.]),
        ] {
            assert!(segment_penalty(a, b, &[(c, d)]).is_none());
        }
        assert_eq!(
            segment_penalty([0., 0.], [200., 200.], &[([0., 200.], [200., 0.])]),
            Some(CROSSING_COST)
        );
        assert_eq!(
            segment_penalty([0., 100.], [200., 100.], &[([100., 0.], [100., 200.])]),
            Some(CROSSING_COST)
        );
        let occupied = [([0., 0.], [400., 400.])];
        let path = search([40., 40.], [300., 300.], &[], &occupied, None);
        assert!(path.len() > 2);
        assert!(route_cost(&path, &[], &occupied, None).is_some());
    }
}
