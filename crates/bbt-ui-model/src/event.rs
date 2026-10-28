// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Intent only: the app/mux performs effects; no callback or command execution.

use crate::model::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocateWhere {
    Pane,
    Row,
    Both,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    FocusChanged {
        seat: SeatId,
        prev: Option<SeatId>,
        stage_switched: bool,
    },
    Locate {
        seat: SeatId,
        where_: LocateWhere,
    },
    SwarmToggled {
        swarm: SwarmId,
        open: bool,
    },
    SidebarResized {
        width: f32,
    },
    SplitResized {
        tab: TabId,
        path: NodePath,
        ratio: f32,
    },
    Launch {
        swarm: SwarmId,
        harness: HarnessId,
        name: String,
        where_: Where,
        place: Placement,
    },
    Place {
        seat: SeatId,
        place: Placement,
    },
    Close {
        seat: SeatId,
    },
    Undo {
        seat: SeatId,
    },
    Mute {
        seat: SeatId,
        on: bool,
    },
    SwarmNew,
    SwarmRenamed {
        swarm: SwarmId,
        name: String,
    },
    SwarmRecolored {
        swarm: SwarmId,
        color: Hsl,
    },
    OpenSettings {
        section: Section,
    },
    CloseSettings,
    SettingChanged {
        section: Section,
        key: String,
        value: String,
    },
    ShadowFilter {
        shadow: Option<ShadowId>,
    },
    UnreadCleared {
        seat: SeatId,
    },
    Pulse {
        row: RowId,
        ms: u32,
    },
    Notice {
        swarm: SwarmId,
        item: StreamItem,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Update {
    SeatActivity {
        seat: SeatId,
        activity: Activity,
        grade: Grade,
    },
    SeatGeneration {
        seat: SeatId,
        generation: u64,
    },
    MessageArrived {
        seat: SeatId,
        unread: u32,
        role: Role,
    },
    SeatClosedByDaemon {
        seat: SeatId,
    },
}
