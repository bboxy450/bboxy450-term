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
        "X501", "X502", "X503", "X601", "X602", "X603", "X701", "X702", "X801", "X802", "X901",
        "X902", "X903",
    ];
    assert_eq!(codes, required.into_iter().map(str::to_string).collect());
}
#[test]
fn sound_fixtures_match_interpreter_graphs() {
    for entry in std::fs::read_dir(fixture_dir("sound")).unwrap() {
        let path = entry.unwrap().path();
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
