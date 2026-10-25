//! An opt-in recording port for unattended fixtures, with no OS presentation.
use bboxy450_desktop_notifications::{
    Authorization, Error, Notification, NotificationId, Notifier, Response,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
};

const CAPACITY: usize = 1024;
const HISTORY: usize = 256;
const SNAPSHOT_ROWS: usize = 64;

#[derive(Clone)]
pub struct Recorder {
    state: Arc<Mutex<State>>,
    response: Arc<dyn Fn(Response) + Send + Sync>,
}

#[derive(Default)]
struct State {
    outstanding: BTreeMap<NotificationId, Notification>,
    records: VecDeque<Value>,
    sequence: u64,
    omitted_records: u64,
    posts: u64,
    replacements: u64,
    removals: u64,
    activations: u64,
}
impl State {
    fn record(
        &mut self,
        operation: &str,
        stage: &str,
        id: Option<&NotificationId>,
        outcome: Option<&str>,
    ) {
        self.sequence += 1;
        if self.records.len() == HISTORY {
            self.records.pop_front();
            self.omitted_records += 1;
        }
        let id = id.map(|id| {
            if id.0.len() <= 128 {
                id.0.as_str()
            } else {
                "[overlong id]"
            }
        });
        self.records
            .push_back(json!({"sequence":self.sequence,"operation":operation,
            "stage":stage,"id":id,"outcome":outcome}));
    }
}

impl Recorder {
    pub fn new(response: Arc<dyn Fn(Response) + Send + Sync>) -> Self {
        Self {
            state: Arc::default(),
            response,
        }
    }

    /// A memory-only, non-waiting read for an event-loop owner. Missing rows are
    /// counted; the history never substitutes for an OS-delivery receipt.
    pub fn snapshot(&self) -> Result<Value, &'static str> {
        let state = self
            .state
            .try_lock()
            .map_err(|_| "notification recording is busy; retry the read")?;
        let outstanding: Vec<_> = state
            .outstanding
            .values()
            .take(SNAPSHOT_ROWS)
            .map(|n| json!({"id":n.id.0,"title":n.title,"body":n.body}))
            .collect();
        Ok(bbt_mux::debug::sanitize(
            json!({"backend":"recording","os_presentation":false,
            "posts":state.posts,"replacements":state.replacements,"removals":state.removals,"activations":state.activations,
            "outstanding_count":state.outstanding.len(),"outstanding":outstanding,
            "omitted_outstanding":state.outstanding.len().saturating_sub(SNAPSHOT_ROWS),
            "records":state.records,"omitted_records":state.omitted_records,
            "activation_basis":"callback submission; the GUI owner separately acknowledges completed focus"}),
        ))
    }

    /// Exercise the same callback the native delegate calls. The lock is
    /// released first, and an absent/withdrawn id cannot focus another seat.
    pub fn activate(&self, id: &NotificationId) -> Result<(), &'static str> {
        {
            let mut state = self
                .state
                .try_lock()
                .map_err(|_| "notification recording is busy; retry activation")?;
            state.record("activate", "request", Some(id), None);
            if !state.outstanding.contains_key(id) {
                state.record("activate", "completion", Some(id), Some("not_outstanding"));
                return Err("notification is not outstanding");
            }
            state.activations += 1;
            state.record(
                "activate",
                "completion",
                Some(id),
                Some("callback_submitted"),
            );
        }
        (self.response)(Response::Clicked(id.clone()));
        Ok(())
    }

    #[allow(
        clippy::disallowed_methods,
        reason = "recording Notifier methods run on the guarded notification IO worker"
    )]
    fn perform(
        &self,
        operation: &str,
        id: Option<&NotificationId>,
        change: impl FnOnce(&mut State) -> Result<(), Error>,
    ) -> Result<(), Error> {
        bbt_rt::assert_not_ui_thread();
        let mut state = self
            .state
            .lock()
            .map_err(|_| Error::Backend("recording state unavailable".into()))?;
        let operation =
            if operation == "post" && id.is_some_and(|id| state.outstanding.contains_key(id)) {
                "replace"
            } else {
                operation
            };
        state.record(operation, "request", id, None);
        let result = change(&mut state);
        state.record(
            operation,
            "completion",
            id,
            Some(if result.is_ok() {
                "recorded"
            } else {
                "refused"
            }),
        );
        result
    }
}

impl Notifier for Recorder {
    fn authorize(&self) -> Result<Authorization, Error> {
        self.perform("authorize", None, |_| Ok(()))?;
        Ok(Authorization::Granted)
    }
    fn post(&self, notice: &Notification) -> Result<(), Error> {
        self.perform("post", Some(&notice.id), |state| {
            if notice.id.0.len() > 128 || notice.title.len() > 1024 || notice.body.len() > 4096 {
                return Err(Error::Backend(
                    "recording notification exceeds its field bound".into(),
                ));
            }
            if state.outstanding.len() >= CAPACITY && !state.outstanding.contains_key(&notice.id) {
                return Err(Error::Backend(
                    "recording notification capacity reached".into(),
                ));
            }
            state.posts += 1;
            state.replacements += u64::from(state.outstanding.contains_key(&notice.id));
            state.outstanding.insert(notice.id.clone(), notice.clone());
            Ok(())
        })
    }
    fn remove(&self, id: &NotificationId) -> Result<(), Error> {
        self.perform("remove", Some(id), |state| {
            state.removals += 1;
            state.outstanding.remove(id);
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn stable_id_replaces_and_only_an_outstanding_id_can_activate_the_callback() {
        let (send, received) = mpsc::channel();
        let backend = Recorder::new(Arc::new(move |response| send.send(response).unwrap()));
        let id = NotificationId("exact-seat".into());
        let mut notice = Notification {
            id: id.clone(),
            title: "fixture".into(),
            body: "first".into(),
        };
        backend.authorize().unwrap();
        backend.post(&notice).unwrap();
        notice.body = "replacement".into();
        backend.post(&notice).unwrap();
        let snapshot = backend.snapshot().unwrap();
        assert_eq!(snapshot["outstanding_count"], 1);
        assert_eq!(snapshot["posts"], 2);
        assert_eq!(snapshot["outstanding"][0]["body"], "replacement");
        assert_eq!(snapshot["backend"], "recording");
        assert_eq!(snapshot["os_presentation"], false);
        backend.activate(&id).unwrap();
        assert_eq!(received.try_recv().unwrap(), Response::Clicked(id.clone()));
        backend.remove(&id).unwrap();
        assert!(backend.activate(&id).is_err());
        assert!(received.try_recv().is_err());
        let snapshot = backend.snapshot().unwrap();
        assert_eq!(snapshot["outstanding_count"], 0);
        let rows = snapshot["records"].as_array().unwrap();
        assert!(
            rows.windows(2)
                .all(|rows| rows[0]["sequence"].as_u64().unwrap() + 1
                    == rows[1]["sequence"].as_u64().unwrap())
        );
        assert!(
            rows.iter()
                .any(|r| r["operation"] == "post" && r["stage"] == "request")
        );
        assert!(
            rows.iter()
                .any(|r| r["operation"] == "post" && r["stage"] == "completion")
        );
    }

    #[test]
    fn recording_readback_redacts_encoded_key_bytes() {
        let backend = Recorder::new(Arc::new(|_| {}));
        let key = "ab".repeat(32);
        let bytes = serde_json::to_string(&vec![171u8; 32]).unwrap();
        backend
            .post(&Notification {
                id: NotificationId("fixture".into()),
                title: key.clone(),
                body: bytes.clone(),
            })
            .unwrap();
        let snapshot = backend.snapshot().unwrap().to_string();
        assert!(!snapshot.contains(&key));
        assert!(!snapshot.contains(&bytes));
    }

    #[test]
    fn refused_ids_and_retained_history_stay_bounded_before_redaction() {
        let backend = Recorder::new(Arc::new(|_| {}));
        let id = NotificationId("ü".repeat(2048));
        assert!(
            backend
                .post(&Notification {
                    id: id.clone(),
                    title: "fixture".into(),
                    body: "fixture".into(),
                })
                .is_err()
        );
        assert!(backend.activate(&id).is_err());
        for _ in 0..HISTORY {
            backend.remove(&id).unwrap();
        }
        let state = backend.state.try_lock().unwrap();
        assert_eq!(state.records.len(), HISTORY);
        assert!(state.omitted_records > 0);
        assert!(
            state
                .records
                .iter()
                .all(|row| row["id"].as_str().unwrap().len() <= 128)
        );
        assert!(state.outstanding.is_empty());
    }
}
