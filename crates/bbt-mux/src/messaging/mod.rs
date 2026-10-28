// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Local signed messaging, durable routing and serialized harness delivery.
//! Local issuer material is distinct from the deferred OpenPGP account chain.

pub mod api;
pub mod attention;
pub mod keys;
pub mod ledger;
pub mod pending;
pub mod service;
pub mod tree;
pub mod wake_cost;
pub mod wire;

pub use service::Messaging;
pub use wire::{Envelope, MessageKind, SendRequest};
