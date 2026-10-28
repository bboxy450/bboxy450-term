// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! App policy admission around the reusable observation state machine.
use super::HarnessRecord;
use crate::store::Result;
pub use bboxy450_harness_wake_up::observe::{
    Grade, Observation, OscTap, State, Status, Subagent, WaitingKind, short,
};

pub struct Session {
    pub(super) inner: bboxy450_harness_wake_up::Session,
}
impl Session {
    pub fn record_outcome(&mut self, id: crate::Ulid, outcome: &super::WakeOutcome) {
        if let Ok(outcome) = super::mapping::native_outcome(outcome) {
            self.inner.record_outcome(id, &outcome);
        }
    }

    pub fn new(record: HarnessRecord, binding: super::Binding) -> Result<Self> {
        record.validate()?;
        Ok(Self {
            inner: bboxy450_harness_wake_up::Session::new(
                super::mapping::harness(&record),
                binding,
            )?,
        })
    }
}
impl std::ops::Deref for Session {
    type Target = bboxy450_harness_wake_up::Session;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl std::ops::DerefMut for Session {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
