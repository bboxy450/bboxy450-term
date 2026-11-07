// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! One UI owner, two data revisions and an indefinitely recurring attention cue.
use super::*;

pub fn idle_frame_demand(broken: bool) -> Model {
    let mut m = model(
        "IdleFrameDemand",
        broken,
        &[
            ("revision", 2, 0),
            ("painted", 2, 0),
            ("demand", 1, 0),
            ("waiting", 1, 0),
            ("visible", 1, 1),
            ("active", 1, 0),
            ("tick", 1, 0),
            ("pulse_due", 1, 0),
            ("pulses_this_period", 2, 0),
            ("last_render_kind", 2, 0), // 0 absent, 1 full scene, 2 retained pulse
            ("last_render_basis", 2, 0), // 0 absent, 1 data demand, 2 pulse deadline
        ],
    );
    action(
        &mut m,
        "Output",
        Var(0).lt(Int(2)),
        &[(0, Var(0).plus(Int(1))), (2, Int(1)), (5, Var(4))],
    );
    for waiting in 0..=1 {
        action(
            &mut m,
            &format!("Waiting{waiting}"),
            ne(3, waiting),
            &[(2, Int(1)), (3, Int(waiting)), (5, Var(4)), (7, Int(0))],
        );
    }
    action(
        &mut m,
        "Occlude",
        eq(4, 1),
        &[(4, Int(0)), (5, Int(0)), (6, Int(0)), (7, Int(0))],
    );
    action(
        &mut m,
        "Reveal",
        eq(4, 0),
        &[(2, Int(1)), (4, Int(1)), (5, Int(1))],
    );
    // A period boundary represents >= 1/30 second, independently of display
    // ticks. The real clock duration and CPU cost require the process witness.
    action(
        &mut m,
        "PulseDeadline",
        all([eq(3, 1), eq(4, 1)]),
        &[(5, Int(1)), (7, Int(1)), (8, Int(0))],
    );
    action(&mut m, "DisplayTick", eq(5, 1), &[(6, Int(1))]);
    action(&mut m, "LateTickAfterPause", eq(5, 0), &[]);
    action(
        &mut m,
        "PaintFullScene",
        all([eq(2, 1), eq(4, 1), eq(6, 1)]),
        &[
            (1, Var(0)),
            (2, Int(0)),
            (5, Int(0)),
            (6, Int(0)),
            (7, Int(0)),
            (9, Int(1)),
            (10, Int(1)),
        ],
    );
    let pulse_guard = all([
        eq(2, 0),
        eq(3, 1),
        eq(4, 1),
        eq(6, 1),
        eq(7, 1),
        Var(8).lt(Int(2)),
    ]);
    let pulse_updates = [
        (5, Int(0)),
        (6, Int(0)),
        (7, Int(0)),
        (8, Var(8).plus(Int(1))),
        (9, Int(2)),
        (10, Int(2)),
    ];
    action(
        &mut m,
        "PaintRetainedPulse",
        pulse_guard.clone(),
        &pulse_updates,
    );
    if broken {
        // Mutations of actual admission rules, with no synthetic failure bit.
        action(
            &mut m,
            "ForcedContinuousRedraw",
            all([eq(2, 0), eq(3, 0), eq(4, 1), eq(7, 0)]),
            &[(5, Int(1))],
        );
        action(
            &mut m,
            "ArmPulseBeforeDeadline",
            all([eq(2, 0), eq(3, 1), eq(4, 1), eq(7, 0), eq(8, 1)]),
            &[(5, Int(1)), (7, Int(1))],
        );
        let mut full_scene_pulse = pulse_updates.to_vec();
        full_scene_pulse[4] = (9, Int(1));
        action(
            &mut m,
            "RebuildFullSceneForPulse",
            pulse_guard,
            &full_scene_pulse,
        );
    }
    safe(
        &mut m,
        "PausedWithoutDemand",
        implies(all([eq(2, 0), eq(7, 0)]), eq(5, 0)),
    );
    safe(
        &mut m,
        "NoDrawableWithoutActiveLink",
        implies(eq(6, 1), eq(5, 1)),
    );
    safe(
        &mut m,
        "OcclusionCancelsWork",
        implies(eq(4, 0), all([eq(5, 0), eq(6, 0), eq(7, 0)])),
    );
    safe(
        &mut m,
        "PulseOnlyWhileWaitingAndVisible",
        implies(eq(7, 1), all([eq(3, 1), eq(4, 1)])),
    );
    safe(&mut m, "AtMostOnePulsePerDeadline", Var(8).le(Int(1)));
    safe(
        &mut m,
        "FullScenesRequireDataDemand",
        implies(eq(9, 1), eq(10, 1)),
    );
    safe(
        &mut m,
        "NewestRevisionAfterPaint",
        implies(eq(2, 0), Var(1).eq(Var(0))),
    );
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_frame_mutations_independently_violate_their_budgets() {
        const MUTATIONS: [&str; 3] = [
            "ForcedContinuousRedraw",
            "ArmPulseBeforeDeadline",
            "RebuildFullSceneForPulse",
        ];
        for (mutation, invariant) in [
            ("ForcedContinuousRedraw", "PausedWithoutDemand"),
            ("ArmPulseBeforeDeadline", "AtMostOnePulsePerDeadline"),
            ("RebuildFullSceneForPulse", "FullScenesRequireDataDemand"),
        ] {
            let mut model = idle_frame_demand(true);
            model.actions.retain(|action| {
                !MUTATIONS.contains(&action.name.as_str()) || action.name == mutation
            });
            let checked = model.check(100_000).expect("complete mutation exploration");
            assert!(
                checked
                    .violations
                    .iter()
                    .any(|failure| failure.property == invariant),
                "{mutation} must independently violate {invariant}: {:?}",
                checked.violations,
            );
        }
    }
}
