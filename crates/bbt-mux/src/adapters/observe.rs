use super::{Binding, HarnessRecord};
use crate::{
    Ulid,
    store::{Result, invalid},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Grade {
    Inferred,
    Shim,
    Reported,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Active,
    Idle,
    Waiting,
    Errored,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaitingKind {
    Permission,
    Question,
    Auth,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    pub sequence: u64,
    pub generation: u64,
    pub observed_ms: u64,
    pub source: String,
    pub grade: Grade,
    pub state: State,
    pub waiting_kind: Option<WaitingKind>,
    pub agent_id: Option<String>,
    pub title: Option<String>,
    pub detail: Option<String>,
    pub clear: bool,
    pub resolves: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subagent {
    pub id: String,
    pub title: String,
    pub state: State,
    pub waiting_kind: Option<WaitingKind>,
    pub grade: Grade,
    pub started_sequence: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    pub state: State,
    pub grade: Grade,
    pub source: String,
    pub sequence: u64,
    pub observed_ms: u64,
    pub detail: Option<String>,
    pub waiting_kind: Option<WaitingKind>,
    pub turn_open: bool,
    pub subagents: Vec<Subagent>,
}
impl Status {
    pub(crate) fn same_value(&self, other: &Self) -> bool {
        self.state == other.state
            && self.grade == other.grade
            && self.source == other.source
            && self.detail == other.detail
            && self.waiting_kind == other.waiting_kind
            && self.turn_open == other.turn_open
            && self.subagents == other.subagents
    }
}
#[derive(Clone)]
pub(crate) struct Proof {
    pub binding: Binding,
    pub sequence: u64,
    pub observed_ms: u64,
    pub revision: u64,
}
pub struct Session {
    pub record: HarnessRecord,
    pub binding: Binding,
    pub status: Status,
    pub cwd: Option<String>,
    pub(crate) proof: Option<Proof>,
    pub(crate) revision: u64,
    pub(crate) dialog: bool,
    pub(crate) dialog_key: Option<String>,
    pub(crate) pending: std::collections::BTreeMap<String, String>,
    pub(crate) last_output_ms: u64,
    pub(crate) uncertain: std::collections::BTreeSet<Ulid>,
    replies: Vec<Vec<u8>>,
    history: VecDeque<Observation>,
}
impl Session {
    pub fn new(record: HarnessRecord, binding: Binding) -> Result<Self> {
        record.validate()?;
        binding.validate()?;
        Ok(Self {
            record,
            binding,
            status: Status {
                state: State::Idle,
                grade: Grade::Inferred,
                source: "unobserved".into(),
                sequence: 0,
                observed_ms: 0,
                detail: None,
                waiting_kind: None,
                turn_open: false,
                subagents: Vec::new(),
            },
            cwd: None,
            proof: None,
            revision: 0,
            dialog: false,
            dialog_key: None,
            pending: Default::default(),
            last_output_ms: 0,
            replies: Vec::new(),
            history: VecDeque::new(),
            uncertain: Default::default(),
        })
    }
    pub fn rebind(&mut self, binding: Binding) -> Result<()> {
        binding.validate()?;
        self.binding = binding;
        self.invalidate();
        self.dialog = false;
        self.dialog_key = None;
        self.pending.clear();
        self.status.source = "unobserved".into();
        self.status.subagents.clear();
        Ok(())
    }
    pub fn take_replies(&mut self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.replies)
    }
    pub fn history(&self) -> &VecDeque<Observation> {
        &self.history
    }
    pub fn output(&mut self, now_ms: u64) {
        self.last_output_ms = now_ms;
    }
    pub(crate) fn invalidate(&mut self) {
        self.proof = None;
        self.revision = self.revision.wrapping_add(1);
    }
    pub(crate) fn ready(&mut self, sequence: u64, now_ms: u64) {
        self.revision = self.revision.wrapping_add(1);
        self.proof = Some(Proof {
            binding: self.binding.clone(),
            sequence,
            observed_ms: now_ms,
            revision: self.revision,
        });
    }
    pub(crate) fn observation(
        &self,
        sequence: u64,
        now_ms: u64,
        source: &str,
        grade: Grade,
        state: State,
    ) -> Observation {
        Observation {
            sequence,
            generation: self.binding.generation,
            observed_ms: now_ms,
            source: source.into(),
            grade,
            state,
            waiting_kind: None,
            agent_id: None,
            title: None,
            detail: None,
            clear: false,
            resolves: false,
        }
    }
    pub(crate) fn apply(&mut self, obs: Observation) -> bool {
        if obs.generation != self.binding.generation {
            return false;
        }
        if self.status.source == "exit" && obs.source != "exit" {
            return false;
        }
        if let Some(id) = &obs.agent_id {
            if obs.clear || obs.state == State::Idle {
                self.status.subagents.retain(|s| &s.id != id);
            } else if let Some(row) = self.status.subagents.iter_mut().find(|s| &s.id == id) {
                if obs.grade >= row.grade {
                    row.state = obs.state;
                    row.grade = obs.grade;
                    row.waiting_kind = obs.waiting_kind;
                    if let Some(title) = &obs.title {
                        row.title = title.clone();
                    }
                }
            } else if self.status.subagents.len() < 32 {
                self.status.subagents.push(Subagent {
                    id: id.clone(),
                    title: obs.title.clone().unwrap_or_default(),
                    state: obs.state,
                    waiting_kind: obs.waiting_kind,
                    grade: obs.grade,
                    started_sequence: obs.sequence,
                });
            }
        } else {
            if obs.state == State::Waiting {
                self.dialog = true;
                self.invalidate();
            }
            if obs.resolves {
                self.dialog = false;
            }
            let lower_recent = obs.grade < self.status.grade
                && obs.observed_ms.saturating_sub(self.status.observed_ms) < 5000;
            let off_pty_tie = obs.grade == Grade::Reported
                && obs.source != "7501"
                && self.status.source == "7501"
                && obs.sequence == self.status.sequence
                && obs.observed_ms <= self.status.observed_ms;
            if !lower_recent && !off_pty_tie && (!self.dialog || obs.state == State::Waiting) {
                self.status.state = obs.state;
                self.status.grade = obs.grade;
                self.status.source = obs.source.clone();
                self.status.sequence = obs.sequence;
                self.status.observed_ms = obs.observed_ms;
                self.status.waiting_kind = obs.waiting_kind;
                self.status.turn_open = matches!(obs.state, State::Active | State::Waiting);
                if let Some(detail) = &obs.detail {
                    self.status.detail = Some(short(detail, 192));
                }
                if obs.clear {
                    self.status.subagents.clear();
                }
            }
            if obs.state != State::Idle {
                self.invalidate();
            }
        }
        self.history.push_back(obs);
        if self.history.len() > 256 {
            self.history.pop_front();
        }
        true
    }
    pub fn exit(&mut self, sequence: u64, now_ms: u64, code: u32) -> Observation {
        let mut obs = self.observation(
            sequence,
            now_ms,
            "exit",
            Grade::Reported,
            if code == 0 {
                State::Idle
            } else {
                State::Errored
            },
        );
        obs.resolves = true;
        obs.clear = true;
        obs.detail = Some(format!("exit {code}"));
        self.status.source = "exit".into();
        self.status.observed_ms = 0;
        self.apply(obs.clone());
        self.invalidate();
        obs
    }
    pub fn tick(&mut self, sequence: u64, now_ms: u64) {
        if self.record.state_source == super::StateSource::Cadence {
            let state = if now_ms.saturating_sub(self.last_output_ms) < 2000 {
                State::Active
            } else {
                State::Idle
            };
            let obs = self.observation(sequence, now_ms, "cadence", Grade::Inferred, state);
            self.apply(obs);
        }
    }
    pub fn osc(&mut self, params: &[Vec<u8>], sequence: u64, now_ms: u64) -> Vec<Observation> {
        if params.iter().map(Vec::len).sum::<usize>() > 4096 {
            return Vec::new();
        }
        let Some(code) = params.first() else {
            return Vec::new();
        };
        let payload = params.get(1).map(Vec::as_slice).unwrap_or_default();
        if code == b"7501" && payload == b"?" {
            if self.replies.len() < 32 {
                self.replies.push(b"\x1b]7501;?\x07".to_vec());
            }
            return Vec::new();
        }
        let obs = match code.as_slice() {
            b"7501" => parse_7501(self, params, sequence, now_ms),
            b"7502" => super::hook::parse(self, params, sequence, now_ms),
            b"133" if self.record.kind == "zsh" => {
                let state = match payload {
                    b"B" => State::Idle,
                    b"C" => State::Active,
                    b"D" if params.get(2).is_some_and(|v| v != b"0") => State::Errored,
                    _ => return Vec::new(),
                };
                let mut obs = self.observation(sequence, now_ms, "133", Grade::Reported, state);
                obs.resolves = true;
                self.invalidate();
                if payload == b"B" {
                    self.ready(sequence, now_ms);
                }
                Some(obs)
            }
            b"7" => {
                if let Ok(uri) = std::str::from_utf8(payload)
                    && uri.starts_with("file://")
                {
                    self.cwd = Some(short(uri, 4096));
                }
                None
            }
            b"21337" => {
                let Some(state) = params.iter().skip(1).find_map(|p| match p.as_slice() {
                    b"status=Idle" => Some(State::Idle),
                    b"status=Working" => Some(State::Active),
                    b"status=Waiting" => Some(State::Waiting),
                    _ => None,
                }) else {
                    return Vec::new();
                };
                Some(self.observation(sequence, now_ms, "21337", Grade::Inferred, state))
            }
            _ => None,
        };
        if let Some(obs) = obs {
            self.apply(obs.clone());
            vec![obs]
        } else {
            Vec::new()
        }
    }
}

fn parse_7501(
    session: &Session,
    params: &[Vec<u8>],
    sequence: u64,
    now_ms: u64,
) -> Option<Observation> {
    let text = std::str::from_utf8(params.get(1)?).ok()?;
    let pairs: std::collections::BTreeMap<_, _> =
        text.split(':').filter_map(|p| p.split_once('=')).collect();
    let wire = *pairs.get("state")?;
    let state = match wire {
        "idle" | "done" | "clear" => State::Idle,
        "working" => State::Active,
        "blocked" => State::Waiting,
        "error" => State::Errored,
        _ => return None,
    };
    let mut obs = session.observation(sequence, now_ms, "7501", Grade::Reported, state);
    obs.resolves = wire != "blocked";
    obs.clear = wire == "clear";
    if let Some(id) = pairs.get("id") {
        if session.record.kind == "codex" || !agent_id(id) {
            return None;
        }
        obs.agent_id = Some((*id).into());
    }
    obs.title = pairs.get("title").and_then(|v| decode_text(v, 192));
    obs.detail = pairs.get("msg").and_then(|v| decode_text(v, 2048));
    obs.waiting_kind = pairs.get("kind").and_then(|v| match *v {
        "permission" => Some(WaitingKind::Permission),
        "question" => Some(WaitingKind::Question),
        "auth" => Some(WaitingKind::Auth),
        _ => None,
    });
    Some(obs)
}
pub(crate) fn agent_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 32
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.+-".contains(&b))
}
pub(crate) fn short(value: &str, max: usize) -> String {
    let mut end = value.len().min(max);
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].into()
}
fn decode_text(value: &str, max: usize) -> Option<String> {
    if value.len() > max * 2 {
        return None;
    }
    let bytes = STANDARD.decode(value).ok()?;
    if bytes.len() > max {
        return None;
    }
    String::from_utf8(bytes).ok()
}

type Frames = Vec<Vec<Vec<u8>>>;
#[derive(Clone, Default)]
pub struct OscTap(Arc<Mutex<(Frames, bool)>>);
impl OscTap {
    /// Installed on the live Term; restoration preserves the callback. Draining
    /// during replay is the caller's responsibility; replay never sends replies.
    pub fn install(term: &mut bbt_core::Term) -> Self {
        let tap = Self::default();
        let pending = tap.0.clone();
        term.on_unknown_osc(move |params| {
            let mut queue = pending.lock().expect("OSC collector poisoned");
            if params.iter().map(|p| p.len()).sum::<usize>() > 4096 || queue.0.len() >= 128 {
                queue.1 = true;
                return;
            }
            queue.0.push(params.iter().map(|p| p.to_vec()).collect());
        });
        tap
    }
    pub fn drain(&self) -> Result<Frames> {
        let mut queue = self
            .0
            .lock()
            .map_err(|_| invalid("OSC collector poisoned"))?;
        let frames = std::mem::take(&mut queue.0);
        if std::mem::take(&mut queue.1) {
            return Err(invalid("OSC observation bound exceeded; readiness invalid"));
        }
        Ok(frames)
    }
}
