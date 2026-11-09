// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Remote supervisor link: one protocol between a local mux and the remote
//! supervisor of a swarm. The remote hosts only bboxy450-harness seats over
//! ACP; everything crossing the link is a length-prefixed JSON [frame::Frame]
//! carrying Log IR v1 envelopes byte-for-byte. See
//! `.bboxy/interfaces/remote-supervisor-link.md`.
#![forbid(unsafe_code)]
// A backend daemon crate like bbt-supervisor: the UI-only bbt-rt IO rule does not apply.
#![allow(clippy::disallowed_methods)]

pub mod attach;
pub mod client;
pub mod delivery;
pub mod frame;
pub mod install;
pub mod link;
pub mod resume;
pub mod resume_model;
pub mod serve;
