// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! One integration binary; nextest isolates every test in its own process.

#[path = "support/events.rs"]
mod events;

#[path = "capability_file.rs"]
mod capability_file;
#[path = "chat_notices.rs"]
mod chat_notices;
#[path = "chat_sections.rs"]
mod chat_sections;
#[path = "cockpit.rs"]
mod cockpit;
#[path = "ctl.rs"]
mod ctl;
#[path = "ctl_records.rs"]
mod ctl_records;
#[path = "ctl_report.rs"]
mod ctl_report;
#[path = "custody.rs"]
mod custody;
#[path = "desktop.rs"]
mod desktop;
#[path = "diagnostics.rs"]
mod diagnostics;
#[path = "documents.rs"]
mod documents;
#[path = "documents_observation.rs"]
mod documents_observation;
#[path = "dream_c1.rs"]
mod dream_c1;
#[path = "e2e_night.rs"]
mod e2e_night;
#[path = "formal_contracts.rs"]
mod formal_contracts;
#[path = "gui_events.rs"]
mod gui_events;
#[path = "harness_attachments.rs"]
mod harness_attachments;
#[path = "harness_theme.rs"]
mod harness_theme;
#[path = "headless.rs"]
mod headless;
#[path = "hosts.rs"]
mod hosts;
#[path = "knowledge_e2e.rs"]
mod knowledge_e2e;
#[path = "launcher.rs"]
mod launcher;
#[path = "lifecycle.rs"]
mod lifecycle;
#[path = "messaging_flow.rs"]
mod messaging_flow;
#[path = "messaging_ui.rs"]
mod messaging_ui;
#[path = "native_signing.rs"]
mod native_signing;
#[path = "native_strip.rs"]
mod native_strip;
#[path = "plugin_login.rs"]
mod plugin_login;
#[path = "plugins.rs"]
mod plugins;
#[path = "r5_stall.rs"]
mod r5_stall;
#[path = "recovery.rs"]
mod recovery;
#[path = "remote_launch.rs"]
mod remote_launch;
#[path = "rich_client.rs"]
mod rich_client;
#[path = "rich_text_safety.rs"]
mod rich_text_safety;
#[path = "scrollback.rs"]
mod scrollback;
#[path = "settings.rs"]
mod settings;
#[path = "swarm_workdir.rs"]
mod swarm_workdir;
#[path = "test_account.rs"]
mod test_account;
#[path = "ui_status_probe.rs"]
mod ui_status_probe;
