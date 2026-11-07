// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! One integration binary; nextest isolates every test in its own process.

mod common;

#[path = "adapters.rs"]
mod adapters;
#[path = "attention_policy.rs"]
mod attention_policy;
#[path = "capture_busy.rs"]
mod capture_busy;
#[path = "capture_fairness.rs"]
mod capture_fairness;
#[path = "checkpoint_scheduling.rs"]
mod checkpoint_scheduling;
#[path = "dream_c1.rs"]
mod dream_c1;
#[path = "gates_seats.rs"]
mod gates_seats;
#[path = "gui_endpoint.rs"]
mod gui_endpoint;
#[path = "gui_events.rs"]
mod gui_events;
#[path = "harmony_host.rs"]
mod harmony_host;
#[path = "hook_gate.rs"]
mod hook_gate;
#[path = "identity.rs"]
mod identity;
#[path = "identity_io.rs"]
mod identity_io;
#[path = "identity_local.rs"]
mod identity_local;
#[path = "input_diagnostics.rs"]
mod input_diagnostics;
#[path = "input_latency.rs"]
mod input_latency;
#[path = "knowledge.rs"]
mod knowledge;
#[path = "knowledge_gui.rs"]
mod knowledge_gui;
#[path = "knowledge_provider.rs"]
mod knowledge_provider;
#[path = "knowledge_views.rs"]
mod knowledge_views;
#[path = "logic.rs"]
mod logic;
#[path = "logic_exit.rs"]
mod logic_exit;
#[path = "mcp.rs"]
mod mcp;
#[path = "measurements.rs"]
mod measurements;
#[path = "messaging.rs"]
mod messaging;
#[path = "messaging_flow.rs"]
mod messaging_flow;
#[path = "messaging_suppression.rs"]
mod messaging_suppression;
#[path = "messaging_threads.rs"]
mod messaging_threads;
#[path = "native_unlock.rs"]
mod native_unlock;
#[path = "plugin_attribution.rs"]
mod plugin_attribution;
#[path = "plugin_control.rs"]
mod plugin_control;
#[path = "plugin_host.rs"]
mod plugin_host;
#[path = "plugin_messages.rs"]
mod plugin_messages;
#[path = "plugin_registry.rs"]
mod plugin_registry;
#[path = "plugin_sdk_examples.rs"]
mod plugin_sdk_examples;
#[path = "plugin_setup.rs"]
mod plugin_setup;
#[path = "plugin_transport.rs"]
mod plugin_transport;
#[path = "plugins.rs"]
mod plugins;
#[path = "provenance.rs"]
mod provenance;
#[path = "provider.rs"]
mod provider;
#[path = "provider_fake.rs"]
mod provider_fake;
#[path = "provider_gui.rs"]
mod provider_gui;
#[path = "recovery.rs"]
mod recovery;
#[path = "research_lineage.rs"]
mod research_lineage;
#[path = "ring.rs"]
mod ring;
#[path = "seat_wiring.rs"]
mod seat_wiring;
#[path = "socket_backlog.rs"]
mod socket_backlog;
#[path = "storage_bus_index.rs"]
mod storage_bus_index;
#[path = "swarm_metadata.rs"]
mod swarm_metadata;
#[path = "test_account.rs"]
mod test_account;
#[path = "ui_guard.rs"]
mod ui_guard;
#[path = "workspace_roots.rs"]
mod workspace_roots;
