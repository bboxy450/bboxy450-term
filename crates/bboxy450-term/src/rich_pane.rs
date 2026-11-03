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
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{Notify, mpsc};
#[derive(Debug)]
pub enum Input {
    Edit(ui::text_edit::Command),
    Enter,
    Cancel,
    Click([f32; 2]),
    Wheel([f32; 2], f32),
    Focus(bool),
    Size(Rect),
}
#[derive(Clone, Default)]
pub struct Snapshot {
    pub paints: Arc<Vec<c::Paint>>,
    pub ready: bool,
    pub revision: u64,
    pub rect: Option<Rect>,
    pub projection: Duration,
    pub retained_items: usize,
    pub order_len: usize,
    pub duplicate_durable: u64,
    pub send_rect: Option<Rect>,
    pub error: Option<String>,
    pub cursor: Option<(
        bboxy450_harness::protocol::SessionId,
        bboxy450_harness::protocol::Epoch,
        bboxy450_harness::protocol::Seq,
    )>,
}
struct Request {
    input: Input,
    session: Option<bboxy450_harness::protocol::SessionId>,
}
struct Signal(Arc<Notify>);
impl bbt_rt::Waker for Signal {
    fn wake(&self) {
        self.0.notify_one();
    }
}
pub struct Pane {
    pub generation: u64,
    pub path: PathBuf,
    pub rect: Rect,
    pub snapshot: Snapshot,
    pub dirty: bbt_rt::DirtySet,
    latest: Arc<bbt_rt::Latest<Snapshot>>,
    commands: mpsc::Sender<Request>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Pane {
    fn drop(&mut self) {
        self.task.abort();
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
        let dirty = bbt_rt::DirtySet::new(1, waker);
        let latest = Arc::new(bbt_rt::Latest::new(dirty.paint_source(0)));
        let (commands, mut input) = mpsc::channel(64);
        let mut out = latest.clone();
        let socket = path.clone();
        let task=bbt_rt::handle().spawn(async move {
            let notify=Arc::new(Notify::new());
            let mut bridge=Bridge::new(Arc::new(Signal(notify.clone())));
            bridge.connect(socket,after);
            let mut view=HarnessView::default();let mut rect=rect;
            loop {
                let mut batch=Vec::new();
                tokio::select!{biased;cmd=input.recv()=>{let Some(cmd)=cmd else{return;};batch.push(cmd);},_=notify.notified()=>{}}
                while batch.len()<64 {match input.try_recv(){Ok(cmd)=>batch.push(cmd),Err(_)=>break}}
                let prepared=bboxy450_harness::runtime::blocking(move || {
                    bridge.drain(Duration::from_micros(300));
                    for request in batch {
                        let request:Request=request;
                        let same=request.session==bridge.state.transcript.session;
                        if (!same || !bridge.state.ready) && matches!(request.input,Input::Enter|Input::Click(_)) {
                            view.notice=Some("Session changed or unavailable; wait for its frame before submitting".into());view.invalidate();continue;
                        }
                        if !same && request.session.is_some() && matches!(request.input,Input::Edit(_)){continue;}
                        match request.input {
                        Input::Size(r)=>rect=r,
                        Input::Focus(f)=>{view.focused=f;view.invalidate();},
                        Input::Edit(cmd)=>{
                            let key=match &cmd{ui::text_edit::Command::Move{motion:ui::text_edit::Motion::Up,..}=>Some(ui::modules::approval::Key::Up),ui::text_edit::Command::Move{motion:ui::text_edit::Motion::Down,..}=>Some(ui::modules::approval::Key::Down),_=>None};
                            if !key.is_some_and(|key|view.approval_key(key,&bridge)||view.complete(key)){view.edit(cmd);}
                        },
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
                    }}
                    bridge.set_width(rect.w);
                    let at=Instant::now();
                    let (paints,error,send_rect)=match view.render(&bridge.state,[rect.w,rect.h]) {
                        Ok(frame)=>(Arc::new(paints(frame,rect)),bridge.state.error.clone(),frame.layout.as_ref().and_then(|l|bboxy450_harness::frontends::gui::view::find(l,"composer/send")).map(|n|n.rect)),Err(error)=>(Arc::default(),Some(error),None)
                    };
                    let snapshot=Snapshot{retained_items:bridge.state.transcript.items.len(),order_len:bridge.state.transcript.order.len(),duplicate_durable:bridge.state.transcript.duplicate_durable,send_rect,paints,ready:bridge.state.ready,revision:bridge.state.revision,rect:Some(rect),projection:at.elapsed(),cursor:bridge.state.transcript.session.map(|session|(session,bridge.state.transcript.epoch,bridge.state.transcript.cursor)),error};
                    out.publish(snapshot);
                    (bridge,view,rect,out)
                }).await;
                let (b,v,r,o)=prepared;bridge=b;view=v;rect=r;out=o;
            }
        });
        Self {
            generation,
            path,
            rect,
            snapshot: Snapshot::default(),
            dirty,
            latest,
            commands,
            task,
        }
    }
    pub fn send(&self, input: Input) -> bool {
        self.commands
            .try_send(Request {
                input,
                session: self.snapshot.cursor.map(|(session, _, _)| session),
            })
            .is_ok()
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
        if self.snapshot.rect == Some(self.rect) {
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
