// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! A single test-account launch barrier. Other commands keep their ordinary path.
use bbt_mux::Ulid;

pub(super) struct Gate<T> {
    armed: bool,
    held: Option<(Ulid, T)>,
}
impl<T> Default for Gate<T> {
    fn default() -> Self {
        Self {
            armed: false,
            held: None,
        }
    }
}
impl<T> Gate<T> {
    pub(super) fn arm(&mut self, test_account: bool) -> Result<(), &'static str> {
        if !test_account {
            return Err("test_account_required: spawn gate is refused on real accounts");
        }
        if self.armed || self.held.is_some() {
            return Err("spawn_gate_busy: one launch is already reserved");
        }
        self.armed = true;
        Ok(())
    }
    pub(super) fn capture(&mut self, op: Ulid, take: impl FnOnce() -> T) {
        if self.armed {
            self.armed = false;
            self.held = Some((op, take()));
        }
    }
    pub(super) fn release(&mut self, test_account: bool, op: Ulid) -> Result<T, &'static str> {
        if !test_account {
            return Err("test_account_required: spawn gate is refused on real accounts");
        }
        if !self.held.as_ref().is_some_and(|(held, _)| *held == op) {
            return Err("spawn_gate_not_held: operation does not own the held launch");
        }
        Ok(self.held.take().expect("matched held operation").1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // keeps: spec e2e-script-map §L2 (captured placement before later focus)
    #[test]
    fn only_one_admitted_launch_is_held_and_only_its_operation_releases_it_once() {
        let mut gate = Gate::default();
        let mut queue = std::collections::VecDeque::from(["launch", "focus", "layout"]);
        gate.arm(true).unwrap();
        gate.capture(Ulid(1), || queue.pop_front().unwrap());
        assert_eq!(queue.into_iter().collect::<Vec<_>>(), ["focus", "layout"]);
        assert!(gate.arm(true).unwrap_err().starts_with("spawn_gate_busy:"));
        assert!(
            gate.release(true, Ulid(2))
                .unwrap_err()
                .starts_with("spawn_gate_not_held:")
        );
        gate.capture(Ulid(3), || {
            panic!("later launches do not replace the reservation")
        });
        assert_eq!(gate.release(true, Ulid(1)).unwrap(), "launch");
        assert!(gate.release(true, Ulid(1)).is_err());
        gate.arm(true).unwrap();
    }
    // keeps: spec e2e-runs-unattended (fixture authority is a throwaway test account)
    #[test]
    fn live_accounts_cannot_arm_or_release_and_unarmed_dispatch_is_untouched() {
        let mut gate = Gate::<()>::default();
        assert!(
            gate.arm(false)
                .unwrap_err()
                .starts_with("test_account_required:")
        );
        gate.capture(Ulid(1), || panic!("unarmed ordinary dispatch is untouched"));
        gate.arm(true).unwrap();
        gate.capture(Ulid(1), || ());
        assert!(
            gate.release(false, Ulid(1))
                .unwrap_err()
                .starts_with("test_account_required:")
        );
        gate.release(true, Ulid(1)).unwrap();
    }
}
