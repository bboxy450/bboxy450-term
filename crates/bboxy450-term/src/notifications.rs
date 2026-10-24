//! One bounded native-notification worker. The UI retains at most one latest
//! desired snapshot while its single queue slot is occupied; no native method
//! or completion wait runs in the event loop or the live model/ACK owner.
use crate::attention::Desired;
use bboxy450_desktop_notifications::{Authorization, Error, NotificationId, Notifier};
use std::{
    collections::BTreeSet,
    sync::{
        Arc,
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

const AUTH_RECHECK: Duration = Duration::from_secs(30);

#[derive(Clone, Default, PartialEq, Eq)]
struct Plan {
    desired: Desired,
    known: BTreeSet<NotificationId>,
}

/// The host forwards these through its event proxy; Ready releases UI retry.
pub enum Event {
    Ready,
    Error(String),
}

/// A capacity-one handoff plus one coalesced latest desired state on the UI.
pub struct Worker {
    send: Option<SyncSender<Plan>>,
    pending: Option<Plan>,
    last: Option<Plan>,
    thread: Option<JoinHandle<()>>,
}
impl Worker {
    pub fn start(
        notifier: Box<dyn Notifier>,
        event: Arc<dyn Fn(Event) + Send + Sync>,
    ) -> std::io::Result<Self> {
        let (send, receive) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("bbt-notifications".into())
            .spawn(move || run(notifier, receive, event))?;
        Ok(Self {
            send: Some(send),
            pending: None,
            last: None,
            thread: Some(thread),
        })
    }

    /// Coalesce replaceable desired state, never block on the native worker.
    pub fn submit(&mut self, desired: Desired, known: BTreeSet<NotificationId>) {
        let plan = Plan { desired, known };
        if self.last.as_ref() != Some(&plan) {
            self.last = Some(plan.clone());
            self.pending = Some(plan);
        }
        self.flush();
    }

    pub fn flush(&mut self) {
        let Some(desired) = self.pending.take() else {
            return;
        };
        let Some(send) = &self.send else {
            return;
        };
        match send.try_send(desired) {
            Ok(()) => {}
            Err(TrySendError::Full(desired)) => self.pending = Some(desired),
            Err(TrySendError::Disconnected(_)) => self.send = None,
        }
    }

    /// Off-UI fixture shutdown. Normal app teardown only drops the sender.
    pub fn join(mut self) -> thread::Result<()> {
        self.send.take();
        self.thread.take().expect("notification worker").join()
    }
}

#[allow(
    clippy::disallowed_methods,
    reason = "dedicated notification IO thread; UI uses only try_send"
)]
fn run(
    notifier: Box<dyn Notifier>,
    receive: Receiver<Plan>,
    event: Arc<dyn Fn(Event) + Send + Sync>,
) {
    let mut plan = Plan::default();
    let mut posted = Desired::new();
    let mut withdrawn = BTreeSet::new();
    let mut retry_auth = false;
    let mut last_error = None;
    let mut queued = None;
    'service: loop {
        let next = if let Some(next) = queued.take() {
            Some(next)
        } else if retry_auth && !plan.desired.is_empty() {
            match receive.recv_timeout(AUTH_RECHECK) {
                Ok(next) => Some(next),
                Err(mpsc::RecvTimeoutError::Timeout) => None,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        } else {
            match receive.recv() {
                Ok(next) => Some(next),
                Err(_) => break,
            }
        };
        if let Some(next) = next {
            plan = next;
        }
        let desired = &plan.desired;
        // Withdraw resolved, handled or muted seats even if permission is held.
        // Known current ids also cover a notification posted by an old process.
        let removed: BTreeSet<_> = posted
            .keys()
            .chain(plan.known.iter())
            .filter(|id| !desired.contains_key(*id) && !withdrawn.contains(*id))
            .cloned()
            .collect();
        for id in removed {
            match notifier.remove(&id) {
                Ok(()) => {
                    posted.remove(&id);
                    withdrawn.insert(id);
                }
                Err(error) => report(error, event.as_ref(), &mut last_error),
            }
        }
        withdrawn.retain(|id| plan.known.contains(id) && !desired.contains_key(id));
        retry_auth = false;
        if desired
            .iter()
            .any(|(id, notice)| posted.get(id) != Some(notice))
        {
            let ready = match notifier.authorize() {
                Ok(Authorization::Granted) => true,
                Ok(Authorization::Denied) => {
                    report(Error::Denied, event.as_ref(), &mut last_error);
                    false
                }
                Ok(Authorization::Undetermined) => {
                    retry_auth = true;
                    false
                }
                Err(error) => {
                    retry_auth = true;
                    report(error, event.as_ref(), &mut last_error);
                    false
                }
            };
            if ready {
                for (id, notice) in desired {
                    if posted.get(id) == Some(notice) {
                        continue;
                    }
                    // Authorization can outlive a newer mute/focus update.
                    // Reconcile an admitted replacement before another post,
                    // rather than draining the whole obsolete snapshot first.
                    if let Ok(next) = receive.try_recv() {
                        queued = Some(next);
                        event(Event::Ready);
                        continue 'service;
                    }
                    match notifier.post(&notice.notification) {
                        Ok(()) => {
                            posted.insert(id.clone(), notice.clone());
                            last_error = None;
                        }
                        Err(error) => report(error, event.as_ref(), &mut last_error),
                    }
                }
            }
        }
        event(Event::Ready);
    }
}

fn report(error: Error, event: &dyn Fn(Event), previous: &mut Option<String>) {
    let reason = error.to_string();
    if previous.as_ref() != Some(&reason) {
        event(Event::Error(reason.clone()));
        *previous = Some(reason);
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::disallowed_methods,
        reason = "test/fixture IO, not the UI thread"
    )]
    use super::*;
    use crate::attention::{EpisodeKey, Notice};
    use bboxy450_desktop_notifications::{Notification, NotificationId};
    use std::sync::Mutex;

    struct Fake(Arc<Mutex<Vec<(String, thread::ThreadId)>>>);
    impl Notifier for Fake {
        fn authorize(&self) -> Result<Authorization, Error> {
            Ok(Authorization::Granted)
        }
        fn post(&self, n: &Notification) -> Result<(), Error> {
            self.0.lock().unwrap().push((
                format!("post:{}:{}", n.id.0, n.body),
                thread::current().id(),
            ));
            Ok(())
        }
        fn remove(&self, id: &NotificationId) -> Result<(), Error> {
            self.0
                .lock()
                .unwrap()
                .push((format!("remove:{}", id.0), thread::current().id()));
            Ok(())
        }
    }
    fn notice(body: &str, ordinal: u64) -> Desired {
        let id = NotificationId("seat".into());
        [(
            id.clone(),
            Notice {
                episode: EpisodeKey {
                    generation: 1,
                    ordinal,
                },
                notification: Notification {
                    id,
                    title: "seat · Claude".into(),
                    body: body.into(),
                },
            },
        )]
        .into()
    }
    #[test]
    fn replacement_and_removal_use_the_same_id_off_the_caller_thread() {
        let calls = Arc::new(Mutex::new(vec![]));
        let (ready, receive) = mpsc::channel();
        let mut worker = Worker::start(
            Box::new(Fake(calls.clone())),
            Arc::new(move |_| {
                ready.send(()).unwrap();
            }),
        )
        .unwrap();
        let known: BTreeSet<_> = [NotificationId("seat".into())].into();
        worker.submit(notice("first", 1), known.clone());
        receive.recv_timeout(Duration::from_secs(2)).unwrap();
        worker.submit(notice("first", 1), known.clone());
        worker.submit(notice("replacement", 1), known.clone());
        receive.recv_timeout(Duration::from_secs(2)).unwrap();
        worker.submit(Desired::new(), known);
        receive.recv_timeout(Duration::from_secs(2)).unwrap();
        worker.join().unwrap();
        let calls = calls.lock().unwrap();
        assert_eq!(
            calls
                .iter()
                .map(|(call, _)| call.as_str())
                .collect::<Vec<_>>(),
            ["post:seat:first", "post:seat:replacement", "remove:seat"]
        );
        assert!(calls.iter().all(|(_, id)| *id != thread::current().id()));
    }

    #[test]
    fn a_fresh_worker_withdraws_known_inactive_ids_from_an_old_process() {
        let calls = Arc::new(Mutex::new(vec![]));
        let (ready, receive) = mpsc::channel();
        let mut worker = Worker::start(
            Box::new(Fake(calls.clone())),
            Arc::new(move |_| {
                ready.send(()).unwrap();
            }),
        )
        .unwrap();
        worker.submit(Desired::new(), [NotificationId("old-seat".into())].into());
        receive.recv_timeout(Duration::from_secs(2)).unwrap();
        worker.join().unwrap();
        assert_eq!(
            calls
                .lock()
                .unwrap()
                .iter()
                .map(|(call, _)| call.clone())
                .collect::<Vec<_>>(),
            ["remove:old-seat"]
        );
    }

    #[test]
    fn a_mute_admitted_during_authorization_prevents_the_pending_post() {
        struct DelayedAuthorization {
            entered: mpsc::Sender<()>,
            release: Mutex<mpsc::Receiver<()>>,
            removed: mpsc::Sender<()>,
        }
        impl Notifier for DelayedAuthorization {
            fn authorize(&self) -> Result<Authorization, Error> {
                self.entered.send(()).unwrap();
                self.release
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(2))
                    .unwrap();
                Ok(Authorization::Granted)
            }
            fn post(&self, _: &Notification) -> Result<(), Error> {
                panic!("muted pending post executed");
            }
            fn remove(&self, _: &NotificationId) -> Result<(), Error> {
                self.removed.send(()).unwrap();
                Ok(())
            }
        }
        let (entered, started) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let (removed, done) = mpsc::channel();
        let mut worker = Worker::start(
            Box::new(DelayedAuthorization {
                entered,
                release: Mutex::new(wait),
                removed,
            }),
            Arc::new(|_| {}),
        )
        .unwrap();
        let known: BTreeSet<_> = [NotificationId("seat".into())].into();
        worker.submit(notice("waiting", 1), known.clone());
        started.recv_timeout(Duration::from_secs(2)).unwrap();
        worker.submit(Desired::new(), known);
        release.send(()).unwrap();
        done.recv_timeout(Duration::from_secs(2)).unwrap();
        worker.join().unwrap();
    }
}
