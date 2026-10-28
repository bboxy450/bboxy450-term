// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! UI log calls only enqueue memory. stderr writes run on Tokio's IO side.
use crate::UiEvent;
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc, oneshot};
use winit::event_loop::EventLoopProxy;

struct Line {
    text: String,
    done: Option<oneshot::Sender<()>>,
}
#[derive(Clone)]
pub struct Logger(mpsc::Sender<Line>);
impl Logger {
    pub fn new() -> Self {
        let (sender, mut receiver) = mpsc::channel::<Line>(64);
        bbt_rt::handle().spawn(async move {
            let mut stderr = tokio::io::stderr();
            while let Some(line) = receiver.recv().await {
                let _ = stderr
                    .write_all(format!("{}\n", line.text).as_bytes())
                    .await;
                if let Some(done) = line.done {
                    let _ = stderr.flush().await;
                    let _ = done.send(());
                }
            }
        });
        Self(sender)
    }
    pub fn write(&self, text: impl Into<String>) {
        let _ = self.0.try_send(Line {
            text: text.into(),
            done: None,
        });
    }
    pub fn finish(&self, text: String, proxy: EventLoopProxy<UiEvent>) {
        self.finish_with_sink(
            text,
            std::sync::Arc::new(move |event| {
                let _ = proxy.send_event(event);
            }),
        );
    }
    pub fn finish_with_sink(
        &self,
        text: String,
        events: std::sync::Arc<dyn Fn(UiEvent) + Send + Sync>,
    ) {
        let sender = self.0.clone();
        bbt_rt::handle().spawn(async move {
            let (done, received) = oneshot::channel();
            let _ = sender
                .send(Line {
                    text,
                    done: Some(done),
                })
                .await;
            let _ = received.await;
            events(UiEvent::Quit);
        });
    }
}
