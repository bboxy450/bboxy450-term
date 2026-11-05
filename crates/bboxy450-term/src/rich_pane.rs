// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Disposable native face. IO, Markdown and HarnessView projection stay on workers.
//! UI drain allocation: 0.3ms aggregate. UI projection/composition: 0.6ms aggregate.
//! Transcript/cache allocation: 32MiB per view, not a measured heap bound.
use bboxy450_harness::frontends::gui::{
    bridge::Bridge,
    view::{Action, Frame, HarnessView},
};
use bbt_ui::{self as ui, components as c};
use bbt_ui_model::{
    draw::{DrawList, Prim},
    layout::Rect,
};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::{Notify, mpsc};
#[derive(Debug)]
pub enum Input {
    Edit(ui::text_edit::Command),
    Attach(bboxy450_harness::frontends::gui::attachments::Intake),
    Enter,
    Tab,
    Cancel,
    Click([f32; 2]),
    Wheel([f32; 2], f32),
    Wheel2D([f32; 2], [f32; 2]),
    Focus(bool),
    Size(Rect),
}
impl Input {
    /// Native paste and private fixtures enter the same identity-fenced worker queue.
    pub fn clipboard(content: bboxy450_term_desktop::ClipboardContent) -> Self {
        use bboxy450_harness::frontends::gui::attachments::Intake;
        use bboxy450_term_desktop::{ClipboardContent, ClipboardImageFormat};
        match content {
            ClipboardContent::Text(text) => Self::Edit(ui::text_edit::Command::Insert(text)),
            ClipboardContent::Image { bytes, format } => Self::Attach(match format {
                ClipboardImageFormat::Png => Intake::Png(bytes),
                ClipboardImageFormat::Tiff => Intake::EncodedImage {
                    bytes,
                    png: |bytes| {
                        bboxy450_term_desktop::clipboard_image_png(
                            bytes,
                            ClipboardImageFormat::Tiff,
                        )
                        .map_err(|e| e.to_string())
                    },
                },
            }),
        }
    }
}
#[derive(Clone, Default)]
pub struct Snapshot {
    pub theme: ui::Theme,
    pub appearance_revision: u64,
    pub paints: Arc<Vec<c::Paint>>,
    pub text_runs: Arc<Vec<ui::text_selection::Run>>,
    pub controls: Arc<Vec<(Rect, ui::cursor::Cursor)>>,
    pub links: Arc<ui::modules::document_modal::controller::Regions>,
    pub ready: bool,
    pub image_intake: bool,
    pub images: Vec<Arc<bbt_render::chrome::icons::Image>>,
    pub attachment_count: usize,
    pub pending_attachment_count: usize,
    pub revision: u64,
    pub generation: u64,
    /// Worker completion receipt, including rejected captured input.
    pub completed_inputs: u64,
    pub notice: Option<String>,
    pub rect: Option<Rect>,
    pub projection: Duration,
    pub retained_items: usize,
    pub order_len: usize,
    pub duplicate_durable: u64,
    pub send_rect: Option<Rect>,
    pub ime_caret: Option<Rect>,
    pub focused: bool,
    pub composer_len: usize,
    pub scroll: [f32; 2],
    pub approval_choices: Vec<[f32; 4]>,
    pub error: Option<String>,
    pub cursor: Option<(
        bboxy450_harness::protocol::SessionId,
        bboxy450_harness::protocol::Epoch,
        bboxy450_harness::protocol::Seq,
    )>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Identity {
    session: bboxy450_harness::protocol::SessionId,
    epoch: bboxy450_harness::protocol::Epoch,
    generation: u64,
}
struct Request {
    input: Input,
    identity: Option<Identity>,
}
impl Request {
    fn remote_allowed(&self, current: Option<Identity>, ready: bool, retired: bool) -> bool {
        let remote = matches!(
            &self.input,
            Input::Enter | Input::Click(_) | Input::Cancel | Input::Attach(_) | Input::Tab
        );
        !remote || (ready && !retired && self.identity.is_some() && self.identity == current)
    }
}
struct Signal(Arc<Notify>);
impl bbt_rt::Waker for Signal {
    fn wake(&self) {
        self.0.notify_one();
    }
}
/// Per-seat draft custody is handed off after accepted local edits drain.
/// Hidden views retain no transcript or layout worker.
pub type Draft = Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Receiver<bboxy450_harness::frontends::gui::view::Draft>>>>;
pub struct Pane {
    appearance: Arc<AtomicU64>,
    appearance_wake: Arc<Notify>,
    pub generation: u64,
    pub path: PathBuf,
    pub rect: Rect,
    pub snapshot: Snapshot,
    pub dirty: bbt_rt::DirtySet,
    latest: Arc<bbt_rt::Latest<Snapshot>>,
    commands: mpsc::Sender<Request>,
    _task: tokio::task::JoinHandle<()>,
    retired: Arc<AtomicBool>,
    published_inputs: Arc<AtomicU64>,
}
impl Drop for Pane {
    fn drop(&mut self) {
        self.retired.store(true, Ordering::Release);
        // Dropping the sender closes the queue after accepted local edits drain.
    }
}
impl Pane {
    pub fn connect(
        path: PathBuf,
        generation: u64,
        rect: Rect,
        after: Option<(
            bboxy450_harness::protocol::Epoch,
            bboxy450_harness::protocol::Seq,
        )>,
        waker: Arc<dyn bbt_rt::Waker>,
    ) -> Self {
        Self::connect_retained(path, generation, rect, after, waker, Draft::default())
    }
    pub fn connect_retained(
        path: PathBuf,
        generation: u64,
        rect: Rect,
        after: Option<(bboxy450_harness::protocol::Epoch, bboxy450_harness::protocol::Seq)>,
        waker: Arc<dyn bbt_rt::Waker>,
        draft: Draft,
    ) -> Self {
        let (save_draft, next_draft) = tokio::sync::oneshot::channel();
        // Reserve the next position synchronously, before either worker is scheduled.
        let previous_draft = draft.lock().unwrap().replace(next_draft);
        let dirty = bbt_rt::DirtySet::new(1, waker);
        let latest = Arc::new(bbt_rt::Latest::new(dirty.paint_source(0)));
        let (commands, mut input) = mpsc::channel(64);
        let appearance = Arc::new(AtomicU64::new(0));
        let appearance_wake = Arc::new(Notify::new());
        let worker_appearance = appearance.clone();
        let worker_appearance_wake = appearance_wake.clone();
        let mut out = latest.clone();
        let socket = path.clone();
        let retired = Arc::new(AtomicBool::new(false));
        let worker_retired = retired.clone();
        let published_inputs = Arc::new(AtomicU64::new(0));
        let worker_published = published_inputs.clone();
        let task=bbt_rt::handle().spawn(async move {
            let saved = match previous_draft { Some(previous) => previous.await.ok(), None => None };
            let notify=Arc::new(Notify::new());
            let mut bridge=Bridge::new(Arc::new(Signal(notify.clone())));
            let mut view=HarnessView::default();
            if let Some(draft) = saved { view.restore_draft(draft); }
            view.anim=bboxy450_harness::frontends::gui::anim::Clock::live(bboxy450_harness::frontends::gui::anim::Clock::reduced_motion_preferred());
            // Embedded text is one zoom step smaller; standalone keeps its own size.
            view.restore_zoom(view.zoom().step(-1),&bridge);
            bridge.connect(socket,after);
            let mut rect=rect;
            let mut completed_inputs=0_u64;
            let started=Instant::now();
            loop {
                let mut batch=Vec::new();
                let animation=view.anim.next_frame();
                tokio::select!{biased;cmd=input.recv()=>{let Some(cmd)=cmd else{break;};batch.push(cmd);},_=notify.notified()=>{},_=worker_appearance_wake.notified()=>{},_=async { match animation { Some(ms)=>tokio::time::sleep(Duration::from_secs_f32((ms/1000.).max(1./60.))).await,None=>std::future::pending::<()>().await }}=>{}}
                while batch.len()<64 {match input.try_recv(){Ok(cmd)=>batch.push(cmd),Err(_)=>break}}
                let appearance=worker_appearance.clone();
                let retired=worker_retired.clone();
                let published=worker_published.clone();
                let prepared=bboxy450_harness::runtime::blocking(move || {
                    view.anim.tick(started.elapsed().as_secs_f32()*1000.);
                    let appearance_revision=appearance.load(Ordering::Acquire);
                    let theme=if appearance_revision & 1 == 1 { ui::Theme::Light } else { ui::Theme::Dark };
                    view.set_theme(theme); bridge.set_theme(theme);
                    bridge.drain(Duration::from_micros(300));
                    view.update_inputs(&bridge);
                    for request in batch {
                        let request:Request=request;
                        completed_inputs+=1;
                        let current=bridge.state.transcript.session.map(|session|Identity{session,epoch:bridge.state.transcript.epoch,generation});
                        let same=request.identity==current;
                        if !request.remote_allowed(current,bridge.state.ready,retired.load(Ordering::Acquire)) {
                            view.notice=Some("Session changed or unavailable; wait for its frame before submitting".into());view.invalidate();continue;
                        }
                        if !same && request.identity.is_some() && matches!(request.input,Input::Edit(_)){continue;}
                        match request.input {
                        Input::Size(r)=>rect=r,
                        Input::Attach(source)=>view.intake(source,&bridge),
                        Input::Focus(f)=>{view.focused=f;view.invalidate();},
                        Input::Edit(cmd)=>{
                            let key=match &cmd{ui::text_edit::Command::Move{motion:ui::text_edit::Motion::Up,..}=>Some(ui::modules::approval::Key::Up),ui::text_edit::Command::Move{motion:ui::text_edit::Motion::Down,..}=>Some(ui::modules::approval::Key::Down),_=>None};
                            if !key.is_some_and(|key|view.approval_key(key,&bridge)||view.complete(key)){view.edit(cmd);}
                        },
                        Input::Tab=>{view.run_completion(&bridge);},
                        Input::Enter=>{if !view.approval_key(ui::modules::approval::Key::Enter,&bridge)&&!view.run_completion(&bridge){send_draft(&mut view,&bridge);}},
                        Input::Cancel=>{view.cancel_prompt(&bridge);},
                        // Adapted from bboxy450-harness gui/window.rs pointer routing.
                        // https://github.com/bboxy450/bboxy450-harness/blob/main/ATTRIBUTION.md
                        Input::Click(at)=>{
                            if let Some((_,action))=view.frame.actions.iter().find(|(r,_)|hit(*r,at)).cloned(){
                                if matches!(action,Action::Stop|Action::StopAt(_)){
                                    if matches!(action,Action::StopAt(epoch) if epoch!=bridge.state.transcript.epoch){view.notice=Some("Session replaced".into());view.invalidate();}else{let _=bridge.send(bboxy450_harness::protocol::Op::Interrupt{turn:None});}
                                }else{view.activate(action,&bridge);}
                            } else if let Some(layout)=&view.frame.layout && let Ok(regions)=ui::modules::composer::Regions::new(layout) {
                                match regions.hit(at) {
                                    Some(ui::modules::composer::Hit::Send)=>{if view.send_stops(&bridge.state){view.activate(Action::Interrupt,&bridge);}else{send_draft(&mut view,&bridge);}},
                                    Some(ui::modules::composer::Hit::Body)=>{view.composer.handle(ui::modules::composer::Command::Focus(ui::modules::composer::Field::Body));view.invalidate();},
                                    _=>{},
                                }
                            }
                        },
                        Input::Wheel(at,dy)=>{view.wheel(at,dy);},
                        Input::Wheel2D(at,delta)=>{view.wheel_delta(at,delta);},
                    }}
                    bridge.set_width(rect.w);
                    let at=Instant::now();
                    let (paints,error,send_rect)=match view.render(&bridge.state,[rect.w,rect.h]) {
                        Ok(frame)=>(Arc::new(paints(frame,rect)),bridge.state.error.clone(),frame.layout.as_ref().and_then(|l|bboxy450_harness::frontends::gui::view::find(l,"composer/send")).map(|n|n.rect)),Err(error)=>(Arc::default(),Some(error),None)
                    };
                    let text_runs=Arc::new(selection_runs(&view.frame,rect));
                    let controls=Arc::new(view.frame.actions.iter().map(|(rect,action)| (*rect, if matches!(action, Action::Menu(_)) {ui::cursor::Cursor::Default} else {ui::cursor::Cursor::Pointer})).collect());
                    let approval_choices=view.frame.actions.iter().filter_map(|(r,a)|matches!(a,Action::Approval(..)).then_some([r.x,r.y,r.w,r.h])).take(16).collect();
                    view.anim.frame_cost(at.elapsed().as_secs_f32()*1000.);
                    let ime_caret=view.frame.layout.as_ref().and_then(|l|ui::modules::composer::Regions::new(l).ok()).and_then(|r|r.caret);
                    let snapshot=Snapshot{theme,appearance_revision,text_runs,controls,links:view.frame.links.clone(),image_intake:bridge.image_intake_available(),images:view.attachment_images(),attachment_count:view.attachment_count(),pending_attachment_count:view.pending_attachment_count(),generation,completed_inputs,notice:view.notice.clone(),retained_items:bridge.state.transcript.items.len(),order_len:bridge.state.transcript.order.len(),duplicate_durable:bridge.state.transcript.duplicate_durable,send_rect,ime_caret,focused:view.focused,composer_len:view.composer.view.text.len(),scroll:[view.frame.scroll,view.frame.max_scroll],approval_choices,paints,ready:bridge.state.ready,revision:bridge.state.revision,rect:Some(rect),projection:at.elapsed(),cursor:bridge.state.transcript.session.map(|session|(session,bridge.state.transcript.epoch,bridge.state.transcript.cursor)),error};
                    out.publish(snapshot);
                    published.store(completed_inputs, Ordering::Release);
                    (bridge,view,rect,out,completed_inputs)
                }).await;
                let (b,v,r,o,c)=prepared;bridge=b;view=v;rect=r;out=o;completed_inputs=c;
            }
            let _ = save_draft.send(view.take_draft());
        });
        Self {
            appearance,
            appearance_wake,
            generation,
            path,
            rect,
            snapshot: Snapshot::default(),
            dirty,
            latest,
            commands,
            _task: task,
            retired,
            published_inputs,
        }
    }
    /// Latest local appearance bypasses the bounded input queue. No agent op,
    /// identity grant or OS lookup is involved; obsolete completed paint is fenced.
    pub fn set_theme(&self, theme: ui::Theme) -> bool {
        let light = u64::from(theme == ui::Theme::Light);
        let changed = self
            .appearance
            .try_update(Ordering::AcqRel, Ordering::Acquire, |old| {
                ((old & 1) != light).then(|| ((old >> 1).wrapping_add(1) << 1) | light)
            })
            .is_ok();
        if changed {
            self.appearance_wake.notify_one();
        }
        changed
    }
    pub fn send(&self, input: Input) -> bool {
        self.commands
            .try_send(Request {
                input,
                identity: self.snapshot.cursor.map(|(session, epoch, _)| Identity {
                    session,
                    epoch,
                    generation: self.snapshot.generation,
                }),
            })
            .is_ok()
    }
    /// Observe completed command publication without consuming the Latest slot.
    /// Used by unattended diagnostics to coordinate a pending-frame witness.
    pub fn published_inputs(&self) -> u64 {
        self.published_inputs.load(Ordering::Acquire)
    }
    pub fn resize(&mut self, rect: Rect) {
        if self.rect != rect && self.send(Input::Size(rect)) {
            self.rect = rect;
        }
    }
    /// Constant-work Latest exchange; callers share one aggregate deadline.
    pub fn drain(&mut self) -> bool {
        self.dirty.drain();
        let changed = if let Some(next) = self.latest.take() {
            self.snapshot = next;
            true
        } else {
            false
        };
        self.latest.complete_drain();
        changed
    }
    pub fn painted(&self) -> &[c::Paint] {
        if self.snapshot.rect == Some(self.rect)
            && self.snapshot.appearance_revision == self.appearance.load(Ordering::Acquire)
        {
            &self.snapshot.paints
        } else {
            &[]
        }
    }
}
fn send_draft(view: &mut HarnessView, bridge: &Bridge) {
    if view.composer.view.text.trim() == "/stop" {
        view.edit(ui::text_edit::Command::SelectAll);
        view.edit(ui::text_edit::Command::Insert("/interrupt".into()));
    }
    view.activate(Action::Send, bridge);
}
fn shift_rect(r: &mut Rect, at: [f32; 2]) {
    r.x += at[0];
    r.y += at[1];
}
fn shift_list(mut list: DrawList, at: [f32; 2]) -> DrawList {
    fn shift(p: &mut Prim, at: [f32; 2]) {
        match p {
            Prim::Path { origin, .. } | Prim::PathShadow { origin, .. } => {
                origin[0] += at[0];
                origin[1] += at[1];
            }
            Prim::Text {
                origin, baseline, ..
            }
            | Prim::TextShadow {
                origin, baseline, ..
            } => {
                origin[0] += at[0];
                origin[1] += at[1];
                if let Some(y) = baseline {
                    *y += at[1];
                }
            }
            Prim::Rect { rect, .. }
            | Prim::Gloss { rect, .. }
            | Prim::Icon { rect, .. }
            | Prim::Image { rect, .. }
            | Prim::Clip { rect, .. } => shift_rect(rect, at),
            Prim::Hairline { from, to, .. } => {
                for p in [from, to] {
                    p[0] += at[0];
                    p[1] += at[1];
                }
            }
            Prim::Group {
                primitives, mask, ..
            } => {
                for p in primitives {
                    shift(p, at);
                }
                if let Some(mask) = mask {
                    *mask = mask.translated(at);
                }
            }
            Prim::Unclip => {}
        }
    }
    for p in &mut list.primitives {
        fn names(p: &mut Prim) {
            match p {
                Prim::Icon { icon, .. } => *icon = format!("harness-rich/{icon}").into(),
                Prim::Image { image, .. } if image.as_ref() == "bboxy-mark" => {
                    *image = "harness-rich/bboxy-mark".into()
                }
                Prim::Group { primitives, .. } => {
                    for p in primitives {
                        names(p)
                    }
                }
                _ => {}
            }
        }
        names(p);
        shift(p, at);
    }
    list
}
/// Compose the shared native Frame with the existing term renderer, preserving rim order.
/// No alternate transcript renderer or PTY parsing is involved.
fn selection_runs(frame: &Frame, rect: Rect) -> Vec<ui::text_selection::Run> {
    fn selectable(node: &mut c::Layout) {
        if let c::LayoutKind::Named(name) = &node.kind
            && (name.starts_with("toolhead/")
                || name.starts_with("more/")
                || name.starts_with("approval/")
                || name.starts_with("codeleft/")
                || name.starts_with("coderight/")
                || name.ends_with("/toggle"))
        {
            node.children.clear();
            return;
        }
        if let c::LayoutKind::Text { selectable, .. } = &mut node.kind {
            *selectable = true;
        }
        for child in &mut node.children {
            selectable(child);
        }
    }
    let mut runs = Vec::new();
    if let Some(layout) = frame
        .layout
        .as_ref()
        .and_then(|layout| bboxy450_harness::frontends::gui::view::find(layout, "transcript"))
    {
        let mut layout = layout.clone();
        selectable(&mut layout);
        bboxy450_harness::frontends::gui::view::translate(&mut layout, rect.x, rect.y);
        ui::text_selection::collect(&layout, rect, &mut runs);
    }
    runs
}

pub fn paints(frame: &Frame, rect: Rect) -> Vec<c::Paint> {
    use bbt_render::chrome::materials as m;
    let mut first = DrawList::default();
    first.clip(rect, [0.; 4]);
    first
        .primitives
        .extend(shift_list(frame.list.clone(), [rect.x, rect.y]).primitives);
    first.unclip();
    let mut result = vec![c::Paint::Draw(first)];
    // Adapted from bboxy450-term src/gpu.rs::rim_material, in reverse, so the
    // shared Frame uses the existing ordered material pass without a renderer fork.
    // https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
    for surface in &frame.rims {
        if let m::Material::Rim(r) = surface.material {
            let mut bounds = surface.rect;
            shift_rect(&mut bounds, [rect.x, rect.y]);
            result.push(c::Paint::Material {
                rect: bounds,
                radii: surface.radii,
                kind: ui::Material::Rim,
                color: r.color,
                blur: None,
                rim: Some(ui::Rim {
                    activity: match r.activity {
                        m::Activity::Idle => ui::GlowActivity::Idle,
                        m::Activity::Active => ui::GlowActivity::Active,
                        m::Activity::Waiting => ui::GlowActivity::Waiting,
                        m::Activity::Errored => ui::GlowActivity::Errored,
                    },
                    inferred_multiplier: 1.,
                    focused: r.focus > 0.,
                    seed: r.phase_ms as u32,
                    tint: Some(r.color.0),
                    depth: Some(r.depth),
                    size: Some(r.size),
                    outer: Some(r.outer),
                    rim_alpha: Some(r.alpha),
                    swell: r.swell,
                    attention: r.attention,
                    ..Default::default()
                }),
                clips: std::iter::once(c::Clip {
                    rect,
                    radii: [0.; 4],
                })
                .chain(surface.clips.iter().map(|clip| {
                    let mut bounds = clip.rect;
                    shift_rect(&mut bounds, [rect.x, rect.y]);
                    c::Clip {
                        rect: bounds,
                        radii: clip.radii,
                    }
                }))
                .collect(),
            });
        }
    }
    let mut over = DrawList::default();
    over.clip(rect, [0.; 4]);
    over.primitives
        .extend(shift_list(frame.over.clone(), [rect.x, rect.y]).primitives);
    over.unclip();
    result.push(c::Paint::Draw(over));
    result
}

fn hit(r: Rect, at: [f32; 2]) -> bool {
    at[0] >= r.x && at[0] < r.right() && at[1] >= r.y && at[1] < r.bottom()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn draft_handoff_drains_last_edit_before_reopening_and_isolates_panels() {
        let temp = tempfile::tempdir().unwrap();
        let rect = Rect { x: 0., y: 0., w: 800., h: 650. };
        let a = Draft::default();
        let b = Draft::default();
        let wake: Arc<dyn bbt_rt::Waker> = Arc::new(Signal(Arc::new(Notify::new())));
        for (draft, text) in [(a.clone(), "first panel"), (b.clone(), "second panel")] {
            let pane = Pane::connect_retained(temp.path().join("missing.sock"), 1, rect, None, wake.clone(), draft);
            assert!(pane.send(Input::Edit(ui::text_edit::Command::Insert(text.into()))));
            drop(pane); // No wait for a snapshot: exercise queued input at detach.
        }
        let pane = Pane::connect_retained(temp.path().join("missing.sock"), 1, rect, None, wake, a.clone());
        assert!(pane.send(Input::Edit(ui::text_edit::Command::Insert(" retained".into()))));
        drop(pane);
        for (draft, expected) in [(a, "first panel retained"), (b, "second panel")] {
            let saved = draft.lock().unwrap().take().unwrap();
            let saved = tokio::time::timeout(Duration::from_secs(10), saved).await.unwrap().unwrap();
            let mut view = HarnessView::default();
            view.restore_draft(saved);
            assert_eq!(view.composer.view.text, expected);
        }
    }
    #[test]
    fn cancel_requires_complete_ready_nonretired_identity() {
        let identity = Identity {
            session: serde_json::from_str("\"00000000000000000000000001\"").unwrap(),
            epoch: bboxy450_harness::protocol::Epoch(1),
            generation: 1,
        };
        let request = Request {
            input: Input::Cancel,
            identity: Some(identity),
        };
        assert!(request.remote_allowed(Some(identity), true, false));
        assert!(!request.remote_allowed(Some(identity), false, false));
        assert!(!request.remote_allowed(Some(identity), true, true));
        assert!(!request.remote_allowed(None, true, false));
        for changed in [
            Identity {
                session: serde_json::from_str("\"00000000000000000000000002\"").unwrap(),
                ..identity
            },
            Identity {
                epoch: bboxy450_harness::protocol::Epoch(2),
                ..identity
            },
            Identity {
                generation: 2,
                ..identity
            },
        ] {
            assert!(!request.remote_allowed(Some(changed), true, false));
        }
    }
}
