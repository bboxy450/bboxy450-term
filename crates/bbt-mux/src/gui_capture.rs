//! Window capture on the operator's behalf: a `GuiAction::Capture` from any operator door
//! (the window or `ctl`) is relayed to the window's own session as `GuiReply::CaptureAsk`;
//! the window writes the PNG off its UI thread and answers with `GuiAction::Captured`,
//! which completes the asker's wait. Logic renders nothing; it only relays.
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

pub(crate) struct Captures {
    asks: broadcast::Sender<Ask>,
    pending: Mutex<HashMap<Ulid, oneshot::Sender<Option<String>>>>,
}
impl Default for Captures {
    fn default() -> Self {
        Self {
            asks: broadcast::channel(8).0,
            pending: Mutex::default(),
        }
    }
}
impl Captures {
    /// The window's session listens here.
    pub(crate) fn subscribe(&self) -> broadcast::Receiver<Ask> {
        self.asks.subscribe()
    }
    /// Asks the window for `ask` and waits for its answer.
    pub(crate) async fn ask(&self, ask: Ask) -> Result<(), String> {
        if !std::path::Path::new(&ask.path).is_absolute() {
            return Err("capture path must be absolute".into());
        }
        let id = ask.id;
        let (done, answer) = oneshot::channel();
        self.lock().insert(id, done);
        if self.asks.send(ask).is_err() {
            self.lock().remove(&id);
            return Err("no window is attached to capture".into());
        }
        let answer = tokio::time::timeout(WAIT, answer).await;
        self.lock().remove(&id);
        match answer {
            Ok(Ok(None)) => Ok(()),
            Ok(Ok(Some(error))) => Err(error),
            Ok(Err(_)) => Err("the window dropped the capture".into()),
            Err(_) => Err("the window did not answer the capture in time".into()),
        }
    }
    /// The window's answer; an id nobody waits for is ignored.
    pub(crate) fn answer(&self, id: Ulid, error: Option<String>) {
        if let Some(done) = self.lock().remove(&id) {
            let _ = done.send(error);
        }
    }
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<Ulid, oneshot::Sender<Option<String>>>> {
        self.pending.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn an_ask_completes_with_the_windows_answer() {
        let captures = Arc::new(Captures::default());
        let path = "/tmp/never-written.png".to_string();
        let ask = Ask { id: Ulid::new(), path: path.clone(), seat: None };
        assert_eq!(
            captures.ask(ask.clone()).await,
            Err("no window is attached to capture".into())
        );
        let relative = Ask { path: "x.png".into(), ..ask.clone() };
        assert_eq!(captures.ask(relative).await, Err("capture path must be absolute".into()));
        let mut window = captures.subscribe();
        let asker = tokio::spawn({
            let captures = captures.clone();
            let ask = ask.clone();
            async move { captures.ask(ask).await }
        });
        let seen = window.recv().await.unwrap();
        assert_eq!((seen.id, seen.path), (ask.id, path));
        captures.answer(Ulid::new(), Some("not this one".into()));
        captures.answer(ask.id, Some("readback failed".into()));
        assert_eq!(asker.await.unwrap(), Err("readback failed".into()));
    }
}
