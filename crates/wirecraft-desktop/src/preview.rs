//! Placement and link previews: the design as the pending click would leave it.
//! A cheap outline follows the pointer at once; the routed projection of a cloned
//! document with the would-be entity or link follows in the background.
use super::*;
use std::time::Duration;
use wirecraft_core::{
    NodeKind,
    components::{EXTERIOR, EXTERIOR_DIAMETER},
};

/// What the pending click would create. Equal keys share one routed preview.
#[derive(Clone, PartialEq)]
pub(super) struct Key {
    revision: u64,
    parent: String,
    mode: Mode,
    source: Option<String>,
    exterior: Option<[f64; 2]>,
    target: Option<String>,
    /// A picked-up link end: the link and whether it is the source end.
    rewire: Option<(String, bool)>,
    /// Pointer position, for previews that are placed there.
    at: Option<[f64; 2]>,
}
impl Key {
    /// The same pending action, wherever the pointer is.
    fn same_action(&self, other: &Key) -> bool {
        Key {
            at: None,
            ..self.clone()
        } == Key {
            at: None,
            ..other.clone()
        }
    }
}
pub(super) struct Preview {
    key: Key,
    pub scene: Arc<scene::Scene>,
    /// IDs the click would create, highlighted in the preview.
    pub created: BTreeSet<String>,
}

/// Adds the would-be entities and link to a copy of the design.
fn build(
    document: &Document,
    key: &Key,
    appearance: wirecraft_runtime::config::Appearance,
) -> Option<Preview> {
    let mut d = document.clone();
    let mut created = BTreeSet::new();
    let exterior = |d: &mut Document, [x, y]: [f64; 2]| -> Option<String> {
        let id = d.create(&key.parent, "port", NodeKind::Node).ok()?;
        d.patch(&id, &json!({"stereotype":EXTERIOR,"layout":{"x":x,"y":y,"width":EXTERIOR_DIAMETER,"height":EXTERIOR_DIAMETER}}))
            .ok()?;
        Some(id)
    };
    let centred = |[x, y]: [f64; 2]| [x - EXTERIOR_DIAMETER / 2., y - EXTERIOR_DIAMETER / 2.];
    match key.mode {
        Mode::Node | Mode::Type => {
            let at = key.at?;
            let (name, kind) = if key.mode == Mode::Type {
                ("New type", NodeKind::Type)
            } else {
                ("New processor", NodeKind::Processor)
            };
            let id = d.create(&key.parent, name, kind).ok()?;
            d.patch(&id, &json!({"layout":{"x":at[0],"y":at[1]}}))
                .ok()?;
            created.insert(id);
        }
        Mode::Port => {
            created.insert(exterior(&mut d, centred(key.at?))?);
        }
        Mode::Link => {
            let source = match (&key.source, key.exterior) {
                (Some(id), _) => id.clone(),
                (None, Some(at)) => {
                    let id = exterior(&mut d, at)?;
                    created.insert(id.clone());
                    id
                }
                (None, None) => return None,
            };
            let target = match &key.target {
                Some(id) => id.clone(),
                None => {
                    let id = exterior(&mut d, centred(key.at?))?;
                    created.insert(id.clone());
                    id
                }
            };
            let publishes = key.exterior.is_some() || key.target.is_none();
            let link = wirecraft_core::Link {
                source_node_id: source,
                target_node_id: target,
                source_port_name: "out".into(),
                target_port_name: "in".into(),
                ..Default::default()
            };
            created.insert(
                d.create_link(if publishes { "port" } else { "packet" }, link)
                    .ok()?,
            );
            let _ = wirecraft_core::components::normalize(&mut d);
        }
        Mode::Select => {
            let (link, source) = key.rewire.clone()?;
            let entity = match &key.target {
                Some(id) => id.clone(),
                None => {
                    let id = exterior(&mut d, centred(key.at?))?;
                    created.insert(id.clone());
                    id
                }
            };
            d.reconnect(&link, source, &entity).ok()?;
            let _ = wirecraft_core::components::normalize(&mut d);
            created.insert(link);
        }
    }
    let mut scene = scene::Scene::new(&d, &key.parent).with_appearance(appearance);
    scene.pending = created.clone();
    Some(Preview {
        key: key.clone(),
        scene: Arc::new(scene),
        created,
    })
}

impl Wirecraft {
    /// Whether the pointer currently previews a placement or a link under construction.
    pub(super) fn placing(&self) -> bool {
        match self.mode {
            Mode::Node | Mode::Type | Mode::Port => true,
            Mode::Link => self.link_source.is_some() || self.exterior_source.is_some(),
            Mode::Select => self.rewire.is_some(),
        }
    }
    /// The selected link's ends, where a press picks one up.
    fn selected_link_ends(&self) -> Option<([f64; 2], [f64; 2])> {
        if self.mode != Mode::Select || self.rewire.is_some() || self.selected.len() != 1 {
            return None;
        }
        let id = self.selected.iter().next()?;
        let edge = self.scene.drawing.edges.get(id)?;
        if edge.annotation_only {
            return None;
        }
        Some((*edge.points.first()?, *edge.points.last()?))
    }
    pub(super) fn link_end_handles(&self) -> Vec<[f64; 2]> {
        self.selected_link_ends()
            .map(|(a, b)| vec![a, b])
            .unwrap_or_default()
    }
    /// Whether `world` presses the selected link's source end (true) or target end.
    pub(super) fn picked_link_end(&self, world: [f64; 2]) -> Option<bool> {
        let (a, b) = self.selected_link_ends()?;
        let reach = 9. / self.viewport.zoom;
        let near = |p: [f64; 2]| (p[0] - world[0]).hypot(p[1] - world[1]) <= reach;
        if near(a) {
            Some(true)
        } else if near(b) {
            Some(false)
        } else {
            None
        }
    }
    /// An entity under the pointer that a link click would target.
    fn hovered_target(&self, pointer: [f64; 2]) -> Option<String> {
        self.scene
            .pick(pointer, 4. / self.viewport.zoom, false)
            .filter(|id| Some(id) != self.link_source.as_ref())
            .filter(|id| self.scene.nodes.contains_key(id))
    }
    fn preview_key(&self) -> Option<Key> {
        if !self.placing() || self.drag_start.is_some() {
            return None;
        }
        let pointer = self.hover_world?;
        let linking = self.mode == Mode::Link || self.rewire.is_some();
        let target = linking.then(|| self.hovered_target(pointer)).flatten();
        // Empty canvas as both ends is not a link.
        if self.mode == Mode::Link && self.exterior_source.is_some() && target.is_none() {
            return None;
        }
        Some(Key {
            revision: self.snapshot.revision,
            parent: self.parent.clone(),
            mode: self.mode,
            source: self.link_source.clone().filter(|_| self.mode == Mode::Link),
            exterior: self.exterior_source.filter(|_| self.mode == Mode::Link),
            rewire: self.rewire.clone().filter(|_| self.mode == Mode::Select),
            at: target.is_none().then_some(pointer),
            target,
        })
    }
    /// The latest routed preview of the pending action, possibly a few pointer moves old.
    pub(super) fn current_preview(&self) -> Option<&Preview> {
        let key = self.preview_key()?;
        self.preview.as_ref().filter(|p| p.key.same_action(&key))
    }
    pub(super) fn queue_preview(&mut self, cx: &mut Context<Self>) {
        if self.preview_key().is_none()
            || self.preview.as_ref().map(|p| &p.key) == self.preview_key().as_ref()
        {
            return;
        }
        self.preview_timer = Some(cx.spawn(async move |this, cx| {
            Timer::after(Duration::from_millis(50)).await;
            let _ = this.update(cx, |this, cx| this.start_preview(cx));
        }));
    }
    fn start_preview(&mut self, cx: &mut Context<Self>) {
        if self.preview_inflight {
            return;
        }
        let Some(key) = self.preview_key() else {
            return;
        };
        let snapshot = self.snapshot.clone();
        let appearance = self.options.saved.appearance.clone();
        self.preview_inflight = true;
        let work = cx
            .background_executor()
            .spawn(async move { build(&snapshot.document, &key, appearance) });
        cx.spawn(async move |this, cx| {
            let preview = work.await;
            let _ = this.update(cx, |this, cx| {
                this.preview_inflight = false;
                this.preview = preview;
                // The pointer kept moving while routing: route its latest position too.
                this.queue_preview(cx);
                cx.notify();
            });
        })
        .detach();
    }
    /// Immediate overlay: the outline under the pointer and, until the routed preview
    /// arrives, a straight link from its source to the pointer.
    pub(super) fn rubber_band(&self) -> Option<scene::RubberBand> {
        if !self.placing() {
            return None;
        }
        let pointer = self.hover_world?;
        let circle = |[x, y]: [f64; 2]| Rect {
            x: x - EXTERIOR_DIAMETER / 2.,
            y: y - EXTERIOR_DIAMETER / 2.,
            width: EXTERIOR_DIAMETER,
            height: EXTERIOR_DIAMETER,
        };
        let mut band = scene::RubberBand::default();
        let target = (self.mode == Mode::Link || self.rewire.is_some())
            .then(|| self.hovered_target(pointer))
            .flatten();
        band.target = target
            .as_ref()
            .and_then(|id| self.scene.drawing.shapes.get(id))
            .map(|s| s.rect);
        // Routed: outline exactly what the preview would create.
        if let Some(preview) = self.current_preview() {
            for id in &preview.created {
                if let Some(shape) = preview.scene.drawing.shapes.get(id) {
                    band.ghosts.push((shape.rect, shape.exterior.is_some()));
                }
            }
            return Some(band);
        }
        match self.mode {
            Mode::Node | Mode::Type => {
                let layout = wirecraft_core::Layout::default();
                band.ghosts.push((
                    Rect {
                        x: pointer[0],
                        y: pointer[1],
                        width: layout.width,
                        height: layout.height,
                    },
                    false,
                ));
            }
            Mode::Port => band.ghosts.push((circle(pointer), true)),
            Mode::Link => {
                let start = if let Some(source) = &self.link_source {
                    let r = self.scene.drawing.shapes.get(source)?.rect;
                    // Leave from the side facing the pointer, like a data link's nearer side.
                    let right = pointer[0] >= r.x + r.width / 2.;
                    [if right { r.x + r.width } else { r.x }, r.y + r.height / 2.]
                } else {
                    let [x, y] = self.exterior_source?;
                    let centre = [x + EXTERIOR_DIAMETER / 2., y + EXTERIOR_DIAMETER / 2.];
                    band.ghosts.push((circle(centre), true));
                    let right = pointer[0] >= centre[0];
                    [
                        centre[0]
                            + if right {
                                EXTERIOR_DIAMETER / 2.
                            } else {
                                -EXTERIOR_DIAMETER / 2.
                            },
                        centre[1],
                    ]
                };
                if self.link_source.is_some() && target.is_none() {
                    // Releasing on empty canvas publishes an output port there.
                    band.ghosts.push((circle(pointer), true));
                }
                band.line = Some((start, pointer));
            }
            Mode::Select => {
                let (link, source) = self.rewire.as_ref()?;
                let points = &self.scene.drawing.edges.get(link)?.points;
                // The end that stays; the line keeps the link's flow direction.
                let fixed = if *source {
                    *points.last()?
                } else {
                    *points.first()?
                };
                if target.is_none() {
                    band.ghosts.push((circle(pointer), true));
                }
                band.line = Some(if *source {
                    (pointer, fixed)
                } else {
                    (fixed, pointer)
                });
            }
        }
        Some(band)
    }
}
