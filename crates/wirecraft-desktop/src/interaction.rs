//! Canvas/tree gestures and bounded asynchronous drag routing.
use super::*;
use std::time::Duration;

pub(super) fn selection_operation(modifiers: Modifiers) -> SelectionOperation {
    if modifiers.control {
        SelectionOperation::Remove
    } else if modifiers.shift {
        SelectionOperation::Add
    } else {
        SelectionOperation::Replace
    }
}
impl Wirecraft {
    fn local(&self, p: Point<Pixels>) -> [f64; 2] {
        let b = self.bounds.get();
        [
            f64::from(f32::from(p.x - b.origin.x)),
            f64::from(f32::from(p.y - b.origin.y)),
        ]
    }
    pub(super) fn down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus.focus(window);
        let local = self.local(event.position);
        if event.button == MouseButton::Right {
            self.cancel_pointer_gesture(cx);
            self.context_origin = Some(event.position);
            self.menu = Some("Context".into());
            self.tool("archetypes.list");
            cx.notify();
            return;
        }
        if event.button == MouseButton::Middle {
            self.pan_start = Some((local, self.viewport));
            return;
        }
        if self.has_draft(cx) || self.editor_request_pending {
            self.status = "Apply the editor draft before changing selection".into();
            cx.notify();
            return;
        }
        if self.move_pending {
            return;
        }
        let world = self.viewport.world(local);
        let tolerance = 6. / self.viewport.zoom;
        let hit = self.scene.pick(world, tolerance, self.mode == Mode::Select);
        match self.mode {
            Mode::Node | Mode::Type => {
                self.client.send("threadwork.create_entity",json!({"parentId":self.parent,"name":if self.mode==Mode::Type{"New type"}else{"New processor"},"kind":if self.mode==Mode::Type{"Type"}else{"Processor"},"attributes":{"layout":{"x":world[0],"y":world[1]}}}));
            }
            Mode::Link => {
                if let Some(id) = hit {
                    if let Some(source) = self.link_source.take() {
                        self.client.send("threadwork.create_link",json!({"name":"packet","sourceNodeId":source,"targetNodeId":id,"sourcePortName":"out","targetPortName":"in"}));
                    } else {
                        self.link_source = Some(id);
                        self.status = "Choose the target entity".into();
                    }
                }
            }
            Mode::Select => {
                if let Some(id) = hit {
                    let operation = interaction::selection_operation(event.modifiers);
                    self.pending_pick = None;
                    if event.modifiers.shift || event.modifiers.control {
                        self.selected = operation.apply(&self.selected, BTreeSet::from([id]));
                        self.load_editor(window, cx);
                        self.persist_ui(cx);
                        cx.notify();
                        return;
                    }
                    if self.selected.contains(&id) {
                        self.pending_pick = Some(id.clone());
                    } else {
                        self.selected = BTreeSet::from([id.clone()]);
                    }
                    self.load_editor(window, cx);
                    if let Some(shape) = self.scene.drawing.shapes.get(&id) {
                        let r = shape.rect;
                        if (world[0] - r.x - r.width).abs() < 10. / self.viewport.zoom
                            && (world[1] - r.y - r.height).abs() < 10. / self.viewport.zoom
                        {
                            self.resize_start = Some((id, world, r));
                            cx.notify();
                            return;
                        }
                        if shape.composite && world[0] < r.x + 28. && world[1] < r.y + 28. {
                            let expanded = !shape.expanded;
                            let layout = &self.snapshot.document.nodes[&id].layout;
                            self.client.send("threadwork.update_entity",json!({"nodeId":id,"patch":{"layout":{"isExpanded":expanded,"width":if expanded{layout.open_width}else{layout.closed_width},"height":if expanded{layout.open_height}else{layout.closed_height}}}}));
                            cx.notify();
                            return;
                        }
                    }
                    if event.click_count == 2 {
                        if self.snapshot.document.nodes[&id].is_composite() {
                            self.parent = id;
                            self.rebuild();
                            self.viewport = Viewport::default();
                        } else {
                            self.tab = "Editor".into();
                        }
                    } else {
                        if self.scene.nodes.contains_key(&id) {
                            self.drag_start = Some(world);
                            self.drag_ids = self
                                .selected
                                .iter()
                                .flat_map(|id| self.snapshot.document.descendants(id))
                                .collect();
                            self.drag_delta = [0., 0.];
                        }
                    }
                } else {
                    self.selection_operation = interaction::selection_operation(event.modifiers);
                    self.selection_base = self.selected.clone();
                    self.selected = self
                        .selection_operation
                        .apply(&self.selection_base, BTreeSet::new());
                    self.pending_pick = None;
                    self.selection_start = Some(world);
                    self.selection_end = Some(world);
                }
            }
        }
        cx.notify();
    }
    fn moved(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        let local = self.local(event.position);
        if let Some((start, view)) = self.pan_start {
            self.viewport.x = view.x + local[0] - start[0];
            self.viewport.y = view.y + local[1] - start[1];
            cx.notify();
        } else if let Some((_, start, _)) = &self.resize_start {
            let world = self.viewport.world(local);
            self.drag_delta = [world[0] - start[0], world[1] - start[1]];
            cx.notify();
        } else if let Some(start) = self.drag_start {
            let world = self.viewport.world(local);
            let delta = [world[0] - start[0], world[1] - start[1]];
            if delta[0].hypot(delta[1]) * self.viewport.zoom >= 3. || self.drag_delta != [0., 0.] {
                self.drag_delta = delta;
                self.pending_pick = None;
                self.queue_reroute(cx);
            }
            cx.notify();
        } else if let Some(start) = self.selection_start {
            let end = self.viewport.world(local);
            self.selection_end = Some(end);
            let hits = self.scene.window_selection(start, end);
            self.selected = self.selection_operation.apply(&self.selection_base, hits);
            cx.notify();
        }
    }
    fn up(&mut self, _: &MouseUpEvent, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((id, _, r)) = self.resize_start.take() {
            let n = &self.snapshot.document.nodes[&id];
            let width = (r.width + self.drag_delta[0]).max(100.);
            let height = (r.height + self.drag_delta[1]).max(70.);
            let mut layout = json!({"width":width,"height":height});
            if n.is_composite() && n.layout.is_expanded {
                layout["openWidth"] = json!(width);
                layout["openHeight"] = json!(height);
            } else {
                layout["closedWidth"] = json!(width);
                layout["closedHeight"] = json!(height);
            }
            self.client.send(
                "threadwork.update_entity",
                json!({"nodeId":id,"patch":{"layout":layout}}),
            );
        } else if self.drag_start.take().is_some()
            && (self.drag_delta[0].abs() > 0.5 || self.drag_delta[1].abs() > 0.5)
        {
            let mut ids = self.selected.clone();
            let mut stack = self.selected.iter().cloned().collect::<Vec<_>>();
            while let Some(id) = stack.pop() {
                if let Some(n) = self.snapshot.document.nodes.get(&id) {
                    for child in &n.children {
                        if ids.insert(child.clone()) {
                            stack.push(child.clone());
                        }
                    }
                }
            }
            let changes=ids.iter().filter_map(|id|self.snapshot.document.nodes.get(id)).filter(|n|!n.is_link()).map(|n|json!({"nodeId":n.id,"patch":{"layout":{"x":n.layout.x+self.drag_delta[0],"y":n.layout.y+self.drag_delta[1]}}})).collect::<Vec<_>>();
            let mut request = Request::new("entities.patch", json!({"changes":changes}));
            request.expected_revision = Some(self.snapshot.revision);
            self.move_pending = true;
            self.reroute_timer = None;
            self.reroute_epoch += 1;
            let _ = self.client.sender.send(Message::Execute(request));
        }
        if let Some(id) = self.pending_pick.take() {
            self.selected = BTreeSet::from([id]);
        }
        if !self.move_pending {
            self.clear_drag_preview();
        }
        // Panning is allowed while an editor draft exists. It must never reload it.
        if self.pan_start.is_none() {
            self.load_editor(window, cx);
        }
        self.pan_start = None;
        self.selection_start = None;
        self.selection_end = None;
        self.persist_ui(cx);
        cx.notify();
    }
    pub(super) fn wheel(
        &mut self,
        event: &ScrollWheelEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let delta = event
            .delta
            .pixel_delta(px(20. * self.options.saved.appearance.wheel_rows / 3.));
        {
            self.viewport.zoom_at(
                self.local(event.position),
                (f64::from(f32::from(delta.y)) * 0.005).exp(),
            );
        }
        self.persist_ui(cx);
        cx.notify();
    }

    pub(super) fn tree_pick(
        &mut self,
        id: String,
        modifiers: Modifiers,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.has_draft(cx) || self.editor_request_pending || self.move_pending {
            self.status = "Apply the editor draft before changing entity".into();
            cx.notify();
            return;
        }
        self.selected =
            selection_operation(modifiers).apply(&self.selected, BTreeSet::from([id.clone()]));
        self.reveal_tree_selection(window, cx);
        self.load_editor(window, cx);
        self.persist_ui(cx);
        cx.notify();
    }
    pub(super) fn reveal_tree_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected.len() != 1 {
            return;
        }
        let id = self.selected.first().unwrap();
        if !self.scene.contains_entity(id)
            && id != &self.snapshot.document.root_node_id
            && let Some(parent) = self
                .snapshot
                .document
                .nodes
                .get(id)
                .and_then(|n| n.parent_id.clone())
        {
            self.parent = parent;
            self.rebuild();
            self.fit(&Fit, window, cx);
        }
    }
    pub(super) fn reparent_from_tree(&mut self, id: &str, target: &str, cx: &mut Context<Self>) {
        if self.has_draft(cx) || self.editor_request_pending || self.move_pending {
            self.status = "Apply the editor draft before moving entities".into();
            cx.notify();
            return;
        }
        let document = &self.snapshot.document;
        if id == document.root_node_id || document.descendants(id).contains(target) {
            self.status = "Cannot move the root or drop an entity into its own subtree".into();
            cx.notify();
            return;
        }
        if document.nodes.get(target).is_none_or(|n| {
            n.is_link()
                || n.kind == wirecraft_core::NodeKind::Type
                || n.kind == wirecraft_core::NodeKind::Note
        }) {
            self.status = "Drop on a processing entity or composite".into();
            cx.notify();
            return;
        }
        if document
            .nodes
            .get(id)
            .is_some_and(|n| n.parent_id.as_deref() == Some(target))
        {
            return;
        }
        self.selected = BTreeSet::from([id.into()]);
        self.collapsed.remove(target);
        let mut request = Request::new(
            "threadwork.move_entity",
            json!({"nodeId":id,"parentId":target}),
        );
        request.expected_revision = Some(self.snapshot.revision);
        let _ = self.client.sender.send(Message::Execute(request));
        self.persist_ui(cx);
        cx.notify();
    }
    pub(super) fn pointer_moved(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.pressed_button.is_none()
            && (self.panel_drag.is_some()
                || self.pan_start.is_some()
                || self.drag_start.is_some()
                || self.resize_start.is_some()
                || self.selection_start.is_some())
        {
            let release = MouseUpEvent {
                position: event.position,
                modifiers: event.modifiers,
                button: if self.pan_start.is_some() {
                    MouseButton::Middle
                } else {
                    MouseButton::Left
                },
                ..Default::default()
            };
            self.pointer_up(&release, window, cx);
            return;
        }
        self.move_panel(event, window, cx);
        if self.pan_start.is_some()
            || self.drag_start.is_some()
            || self.resize_start.is_some()
            || self.selection_start.is_some()
        {
            self.moved(event, window, cx);
        }
    }
    pub(super) fn pointer_up(
        &mut self,
        event: &MouseUpEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.button == MouseButton::Left {
            self.end_panel(event, window, cx);
        }
        if self.pan_start.is_some()
            || (event.button == MouseButton::Left
                && (self.drag_start.is_some()
                    || self.resize_start.is_some()
                    || self.selection_start.is_some()
                    || self.pending_pick.is_some()))
        {
            self.up(event, window, cx);
        }
    }
    pub(super) fn cancel_pointer_gesture(&mut self, cx: &mut Context<Self>) {
        if self.selection_start.is_some() {
            self.selected = self.selection_base.clone();
        }
        self.pan_start = None;
        self.drag_start = None;
        self.resize_start = None;
        self.selection_start = None;
        self.selection_end = None;
        self.pending_pick = None;
        self.panel_drag = None;
        if !self.move_pending {
            self.clear_drag_preview();
        }
        cx.notify();
    }
    pub(super) fn pointer_capture(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity().downgrade();
        canvas(
            |_, _, _| (),
            move |_, _, window, _| {
                let moved = entity.clone();
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, w, cx| {
                    if phase == DispatchPhase::Capture {
                        let _ = moved.update(cx, |this, cx| this.pointer_moved(event, w, cx));
                    }
                });
                let released = entity.clone();
                window.on_mouse_event(move |event: &MouseUpEvent, phase, w, cx| {
                    if phase == DispatchPhase::Capture {
                        let _ = released.update(cx, |this, cx| this.pointer_up(event, w, cx));
                    }
                });
            },
        )
        .absolute()
        .inset_0()
        .size_full()
    }
    pub(super) fn clear_drag_preview(&mut self) {
        self.reroute_timer = None;
        self.reroute_epoch += 1;
        self.drag_scene = None;
        self.drag_delta = [0., 0.];
        self.drag_ids.clear();
    }
    pub(super) fn queue_reroute(&mut self, cx: &mut Context<Self>) {
        // Reset the quiet-period timer. At most one CPU projection runs at a time.
        self.reroute_timer = Some(cx.spawn(async move |this, cx| {
            Timer::after(Duration::from_millis(80)).await;
            let _ = this.update(cx, |this, cx| this.start_reroute(cx));
        }));
    }
    fn start_reroute(&mut self, cx: &mut Context<Self>) {
        if self.reroute_inflight || self.drag_start.is_none() || self.drag_delta == [0., 0.] {
            return;
        }
        let snapshot = self.snapshot.clone();
        let revision = snapshot.revision;
        let parent = self.parent.clone();
        let ids = self.drag_ids.clone();
        let delta = self.drag_delta;
        let epoch = self.reroute_epoch;
        let appearance = self.options.saved.appearance.clone();
        self.reroute_inflight = true;
        let work = cx.background_executor().spawn(async move {
            let mut document = snapshot.document.clone();
            for id in ids {
                if let Some(n) = document.nodes.get_mut(&id)
                    && !n.is_link()
                {
                    n.layout.x += delta[0];
                    n.layout.y += delta[1];
                }
            }
            Arc::new(scene::Scene::new(&document, &parent).with_appearance(appearance))
        });
        cx.spawn(async move |this, cx| {
            let scene = work.await;
            let _ = this.update(cx, |this, cx| {
                this.reroute_inflight = false;
                if this.reroute_epoch == epoch
                    && this.snapshot.revision == revision
                    && this.drag_start.is_some()
                {
                    this.drag_scene = Some((scene, delta));
                }
                if this.drag_start.is_some()
                    && this
                        .drag_scene
                        .as_ref()
                        .is_none_or(|(_, d)| *d != this.drag_delta)
                {
                    this.queue_reroute(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }
}
