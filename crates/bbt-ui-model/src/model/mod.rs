// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
pub mod ids;
pub mod seat;
pub mod split;
pub mod swarm;
pub mod tab;
pub mod window;

pub use ids::*;
pub use seat::*;
pub use split::*;
pub use swarm::*;
pub use tab::*;
pub use window::*;

mod spawn;
pub use spawn::{FieldKey, Source, SpawnField, SpawnParams, ValueChoice};
