// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![expect(
    clippy::disallowed_methods,
    reason = "Standalone checker and private CLI fixtures perform synchronous filesystem and process I/O; no UI thread exists here."
)]
use super::*;
use std::collections::{BTreeSet, VecDeque};
use std::path::PathBuf;
fn fixture_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(name)
}
#[test]
fn broken_controls_are_exact_diagnostics() {
    let mut codes = BTreeSet::new();
    for path in std::fs::read_dir(fixture_dir("unsound")).unwrap() {
        let path = path.unwrap().path();
        if path.extension().is_none_or(|x| x != "den") {
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap();
        let r = check(&source);
        let filename = path.file_name().unwrap().to_str().unwrap();
        let expected = &filename[..4];
        let actual: Vec<_> = r.diagnostics.iter().map(|d| d.code).collect();
        assert_eq!(
            actual,
            vec![expected],
            "{}: {}",
            path.display(),
            r.to_text()
        );
        codes.insert(expected.to_string());
        assert!(r.diagnostics[0].line > 0 && r.diagnostics[0].column > 0);
        assert!(!r.diagnostics[0].repair.is_empty());
    }
    let required = [
        "X101", "X190", "X201", "X202", "X203", "X204", "X301", "X302", "X303", "X401", "X402",
        "X501", "X502", "X503", "X505", "X601", "X602", "X603", "X651", "X701", "X702", "X703",
        "X801", "X802", "X901", "X902", "X903",
    ];
    assert_eq!(codes, required.into_iter().map(str::to_string).collect());
}
#[test]
fn sound_fixtures_match_interpreter_graphs() {
    for entry in std::fs::read_dir(fixture_dir("sound")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|x| x != "den") {
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap();
        let report = check(&source);
        assert!(
            report.is_sound(),
            "{}: {}",
            path.display(),
            report.to_text()
        );
        let s = parser::parse(&source).unwrap();
        let m = lower::model(&s).unwrap();
        let graph = m.check(eval::STATES).unwrap();
        let start = eval::initial(&s).unwrap();
        let mut seen = BTreeSet::from([start.clone()]);
        let mut queue = VecDeque::from([start]);
        let mut edges = BTreeSet::new();
        while let Some(state) = queue.pop_front() {
            for (hi, args) in eval::instances(&s).unwrap() {
                let (ci, decision) = eval::step(&s, &s.handlers[hi], &state, &args).unwrap();
                let next = match decision {
                    eval::Decision::Accepted(n, _) => n,
                    _ => state.clone(),
                };
                edges.insert((
                    lower::flatten(&s, &state),
                    format!(
                        "{}({args:?})/{}#{hi}:{ci}",
                        s.handlers[hi].event.text, s.handlers[hi].cases[ci].name.text
                    ),
                    lower::flatten(&s, &next),
                ));
                if seen.insert(next.clone()) {
                    assert!(seen.len() <= eval::STATES);
                    queue.push_back(next)
                }
            }
        }
        let lowered_states: BTreeSet<_> = graph.states.iter().cloned().collect();
        let interpreted_states = seen.iter().map(|st| lower::flatten(&s, st)).collect();
        assert_eq!(
            lowered_states,
            interpreted_states,
            "{} exact states",
            path.display()
        );
        let graph_ref = &graph;
        let actions_ref = &m.actions;
        let lowered_edges: BTreeSet<_> = graph
            .edges
            .iter()
            .enumerate()
            .flat_map(|(i, es)| {
                es.iter().map(move |e| {
                    (
                        graph_ref.states[i].clone(),
                        actions_ref[e.action].name.clone(),
                        graph_ref.states[e.to].clone(),
                    )
                })
            })
            .collect();
        assert_eq!(lowered_edges, edges, "{} exact transitions", path.display());
        println!(
            "{} states={} transitions={} changing={}",
            path.display(),
            graph.states.len(),
            edges.len(),
            edges.iter().filter(|(a, _, b)| a != b).count()
        );
    }
}
#[test]
fn remote_delivery_anchor() {
    let source = include_str!("../tests/sound/remote_delivery.den");
    let s = parser::parse(source).unwrap();
    let den = lower::model(&s).unwrap().check(eval::STATES).unwrap();
    let anchor = bboxy450_term_remote::delivery::RemoteDelivery::model()
        .check(eval::STATES)
        .unwrap();
    let changing = |g: &bboxy450_tlc::Report| {
        g.edges
            .iter()
            .enumerate()
            .map(|(i, es)| es.iter().filter(|e| e.to != i).count())
            .sum::<usize>()
    };
    assert_eq!(den.states.len(), anchor.states.len());
    assert_eq!(changing(&den), changing(&anchor));
    println!(
        "Den states={} transitions={} changing={}; anchor states={} transitions={} changing={}",
        den.states.len(),
        den.edges.iter().map(Vec::len).sum::<usize>(),
        changing(&den),
        anchor.states.len(),
        anchor.edges.iter().map(Vec::len).sum::<usize>(),
        changing(&anchor)
    );
}
#[test]
fn json_is_stable_and_escaped() {
    let r = Report {
        records: vec![],
        diagnostics: vec![Diagnostic {
            code: "X101",
            line: 2,
            column: 3,
            record: "\"\\\n\t\u{1}".into(),
            message: "broken\nline".into(),
            repair: "fix".into(),
        }],
    };
    assert_eq!(
        r.to_json(),
        "{\"records\":[],\"diagnostics\":[{\"code\":\"X101\",\"line\":2,\"column\":3,\"record\":\"\\\"\\\\\\n\\t\\u0001\",\"message\":\"broken\\nline\",\"repair\":\"fix\"}]}"
    );
    assert_eq!(r.to_text(), "2:3 X101 broken line. Repair: fix");
    assert!(!r.is_sound());
}

fn paired(body: &str, extra: &str, scenario: &str) -> String {
    format!(
        "module specs.Pair\nspec Pair {{\nintent \"paired\"\nstate {{ x: Bool = false; y: Bool = false }}\n{extra}\nevent Tick(i: Int[0..1])\non Tick(i) {{ case go if i >= 0 => {{ x = !x; y = !y; {body} }} }}\ninvariant PAIR \"flags agree\" = x == y\nscenario \"one tick\" {{ {scenario} }}\n}}"
    )
}
#[test]
fn emitted_argument_overflow_is_a_type_error_even_without_a_scenario_for_it() {
    let source = paired(
        "emit Huge(9223372036854775807 + i)",
        "effect Huge(n: Int[9223372036854775807..9223372036854775807])",
        "Tick(0) ~> accepted emits Huge(9223372036854775807)",
    );
    let r = check(&source);
    assert_eq!(
        r.diagnostics.iter().map(|d| d.code).collect::<Vec<_>>(),
        vec!["X303"],
        "{}",
        r.to_text()
    );
}
#[test]
fn bounded_event_arguments_are_checked_at_the_call() {
    for expr in ["2", "if x then 2 else 2"] {
        let source = paired("", "", &format!("Tick({expr}) ~> accepted"));
        let r = check(&source);
        assert_eq!(
            r.diagnostics.iter().map(|d| d.code).collect::<Vec<_>>(),
            vec!["X303"],
            "{}",
            r.to_text()
        );
    }
}
#[test]
fn bounded_int_updates_are_simultaneous_and_only_reachable_ranges_matter() {
    let source = "module specs._Counter\nspec _Counter {\nintent \"paired counters\"\nstate { x:Int[-1..1]=-1; y:Int[-1..1]=-1 }\nevent Tick\non Tick { case inc if x < 1 => { x = x+1; y = y+1 }; case reset otherwise => {x=-1;y=-1} }\ninvariant PAIR \"counters agree\" = x==y\nscenario \"cycle\" { Tick ~> accepted; expect x==0 && y==0; Tick ~> accepted; Tick ~> accepted; expect x == -1 }\n}";
    let r = check(source);
    assert!(r.is_sound(), "{}", r.to_text());
    assert_eq!(r.records[0].states, 3);
}
#[test]
fn sets_conditionals_quantifiers_and_effects_read_the_prestate() {
    let source = "module specs.SetTest\nspec SetTest {\nintent \"set operations\"\nenum D { A,B }\nstate {s:Set[D]=Set(); t:Set[D]=Set()}\nevent Add(d:D)\neffect Seen(s:Set[D])\non Add(d) {case add if !s(d) => {s += d; t=if s == Set() then Set(d) else Set(D.A,D.B); emit Seen(s)}; case old otherwise => ignore Known}\ninvariant PAIR \"sets agree\" = s==t\nscenario \"add members\" {Add(D.A) ~> accepted emits Seen(Set()); Add(D.B) ~> accepted emits Seen(Set(D.A)); expect forall d:D. s(d) && t(d)}\n}";
    let r = check(source);
    assert!(r.is_sound(), "{}", r.to_text());
    assert_eq!(r.records[0].states, 4);
}

#[test]
fn a_theorem_is_stated_in_lean_for_every_size_of_an_unnamed_domain() {
    let lean = lean(include_str!("../tests/sound/outbox.den"))
        .unwrap()
        .unwrap();
    assert_eq!(lean.module, "DenOutbox");
    assert_eq!(lean.theorems(), ["OUTBOX_001", "OUTBOX_002"]);
    let statements = lean.statements().unwrap();
    for expected in [
        "-- `Msg` is a type of any size",
        "  unacked : Msg → Prop",
        "  | Applied_retire (σ : (State Msg)) (m : Msg) (guard : (σ.up ∧ σ.inflight m)) :",
        "unacked := fun ξ2 => ξ2 ≠ m ∧ σ.unacked ξ2",
        "  ∀ (Msg : Type) (σ : (State Msg)), Reachable σ → OUTBOX_001 σ",
    ] {
        assert!(statements.contains(expected), "{expected}\n{statements}");
    }
    // Rejected and ignored cases change nothing and are not steps.
    assert!(!statements.contains("Submit_known"));
    assert!(lean.check().contains(
        "theorem den_check_OUTBOX_002 : DenOutbox.OUTBOX_002_statement := DenOutbox.OUTBOX_002_proved"
    ));
}
#[test]
fn a_named_member_fixes_its_domain_and_an_integer_owes_its_range() {
    let lean = lean(include_str!("../tests/sound/turnstile.den"))
        .unwrap()
        .unwrap();
    assert_eq!(lean.theorems(), ["TURN_001", "TYPE_OK"]);
    let statements = lean.statements().unwrap();
    for expected in [
        "inductive Gate where\n  | Locked\n  | Open\n  deriving DecidableEq",
        "def TYPE_OK (σ : State) : Prop := ((0 : Int) ≤ σ.coins ∧ σ.coins ≤ (3 : Int))",
        "  ∀ (σ : State), Reachable σ → TURN_001 σ",
    ] {
        assert!(statements.contains(expected), "{expected}\n{statements}");
    }
}
#[test]
fn only_a_sound_spec_with_a_theorem_exports_to_lean() {
    assert!(
        lean(include_str!("../tests/sound/remote_delivery.den"))
            .unwrap()
            .is_none()
    );
    let unsound = include_str!("../tests/unsound/X601-reachable-violation.den")
        .replace("invariant TOGGLE_001", "theorem TOGGLE_001");
    assert_eq!(lean(&unsound).unwrap_err().diagnostics[0].code, "X601");
    let clash = include_str!("../tests/unproved/weakened.den").replace("WEAK_001", "Reachable");
    assert_eq!(lean(&clash).unwrap_err().diagnostics[0].code, "X190");
}
/// The knowledge base holds the specs; the crate keeps copies as fixtures.
#[test]
fn knowledge_base_specs_match_their_fixtures() {
    let kb = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.bboxy/specs/delivery");
    if !kb.is_dir() {
        return; // A packaged crate has no knowledge base.
    }
    for name in ["outbox.den", "outbox.lean"] {
        assert_eq!(
            std::fs::read_to_string(kb.join(name)).unwrap(),
            std::fs::read_to_string(fixture_dir("sound").join(name)).unwrap(),
            "{name}"
        );
    }
}

/// One Den scenario step for an event of the generated Rust reducer and its decision.
fn recorded<S, E: std::fmt::Debug, R: std::fmt::Debug, I: std::fmt::Debug>(
    event: &impl std::fmt::Debug,
    decision: &bbt_yarn::protocol::Decision<S, E, R, I>,
) -> String {
    use bbt_yarn::protocol::Decision;
    // `Send(A)` and `Apply(A)` are Rust's Debug forms; Den writes `Send(Msg.A)`.
    let den = |value: &dyn std::fmt::Debug| format!("{value:?}").replace('(', "(Msg.");
    match decision {
        Decision::Accepted { effects, .. } if effects.is_empty() => {
            format!("{} ~> accepted", den(event))
        }
        Decision::Accepted { effects, .. } => format!(
            "{} ~> accepted emits {}",
            den(event),
            effects
                .iter()
                .map(|e| den(e))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Decision::Rejected { reason } => format!("{} ~> rejected {reason:?}", den(event)),
        Decision::Ignored { reason } => format!("{} ~> ignored {reason:?}", den(event)),
    }
}
/// A deterministic walk over the reducer: every event, in a scrambled order.
macro_rules! record {
    ($protocol:ident, $steps:expr, $allow:expr) => {{
        use bboxy450_term_remote::delivery::$protocol::{Event, Msg, State, step};
        use bbt_yarn::protocol::Decision;
        let events = [
            Event::Submit(Msg::A),
            Event::Send(Msg::A),
            Event::Arrive(Msg::A),
            Event::RemoteCrash,
            Event::Reconnect,
            Event::Submit(Msg::B),
            Event::Send(Msg::B),
            Event::Arrive(Msg::B),
            Event::AckArrive(Msg::A),
            Event::Drop,
            Event::AckArrive(Msg::B),
        ];
        let allow: fn(&State, &Event) -> bool = $allow;
        let (mut state, mut seed, mut lines) = (State::default(), 0x9E37_79B9_u32, vec![]);
        while lines.len() < $steps {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let event = events[(seed >> 16) as usize % events.len()];
            if !allow(&state, &event) {
                continue;
            }
            let decision = step(&state, event);
            lines.push(recorded(&event, &decision));
            if let Decision::Accepted { next, .. } = decision {
                state = next;
            }
        }
        lines.join("\n")
    }};
}
#[test]
fn a_trace_recorded_from_the_rust_reducer_conforms_to_the_spec() {
    let spec = include_str!("../tests/sound/remote_delivery.den");
    // The reducer accepts a crash of a remote that is already down; the spec
    // ignores it (an accepted step must change something). The states agree.
    let trace = record!(RemoteDelivery, 400, |state, event| {
        state.link_up() || !matches!(event, Event::RemoteCrash)
    });
    for outcome in [
        "accepted emits Apply(Msg.A)",
        "rejected AlreadySubmitted",
        "ignored NoAck",
    ] {
        assert!(trace.contains(outcome), "the walk never reached: {outcome}");
    }
    let report = replay(spec, &trace);
    assert!(report.is_sound(), "{}", report.to_text());
    // Teeth, on the real difference: that crash is refused at its own line.
    let crash_while_down = record!(RemoteDelivery, 400, |_, _| true);
    let refused = replay(spec, &crash_while_down);
    assert_eq!(refused.diagnostics[0].code, "X801", "{}", refused.to_text());
    let line = crash_while_down
        .lines()
        .nth(refused.diagnostics[0].line as usize - 1);
    assert_eq!(line, Some("RemoteCrash ~> accepted"));
}
#[test]
fn a_remote_that_forgets_what_it_applied_does_not_conform() {
    use bboxy450_term_remote::delivery::BrokenRemoteDelivery::{Event, Msg, State, step};
    use bbt_yarn::protocol::Decision;
    // Apply, crash, replay: the volatile remote applies the same message again.
    let mut state = State::default();
    let mut lines = vec![];
    for event in [
        Event::Submit(Msg::A),
        Event::Send(Msg::A),
        Event::Arrive(Msg::A),
        Event::RemoteCrash,
        Event::Reconnect,
        Event::Send(Msg::A),
        Event::Arrive(Msg::A),
    ] {
        let decision = step(&state, event);
        lines.push(recorded(&event, &decision));
        if let Decision::Accepted { next, .. } = decision {
            state = next;
        }
    }
    let report = replay(
        include_str!("../tests/sound/remote_delivery.den"),
        &lines.join("\n"),
    );
    assert_eq!(report.diagnostics[0].code, "X801", "{}", report.to_text());
    // The step the spec refuses is the second application, the last line.
    assert_eq!(report.diagnostics[0].line, 7);
    assert_eq!(lines[6], "Arrive(Msg.A) ~> accepted emits Apply(Msg.A)");
}
#[test]
fn a_trace_must_parse_and_name_the_specs_events() {
    let spec = include_str!("../tests/sound/outbox.den");
    assert!(replay(spec, "Submit(Msg.A) ~> accepted\nexpect unacked(Msg.A)\n").is_sound());
    for (trace, code) in [
        (
            "Submit(Msg.A) ~> accepted\nSubmit(Msg.A) ~> accepted",
            "X801",
        ),
        ("Submit(Msg.A) ~> accepted\nexpect acked(Msg.A)", "X802"),
        ("Publish(Msg.A) ~> accepted", "X201"),
        ("Submit(Msg.A) accepted", "X101"),
    ] {
        let report = replay(spec, trace);
        assert_eq!(report.diagnostics[0].code, code, "{}", report.to_text());
    }
    // The refused step is reported at its line in the trace.
    assert_eq!(
        replay(spec, "Drop ~> accepted\nDrop ~> accepted").diagnostics[0].line,
        2
    );
}
