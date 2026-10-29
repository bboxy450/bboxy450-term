// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! A bounded consumer of the document paint that is actually submitted.
//! Named retained leaves supply keys; native draw primitives supply all paint
//! facts. This neither reparses Markdown nor runs another diagram layout.
use bbt_mux::Ulid;
use bbt_proto::gui::*;
use bbt_ui::components as c;
use bbt_ui_model::{draw::Prim, layout::Rect};

const WALK_LIMIT: usize = 4096;
const DEPTH_LIMIT: usize = 64;

pub fn admitted_test_run(
    identity: Option<&bbt_proto::GuiIdentity>,
    headless: bool,
) -> Option<Ulid> {
    use bbt_proto::GuiIdentityState::{Certified, LocalDev};
    if !cfg!(debug_assertions) || !headless {
        return None;
    }
    let identity = identity?;
    if !matches!(identity.state, Certified | LocalDev) {
        return None;
    }
    identity.private_test_run.filter(|id| *id != Ulid::nil())
}

struct TextKey<'a> {
    key: &'a str,
    run: &'a c::PositionedRun,
}
struct PathKey<'a> {
    key: &'a str,
    points: *const c::PathCommand,
    origin: [f32; 2],
}
#[derive(Default)]
struct Keys<'a> {
    texts: Vec<TextKey<'a>>,
    paths: Vec<PathKey<'a>>,
    left: usize,
    truncated: bool,
}
impl<'a> Keys<'a> {
    fn from_layout(root: &'a c::Layout) -> Self {
        let mut keys = Self {
            left: WALK_LIMIT,
            ..Default::default()
        };
        keys.visit(root, "document", 0);
        keys
    }
    fn visit(&mut self, node: &'a c::Layout, mut key: &'a str, depth: usize) {
        if self.left == 0 || depth == DEPTH_LIMIT {
            self.truncated = true;
            return;
        }
        self.left -= 1;
        if let c::LayoutKind::Named(name) = &node.kind {
            key = name;
        }
        match &node.kind {
            c::LayoutKind::Text { runs, .. } => {
                for run in runs {
                    if self.texts.len() == DOCUMENT_TEXT_RUNS {
                        self.truncated = true;
                        break;
                    }
                    self.texts.push(TextKey { key, run });
                }
            }
            c::LayoutKind::Path { points, .. } => {
                if self.paths.len() == DOCUMENT_PATHS {
                    self.truncated = true;
                } else {
                    self.paths.push(PathKey {
                        key,
                        points: points.as_ptr(),
                        origin: [node.rect.x, node.rect.y],
                    });
                }
            }
            _ => {}
        }
        for child in &node.children {
            if self.left == 0 {
                self.truncated = true;
                break;
            }
            self.visit(child, key, depth + 1);
        }
    }
    fn text(&self, value: &str, origin: [f32; 2]) -> Option<&TextKey<'_>> {
        self.texts.iter().find(|candidate| {
            let run = candidate.run;
            run.text == value
                && near(run.rect.x, origin[0])
                && near(
                    run.rect.y + (run.line_height - run.style.size * 1.4) * 0.5,
                    origin[1],
                )
        })
    }
    fn path(&self, points: *const c::PathCommand, origin: [f32; 2]) -> Option<&PathKey<'_>> {
        self.paths.iter().find(|p| {
            p.points == points && near(p.origin[0], origin[0]) && near(p.origin[1], origin[1])
        })
    }
}
fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.001
}
fn rectangle(rect: Rect) -> [f32; 4] {
    [rect.x, rect.y, rect.w, rect.h]
}
fn intersection(clips: &[UiDocumentClip]) -> Option<[f32; 4]> {
    clips.iter().map(|c| c.rect).reduce(|a, b| {
        let x = a[0].max(b[0]);
        let y = a[1].max(b[1]);
        [
            x,
            y,
            (a[0] + a[2]).min(b[0] + b[2]).max(x) - x,
            (a[1] + a[3]).min(b[1] + b[3]).max(y) - y,
        ]
    })
}
struct Collector<'a> {
    keys: Keys<'a>,
    observed: UiDocumentObservation,
    left: usize,
    commands: usize,
    bytes: usize,
    stopped: bool,
}
impl Collector<'_> {
    fn clip_ids(&mut self, clips: &[UiDocumentClip]) -> Option<Vec<u16>> {
        let mut ids = Vec::with_capacity(clips.len());
        for clip in clips {
            let index = if let Some(index) = self.observed.clips.iter().position(|c| c == clip) {
                index
            } else {
                if self.observed.clips.len() == DOCUMENT_CLIPS || !self.room(clip) {
                    self.observed.omitted.walk_truncated = true;
                    self.stopped = true;
                    return None;
                }
                let index = self.observed.clips.len();
                self.observed.clips.push(clip.clone());
                index
            };
            ids.push(index as u16);
        }
        Some(ids)
    }
    fn room<T: serde::Serialize>(&mut self, value: &T) -> bool {
        let bytes =
            serde_json::to_vec(value).map_or(DOCUMENT_OBSERVATION_BYTES + 1, |v| v.len() + 1);
        if self.bytes.saturating_add(bytes) > DOCUMENT_OBSERVATION_BYTES - 512 {
            self.observed.omitted.bytes = self.observed.omitted.bytes.saturating_add(bytes as u32);
            self.stopped = true;
            return false;
        }
        self.bytes += bytes;
        true
    }
    fn take(&mut self, depth: usize) -> bool {
        if self.stopped || self.left == 0 || depth == DEPTH_LIMIT {
            self.observed.omitted.walk_truncated = true;
            return false;
        }
        self.left -= 1;
        true
    }
    fn paints(&mut self, paints: &[c::Paint], opacity: f32, depth: usize) {
        for paint in paints {
            if !self.take(depth) {
                break;
            }
            match paint {
                c::Paint::Group {
                    ops,
                    opacity: alpha,
                    saturation,
                    mask,
                } => {
                    if *saturation != 1. || mask.is_some() {
                        self.observed.omitted.unsupported_paint = true;
                    }
                    self.paints(ops, opacity * alpha, depth + 1);
                }
                c::Paint::Draw(list) | c::Paint::Underlay(list) => {
                    self.primitives(&list.primitives, &[], opacity, depth + 1)
                }
                c::Paint::Material { .. } => {}
            }
        }
    }
    fn primitives(
        &mut self,
        primitives: &[Prim],
        parent: &[UiDocumentClip],
        opacity: f32,
        depth: usize,
    ) {
        let mut clips = parent.to_vec();
        for primitive in primitives {
            if !self.take(depth) {
                break;
            }
            match primitive {
                Prim::Clip { rect, radii } => {
                    if clips.len() == 16 {
                        self.observed.omitted.walk_truncated = true;
                        self.stopped = true;
                        break;
                    }
                    clips.push(UiDocumentClip {
                        rect: rectangle(*rect),
                        radii: *radii,
                    });
                }
                Prim::Unclip => {
                    clips.pop();
                }
                Prim::Group {
                    primitives,
                    opacity: alpha,
                    saturation,
                    mask,
                } => {
                    if *saturation != 1. || mask.is_some() {
                        self.observed.omitted.unsupported_paint = true;
                    }
                    self.primitives(primitives, &clips, opacity * alpha, depth + 1);
                }
                Prim::Text {
                    origin,
                    baseline,
                    text,
                    style,
                    max_width,
                    color,
                } => {
                    if self.observed.texts.len() == DOCUMENT_TEXT_RUNS {
                        self.observed.omitted.text_runs += 1;
                        self.stopped = true;
                        break;
                    }
                    let Some(clip_ids) = self.clip_ids(&clips) else {
                        break;
                    };
                    let Some(key) = self.keys.text(text, *origin) else {
                        self.observed.omitted.text_runs += 1;
                        self.observed.omitted.unsupported_paint = true;
                        continue;
                    };
                    if key.key.len() > 512 || text.len() > 16 * 1024 {
                        self.observed.omitted.text_runs += 1;
                        self.stopped = true;
                        break;
                    }
                    let mut color = color.0;
                    color[3] *= opacity;
                    let fact = UiDocumentText {
                        key: key.key.into(),
                        text: text.to_string(),
                        origin: *origin,
                        rect: rectangle(key.run.rect),
                        baseline: *baseline,
                        font_size: style.pixels(),
                        line_height: key.run.line_height,
                        weight: style.weight,
                        mono: style.mono,
                        italic: style.italic,
                        tracking_em: style.tracking_em,
                        max_width: *max_width,
                        clip: intersection(&clips),
                        clips: clip_ids,
                        color,
                    };
                    if self.room(&fact) {
                        self.observed.texts.push(fact);
                    } else {
                        self.observed.omitted.text_runs += 1;
                    }
                }
                Prim::Path {
                    origin,
                    scale,
                    points,
                    stroke,
                    color,
                    fill,
                } => {
                    if self.observed.paths.len() == DOCUMENT_PATHS {
                        self.observed.omitted.paths += 1;
                        self.stopped = true;
                        break;
                    }
                    if points.len() > DOCUMENT_PATH_COMMANDS.saturating_sub(self.commands) {
                        self.observed.omitted.paths += 1;
                        self.observed.omitted.path_commands = self
                            .observed
                            .omitted
                            .path_commands
                            .saturating_add(points.len() as u32);
                        self.stopped = true;
                        break;
                    }
                    let Some(clip_ids) = self.clip_ids(&clips) else {
                        break;
                    };
                    let Some(key) = self.keys.path(points.as_ptr(), *origin) else {
                        self.observed.omitted.paths += 1;
                        self.observed.omitted.unsupported_paint = true;
                        continue;
                    };
                    if key.key.len() > 512 {
                        self.observed.omitted.paths += 1;
                        self.stopped = true;
                        break;
                    }
                    let commands = points
                        .iter()
                        .map(|command| match command {
                            c::PathCommand::Move(p) => UiDocumentPathCommand::Move(*p),
                            c::PathCommand::Line(p) => UiDocumentPathCommand::Line(*p),
                            c::PathCommand::Cubic(p) => UiDocumentPathCommand::Cubic(*p),
                        })
                        .collect();
                    let alpha = |mut c: [f32; 4]| {
                        c[3] *= opacity;
                        c
                    };
                    let fact = UiDocumentPath {
                        key: key.key.into(),
                        origin: *origin,
                        scale: *scale,
                        commands,
                        stroke_width: stroke.width,
                        clip: intersection(&clips),
                        clips: clip_ids,
                        color: alpha(color.0),
                        fill: fill.map(|v| alpha(v.0)),
                    };
                    if self.room(&fact) {
                        self.commands += points.len();
                        self.observed.paths.push(fact);
                    } else {
                        self.observed.omitted.paths += 1;
                    }
                }
                _ => {}
            }
        }
    }
}

/// `paints` is the same document vector moved into the renderer's foreground.
pub fn collect(
    layout: &c::Layout,
    paints: &[c::Paint],
    source: UiDocumentSource,
    private_test_run: Ulid,
    frame_seq: u64,
) -> Option<UiDocumentObservation> {
    let keys = Keys::from_layout(layout);
    let mut observed = UiDocumentObservation {
        frame_seq,
        private_test_run,
        source,
        clips: vec![],
        texts: vec![],
        paths: vec![],
        omitted: Default::default(),
    };
    observed.omitted.walk_truncated = keys.truncated;
    let bytes = serde_json::to_vec(&observed).ok()?.len();
    if bytes > DOCUMENT_OBSERVATION_BYTES - 512 {
        return None;
    }
    let mut collector = Collector {
        keys,
        observed,
        left: WALK_LIMIT,
        commands: 0,
        bytes,
        stopped: false,
    };
    collector.paints(paints, 1., 0);
    if collector.stopped {
        collector.observed.omitted.walk_truncated = true;
    }
    collector.observed.validate().ok()?;
    Some(collector.observed)
}
