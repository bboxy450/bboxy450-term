//! Explicit boundary between the reusable kit, app wire and PTY host.
use super::{HarnessRecord, WakeOutcome};
use crate::{
    LaunchSpec, Ulid,
    store::{Result, invalid},
};
use bboxy450_harness_wake_up as kit;

pub fn harness(record: &HarnessRecord) -> kit::HarnessRecord {
    kit::HarnessRecord {
        kind: record.kind.clone(),
        name: record.name.clone(),
        command: record.command.clone(),
        wake_method: record.wake_method,
        state_source: record.state_source,
        hooks: record.hooks,
    }
}
pub fn state(state: kit::State) -> bbt_proto::messaging::State {
    use bbt_proto::messaging::State as Wire;
    match state {
        kit::State::Active => Wire::Active,
        kit::State::Idle => Wire::Idle,
        kit::State::Waiting => Wire::Waiting,
        kit::State::Errored => Wire::Errored,
    }
}
pub fn native_state(state: bbt_proto::messaging::State) -> kit::State {
    use bbt_proto::messaging::State as Wire;
    match state {
        Wire::Active => kit::State::Active,
        Wire::Idle => kit::State::Idle,
        Wire::Waiting => kit::State::Waiting,
        Wire::Errored => kit::State::Errored,
    }
}
pub fn waiting(kind: kit::WaitingKind) -> bbt_proto::WaitKind {
    match kind {
        kit::WaitingKind::Permission => bbt_proto::WaitKind::Permission,
        kit::WaitingKind::Question => bbt_proto::WaitKind::Question,
        kit::WaitingKind::Auth => bbt_proto::WaitKind::Auth,
    }
}
pub fn native_waiting(kind: bbt_proto::WaitKind) -> kit::WaitingKind {
    match kind {
        bbt_proto::WaitKind::Permission => kit::WaitingKind::Permission,
        bbt_proto::WaitKind::Question => kit::WaitingKind::Question,
        bbt_proto::WaitKind::Auth => kit::WaitingKind::Auth,
    }
}
pub fn grade(grade: kit::Grade) -> bbt_proto::Grade {
    match grade {
        kit::Grade::Reported => bbt_proto::Grade::Reported,
        kit::Grade::Shim => bbt_proto::Grade::Shim,
        kit::Grade::Inferred => bbt_proto::Grade::Inferred,
    }
}
pub fn native_grade(grade: bbt_proto::Grade) -> kit::Grade {
    match grade {
        bbt_proto::Grade::Reported => kit::Grade::Reported,
        bbt_proto::Grade::Shim => kit::Grade::Shim,
        bbt_proto::Grade::Inferred => kit::Grade::Inferred,
    }
}
pub fn evidence(evidence: kit::Evidence) -> super::Evidence {
    match evidence {
        kit::Evidence::SenderStored => super::Evidence::SenderStored,
        kit::Evidence::RecipientStored => super::Evidence::RecipientStored,
        kit::Evidence::NativeQueued => super::Evidence::NativeQueued,
        kit::Evidence::NativeWritten => super::Evidence::NativeWritten,
        kit::Evidence::NativeUnknown => super::Evidence::NativeUnknown,
    }
}
pub fn native_evidence(evidence: super::Evidence) -> Result<kit::Evidence> {
    Ok(match evidence {
        super::Evidence::SenderStored => kit::Evidence::SenderStored,
        super::Evidence::RecipientStored => kit::Evidence::RecipientStored,
        super::Evidence::NativeQueued => kit::Evidence::NativeQueued,
        super::Evidence::NativeWritten => kit::Evidence::NativeWritten,
        super::Evidence::NativeUnknown => kit::Evidence::NativeUnknown,
        super::Evidence::ApplicationAcknowledged => {
            return Err(invalid("adapter cannot claim application acknowledgement"));
        }
    })
}
pub fn outcome(outcome: kit::WakeOutcome) -> WakeOutcome {
    WakeOutcome {
        evidence: evidence(outcome.evidence),
        native_id: outcome.native_id,
        note: outcome.note,
    }
}
pub fn native_outcome(outcome: &WakeOutcome) -> Result<kit::WakeOutcome> {
    Ok(kit::WakeOutcome {
        evidence: native_evidence(outcome.evidence)?,
        native_id: outcome.native_id.clone(),
        note: outcome.note.clone(),
    })
}
pub fn launch_plan(plan: kit::LaunchPlan, seat: Ulid, generation: u64) -> super::LaunchPlan {
    let mut spec = LaunchSpec::new(plan.program, plan.cwd);
    spec.seat = seat;
    spec.generation = generation;
    spec.args = plan.args;
    spec.env = plan.env;
    spec.companion = plan.runtime.map(|r| {
        Box::new(bbt_supervisor::protocol::CompanionSpec {
            program: r.program,
            args: r.args,
            env: r.env,
            ready_socket: r.endpoint,
        })
    });
    super::LaunchPlan {
        spec,
        overlay: plan.overlay,
        harness_session_id: plan.harness_session_id,
        notes: plan.notes,
        shared_runtime: plan.shared_runtime,
    }
}
pub fn native_launch_plan(plan: super::LaunchPlan) -> kit::LaunchPlan {
    kit::LaunchPlan {
        program: plan.spec.program,
        args: plan.spec.args,
        cwd: plan.spec.cwd,
        env: plan.spec.env,
        runtime: plan.spec.companion.map(|c| kit::launch::RuntimePlan {
            program: c.program,
            args: c.args,
            env: c.env,
            endpoint: c.ready_socket,
        }),
        overlay: plan.overlay,
        harness_session_id: plan.harness_session_id,
        notes: plan.notes,
        shared_runtime: plan.shared_runtime,
    }
}

/// Host facts plus the app's render/tool state form the kit's arbiter view.
pub fn arbiter(
    binding: kit::Binding,
    context: &crate::protocol::InputContext,
    active_tool: bool,
    sync_pending: bool,
    screen_switch_pending: bool,
    bracketed_paste: bool,
) -> kit::ArbiterView {
    kit::ArbiterView {
        binding,
        operator_epoch: context.epoch,
        foreground_pid: context.foreground_pid,
        operator_draft: context.draft,
        active_tool,
        sync_pending,
        screen_switch_pending,
        canonical: context.canonical,
        echo: context.echo,
        bracketed_paste,
    }
}
/// Map the arbiter-owned facts back, retaining host-only sequence/settling data.
pub fn input_context(
    view: &kit::ArbiterView,
    mut stamp: crate::protocol::InputContext,
) -> crate::protocol::InputContext {
    stamp.epoch = view.operator_epoch;
    stamp.foreground_pid = view.foreground_pid;
    stamp.draft = view.operator_draft;
    stamp.canonical = view.canonical;
    stamp.echo = view.echo;
    stamp
}
/// The MCP command owns the ID, text and explicit Enter choice.
pub fn literal(id: Ulid, text: &str, submit: bool) -> kit::guard::LiteralInput<'_> {
    kit::guard::LiteralInput { id, text, submit }
}
pub fn literal_parts(input: kit::guard::LiteralInput<'_>) -> (Ulid, &str, bool) {
    (input.id, input.text, input.submit)
}

#[cfg(test)]
mod tests {
    use super::*;
    // keeps: decision #274
    #[test]
    fn state_round_trip() {
        for value in [
            kit::State::Active,
            kit::State::Idle,
            kit::State::Waiting,
            kit::State::Errored,
        ] {
            assert_eq!(native_state(state(value)), value);
            assert_eq!(
                serde_json::to_value(value).unwrap(),
                serde_json::to_value(state(value)).unwrap()
            );
        }
    }
    // keeps: decision #274
    #[test]
    fn waiting_kind_round_trip() {
        for value in [
            kit::WaitingKind::Permission,
            kit::WaitingKind::Question,
            kit::WaitingKind::Auth,
        ] {
            assert_eq!(native_waiting(waiting(value)), value);
        }
    }
    // keeps: decision #274
    #[test]
    fn grade_round_trip() {
        for value in [kit::Grade::Reported, kit::Grade::Shim, kit::Grade::Inferred] {
            assert_eq!(native_grade(grade(value)), value);
        }
    }
    // keeps: decision #274
    #[test]
    fn evidence_and_outcome_round_trip() {
        for value in [
            kit::Evidence::SenderStored,
            kit::Evidence::RecipientStored,
            kit::Evidence::NativeQueued,
            kit::Evidence::NativeWritten,
            kit::Evidence::NativeUnknown,
        ] {
            assert_eq!(native_evidence(evidence(value)).unwrap(), value);
            let native = kit::WakeOutcome {
                evidence: value,
                native_id: Some("correlation".into()),
                note: "unverified receiver".into(),
            };
            let wire = outcome(native.clone());
            assert_eq!(native_outcome(&wire).unwrap(), native);
            assert_eq!(
                serde_json::to_value(&wire).unwrap(),
                serde_json::to_value(&native).unwrap()
            );
        }
        assert!(native_evidence(super::super::Evidence::ApplicationAcknowledged).is_err());
    }
    // keeps: decision #274
    #[test]
    fn harness_methods_round_trip_through_wire() {
        for method in [
            kit::WakeMethod::ClaudeSocket,
            kit::WakeMethod::CodexRpc,
            kit::WakeMethod::Pty,
            kit::WakeMethod::Operator,
        ] {
            for source in [
                kit::StateSource::Reported,
                kit::StateSource::Hooks,
                kit::StateSource::Cadence,
            ] {
                let mut record = HarnessRecord::builtin("claude").unwrap();
                record.wake_method = method;
                record.state_source = source;
                let wire = serde_json::to_vec(&record).unwrap();
                let decoded: HarnessRecord = serde_json::from_slice(&wire).unwrap();
                assert_eq!(decoded, record);
                let native = harness(&decoded);
                assert_eq!(native.wake_method, method);
                assert_eq!(native.state_source, source);
                assert_eq!(native.command, record.command);
                assert_eq!(native.name, record.name);
                assert_eq!(native.kind, record.kind);
                assert_eq!(native.hooks, record.hooks);
            }
        }
    }
    // keeps: decision #274
    #[test]
    fn launch_plan_round_trip_keeps_host_identity_and_default_size() {
        let id = Ulid::new();
        let native = kit::LaunchPlan {
            program: "/bin/zsh".into(),
            args: vec!["-il".into()],
            cwd: "/tmp".into(),
            env: [("A".into(), "B".into())].into(),
            overlay: "/tmp/fixture-overlay".into(),
            harness_session_id: Some("session".into()),
            notes: vec!["same note"],
            shared_runtime: Some("/tmp/borrowed-daemon.sock".into()),
            runtime: Some(kit::launch::RuntimePlan {
                program: "/bin/sleep".into(),
                args: vec!["600".into()],
                env: [("RUNTIME_ONLY".into(), "value".into())].into(),
                endpoint: "/tmp/fixture-overlay/service.sock".into(),
            }),
        };
        let expected_runtime = native.runtime.clone();
        let expected_shared = native.shared_runtime.clone();
        let expected = serde_json::json!({"program":native.program,"args":native.args,"cwd":native.cwd,"env":native.env,
            "overlay":native.overlay,"session":native.harness_session_id,"notes":native.notes});
        let host = launch_plan(native, id, 7);
        assert_eq!(host.spec.seat, id);
        assert_eq!(host.spec.generation, 7);
        assert_eq!(host.spec.size, crate::SeatSize::default());
        host.spec.validate().unwrap();
        let back = native_launch_plan(host);
        assert_eq!(back.runtime, expected_runtime);
        assert_eq!(back.shared_runtime, expected_shared);
        assert_eq!(
            serde_json::json!({"program":back.program,"args":back.args,"cwd":back.cwd,"env":back.env,
            "overlay":back.overlay,"session":back.harness_session_id,"notes":back.notes}),
            expected
        );
    }
    // keeps: decision #274
    #[test]
    fn arbiter_round_trip_keeps_host_only_stamp() {
        let binding = kit::Binding {
            seat: Ulid::new(),
            session: "native-session".into(),
            pid: 42,
            process_start: "start".into(),
            generation: 3,
            attachment_generation: 9,
            runtime: None,
        };
        for flag in [false, true] {
            let context = crate::protocol::InputContext {
                epoch: 5,
                last_input_ms: 8,
                sequence: 13,
                draft: flag,
                foreground_pid: Some(42),
                canonical: flag,
                echo: !flag,
                settled: !flag,
            };
            let view = arbiter(binding.clone(), &context, flag, !flag, flag, !flag);
            assert_eq!(view.binding, binding);
            assert_eq!(view.active_tool, flag);
            assert_eq!(view.sync_pending, !flag);
            assert_eq!(view.screen_switch_pending, flag);
            assert_eq!(view.bracketed_paste, !flag);
            assert_eq!(input_context(&view, context.clone()), context);
        }
    }
    // keeps: decision #274
    #[test]
    fn literal_input_round_trip_keeps_explicit_enter_and_identity() {
        assert_eq!(
            kit::guard::MAX_INPUT_BYTES,
            crate::protocol::MAX_INPUT_BYTES
        );
        for submit in [false, true] {
            let id = Ulid::new();
            let text = "literal\ntext";
            assert_eq!(literal_parts(literal(id, text, submit)), (id, text, submit));
        }
    }
}
