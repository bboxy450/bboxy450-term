use std::time::Instant;

use crate::tokens::{TOKENS, Theme};
use super::{SeatId, SwarmId, TaskId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleClass { Oracle, Worker }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role { pub name: String, pub class: RoleClass }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitKind { Permission, Question, Auth }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activity { Idle, Active, Waiting(WaitKind), Errored }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grade { Reported, Shim, Inferred }
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Where { Local, Ssh { host: String }, Cloud { provider: String, host: String } }
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lifecycle { Open, ClosedGrace { until: Instant }, ClosedRetained }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hsl { pub hue: f32, pub saturation: f32, pub lightness: f32 }
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum SeatColor { #[default] Derived, Override(Hsl) }

impl SeatColor {
    /// FNV-1a over UTF-16 code units, matching components.js rather than UTF-8.
    pub fn resolve(self, key: &str, theme: Theme) -> Hsl {
        if let Self::Override(color) = self { return color; }
        let hash = key.encode_utf16().fold(2_166_136_261u32, |h, c| (h ^ c as u32).wrapping_mul(16_777_619));
        let config = TOKENS.seat_color;
        let steps = config.hue_steps as u32;
        let bands = match theme { Theme::Dark => config.bands_dark, Theme::Light => config.bands_light };
        Hsl {
            hue: ((hash % steps) as f64 * 360.0 / steps as f64).floor() as f32,
            saturation: config.saturation as f32,
            lightness: bands[(hash / steps) as usize % bands.len()] as f32,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Seat {
    pub id: SeatId,
    pub swarm: SwarmId,
    pub key: [u8; 32],
    pub name: String,
    pub harness: HarnessId,
    pub role: Option<Role>,
    pub title: String,
    pub activity: Activity,
    pub grade: Grade,
    pub where_: Where,
    pub muted: bool,
    pub unread: Option<(u32, Role)>,
    pub lifecycle: Lifecycle,
    pub process_generation: u64,
    pub color: SeatColor,
    pub settings: bool,
    /// References only: task rows and graph projections belong to W3.2b.
    pub tasks: Vec<TaskId>,
}

impl Seat {
    pub fn new(id: SeatId, swarm: SwarmId, key: [u8; 32], name: impl Into<String>, harness: impl Into<String>) -> Self {
        Self {
            id, swarm, key, name: name.into(), harness: HarnessId(harness.into()), role: None,
            title: String::new(), activity: Activity::Idle, grade: Grade::Inferred,
            where_: Where::Local, muted: false, unread: None, lifecycle: Lifecycle::Open,
            process_generation: 1, color: SeatColor::Derived, settings: false, tasks: Vec::new(),
        }
    }
}
