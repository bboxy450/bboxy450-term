//! What another operator door asks of the window, relayed to the window's own session.
//! Capture: a `GuiAction::Capture` from any operator door (the window or `ctl`) becomes
//! `GuiReply::CaptureAsk`; the window writes the PNG off its UI thread and answers with
//! `GuiAction::Captured`, which completes the asker's wait. Spawn: a seat spawned through
//! `ctl` becomes `GuiReply::OperatorSpawned`, so the window places it on stage. Logic
//! renders nothing; it only relays.
use crate::Ulid;
use std::{collections::HashMap, sync::Mutex, time::Duration};
use tokio::sync::{broadcast, oneshot};

/// How long an asker waits for the window to present, read back and write the frame;
/// inside `ctl`'s 15 s read timeout.
const WAIT: Duration = Duration::from_secs(10);

/// One capture the window is asked to make.
#[derive(Debug, Clone)]
pub(crate) struct Ask {
    pub id: Ulid,
    pub path: String,
    pub seat: Option<Ulid>,
}

/// What the window's session forwards.
#[derive(Debug, Clone)]
pub(crate) enum Notice {
    Ui {
        id: Ulid,
        request: bbt_proto::gui::UiRequest,
    },
    Capture(Ask),
    App {
        id: Ulid,
        request: bbt_proto::gui::AppRequest,
    },
    AppStopping {
        id: Ulid,
    },
    Spawned {
        op: Ulid,
        seat: bbt_proto::SeatId,
        spec: bbt_proto::SpawnSpec,
    },
    Close {
        op: Ulid,
        seat: bbt_proto::SeatId,
    },
    UndoClose {
        op: Ulid,
        seat: bbt_proto::SeatId,
    },
    Focus {
        op: Ulid,
        seat: bbt_proto::SeatId,
    },
}
impl Notice {
    pub(crate) fn reply(self) -> bbt_proto::GuiReply {
        match self {
            Self::Ui { id, request } => bbt_proto::GuiReply::UiAsk { op: id, request },
            Self::App { id, request } => bbt_proto::GuiReply::AppAsk { op: id, request },
            Self::AppStopping { id } => bbt_proto::GuiReply::AppStopping { op: id },
            Self::Capture(ask) => bbt_proto::GuiReply::CaptureAsk {
                op: ask.id,
                path: ask.path,
                seat: ask.seat,
            },
            Self::Spawned { op, seat, spec } => {
                bbt_proto::GuiReply::OperatorSpawned { op, seat, spec }
            }
            Self::Close { op, seat } => bbt_proto::GuiReply::OperatorClose { op, seat },
            Self::UndoClose { op, seat } => bbt_proto::GuiReply::OperatorUndoClose { op, seat },
            Self::Focus { op, seat } => bbt_proto::GuiReply::OperatorFocus { op, seat },
        }
    }
}

pub(crate) struct Captures {
    asks: broadcast::Sender<Notice>,
    pending: Mutex<HashMap<Ulid, oneshot::Sender<Result<bbt_proto::gui::CaptureFrame, String>>>>,
    app: Mutex<HashMap<Ulid, oneshot::Sender<bbt_proto::gui::AppStatus>>>,
    focus: Mutex<HashMap<Ulid, oneshot::Sender<Result<bbt_proto::gui::FocusFrame, String>>>>,
    ui: Mutex<HashMap<Ulid, oneshot::Sender<Result<bbt_proto::gui::UiLayout, String>>>>,
}
impl Default for Captures {
    fn default() -> Self {
        Self {
            asks: broadcast::channel(8).0,
            pending: Mutex::default(),
            app: Mutex::default(),
            focus: Mutex::default(),
            ui: Mutex::default(),
        }
    }
}
impl Captures {
    pub(crate) async fn ui(
        &self,
        id: Ulid,
        request: bbt_proto::gui::UiRequest,
    ) -> Result<bbt_proto::gui::UiLayout, String> {
        self.ui_wait(id, Notice::Ui { id, request }).await
    }
    async fn ui_wait(&self, id: Ulid, notice: Notice) -> Result<bbt_proto::gui::UiLayout, String> {
        let (done, answer) = oneshot::channel();
        {
            let mut pending = self.ui.lock().unwrap_or_else(|e| e.into_inner());
            pending.retain(|_, value| !value.is_closed());
            if pending.len() >= 64 || pending.contains_key(&id) {
                return Err("Busy: UI relay is full or this operation is pending".into());
            }
            pending.insert(id, done);
        }
        let result = if self.asks.send(notice).is_err() {
            Err("endpoint_unavailable: no GUI owner is attached".into())
        } else {
            match tokio::time::timeout(WAIT, answer).await {
                Ok(Ok(result)) => result,
                _ => Err("timeout: GUI owner did not answer the UI request".into()),
            }
        };
        self.ui
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&id);
        result
    }
    pub(crate) fn ui_answer(&self, id: Ulid, result: Result<bbt_proto::gui::UiLayout, String>) {
        if let Some(done) = self
            .ui
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&id)
        {
            let _ = done.send(result);
        }
    }
    /// Lifecycle relays use the capture owner's existing admitted GUI session.
    pub(crate) async fn app(
        &self,
        id: Ulid,
        request: bbt_proto::gui::AppRequest,
    ) -> Result<bbt_proto::gui::AppStatus, String> {
        let (done, answer) = oneshot::channel();
        {
            let mut pending = self.app.lock().unwrap_or_else(|e| e.into_inner());
            pending.retain(|_, sender| !sender.is_closed());
            if pending.len() >= 64 || pending.contains_key(&id) {
                return Err(
                    "Busy: app lifecycle relay is full or this operation is pending".into(),
                );
            }
            pending.insert(id, done);
        }
        let sent = self.asks.send(Notice::App { id, request });
        let result = if sent.is_err() {
            Err("app endpoint unavailable: no GUI client is attached; inspect debug ps".into())
        } else {
            match tokio::time::timeout(WAIT, answer).await {
                Ok(Ok(status)) => Ok(status),
                _ => Err(
                    "app endpoint unavailable: GUI client did not answer; inspect debug ps".into(),
                ),
            }
        };
        self.app
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&id);
        result
    }
    pub(crate) fn app_answer(&self, id: Ulid, status: bbt_proto::gui::AppStatus) {
        if let Some(sender) = self
            .app
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&id)
        {
            let _ = sender.send(status);
        }
    }
    pub(crate) fn app_stopping(&self, id: Ulid) {
        let _ = self.asks.send(Notice::AppStopping { id });
    }
    /// The window's session listens here.
    pub(crate) fn subscribe(&self) -> broadcast::Receiver<Notice> {
        self.asks.subscribe()
    }
    /// Asks the window for `ask` and waits for its answer.
    pub(crate) async fn ask(&self, ask: Ask) -> Result<bbt_proto::gui::CaptureFrame, String> {
        if !std::path::Path::new(&ask.path).is_absolute() {
            return Err("capture path must be absolute".into());
        }
        let id = ask.id;
        let (done, answer) = oneshot::channel();
        self.lock().insert(id, done);
        if self.asks.send(Notice::Capture(ask)).is_err() {
            self.lock().remove(&id);
            return Err("no window is attached to capture".into());
        }
        let answer = tokio::time::timeout(WAIT, answer).await;
        self.lock().remove(&id);
        match answer {
            Ok(Ok(answer)) => answer,
            Ok(Err(_)) => Err("the window dropped the capture".into()),
            Err(_) => Err("the window did not answer the capture in time".into()),
        }
    }
    /// Tells the window about a seat spawned through another door; no window, no notice.
    pub(crate) fn spawned(&self, op: Ulid, seat: bbt_proto::SeatId, spec: bbt_proto::SpawnSpec) {
        let _ = self.asks.send(Notice::Spawned { op, seat, spec });
    }
    /// Whether a window's session is listening for relayed asks.
    pub(crate) fn attached(&self) -> bool {
        self.asks.receiver_count() > 0
    }
    /// Resolve only after the owner confirms a completed frame, never on relay.
    pub(crate) async fn focus(
        &self,
        op: Ulid,
        seat: bbt_proto::SeatId,
    ) -> Result<bbt_proto::gui::FocusFrame, String> {
        let (done, answer) = oneshot::channel();
        {
            let mut pending = self.focus.lock().unwrap_or_else(|e| e.into_inner());
            pending.retain(|_, sender| !sender.is_closed());
            if pending.len() >= 64 || pending.contains_key(&op) {
                return Err("Busy: focus relay is full or this operation is pending".into());
            }
            pending.insert(op, done);
        }
        let result = if self.asks.send(Notice::Focus { op, seat }).is_err() {
            Err("no window is attached to focus".into())
        } else {
            match tokio::time::timeout(WAIT, answer).await {
                Ok(Ok(result)) => result,
                _ => Err("focus_timeout: the GUI owner did not acknowledge focus".into()),
            }
        };
        self.focus
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&op);
        result.and_then(|frame| {
            if frame.seat == seat && frame.frame_seq > 0 {
                Ok(frame)
            } else {
                Err("focus_superseded: GUI acknowledged another pane or an incomplete frame".into())
            }
        })
    }
    pub(crate) fn focused(&self, id: Ulid, result: Result<bbt_proto::gui::FocusFrame, String>) {
        if let Some(done) = self
            .focus
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&id)
        {
            let _ = done.send(result);
        }
    }
    /// The window derives Position and uses its ordinary close/undo controller.
    pub(crate) async fn close(
        &self,
        op: Ulid,
        seat: bbt_proto::SeatId,
        undo: bool,
    ) -> Result<bbt_proto::gui::UiLayout, String> {
        let layout = self
            .ui_wait(
                op,
                if undo {
                    Notice::UndoClose { op, seat }
                } else {
                    Notice::Close { op, seat }
                },
            )
            .await?;
        if layout.frame_seq == 0 {
            return Err("close_frame_incomplete: the GUI owner returned no produced frame".into());
        }
        Ok(layout)
    }
    /// The window's answer; an id nobody waits for is ignored.
    pub(crate) fn answer(&self, id: Ulid, answer: Result<bbt_proto::gui::CaptureFrame, String>) {
        if let Some(done) = self.lock().remove(&id) {
            let _ = done.send(answer);
        }
    }
    fn lock(
        &self,
    ) -> std::sync::MutexGuard<
        '_,
        HashMap<Ulid, oneshot::Sender<Result<bbt_proto::gui::CaptureFrame, String>>>,
    > {
        self.pending.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn focus_waits_for_the_gui_frame_answer_and_keeps_its_correlation() {
        let captures = Arc::new(Captures::default());
        let seat = bbt_proto::SeatId::from_u128(5);
        assert!(
            captures
                .focus(Ulid(1), seat)
                .await
                .unwrap_err()
                .contains("no window")
        );
        let mut owner = captures.subscribe();
        let task = tokio::spawn({
            let captures = captures.clone();
            async move { captures.focus(Ulid(2), seat).await }
        });
        assert!(
            matches!(owner.recv().await.unwrap(),Notice::Focus {op,seat:id} if op==Ulid(2) && id==seat)
        );
        assert!(
            !task.is_finished(),
            "a relay receipt is not an applied focus"
        );
        captures.focused(
            Ulid(999),
            Ok(bbt_proto::gui::FocusFrame {
                seat,
                generation: 7,
                frame_seq: 9,
            }),
        );
        assert!(
            !task.is_finished(),
            "another operation cannot acknowledge this focus"
        );
        captures.focused(
            Ulid(2),
            Ok(bbt_proto::gui::FocusFrame {
                seat,
                generation: 7,
                frame_seq: 9,
            }),
        );
        assert_eq!(task.await.unwrap().unwrap().frame_seq, 9);
    }

    #[tokio::test]
    async fn close_and_undo_share_the_ui_wait_and_do_not_acknowledge_the_relay() {
        use bbt_proto::gui::UiLayout;
        let captures = Arc::new(Captures::default());
        let seat = bbt_proto::SeatId::from_u128(5);
        assert!(
            captures
                .close(Ulid(1), seat, false)
                .await
                .unwrap_err()
                .starts_with("endpoint_unavailable:")
        );
        let mut owner = captures.subscribe();
        for (undo, frame_seq) in [(false, 11), (true, 12), (false, 0)] {
            let id = Ulid::new();
            let request = tokio::spawn({
                let captures = captures.clone();
                async move { captures.close(id, seat, undo).await }
            });
            match owner.recv().await.unwrap() {
                Notice::Close { op, seat: seen } if !undo => assert_eq!((op, seen), (id, seat)),
                Notice::UndoClose { op, seat: seen } if undo => assert_eq!((op, seen), (id, seat)),
                other => panic!("wrong close relay: {other:?}"),
            }
            assert!(!request.is_finished(), "a relay is not a produced frame");
            captures.ui_answer(Ulid::new(), Err("wrong operation".into()));
            assert!(
                !request.is_finished(),
                "another operation cannot acknowledge this close"
            );
            captures.ui_answer(
                id,
                Ok(UiLayout {
                    frame_seq,
                    size: [1600., 900.],
                    swarm: None,
                    tab: None,
                    focused: None,
                    launcher_open: false,
                    page: None,
                    document: None,
                    panes: Vec::new(),
                    tree: Vec::new(),
                }),
            );
            let answer = request.await.unwrap();
            if frame_seq == 0 {
                assert!(answer.unwrap_err().starts_with("close_frame_incomplete:"));
            } else {
                assert_eq!(answer.unwrap().frame_seq, frame_seq);
            }
            assert!(captures.ui.lock().unwrap().is_empty());
        }
    }

    #[tokio::test]
    async fn lifecycle_uses_the_gui_owner_and_only_commits_stop_after_the_reply() {
        let captures = Arc::new(Captures::default());
        let id = Ulid::new();
        assert!(
            captures
                .app(id, bbt_proto::gui::AppRequest::Status)
                .await
                .unwrap_err()
                .starts_with("app endpoint unavailable:")
        );
        let mut owner = captures.subscribe();
        let task = tokio::spawn({
            let captures = captures.clone();
            async move { captures.app(id, bbt_proto::gui::AppRequest::Stop).await }
        });
        assert!(
            matches!(owner.recv().await.unwrap(),Notice::App {id:seen,request:bbt_proto::gui::AppRequest::Stop} if seen==id)
        );
        let status = bbt_proto::gui::AppStatus {
            running: true,
            mode: "headless".into(),
            unattended: false,
            backend_ready: true,
            gui_ready: true,
            events_ready: true,
            custody_backend: "memory".into(),
            generation: 7,
            frame_seq: 1,
            windows: 0,
            close: serde_json::Value::Null,
        };
        captures.app_answer(id, status.clone());
        assert_eq!(task.await.unwrap().unwrap(), status);
        assert!(matches!(
            owner.try_recv(),
            Err(broadcast::error::TryRecvError::Empty)
        ));
        captures.app_stopping(id);
        assert!(matches!(owner.recv().await.unwrap(),Notice::AppStopping {id:seen} if seen==id));
    }

    #[tokio::test]
    async fn ui_asks_are_correlated_and_do_not_invent_an_owner() {
        use bbt_proto::gui::UiRequest;
        let captures = Arc::new(Captures::default());
        let id = Ulid::new();
        assert!(
            captures
                .ui(id, UiRequest::Layout)
                .await
                .unwrap_err()
                .starts_with("endpoint_unavailable:")
        );
        let mut owner = captures.subscribe();
        let request = tokio::spawn({
            let captures = captures.clone();
            async move { captures.ui(id, UiRequest::Key(vec!["cmd-t".into()])).await }
        });
        assert!(
            matches!(owner.recv().await.unwrap(), Notice::Ui { id: seen, request: UiRequest::Key(keys) } if seen == id && keys == ["cmd-t"])
        );
        captures.ui_answer(Ulid::new(), Err("wrong operation".into()));
        assert!(!request.is_finished());
        captures.ui_answer(id, Err("ui_key_not_applicable: test refusal".into()));
        assert_eq!(
            request.await.unwrap(),
            Err("ui_key_not_applicable: test refusal".into())
        );
        assert!(captures.ui.lock().unwrap().is_empty());
    }
    #[tokio::test]
    async fn an_ask_completes_with_the_windows_answer() {
        let captures = Arc::new(Captures::default());
        let path = "/tmp/never-written.png".to_string();
        let ask = Ask {
            id: Ulid::new(),
            path: path.clone(),
            seat: None,
        };
        assert_eq!(
            captures.ask(ask.clone()).await,
            Err("no window is attached to capture".into())
        );
        let relative = Ask {
            path: "x.png".into(),
            ..ask.clone()
        };
        assert_eq!(
            captures.ask(relative).await,
            Err("capture path must be absolute".into())
        );
        let mut window = captures.subscribe();
        let asker = tokio::spawn({
            let captures = captures.clone();
            let ask = ask.clone();
            async move { captures.ask(ask).await }
        });
        let Notice::Capture(seen) = window.recv().await.unwrap() else {
            panic!("a capture notice")
        };
        assert_eq!((seen.id, seen.path), (ask.id, path));
        captures.answer(Ulid::new(), Err("not this one".into()));
        captures.answer(ask.id, Err("readback failed".into()));
        assert_eq!(asker.await.unwrap(), Err("readback failed".into()));
    }
}
