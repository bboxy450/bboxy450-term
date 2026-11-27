// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use std::{
    collections::{BTreeMap, VecDeque},
    io::{self, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};

use base64::{Engine, engine::general_purpose::STANDARD};
use bboxy450_cef::cdp::{Client, Incoming};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
    runtime::Handle,
    sync::{OwnedSemaphorePermit, Semaphore, mpsc, oneshot},
    time::{Instant, timeout, timeout_at},
};
use ulid::Ulid;

use crate::{
    logic::{Command as LogicCommand, LogicClient, RootBinding},
    mcp::{self, Call, SeatActor},
    store::Result,
};

type Reply = oneshot::Sender<Result<Value>>;

const CALL_MS: u64 = 8_000;
const STOP_MS: u64 = 800;
const FRAME_BYTES: usize = 24 * 1024 * 1024;
const EVENT_BYTES: usize = 96 * 1024;
const EVENTS: usize = 128;
const READY: &str = "BBoxy.browserReady";

#[derive(Default)]
pub(in crate::logic) struct Sessions(BTreeMap<Ulid, Slot>);

struct Slot {
    generation: u64,
    commands: mpsc::Sender<Work>,
    stop: Option<oneshot::Sender<Option<Close>>>,
    busy: Arc<Semaphore>,
    task: tokio::task::JoinHandle<()>,
}

struct Close {
    op: Ulid,
    reply: Reply,
}

pub(in crate::logic) struct Capture {
    pub actor: SeatActor,
    pub binding: RootBinding,
    pub call: Call,
    pub png: Vec<u8>,
    pub reply: Reply,
}

pub(super) struct Submission {
    pub actor: SeatActor,
    pub call: Call,
    pub action: Action,
    pub binding: Option<RootBinding>,
    pub reply: Reply,
}

struct Work {
    request: Submission,
    _turn: OwnedSemaphorePermit,
}

pub(super) enum Action {
    Open(Open),
    Cdp(Cdp),
    Screenshot(Screenshot),
    Close,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Open {
    url: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Cdp {
    method: String,
    params: Option<serde_json::Map<String, Value>>,
    timeout_ms: Option<u64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Screenshot {
    #[serde(default)]
    full_page: bool,
}

impl Action {
    pub(super) fn parse(call: &Call) -> std::result::Result<Self, String> {
        use mcp::tools::parse;
        match call.tool.as_str() {
            "browser_open" => {
                let open: Open = parse(&call.args)?;
                if open.url != "about:blank"
                    && !["https://", "http://", "data:"]
                        .iter()
                        .any(|prefix| open.url.starts_with(prefix))
                {
                    return Err("url must be HTTP(S), data:, or about:blank".into());
                }
                Ok(Self::Open(open))
            }
            "browser_cdp" => {
                let cdp: Cdp = parse(&call.args)?;
                if cdp.method.is_empty()
                    || cdp.method.len() > 128
                    || !cdp
                        .method
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'.')
                    || !matches!(cdp.timeout_ms, None | Some(1..=CALL_MS))
                {
                    return Err("invalid CDP method or timeout_ms (1..8000)".into());
                }
                Ok(Self::Cdp(cdp))
            }
            "browser_screenshot" => parse(&call.args).map(Self::Screenshot),
            "browser_close" => parse::<mcp::tools::Empty>(&call.args).map(|_| Self::Close),
            _ => unreachable!(),
        }
    }
}

impl Sessions {
    pub(super) fn call(
        &mut self,
        handle: &Handle,
        logic: &LogicClient,
        profile_root: &Path,
        request: Submission,
    ) {
        let seat = request.actor.seat_id;
        self.forget_retired(seat);
        let generation = request.actor.process_generation;
        let op = request.call.op_id;

        if let Some(slot) = self.0.get_mut(&seat) {
            if let Action::Close = request.action
                && slot.generation == generation
                && let Some(stop) = slot.stop.take()
            {
                if let Err(Some(close)) = stop.send(Some(Close {
                    op,
                    reply: request.reply,
                })) {
                    let _ = close
                        .reply
                        .send(Ok(mcp::success(op, json!({"closed": false}))));
                }
                return;
            }
            if slot.generation != generation || slot.stop.is_none() {
                let _ = request.reply.send(Ok(mcp::error(
                    op,
                    "browser_closing",
                    "previous browser is closing",
                )));
                return;
            }
        } else if matches!(request.action, Action::Open(_)) {
            let (commands, receive) = mpsc::channel(1);
            let (stop, stopped) = oneshot::channel();
            let task = handle.spawn(run(
                logic.clone(),
                profile_root.to_owned(),
                seat,
                receive,
                stopped,
            ));
            self.0.insert(
                seat,
                Slot {
                    generation,
                    commands,
                    stop: Some(stop),
                    busy: Arc::new(Semaphore::new(1)),
                    task,
                },
            );
        } else {
            let result = if matches!(request.action, Action::Close) {
                mcp::success(op, json!({"closed": false}))
            } else {
                mcp::error(op, "browser_absent", "call browser_open first")
            };
            let _ = request.reply.send(Ok(result));
            return;
        }
        let slot = &self.0[&seat];
        let Ok(turn) = slot.busy.clone().try_acquire_owned() else {
            let _ = request.reply.send(Ok(mcp::error(
                op,
                "browser_busy",
                "this seat already has a browser request in flight",
            )));
            return;
        };
        if let Err(error) = slot.commands.try_send(Work {
            request,
            _turn: turn,
        }) {
            let _ = error.into_inner().request.reply.send(Ok(mcp::error(
                op,
                "browser_disconnected",
                "browser stopped; call browser_open again",
            )));
        }
    }

    pub(in crate::logic) fn forget_retired(&mut self, seat: Ulid) {
        if self
            .0
            .get(&seat)
            .is_some_and(|slot| slot.commands.is_closed())
        {
            self.0.remove(&seat);
        }
    }

    pub(in crate::logic) fn retire(&mut self, seat: Ulid, generation: u64) {
        self.forget_retired(seat);
        if let Some(slot) = self.0.get_mut(&seat)
            && slot.generation == generation
        {
            // Dropping the independent stop sender interrupts pipe IO immediately.
            slot.stop.take();
        }
    }

    pub(in crate::logic) fn begin_shutdown(&mut self) {
        for slot in self.0.values_mut() {
            slot.stop.take();
        }
    }

    pub(in crate::logic) fn stop_all(&mut self, handle: &Handle) {
        let slots = std::mem::take(&mut self.0);
        let jobs: Vec<_> = slots.into_values().map(|slot| slot.task).collect();
        handle.block_on(async {
            for job in jobs {
                let _ = job.await;
            }
        });
    }
}

async fn run(
    logic: LogicClient,
    profile_root: PathBuf,
    seat: Ulid,
    mut commands: mpsc::Receiver<Work>,
    mut stop: oneshot::Receiver<Option<Close>>,
) {
    let mut process = None;
    let mut work = None::<Work>;
    let result = tokio::select! {
        biased;
        close = &mut stop => Ok(close.ok().flatten()),
        result = serve(&logic, &profile_root, seat, &mut commands, &mut process, &mut work) => result.map(|()| None),
    };
    let cleanup = if let Some(process) = process {
        process.stop().await
    } else {
        Ok(())
    };
    // Close admission after reaping the old host and removing its profile. The
    // next open after a reported crash can immediately start a fresh child.
    commands.close();
    let _ = logic.tx.try_send(LogicCommand::BrowserRetired(seat));
    let (close, reason) = match result {
        Ok(close) => (
            close,
            Problem::new("browser_closed", "browser closed during the request"),
        ),
        Err(reason) => (None, reason),
    };
    if let Some(work) = work.or_else(|| commands.try_recv().ok()) {
        let _ = work
            .request
            .reply
            .send(Ok(reason.value(work.request.call.op_id)));
    }
    if let Some(close) = close {
        let result = match cleanup {
            Ok(()) => mcp::success(close.op, json!({"closed": true})),
            Err(error) => Problem::from(error).value(close.op),
        };
        let _ = close.reply.send(Ok(result));
    }
}

async fn serve(
    logic: &LogicClient,
    profile_root: &Path,
    seat: Ulid,
    commands: &mut mpsc::Receiver<Work>,
    process: &mut Option<Process>,
    work: &mut Option<Work>,
) -> Flow<()> {
    loop {
        *work = match process {
            Some(process) => loop {
                tokio::select! {
                    queued = commands.recv() => break queued,
                    event = process.message() => { event?; }
                }
            },
            None => commands.recv().await,
        };
        let Some(current) = work.as_ref() else {
            return Ok(());
        };
        let limit = match &current.request.action {
            Action::Cdp(cdp) => cdp.timeout_ms.unwrap_or(CALL_MS),
            _ => CALL_MS,
        };
        let deadline = Instant::now() + Duration::from_millis(limit);
        if process.is_none() {
            let child = Process::spawn(profile_root, seat)?;
            let _ = process.insert(child);
            process
                .as_mut()
                .expect("just started")
                .ready(deadline)
                .await?;
        }
        let process = process.as_mut().expect("started");
        let outcome = match &current.request.action {
            Action::Open(open) => process.open(&open.url, deadline).await.map(Output::Value),
            Action::Cdp(cdp) => process.cdp(cdp, deadline).await.map(Output::Value),
            Action::Screenshot(capture) => process
                .screenshot(capture.full_page, deadline)
                .await
                .map(Output::Png),
            Action::Close => unreachable!(),
        };
        if matches!(&outcome, Err(error) if error.fatal) {
            return outcome.map(|_| ());
        }
        let Work {
            request:
                Submission {
                    actor,
                    call,
                    binding,
                    reply,
                    ..
                },
            _turn,
        } = work.take().expect("active work");
        // Release the flight slot before returning the result.
        drop(_turn);
        match outcome {
            Ok(Output::Png(png)) => {
                let capture = Capture {
                    actor,
                    binding: binding.expect("admitted workspace"),
                    call,
                    png,
                    reply,
                };
                logic
                    .tx
                    .send(LogicCommand::BrowserCapture(Box::new(capture)))
                    .await
                    .map_err(|_| Problem::crash("logic stopped"))?;
            }
            Ok(Output::Value(value)) => {
                let _ = reply.send(Ok(bounded(call.op_id, value)));
            }
            Err(error) => {
                let _ = reply.send(Ok(error.value(call.op_id)));
            }
        }
    }
}

enum Output {
    Value(Value),
    Png(Vec<u8>),
}

type Flow<T> = std::result::Result<T, Problem>;

struct Problem {
    code: &'static str,
    message: String,
    fatal: bool,
}

impl Problem {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            fatal: false,
        }
    }

    fn crash(message: impl Into<String>) -> Self {
        Self {
            fatal: true,
            ..Self::new("browser_disconnected", message)
        }
    }

    fn timed_out() -> Self {
        Self::new("browser_timeout", "browser request timed out")
    }

    fn value(self, op: Ulid) -> Value {
        mcp::error(op, self.code, mcp::tools::clip(&self.message, 1024))
    }
}

impl From<io::Error> for Problem {
    fn from(error: io::Error) -> Self {
        Self::crash(error.to_string())
    }
}

struct Process {
    child: Child,
    pid: u32,
    profile: tempfile::TempDir,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
    line: Vec<u8>,
    cdp: Client,
    events: VecDeque<(usize, Value)>,
    event_bytes: usize,
    dropped: u64,
}

impl Process {
    fn spawn(profile_root: &Path, seat: Ulid) -> Flow<Self> {
        let executable = std::env::current_exe()?;
        if !executable.parent().is_some_and(|directory| {
            directory.join("../Frameworks/Chromium Embedded Framework.framework/Chromium Embedded Framework").is_file()
        }) {
            return Err(Problem::new("browser_unavailable", "install a macOS app bundle built with BROWSER=1"));
        }
        let profile = tempfile::Builder::new()
            .prefix(&format!("browser-{seat}-"))
            .tempdir_in(profile_root.canonicalize()?)?;
        let log = std::fs::File::create(profile.path().join("host.log"))?;
        let mut command = Command::new(executable);
        command
            .arg("browser-host")
            .arg(profile.path())
            .env_clear()
            .env("HOME", profile.path())
            .env("PATH", "/usr/bin:/bin")
            .current_dir(profile.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(log)
            .process_group(0)
            .kill_on_drop(true);
        let mut child = bbt_rt::qos::with_default_qos(|| command.spawn())?;
        Ok(Self {
            pid: child.id().expect("new browser child"),
            input: child.stdin.take(),
            output: BufReader::new(child.stdout.take().expect("piped stdout")),
            child,
            profile,
            line: Vec::new(),
            cdp: Client::new(),
            events: VecDeque::new(),
            event_bytes: 0,
            dropped: 0,
        })
    }

    async fn stop(mut self) -> io::Result<()> {
        self.input.take();
        // Chromium helpers exit on browser shutdown. A hung host and any
        // remaining helpers are in the isolated process group we created.
        let stopped = timeout(Duration::from_millis(STOP_MS), self.child.wait()).await;
        if !matches!(stopped, Ok(Ok(status)) if status.success()) {
            // SAFETY: process_group(0) assigned this host and its helpers an owned group.
            unsafe {
                libc::kill(-(self.pid as i32), libc::SIGKILL);
            }
            let _ = self.child.wait().await?;
        }
        let profile = self.profile;
        tokio::task::spawn_blocking(move || profile.close())
            .await
            .map_err(io::Error::other)?
    }

    async fn ready(&mut self, deadline: Instant) -> Flow<()> {
        loop {
            let message = timeout_at(deadline, self.message())
                .await
                .map_err(|_| Problem::crash("Chromium start timed out"))??;
            if matches!(message, Incoming::Event { method, .. } if method == READY) {
                return Ok(());
            }
        }
    }

    // fill_buf/consume preserves the unfinished line across select cancellations.
    async fn message(&mut self) -> Flow<Incoming> {
        loop {
            let ready = tokio::select! {
                status = self.child.wait() => {
                    return Err(disconnected(self.profile.path(), Some(status?)));
                }
                bytes = self.output.fill_buf() => bytes?,
            };
            if ready.is_empty() {
                return Err(disconnected(self.profile.path(), None));
            }
            let end = ready.iter().position(|byte| *byte == b'\n');
            let used = end.map_or(ready.len(), |end| end + 1);
            if self.line.len() + used > FRAME_BYTES {
                return Err(Problem::crash(
                    "Chromium reply exceeded the CDP frame limit",
                ));
            }
            self.line.extend_from_slice(&ready[..used]);
            self.output.consume(used);
            if end.is_some() {
                break;
            }
        }
        let bytes = self.line.len();
        let incoming = self.cdp.receive(&self.line);
        self.line.clear();
        let incoming =
            incoming.map_err(|_| Problem::crash("Chromium sent an invalid CDP frame"))?;
        if let Incoming::Event {
            method,
            params,
            session,
        } = &incoming
            && method != READY
        {
            if bytes > EVENT_BYTES {
                self.dropped += 1;
            } else {
                while self.events.len() == EVENTS || self.event_bytes + bytes > EVENT_BYTES {
                    let (bytes, _) = self.events.pop_front().expect("buffer exceeded");
                    self.event_bytes -= bytes;
                    self.dropped += 1;
                }
                self.event_bytes += bytes;
                self.events.push_back((
                    bytes,
                    json!({"method":method,"params":params,"sessionId":session}),
                ));
            }
        }
        Ok(incoming)
    }

    async fn command(
        &mut self,
        method: &str,
        params: Value,
        deadline: Instant,
    ) -> Flow<std::result::Result<Value, bboxy450_cef::cdp::Error>> {
        let mut request = self.cdp.request(method, params, None);
        request.bytes.push(b'\n');
        let input = self.input.as_mut().expect("stdin until shutdown");
        timeout_at(deadline, input.write_all(&request.bytes))
            .await
            .map_err(|_| Problem::crash("Chromium did not read the CDP command"))??;
        loop {
            match timeout_at(deadline, self.message()).await {
                Err(_) => {
                    self.cdp.cancel(request.id);
                    return Err(Problem::timed_out());
                }
                Ok(Ok(Incoming::Reply { id, outcome, .. })) if id == request.id => {
                    return Ok(outcome);
                }
                Ok(Ok(_)) => {}
                Ok(Err(error)) => return Err(error),
            }
        }
    }

    async fn required(&mut self, method: &str, params: Value, deadline: Instant) -> Flow<Value> {
        self.command(method, params, deadline)
            .await?
            .map_err(|error| Problem::new("cdp_error", format!("{method}: {}", error.message)))
    }

    async fn open(&mut self, url: &str, deadline: Instant) -> Flow<Value> {
        self.required("Page.enable", json!({}), deadline).await?;
        self.required(
            "Page.setLifecycleEventsEnabled",
            json!({"enabled":true}),
            deadline,
        )
        .await?;
        let navigation = self
            .required("Page.navigate", json!({"url":url}), deadline)
            .await?;
        if let Some(error) = navigation["errorText"].as_str() {
            return Err(Problem::new("browser_navigation_failed", error));
        }
        if navigation["isDownload"] == true {
            return Err(Problem::new(
                "browser_navigation_failed",
                "navigation returned a download",
            ));
        }
        if let Some(loader) = navigation["loaderId"].as_str() {
            let loaded = |method: &str, params: &Value| {
                method == "Page.lifecycleEvent"
                    && params["name"] == "load"
                    && params["loaderId"] == loader
                    && params["frameId"] == navigation["frameId"]
            };
            if !self.events.iter().any(|(_, event)| {
                loaded(
                    event["method"].as_str().unwrap_or_default(),
                    &event["params"],
                )
            }) {
                loop {
                    let message = timeout_at(deadline, self.message())
                        .await
                        .map_err(|_| Problem::timed_out())??;
                    if matches!(message, Incoming::Event { method, params, .. } if loaded(&method, &params))
                    {
                        break;
                    }
                }
            }
        }
        let result = self.required("Runtime.evaluate", json!({
            "expression":"({url:location.href,title:document.title,http_status:['http:','https:'].includes(location.protocol) ? (performance.getEntriesByType('navigation')[0]?.responseStatus || null) : null})",
            "returnByValue":true
        }), deadline).await?;
        let value = &result["result"]["value"];
        if !value.is_object() {
            return Err(Problem::new(
                "browser_navigation_failed",
                "page metadata unavailable",
            ));
        }
        Ok(value.clone())
    }

    async fn cdp(&mut self, call: &Cdp, deadline: Instant) -> Flow<Value> {
        let result = self
            .command(
                &call.method,
                json!(call.params.clone().unwrap_or_default()),
                deadline,
            )
            .await;
        if result.as_ref().is_err_and(|error| error.fatal) {
            return result.map(|_| Value::Null);
        }
        let events: Vec<_> = self.events.drain(..).map(|(_, event)| event).collect();
        let dropped = std::mem::take(&mut self.dropped);
        self.event_bytes = 0;
        let mut response = json!({"events":events,"events_dropped":dropped});
        match result {
            Ok(Ok(result)) => response["result"] = result,
            Ok(Err(error)) => {
                response["error"] =
                    json!({"code":"cdp_error","cdp_code":error.code,"message":error.message})
            }
            Err(error) => response["error"] = json!({"code":error.code,"message":error.message}),
        }
        Ok(response)
    }

    async fn screenshot(&mut self, full_page: bool, deadline: Instant) -> Flow<Vec<u8>> {
        let mut params = json!({"format":"png","fromSurface":true});
        if full_page {
            let metrics = self
                .required("Page.getLayoutMetrics", json!({}), deadline)
                .await?;
            let size = &metrics["cssContentSize"];
            let width = size["width"].as_f64().unwrap_or(0.0).ceil();
            let height = size["height"].as_f64().unwrap_or(0.0).ceil();
            if !(1.0..=8192.0).contains(&width)
                || !(1.0..=8192.0).contains(&height)
                || width * height > mcp::artifacts::staging::MAX_IMAGE_PIXELS as f64
            {
                return Err(Problem::new(
                    "browser_image_limit",
                    "full page exceeds the screenshot dimension or pixel limit",
                ));
            }
            params["captureBeyondViewport"] = json!(true);
            params["clip"] = json!({"x":0,"y":0,"width":width,"height":height,"scale":1});
        }
        let result = self
            .required("Page.captureScreenshot", params, deadline)
            .await?;
        let png = result["data"]
            .as_str()
            .and_then(|data| STANDARD.decode(data).ok())
            .filter(|png| png.len() <= crate::provenance::stored::MAX_FILE)
            .ok_or_else(|| {
                Problem::new(
                    "browser_image_limit",
                    "Chromium did not return an in-budget PNG",
                )
            })?;
        Ok(png)
    }
}

fn disconnected(profile: &Path, status: Option<std::process::ExitStatus>) -> Problem {
    // Read at most the final 700 bytes, even if Chromium kept a long debug log.
    let detail = (|| -> io::Result<_> {
        let mut file = std::fs::File::open(profile.join("host.log"))?;
        let size = file.metadata()?.len();
        file.seek(SeekFrom::Start(size.saturating_sub(700)))?;
        let mut tail = Vec::new();
        file.take(700).read_to_end(&mut tail)?;
        Ok(String::from_utf8_lossy(&tail).trim().to_owned())
    })()
    .unwrap_or_default();
    let status = status.map_or_else(String::new, |status| format!(" ({status})"));
    Problem::crash(format!(
        "Chromium exited{status}; call browser_open again. {detail}"
    ))
}

fn bounded(op: Ulid, mut data: Value) -> Value {
    let failed = data.get("error").is_some();
    if let Some(message) = data["error"]["message"].as_str() {
        data["error"]["message"] = mcp::tools::clip(message, 1024).into();
    }
    if failed {
        data["status"] = json!("rejected");
    }
    let wrap = |data: &Value| {
        let mut result = mcp::success(op, data.clone());
        result["isError"] = json!(failed);
        result
    };
    let result = wrap(&data);
    if serde_json::to_vec(&result).is_ok_and(|bytes| bytes.len() <= mcp::read_pages::BUDGET) {
        return result;
    }
    let note = "browser result exceeded the MCP result budget";
    if let Some(events) = data.get_mut("events").and_then(Value::as_array_mut) {
        let pruned = events.len() as u64;
        events.clear();
        data["events_dropped"] = json!(data["events_dropped"].as_u64().unwrap_or(0) + pruned);
        data["cut"] = json!(note);
        let result = wrap(&data);
        if serde_json::to_vec(&result).is_ok_and(|bytes| bytes.len() <= mcp::read_pages::BUDGET) {
            return result;
        }
        data["result"] = Value::Null;
        wrap(&data)
    } else {
        mcp::success(
            op,
            json!({"url":null,"title":null,"http_status":data["http_status"],"cut":note}),
        )
    }
}
