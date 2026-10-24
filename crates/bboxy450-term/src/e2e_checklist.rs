//! Local Graph-tab state. Record IO belongs to the host/A-0; the page starts
//! with the canonical promises and no claimed results.
use bbt_ui::modules::e2e_checklist::{Checklist, Record, Task};
use std::sync::Arc;

pub enum LocalTab {
    Graph(Graph),
    Settings {
        swarm: Option<bbt_ui_model::SwarmId>,
        controller: bbt_ui::modules::settings::controller::Controller,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageCommand {
    Close,
    Up,
    Down,
    PageUp,
    PageDown,
    Top,
    Bottom,
}

pub struct Graph {
    pub checklist: Arc<Checklist>,
    pub scroll: f32,
}

impl Graph {
    pub fn new(records: Vec<Record>, tasks: Vec<Task>) -> Result<Self, String> {
        Ok(Self {
            checklist: Arc::new(Checklist {
                promises: bboxy450_term::knowledge::promises()?,
                records,
                tasks,
                faults: vec![],
            }),
            scroll: 0.,
        })
    }

    pub fn scroll_by(&mut self, delta: f32, limit: f32) -> bool {
        if !delta.is_finite() || !limit.is_finite() {
            return false;
        }
        let next = (self.scroll + delta).clamp(0., limit.max(0.));
        let changed = next != self.scroll;
        self.scroll = next;
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbt_ui::modules::e2e_checklist::Tone;

    #[test]
    fn a_new_local_graph_has_no_invented_results_or_run_metadata() {
        let graph = Graph::new(vec![], vec![]).unwrap();
        let out = graph.checklist.project();
        let promises = bboxy450_term::knowledge::promises().unwrap().len();
        assert_eq!(out.rows.len(), promises);
        assert_eq!(out.summary.untested, promises);
        assert_eq!(out.summary.tone, Tone::Red);
        assert!(out.rows.iter().all(|row| row.record.is_none()));
    }

    #[test]
    fn the_host_can_replace_its_snapshot_and_scroll_stays_local_and_bounded() {
        let seed = Graph::new(vec![], vec![]).unwrap();
        let records = seed
            .checklist
            .promises
            .iter()
            .map(|p| Record {
                function: p.id.clone(),
                status: "pass".into(),
                run_by: Some("seat:fixture".into()),
                ..Default::default()
            })
            .collect();
        let mut graph = Graph::new(records, vec![]).unwrap();
        assert_eq!(graph.checklist.project().summary.tone, Tone::Green);
        assert!(graph.scroll_by(80., 100.));
        assert_eq!(graph.scroll, 80.);
        assert!(graph.scroll_by(80., 100.));
        assert_eq!(graph.scroll, 100.);
        assert!(!graph.scroll_by(f32::NAN, 100.));
        assert_eq!(graph.scroll, 100.);
        assert!(graph.scroll_by(-200., 100.));
        assert_eq!(graph.scroll, 0.);
    }
}
