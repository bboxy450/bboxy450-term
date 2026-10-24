//! Read-only checklist projection. The host supplies the promises parsed from
//! e2e-acceptance §1 and its current record/task snapshot; this module does no IO.
use crate::{self as ui, components as c};
use bbt_ui_model::{
    STYLE, TOKENS,
    draw::Color as Rgba,
    layout::{Axis, Rect, TextStyle as MeasureStyle},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Promise {
    pub id: String,
    pub group: String,
    pub text: String,
}

/// Parse the single canonical list in e2e-acceptance §1. A missing section,
/// group, count, repeated id or truncated list is an error, never an empty view.
pub fn parse_promises(source: &str) -> Result<Vec<Promise>, String> {
    let mut section = false;
    let mut group = None;
    let mut expected = None;
    let mut promises: Vec<Promise> = vec![];
    for line in source.lines().map(str::trim) {
        if line.starts_with("## 1.") {
            section = true;
            continue;
        }
        if !section {
            continue;
        }
        if line.starts_with("## ") {
            break;
        }
        if let Some(title) = line.strip_prefix("### ") {
            if title.is_empty() {
                return Err("e2e spec: empty group".into());
            }
            group = Some(title.to_owned());
            continue;
        }
        if let Some((id, text)) = line
            .strip_prefix("- **")
            .and_then(|rest| rest.split_once("** "))
        {
            if id.is_empty() || text.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric()) {
                return Err("e2e spec: invalid promise".into());
            }
            if promises.iter().any(|p| p.id == id) {
                return Err(format!("e2e spec: duplicate promise {id}"));
            }
            promises.push(Promise {
                id: id.into(),
                group: group.clone().ok_or("e2e spec: promise has no group")?,
                text: text.into(),
            });
        } else if expected.is_none() {
            let words = line.split_whitespace().collect::<Vec<_>>();
            expected = words.windows(2).find_map(|pair| {
                (pair[1].trim_end_matches(['.', ',', ';', ':']) == "promises")
                    .then(|| pair[0].parse::<usize>().ok())
                    .flatten()
            });
        }
    }
    if !section || promises.is_empty() {
        return Err("e2e spec: section 1 has no promises".into());
    }
    match expected {
        Some(count) if count == promises.len() => Ok(promises),
        Some(count) => Err(format!(
            "e2e spec: declares {count} promises, parsed {}",
            promises.len()
        )),
        None => Err("e2e spec: section 1 has no declared count".into()),
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Link {
    pub to: String,
    pub rel: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Task {
    #[serde(rename = "ref")]
    pub reference: String,
    pub status: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Record {
    pub function: String,
    pub number: Option<u64>,
    pub status: String,
    pub last_run: Option<String>,
    pub run_by: Option<String>,
    pub reason: Option<String>,
    pub result: Option<String>,
    pub links: Vec<Link>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Checklist {
    pub promises: Vec<Promise>,
    pub records: Vec<Record>,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Pass,
    Fail,
    Blocked,
    Untested,
    Manual,
}
impl Status {
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "pass" => Self::Pass,
            "fail" => Self::Fail,
            "blocked" => Self::Blocked,
            "untested" => Self::Untested,
            "manual" => Self::Manual,
            _ => return None,
        })
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Blocked => "blocked",
            Self::Untested => "untested",
            Self::Manual => "manual",
        }
    }
    fn failure(self) -> bool {
        matches!(self, Self::Fail | Self::Blocked)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Green,
    Amber,
    Red,
}

#[derive(Debug)]
pub struct Row<'a> {
    pub promise: &'a Promise,
    pub record: Option<&'a Record>,
    pub status: Status,
    pub fault: Option<String>,
    pub tracking: Vec<Task>,
    pub tracked_by: Vec<String>,
}

#[derive(Debug)]
pub struct Summary {
    pub tone: Tone,
    pub total: usize,
    pub passed: usize,
    /// Includes blocked rows, which are failures for the summary (§4).
    pub failures: usize,
    pub blocked: usize,
    pub untested: usize,
    pub missing: usize,
    pub doubled: usize,
    pub invalid: usize,
    pub manual: usize,
    pub untracked: usize,
    pub faults: Vec<String>,
}

#[derive(Debug)]
pub struct Projection<'a> {
    pub rows: Vec<Row<'a>>,
    pub summary: Summary,
}

fn reference(value: &str) -> &str {
    let value = value.trim();
    let value = value
        .strip_prefix("[[")
        .and_then(|v| v.strip_suffix("]]"))
        .unwrap_or(value);
    let target = value.split_once('|').map_or(value, |(target, _)| target);
    target.strip_suffix('\\').unwrap_or(target).trim()
}

pub fn task_open(status: &str) -> bool {
    matches!(
        status,
        "open" | "claimed" | "running" | "reported" | "blocked"
    )
}

impl Checklist {
    fn tracking(&self, record: &Record) -> Vec<Task> {
        let mut found = vec![];
        for link in &record.links {
            if link.rel != "tracked-by" || reference(&link.to).is_empty() {
                continue;
            }
            let target = reference(&link.to);
            let tasks = self
                .tasks
                .iter()
                .filter(|t| reference(&t.reference) == target)
                .collect::<Vec<_>>();
            let status = if let [task] = tasks.as_slice() {
                task.status.as_str()
            } else {
                "unknown"
            };
            found.push(Task {
                reference: target.into(),
                status: status.into(),
            });
        }
        found
    }

    pub fn project(&self) -> Projection<'_> {
        let mut rows = vec![];
        for promise in &self.promises {
            let records = self
                .records
                .iter()
                .filter(|record| record.function == promise.id)
                .collect::<Vec<_>>();
            let duplicate = self.promises.iter().filter(|p| p.id == promise.id).count() > 1;
            let (record, fault) = if duplicate {
                (None, Some(format!("duplicate promise {}", promise.id)))
            } else {
                match records.as_slice() {
                    [] => (None, Some("no record".into())),
                    [record] => (Some(*record), None),
                    _ => (None, Some(format!("doubled · {} records", records.len()))),
                }
            };
            let mut row = Row {
                promise,
                record,
                status: Status::Untested,
                fault,
                tracking: vec![],
                tracked_by: vec![],
            };
            if let Some(record) = record {
                if let Some(status) = Status::parse(&record.status) {
                    row.status = status;
                    if status.failure() {
                        row.tracking = self.tracking(record);
                        for task in &row.tracking {
                            if task_open(&task.status) && !row.tracked_by.contains(&task.reference)
                            {
                                row.tracked_by.push(task.reference.clone());
                            }
                        }
                    }
                } else {
                    row.fault = Some(format!("invalid status · {}", record.status));
                }
            }
            rows.push(row);
        }
        let mut summary = Summary {
            tone: Tone::Red,
            total: rows.len(),
            passed: rows.iter().filter(|r| r.status == Status::Pass).count(),
            failures: rows.iter().filter(|r| r.status.failure()).count(),
            blocked: rows.iter().filter(|r| r.status == Status::Blocked).count(),
            untested: rows.iter().filter(|r| r.status == Status::Untested).count(),
            missing: rows
                .iter()
                .filter(|r| r.fault.as_deref() == Some("no record"))
                .count(),
            doubled: rows
                .iter()
                .filter(|r| {
                    r.fault
                        .as_deref()
                        .is_some_and(|f| f.starts_with("doubled · "))
                })
                .count(),
            invalid: rows
                .iter()
                .filter(|r| {
                    r.fault
                        .as_deref()
                        .is_some_and(|f| f.starts_with("invalid status · "))
                })
                .count(),
            manual: rows.iter().filter(|r| r.status == Status::Manual).count(),
            untracked: rows
                .iter()
                .filter(|r| r.status.failure() && r.tracked_by.is_empty())
                .count(),
            faults: vec![],
        };
        if self.promises.is_empty() {
            summary.faults.push("checklist has no promises".into());
        }
        for record in &self.records {
            if !self.promises.iter().any(|p| p.id == record.function) {
                summary
                    .faults
                    .push(format!("unknown function: {}", record.function));
            }
        }
        if summary.untested == 0 && summary.untracked == 0 && summary.faults.is_empty() {
            summary.tone = if summary.failures == 0 {
                Tone::Green
            } else {
                Tone::Amber
            };
        }
        Projection { rows, summary }
    }
}

fn named(key: impl Into<String>, value: c::Element) -> c::Element {
    c::Element::Named {
        name: key.into(),
        content: Box::new(value),
    }
}
fn plain(padding: [f32; 4], radius: f32) -> ui::Style {
    let mut s = ui::Style::new(ui::Preset::Card);
    s.overrides.fill = Some(ui::Paint::solid(ui::Color::Transparent));
    s.overrides.padding = Some(padding.map(ui::Length::Px));
    s.overrides.radii = Some([ui::Length::Px(radius); 4]);
    s.overrides.border = Some(ui::Border {
        width: ui::Length::Px(0.),
        paint: ui::Paint::solid(ui::Color::Transparent),
        outside: false,
    });
    s
}
fn font(size: f32, weight: u16, mono: bool, color: ui::Color) -> ui::Text {
    ui::Text {
        size: Some(size),
        weight,
        mono,
        paint: ui::Paint::solid(color),
        line_height: 1.4,
        ..Default::default()
    }
}
fn label(
    key: impl Into<String>,
    value: impl Into<String>,
    style: ui::Text,
    ellipsis: bool,
) -> c::Element {
    let value = value.into();
    if value.is_empty() {
        let mut s = plain([0.; 4], 0.);
        s.overrides.height = Some(ui::Length::Px(0.));
        return block(key, Axis::Row, s, 0., c::Alignment::Center, vec![]);
    }
    let mut t = c::Text::new(value, style);
    t.wrap = false;
    t.ellipsis = ellipsis;
    named(key, c::Element::Text(t))
}
fn block(
    key: impl Into<String>,
    axis: Axis,
    style: ui::Style,
    gap: f32,
    align: c::Alignment,
    children: Vec<c::Item>,
) -> c::Element {
    let mut b = c::Box::new(style, children);
    b.direction = axis;
    b.align = align;
    b.gap = ui::Length::Px(gap);
    named(key, c::Element::Box(b))
}
fn fixed(element: c::Element, width: f32) -> c::Item {
    c::Item {
        element,
        extent: c::Extent::Fixed(width),
    }
}
fn fill(element: c::Element) -> c::Item {
    c::Item {
        element,
        extent: c::Extent::Fill,
    }
}
fn flow(
    key: impl Into<String>,
    gap: f32,
    align: c::Alignment,
    children: Vec<c::Item>,
) -> c::Element {
    let mut f = c::Flow::new(children);
    f.row_gap = ui::Length::Px(gap);
    f.column_gap = ui::Length::Px(gap);
    f.align = align;
    named(key, c::Element::Flow(f))
}
fn hair() -> ui::Border {
    ui::Border {
        width: ui::Length::Hairline,
        paint: ui::Paint {
            color: ui::Color::Line,
            alpha: ui::Alpha::Hairline,
        },
        outside: true,
    }
}
fn tone_color(tone: Tone) -> ui::Color {
    ui::Color::Rgba(
        Rgba::hex(match tone {
            Tone::Green => TOKENS.color.ui.ok,
            Tone::Amber => TOKENS.color.state.waiting,
            Tone::Red => TOKENS.color.state.errored,
        })
        .0,
    )
}
fn status_color(status: Status) -> ui::Color {
    match status {
        Status::Pass => tone_color(Tone::Green),
        Status::Fail | Status::Blocked => tone_color(Tone::Red),
        Status::Manual => ui::Color::Accent,
        Status::Untested => ui::Color::Muted,
    }
}
fn cell(key: &str, row: &Row<'_>, size: [f32; 2], radius: f32) -> c::Element {
    let outlined = row.fault.is_some() || (row.status.failure() && row.tracked_by.is_empty());
    let alpha = if outlined {
        1.
    } else {
        match row.status {
            Status::Untested => 0.28,
            Status::Manual => 0.85,
            _ => 1.,
        }
    };
    let mut s = plain([0.; 4], radius);
    s.overrides.width = Some(ui::Length::Px(size[0]));
    s.overrides.height = Some(ui::Length::Px(size[1]));
    s.overrides.fill = Some(ui::Paint {
        color: status_color(row.status),
        alpha: ui::Alpha::Value(alpha),
    });
    if outlined {
        s.overrides.border = Some(ui::Border {
            width: ui::Length::Px(2.),
            paint: ui::Paint::solid(ui::Color::Foreground),
            outside: false,
        });
    }
    block(key, Axis::Row, s, 0., c::Alignment::Center, vec![])
}
fn strip(key: &str, rows: &[&Row<'_>], size: [f32; 2], radius: f32) -> c::Element {
    block(
        key,
        Axis::Row,
        plain([0.; 4], 0.),
        STYLE.spacing.s1 as f32,
        c::Alignment::Center,
        rows.iter()
            .enumerate()
            .map(|(i, r)| cell(&format!("{key}/c{i}"), r, size, radius).into())
            .collect(),
    )
}
fn stamp(value: Option<&str>) -> &str {
    value
        .filter(|s| s.ends_with('Z') || s.ends_with("+00:00"))
        .and_then(|s| s.get(11..16))
        .unwrap_or("—")
}

impl Summary {
    fn phrases(&self) -> Vec<String> {
        match self.tone {
            Tone::Green => {
                let mut labels = vec![if self.manual == 0 {
                    format!("all {} pass", self.total)
                } else {
                    format!("{} pass", self.passed)
                }];
                if self.manual > 0 {
                    labels.push(format!("{} manual", self.manual));
                }
                labels
            }
            Tone::Amber => {
                let mut labels = vec![format!(
                    "{} failure{} tracked",
                    self.failures,
                    if self.failures == 1 { "" } else { "s" }
                )];
                if self.manual > 0 {
                    labels.push(format!("{} manual", self.manual));
                }
                labels
            }
            Tone::Red => {
                let mut labels = vec![];
                let untested = self
                    .untested
                    .saturating_sub(self.missing + self.doubled + self.invalid);
                if untested > 0 {
                    labels.push(format!("{untested} untested"));
                }
                if self.untracked > 0 {
                    labels.push(format!(
                        "{} fail{} untracked",
                        self.untracked,
                        if self.untracked == 1 { "" } else { "s" }
                    ));
                }
                if self.missing > 0 {
                    labels.push(format!("{} missing", self.missing));
                }
                if self.doubled > 0 {
                    labels.push(format!("{} doubled", self.doubled));
                }
                if self.invalid > 0 {
                    labels.push(format!("{} invalid", self.invalid));
                }
                if !self.faults.is_empty() {
                    labels.push(format!("{} invalid records", self.faults.len()));
                }
                labels
            }
        }
    }
}

impl Checklist {
    /// Existing retained components only; each available width is owned by the host.
    pub fn element_width(&self, ctx: ui::Context, m: &dyn c::Measure, width: f32) -> c::Element {
        if self.promises.is_empty() {
            let tone = tone_color(Tone::Red);
            let foreground = ui::Paint::solid(tone).resolve(ctx).unwrap();
            let surface = Rgba::hex(ctx.theme.palette().surface).0;
            let mixed = std::array::from_fn(|i| {
                if i == 3 {
                    1.
                } else {
                    foreground[i] * 0.13 + surface[i] * 0.87
                }
            });
            let mut s = plain(
                [
                    STYLE.spacing.s8 as f32,
                    STYLE.spacing.s10 as f32,
                    STYLE.spacing.s8 as f32,
                    STYLE.spacing.s10 as f32 - 5.,
                ],
                STYLE.radius.card as f32,
            );
            s.overrides.fill = Some(ui::Paint::solid(ui::Color::Rgba(mixed)));
            s.overrides.border = Some(ui::Border {
                width: ui::Length::Px(5.),
                paint: ui::Paint::solid(tone),
                outside: false,
            });
            s.overrides.border_sides = Some([false, false, false, true]);
            let mut text = c::Text::new(
                "",
                font(STYLE.r#type.row as f32, 400, false, ui::Color::Foreground),
            );
            text.runs = vec![
                c::TextRun {
                    value: "No checklist: ".into(),
                    style: font(STYLE.r#type.row as f32, 700, false, ui::Color::Foreground),
                },
                c::TextRun {
                    value: "the e2e catalogue is empty: no promises parsed from spec #258 §1"
                        .into(),
                    style: font(STYLE.r#type.row as f32, 400, false, ui::Color::Foreground),
                },
            ];
            return block(
                "e2e/error",
                Axis::Column,
                s,
                0.,
                c::Alignment::Stretch,
                vec![named("e2e/error/text", c::Element::Text(text)).into()],
            );
        }
        let out = self.project();
        let mut groups = vec![];
        for row in &out.rows {
            if !groups.contains(&row.promise.group.as_str()) {
                groups.push(row.promise.group.as_str());
            }
        }
        let tone = tone_color(out.summary.tone);
        let state = match out.summary.tone {
            Tone::Green => "green",
            Tone::Amber => "amber",
            Tone::Red => "red",
        };
        let mut word = font(STYLE.r#type.display as f32, 800, false, tone);
        word.uppercase = true;
        word.tracking_em = 0.08;
        let phrases = out.summary.phrases();
        let mut say = c::Text::new(
            "",
            font(
                STYLE.r#type.display as f32,
                600,
                false,
                ui::Color::Foreground,
            ),
        );
        say.max_content_basis = true;
        say.runs = phrases
            .iter()
            .enumerate()
            .map(|(i, value)| c::TextRun {
                value: if i == 0 {
                    value.clone()
                } else {
                    format!(" · {value}")
                },
                style: font(
                    if i == 0 {
                        STYLE.r#type.display as f32
                    } else {
                        STYLE.r#type.heading as f32
                    },
                    if i == 0 { 900 } else { 500 },
                    false,
                    if i == 0 {
                        ui::Color::Foreground
                    } else {
                        ui::Color::Muted
                    },
                ),
            })
            .collect();
        let mut runs = self
            .records
            .iter()
            .filter(|r| r.last_run.is_some())
            .collect::<Vec<_>>();
        runs.sort_by_key(|r| r.last_run.as_deref());
        let seats = runs
            .iter()
            .filter_map(|r| r.run_by.as_deref())
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        let meta = if let Some(last) = runs.last() {
            format!(
                "{} promises · last run {} UTC · {seats} seat{}",
                out.rows.len(),
                stamp(last.last_run.as_deref()),
                if seats == 1 { "" } else { "s" }
            )
        } else {
            format!("{} promises · never run", out.rows.len())
        };
        let metadata = named(
            "e2e/head/meta-wrap",
            c::Element::Stack(c::Stack {
                direction: Axis::Row,
                gap: ui::Length::Px(0.),
                align: c::Alignment::Baseline,
                children: vec![
                    fill(c::Element::Spacer),
                    label(
                        "e2e/head/meta",
                        meta,
                        font(STYLE.r#type.label as f32, 400, false, ui::Color::Muted),
                        false,
                    )
                    .into(),
                ],
            }),
        );
        let top = flow(
            "e2e/head/top",
            STYLE.spacing.s8 as f32,
            c::Alignment::Baseline,
            vec![
                label("e2e/head/word", state, word, false).into(),
                named("e2e/head/say", c::Element::Text(say)).into(),
                c::Item {
                    element: metadata,
                    extent: c::Extent::Content {
                        grow: 1.,
                        shrink: 0.,
                        min: 0.,
                        max: None,
                        max_fraction: None,
                    },
                },
            ],
        );
        let cells = flow(
            "e2e/strip",
            STYLE.spacing.s6 as f32,
            c::Alignment::Start,
            groups
                .iter()
                .enumerate()
                .map(|(i, group)| {
                    strip(
                        &format!("e2e/strip/g{i}"),
                        &out.rows
                            .iter()
                            .filter(|r| r.promise.group == *group)
                            .collect::<Vec<_>>(),
                        [14., 22.],
                        3.,
                    )
                    .into()
                })
                .collect(),
        );
        let counts = [
            (Status::Pass, out.summary.passed),
            (Status::Fail, out.summary.failures - out.summary.blocked),
            (Status::Blocked, out.summary.blocked),
            (Status::Untested, out.summary.untested),
            (Status::Manual, out.summary.manual),
        ];
        let count_row = flow(
            "e2e/counts",
            STYLE.spacing.s9 as f32,
            c::Alignment::Center,
            counts
                .into_iter()
                .map(|(status, count)| {
                    let key = format!("e2e/counts/{}", status.as_str());
                    let mut swatch = plain([0.; 4], 3.);
                    swatch.overrides.width = Some(ui::Length::Px(8.));
                    swatch.overrides.height = Some(ui::Length::Px(12.));
                    swatch.overrides.fill = Some(ui::Paint {
                        color: status_color(status),
                        alpha: ui::Alpha::Value(match status {
                            Status::Untested => 0.28,
                            Status::Manual => 0.85,
                            _ => 1.,
                        }),
                    });
                    let mut s = plain([0.; 4], 0.);
                    if count == 0 {
                        s.overrides.opacity = Some(0.6);
                    }
                    s.overrides.text = Some(font(
                        STYLE.r#type.row as f32,
                        400,
                        false,
                        ui::Color::Foreground,
                    ));
                    let mut count = c::Box::new(
                        s,
                        vec![
                            c::Element::Baseline {
                                // An empty retained box has baseline zero;
                                // CSS inline-block uses its bottom, then -1px.
                                offset: 12. - 1.,
                                content: Box::new(block(
                                    format!("{key}/swatch"),
                                    Axis::Row,
                                    swatch,
                                    0.,
                                    c::Alignment::Center,
                                    vec![],
                                )),
                            }
                            .into(),
                            label(
                                format!("{key}/label"),
                                format!("{count} {}", status.as_str()),
                                font(
                                    STYLE.r#type.row as f32,
                                    400,
                                    false,
                                    if count == 0 {
                                        ui::Color::Muted
                                    } else {
                                        ui::Color::Foreground
                                    },
                                ),
                                false,
                            )
                            .into(),
                        ],
                    );
                    count.inline = true;
                    count.align = c::Alignment::Baseline;
                    count.gap = ui::Length::Px(STYLE.spacing.s3 as f32);
                    named(&key, c::Element::Box(count)).into()
                })
                .collect(),
        );
        let rgba = ui::Paint::solid(tone).resolve(ctx).unwrap();
        let surface = Rgba::hex(ctx.theme.palette().surface).0;
        let mixed = std::array::from_fn(|i| {
            if i == 3 {
                1.
            } else {
                rgba[i] * 0.13 + surface[i] * 0.87
            }
        });
        let mut head_style = plain(
            [
                STYLE.spacing.s8 as f32,
                STYLE.spacing.s10 as f32,
                STYLE.spacing.s8 as f32,
                STYLE.spacing.s10 as f32 - 5.,
            ],
            STYLE.radius.card as f32,
        );
        head_style.overrides.fill = Some(ui::Paint::solid(ui::Color::Rgba(mixed)));
        head_style.overrides.border = Some(ui::Border {
            width: ui::Length::Px(5.),
            paint: ui::Paint::solid(tone),
            outside: false,
        });
        head_style.overrides.border_sides = Some([false, false, false, true]);
        head_style.overrides.rings = Some(vec![hair()]);
        let head = block(
            "e2e/head",
            Axis::Column,
            head_style,
            0.,
            c::Alignment::Stretch,
            vec![
                top.into(),
                c::Item {
                    element: c::Element::Spacer,
                    extent: c::Extent::Fixed(STYLE.spacing.s8 as f32),
                },
                cells.into(),
                c::Item {
                    element: c::Element::Spacer,
                    extent: c::Extent::Fixed(STYLE.spacing.s6 as f32),
                },
                count_row.into(),
            ],
        );
        let gap = STYLE.spacing.s8 as f32;
        let columns = ((width + gap) / (460. + gap)).floor().max(1.) as usize;
        let column_width = ((width - gap * (columns - 1) as f32) / columns as f32).max(460.);
        let cards = flow(
            "e2e/groups",
            gap,
            c::Alignment::Start,
            groups
                .iter()
                .enumerate()
                .map(|(gi, group)| {
                    let rows = out
                        .rows
                        .iter()
                        .filter(|r| r.promise.group == *group)
                        .collect::<Vec<_>>();
                    fixed(self.group_element(ctx, m, gi, group, &rows), column_width)
                })
                .collect(),
        );
        block(
            "e2e",
            Axis::Column,
            plain([0.; 4], 0.),
            gap,
            c::Alignment::Stretch,
            vec![head.into(), cards.into()],
        )
    }

    fn group_element(
        &self,
        ctx: ui::Context,
        m: &dyn c::Measure,
        index: usize,
        title: &str,
        rows: &[&Row<'_>],
    ) -> c::Element {
        let key = format!("e2e/g{index}");
        let good = rows.iter().filter(|r| r.status == Status::Pass).count();
        let heading = block(
            format!("{key}/head"),
            Axis::Row,
            plain(
                [
                    STYLE.spacing.s2 as f32,
                    STYLE.spacing.s7 as f32,
                    STYLE.spacing.s4 as f32,
                    STYLE.spacing.s7 as f32,
                ],
                0.,
            ),
            STYLE.spacing.s5 as f32,
            c::Alignment::Center,
            vec![
                label(
                    format!("{key}/title"),
                    title,
                    font(STYLE.r#type.row as f32, 700, false, ui::Color::Foreground),
                    false,
                )
                .into(),
                strip(&format!("{key}/strip"), rows, [6., 10.], 1.5).into(),
                fill(c::Element::Spacer),
                label(
                    format!("{key}/count"),
                    format!("{good} / {} pass", rows.len()),
                    font(STYLE.r#type.small as f32, 400, false, ui::Color::Muted),
                    false,
                )
                .into(),
            ],
        );
        let mut children = vec![heading.into()];
        children.extend(
            rows.iter()
                .enumerate()
                .map(|(i, r)| self.row_element(ctx, m, &format!("{key}/r{i}"), r).into()),
        );
        let mut s = plain(
            [STYLE.spacing.s4 as f32, 0., STYLE.spacing.s3 as f32, 0.],
            STYLE.radius.card as f32,
        );
        s.overrides.fill = Some(ui::Paint::solid(ui::Color::Surface));
        s.overrides.border = Some(hair());
        block(key, Axis::Column, s, 0., c::Alignment::Stretch, children)
    }

    fn row_element(
        &self,
        _ctx: ui::Context,
        m: &dyn c::Measure,
        key: &str,
        row: &Row<'_>,
    ) -> c::Element {
        let color = status_color(row.status);
        let mark = match row.status {
            Status::Pass => ui::marks::Mark::Check,
            Status::Fail => ui::marks::Mark::Cross,
            Status::Blocked => ui::marks::Mark::Blocked,
            Status::Untested => ui::marks::Mark::Open,
            Status::Manual => ui::marks::Mark::Half,
        };
        let mut state_style = font(STYLE.r#type.micro as f32, 400, true, color);
        state_style.uppercase = true;
        state_style.tracking_em = 0.06;
        let small = font(STYLE.r#type.small as f32, 400, false, ui::Color::Foreground);
        let mut when = c::Text::new(
            stamp(row.record.and_then(|r| r.last_run.as_deref())),
            font(STYLE.r#type.small as f32, 400, true, ui::Color::Muted),
        );
        when.wrap = false;
        when.align = c::Alignment::End;
        let line = block(
            format!("{key}/line"),
            Axis::Row,
            plain([0.; 4], 0.),
            STYLE.spacing.s4 as f32,
            c::Alignment::Center,
            vec![
                fixed(
                    label(
                        format!("{key}/id"),
                        &row.promise.id,
                        font(STYLE.r#type.small as f32, 400, true, ui::Color::Muted),
                        false,
                    ),
                    26.,
                ),
                fixed(
                    c::Element::Stack(c::Stack {
                        direction: Axis::Row,
                        gap: ui::Length::Px(0.),
                        align: c::Alignment::Center,
                        children: vec![fixed(
                            named(
                                format!("{key}/mark"),
                                mark.element(11., ui::Paint::solid(color)),
                            ),
                            11.,
                        )],
                    }),
                    12.,
                ),
                fill(label(format!("{key}/text"), &row.promise.text, small, true)),
                label(
                    format!("{key}/status"),
                    row.fault.as_deref().unwrap_or(row.status.as_str()),
                    state_style,
                    false,
                )
                .into(),
                fixed(named(format!("{key}/when"), c::Element::Text(when)), 34.),
                fixed(
                    label(
                        format!("{key}/by"),
                        row.record
                            .and_then(|r| r.run_by.as_deref())
                            .unwrap_or("")
                            .strip_prefix("seat:")
                            .unwrap_or(row.record.and_then(|r| r.run_by.as_deref()).unwrap_or("")),
                        font(STYLE.r#type.small as f32, 400, false, ui::Color::Muted),
                        true,
                    ),
                    104.,
                ),
            ],
        );
        let mut children = vec![line.into()];
        let mut notes = vec![];
        if row.status.failure() {
            for (i, task) in row.tracking.iter().enumerate() {
                let open = task_open(&task.status);
                let value = format!("{} {}", task.reference, task.status);
                let style = font(
                    STYLE.r#type.tiny as f32,
                    400,
                    true,
                    if open {
                        ui::Color::Foreground
                    } else {
                        ui::Color::Muted
                    },
                );
                let text = label(format!("{key}/task{i}/text"), &value, style, false);
                let text = if open {
                    text
                } else {
                    let width = m.width(
                        &value,
                        &MeasureStyle {
                            size: STYLE.r#type.tiny as f32,
                            weight: 400,
                            mono: true,
                            italic: false,
                            tracking_em: 0.,
                        },
                    );
                    c::Element::Layers {
                        base: Box::new(text),
                        overlays: vec![c::Placed {
                            content: c::Element::Rects(c::Rects {
                                size: [width, 1.],
                                rects: std::sync::Arc::from([(
                                    Rect::new(0., 0., width, 1.),
                                    ui::Paint::solid(ui::Color::Muted),
                                )]),
                                repeat: None,
                            }),
                            left: 0.,
                            bottom: STYLE.r#type.tiny as f32 * 0.7,
                        }],
                    }
                };
                let mut s = plain(
                    [0., 5., 0., if open { 5. } else { 7. }],
                    STYLE.radius.tag as f32,
                );
                s.overrides.fill = Some(ui::Paint {
                    color: ui::Color::Foreground,
                    alpha: ui::Alpha::Badge,
                });
                if open {
                    s.overrides.border = Some(ui::Border {
                        width: ui::Length::Px(2.),
                        paint: ui::Paint::solid(tone_color(Tone::Amber)),
                        outside: false,
                    });
                    s.overrides.border_sides = Some([false, false, false, true]);
                }
                notes.push(
                    block(
                        format!("{key}/task{i}"),
                        Axis::Row,
                        s,
                        0.,
                        c::Alignment::Center,
                        vec![text.into()],
                    )
                    .into(),
                );
            }
            if row.tracked_by.is_empty() {
                notes.push(
                    label(
                        format!("{key}/untracked"),
                        "untracked",
                        font(STYLE.r#type.small as f32, 700, false, tone_color(Tone::Red)),
                        false,
                    )
                    .into(),
                );
            }
            if let Some(reason) = row.record.and_then(|r| r.reason.as_deref()) {
                notes.push(
                    label(
                        format!("{key}/reason"),
                        reason,
                        font(STYLE.r#type.small as f32, 400, false, ui::Color::Muted),
                        false,
                    )
                    .into(),
                );
            }
        } else if row.status == Status::Manual {
            notes.push(
                label(
                    format!("{key}/reason"),
                    format!(
                        "Oracle’s eye{}",
                        row.record
                            .and_then(|r| r.reason.as_deref())
                            .map_or_else(String::new, |reason| format!(" · {reason}"))
                    ),
                    font(STYLE.r#type.small as f32, 400, false, ui::Color::Muted),
                    false,
                )
                .into(),
            );
        }
        if !notes.is_empty() {
            let note = flow(
                format!("{key}/notes"),
                STYLE.spacing.s4 as f32,
                c::Alignment::Center,
                notes,
            );
            children.push(
                block(
                    format!("{key}/note"),
                    Axis::Row,
                    plain([1., 0., 0., 26. + 12. + 2. * STYLE.spacing.s4 as f32], 0.),
                    0.,
                    c::Alignment::Start,
                    vec![fill(note)],
                )
                .into(),
            );
        }
        let mut s = plain(
            [3., STYLE.spacing.s7 as f32, 3., STYLE.spacing.s7 as f32],
            0.,
        );
        if row.fault.is_some() || (row.status.failure() && row.tracked_by.is_empty()) {
            s.overrides.fill = Some(ui::Paint {
                color: tone_color(Tone::Red),
                alpha: ui::Alpha::Value(0.12),
            });
        }
        block(key, Axis::Column, s, 0., c::Alignment::Stretch, children)
    }
}
