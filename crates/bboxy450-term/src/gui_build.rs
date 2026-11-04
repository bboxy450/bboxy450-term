// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! A compiled source stamp and a process-birth GUI instance; never a backend generation.
use std::sync::OnceLock;
pub fn identity() -> bbt_proto::gui::GuiProcess {
    static INSTANCE: OnceLock<bbt_mux::Ulid> = OnceLock::new();
    bbt_proto::gui::GuiProcess {
        pid: std::process::id(),
        instance: *INSTANCE.get_or_init(bbt_mux::Ulid::new),
        source_revision: env!("BBOXY_GUI_SOURCE").into(),
    }
}
