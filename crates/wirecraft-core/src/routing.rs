//! Deterministic octilinear routing. Port placement precedes routing so neighboring
//! channels never share a port or a composite boundary crossing.
use crate::{
    Document,
    drawing::{BoundaryPort, Drawing},
    viewport::Rect,
};
use std::collections::{BTreeMap, BinaryHeap};

const GAP: f64 = 10.;
const STUB: f64 = 20.;
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
                angle(&ports[*a])
                    .total_cmp(&angle(&ports[*b]))
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
                    if *right { r.x + r.width } else { r.x },
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
    for (id, (path, common)) in paths {
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
                        .filter(|v| !v.flag)
                        .map(|v| expand(v.rect, 4.)),
                )
                .collect::<Vec<_>>();
            let mut segment = search(start, end, &obstacles, &occupied, bounds);
            if segment.is_empty() {
                // A second search includes the complete obstacle extent; it is
                // also useful for tightly packed imported layouts.
                segment =
                    search_with_margin(start, end, &obstacles, &occupied, bounds, f64::INFINITY);
            }
            if segment.is_empty() {
                points.clear();
                break;
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
        if points.len() < 2 {
            edge.route_error = Some(format!(
                "No clear channel for {}; move overlapping boxes or increase spacing",
                edge.name
            ));
        }
        edge.points = points;
    }
    // Cut orthogonal corners into 45 degree runs, without reducing the obstacle
    // clearance. Routing uses the reserved Manhattan lanes; these short diagonal
    // cuts stay inside the same corner envelope.
    let obstacles = drawing
        .shapes
        .values()
        .filter(|s| !s.composite || !s.expanded)
        .map(|s| expand(s.rect, 8.))
        .collect::<Vec<_>>();
    let original = drawing
        .edges
        .iter()
        .map(|(id, e)| (id.clone(), e.points.clone()))
        .collect::<Vec<_>>();
    for (id, edge) in &mut drawing.edges {
        let mut result = vec![];
        for (i, &p) in edge.points.iter().enumerate() {
            if i == 0 || i + 1 == edge.points.len() {
                result.push(p);
                continue;
            }
            let a = edge.points[i - 1];
            let b = edge.points[i + 1];
            let da = [a[0] - p[0], a[1] - p[1]];
            let db = [b[0] - p[0], b[1] - p[1]];
            let la = da[0].abs() + da[1].abs();
            let lb = db[0].abs() + db[1].abs();
            let cut = 18_f64.min(la / 3.).min(lb / 3.);
            let before = [p[0] + da[0] / la * cut, p[1] + da[1] / la * cut];
            let after = [p[0] + db[0] / lb * cut, p[1] + db[1] / lb * cut];
            let clear = cut > 1.
                && !obstacles
                    .iter()
                    .any(|r| r.intersects_segment(before, after))
                && !original
                    .iter()
                    .filter(|(key, _)| key != id)
                    .any(|(_, path)| {
                        path.windows(2).any(|s| {
                            Rect::from_points(before, after).intersects_segment(s[0], s[1])
                        })
                    });
            if clear {
                result.extend([before, after]);
            } else {
                result.push(p);
            }
        }
        edge.points = result;
    }
    let mut previous: Vec<(Point, Point)> = vec![];
    for edge in drawing.edges.values_mut().filter(|e| !e.annotation_only) {
        for s in edge.points.windows(2) {
            for &(a, b) in &previous {
                if let Some(p) = intersection(s[0], s[1], a, b)
                    && !edge
                        .crossings
                        .iter()
                        .any(|q| (p[0] - q[0]).abs() + (p[1] - q[1]).abs() < 1.)
                {
                    edge.crossings.push(p);
                }
            }
        }
        previous.extend(edge.points.windows(2).map(|s| (s[0], s[1])));
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
    let lu = u[0].hypot(u[1]);
    let lv = v[0].hypot(v[1]);
    if t * lu > 6. && (1. - t) * lu > 6. && s * lv > 6. && (1. - s) * lv > 6. {
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
            if ((a[0] - b[0]).abs() < 1e-6
                && (b[0] - p[0]).abs() < 1e-6
                && (b[1] - a[1]) * (p[1] - b[1]) >= 0.)
                || ((a[1] - b[1]).abs() < 1e-6
                    && (b[1] - p[1]).abs() < 1e-6
                    && (b[0] - a[0]) * (p[0] - b[0]) >= 0.)
            {
                result.pop();
            } else {
                break;
            }
        }
        result.push(p);
    }
    *points = result;
}
fn search(
    a: Point,
    b: Point,
    obstacles: &[Rect],
    occupied: &[(Point, Point)],
    bounds: Option<Rect>,
) -> Vec<Point> {
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
    for coords in [&mut xs, &mut ys] {
        coords.retain(|v| v.is_finite());
        coords.sort_by(f64::total_cmp);
        coords.dedup_by(|a, b| (*a - *b).abs() < 1e-7);
    }
    // Bound pathological imported scenes; normal local corridors are much smaller.
    if xs.len().saturating_mul(ys.len()) > 250_000 {
        return vec![];
    }
    let locate = |p: Point| -> usize {
        ys.binary_search_by(|v| v.total_cmp(&p[1])).unwrap() * xs.len()
            + xs.binary_search_by(|v| v.total_cmp(&p[0])).unwrap()
    };
    let point = |i: usize| [xs[i % xs.len()], ys[i / xs.len()]];
    let start = locate(a);
    let goal = locate(b);
    let mut costs = vec![u64::MAX; xs.len() * ys.len() * 3];
    let mut prev = vec![usize::MAX; costs.len()];
    let mut heap = BinaryHeap::new();
    costs[start * 3 + 2] = 0;
    heap.push(std::cmp::Reverse((0u64, 0u64, start * 3 + 2)));
    let mut found = None;
    while let Some(std::cmp::Reverse((_, cost, state))) = heap.pop() {
        if costs[state] != cost {
            continue;
        }
        let i = state / 3;
        let direction = state % 3;
        if i == goal {
            found = Some(state);
            break;
        }
        let p = point(i);
        let x = i % xs.len();
        let y = i / xs.len();
        for next in [
            x.checked_sub(1).map(|v| y * xs.len() + v),
            (x + 1 < xs.len()).then_some(i + 1),
            y.checked_sub(1).map(|v| v * xs.len() + x),
            (y + 1 < ys.len()).then_some(i + xs.len()),
        ]
        .into_iter()
        .flatten()
        {
            let q = point(next);
            let dir = if (p[0] - q[0]).abs() < 1e-7 { 1 } else { 0 };
            if bounds.is_some_and(|r| !r.contains(q))
                || obstacles.iter().any(|r| r.intersects_segment(p, q))
            {
                continue;
            }
            let mut penalty = 0;
            let mut blocked = false;
            for &(c, d) in &occupied {
                let vertical = (c[0] - d[0]).abs() < 1e-7;
                if vertical == (dir == 1) {
                    let axis = if vertical { 1 } else { 0 };
                    let cross = 1 - axis;
                    let overlap = p[axis].max(q[axis]).min(c[axis].max(d[axis]))
                        - p[axis].min(q[axis]).max(c[axis].min(d[axis]));
                    if overlap > 1e-6 {
                        let distance = (p[cross] - c[cross]).abs();
                        if distance < 1e-6 {
                            blocked = true;
                            break;
                        }
                        if distance < GAP - 1e-6 {
                            penalty += 500;
                        }
                    }
                } else if Rect::from_points(c, d).intersects_segment(p, q) {
                    penalty += 800;
                }
            }
            if blocked {
                continue;
            }
            let length = ((p[0] - q[0]).abs() + (p[1] - q[1]).abs()) * 10.;
            let total = cost
                + length.ceil() as u64
                + penalty
                + if direction != 2 && direction != dir {
                    250
                } else {
                    0
                };
            let state_next = next * 3 + dir;
            if total < costs[state_next] {
                costs[state_next] = total;
                prev[state_next] = state;
                let estimate = ((q[0] - b[0]).abs() + (q[1] - b[1]).abs()) * 10.;
                heap.push(std::cmp::Reverse((
                    total + estimate.floor() as u64,
                    total,
                    state_next,
                )));
            }
        }
    }
    let Some(mut state) = found else {
        return vec![];
    };
    let mut path = vec![];
    loop {
        path.push(point(state / 3));
        if prev[state] == usize::MAX {
            break;
        }
        state = prev[state];
    }
    path.reverse();
    simplify(&mut path);
    path
}
