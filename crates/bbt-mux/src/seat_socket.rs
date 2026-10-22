//! Owner-only, capability-bound seat observation and semantic-control socket.
//! Raw input is unavailable here; messaging uses the common adapter/guard broker.

use std::collections::BTreeSet;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader};
use ulid::Ulid;

use crate::bus::Consumer;
use crate::logic::LogicClient;
use crate::store::{Result, invalid};
use crate::streams::{View, ViewStream};

const MAX_LINE: usize = 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub version: u16,
    pub capability: [u8; 32],
    pub seat: Ulid,
    pub generation: u64,
    pub action: Action,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Action {
    Catalogue,
    Call {
        #[serde(flatten)]
        call: crate::mcp::Call,
    },
    Messaging {
        request: crate::messaging::api::Request,
    },
    Hook {
        event: crate::adapters::hook::ShimEvent,
    },
    Attach,
    Snapshot,
    Subscribe,
    Applied {
        view: View,
        sequence: u64,
    },
    GetHistory {
        from: u64,
        to: u64,
    },
    BusSubscribe {
        consumer: Consumer,
    },
    PollEvents {
        consumer: String,
        limit: usize,
    },
    Ack {
        consumer: String,
        stream: String,
        sequence: u64,
    },
}

struct SocketFile {
    path: PathBuf,
    dev: u64,
    ino: u64,
}
impl Drop for SocketFile {
    fn drop(&mut self) {
        if let Ok(meta) = std::fs::symlink_metadata(&self.path)
            && meta.dev() == self.dev
            && meta.ino() == self.ino
        {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}
pub struct SeatEndpoint {
    task: tokio::task::JoinHandle<()>,
    _file: SocketFile,
}
impl Drop for SeatEndpoint {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl SeatEndpoint {
    /// Bind during IO bootstrap. Each endpoint's capability grants observations
    /// of one generation only, not a grant to another seat or an input lease.
    pub fn bind(
        handle: &tokio::runtime::Handle,
        path: &Path,
        client: LogicClient,
        seat: Ulid,
        generation: u64,
        capability: [u8; 32],
    ) -> Result<Self> {
        bbt_rt::assert_not_ui_thread();
        let parent = path
            .parent()
            .ok_or_else(|| invalid("socket requires a parent"))?;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent)?;
        let meta = std::fs::symlink_metadata(parent)?;
        // SAFETY: geteuid takes no pointers and changes no state.
        if !meta.is_dir() || meta.uid() != unsafe { libc::geteuid() } || meta.mode() & 0o077 != 0 {
            return Err(invalid("seat socket parent must be private and owned"));
        }
        let listener = std::os::unix::net::UnixListener::bind(path)?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        let meta = std::fs::symlink_metadata(path)?;
        let file = SocketFile {
            path: path.into(),
            dev: meta.dev(),
            ino: meta.ino(),
        };
        listener.set_nonblocking(true)?;
        let _entered = handle.enter();
        let listener = tokio::net::UnixListener::from_std(listener)?;
        let task=handle.spawn(async move {
            let slots=Arc::new(tokio::sync::Semaphore::new(32));
            let mut peers=tokio::task::JoinSet::new();
            loop {
                tokio::select! {
                    accepted=listener.accept()=> {
                        let Ok((stream,_))=accepted else {break;};
                        let Ok(permit)=slots.clone().try_acquire_owned() else {continue;};
                        let client=client.clone();
                        peers.spawn(async move {let _permit=permit;let _=session(stream,client,seat,generation,capability).await;});
                    }
                    _=peers.join_next(),if !peers.is_empty()=>{}
                }
            }
        });
        Ok(Self { task, _file: file })
    }
}

pub async fn read_line(reader: &mut (impl AsyncBufRead + Unpin)) -> Result<Option<Vec<u8>>> {
    let mut line = Vec::new();
    loop {
        let part = reader.fill_buf().await?;
        if part.is_empty() {
            return if line.is_empty() {
                Ok(None)
            } else {
                Err(invalid("truncated NDJSON frame"))
            };
        }
        let count = part
            .iter()
            .position(|b| *b == b'\n')
            .map_or(part.len(), |n| n + 1);
        if line.len() + count > MAX_LINE {
            return Err(invalid("NDJSON frame exceeds 1 MiB"));
        }
        let done = part[count - 1] == b'\n';
        line.extend_from_slice(&part[..count]);
        reader.consume(count);
        if done {
            line.pop();
            return Ok(Some(line));
        }
    }
}

async fn send(writer: &mut tokio::net::unix::OwnedWriteHalf, value: &impl Serialize) -> Result<()> {
    let mut bytes = serde_json::to_vec(value)?;
    if bytes.len() >= MAX_LINE {
        return Err(invalid("response exceeds negotiated frame bound"));
    }
    bytes.push(b'\n');
    tokio::time::timeout(Duration::from_secs(2), writer.write_all(&bytes)).await??;
    Ok(())
}

async fn session_inner(
    stream: tokio::net::UnixStream,
    client: LogicClient,
    seat: Ulid,
    generation: u64,
    capability: [u8; 32],
    consumers: &mut BTreeSet<String>,
) -> Result<()> {
    let (read, mut write) = stream.into_split();
    let (lines, mut incoming) = tokio::sync::mpsc::channel(2);
    let reader_task = tokio::spawn(async move {
        let mut reader = BufReader::new(read);
        loop {
            let line = read_line(&mut reader).await;
            let done = !matches!(line, Ok(Some(_)));
            if lines.send(line).await.is_err() || done {
                break;
            }
        }
    });
    struct ReaderTask(tokio::task::JoinHandle<()>);
    impl Drop for ReaderTask {
        fn drop(&mut self) {
            self.0.abort();
        }
    }
    let _reader_task = ReaderTask(reader_task);
    let mut view = ViewStream::default();
    let mut subscribed = false;
    let mut last = None;
    let mut tick = tokio::time::interval(Duration::from_millis(20));
    loop {
        tokio::select! {
            line=incoming.recv()=> {
                let Some(Ok(Some(line)))=line else {break;};
                let request:Request=serde_json::from_slice(&line)?;
                if request.version!=1||request.capability!=capability||request.seat!=seat||request.generation!=generation {return Err(invalid("seat socket admission/generation rejected"));}
                // Revalidate against the logic's supervisor-confirmed generation
                // on every request, not merely against this listener's pathname.
                let snapshot=client.snapshot(seat,generation).await?;
                let result:Result<serde_json::Value>=match request.action {
                    Action::Catalogue=>client.control_catalogue(seat,generation).await,
                    Action::Call{call}=>client.control_call(seat,generation,call).await,
                    Action::Messaging{request}=>client.messaging(seat,generation,request).await.and_then(|v|Ok(serde_json::to_value(v)?)),
                    Action::Hook{event}=>Ok(serde_json::json!({"kind":"HookObserved","status":client.hook(seat,generation,event).await?})),
                    Action::Attach|Action::Snapshot=>Ok(serde_json::json!({"kind":"Snapshot","snapshot":snapshot})),
                    Action::Subscribe=>{subscribed=true;view.reset();view.offer(snapshot)?;Ok(serde_json::json!({"kind":"Subscribed"}))}
                    Action::Applied{view:identity,sequence}=>view.applied(identity,sequence).map(|()|serde_json::json!({"kind":"Applied"})),
                    Action::GetHistory{from,to}=>Ok(serde_json::json!({"kind":"History","history":client.history(seat,generation,from,to).await?})),
                    Action::BusSubscribe{consumer}=> {
                        if consumers.len()>=8||consumer.name.is_some()||consumer.filter.iter().any(|p|!(p.starts_with(&format!("seat/{seat}/"))||p.starts_with(&format!("inbox/{seat}/")))) {Err(invalid("seat socket consumer is limited to its own ephemeral topics"))}else {let id=client.bus.subscribe(consumer).await?;consumers.insert(id.clone());Ok(serde_json::json!({"kind":"BusSubscribed","consumer":id}))}
                    }
                    Action::PollEvents{consumer,limit}=> {if !consumers.contains(&consumer){Err(invalid("consumer belongs to another connection"))}else{Ok(serde_json::json!({"kind":"Events","events":client.bus.poll(&consumer,limit.min(128)).await?}))}}
                    Action::Ack{consumer,stream,sequence}=>{if !consumers.contains(&consumer){Err(invalid("consumer belongs to another connection"))}else{client.bus.ack(&consumer,&stream,sequence).await.map(|()|serde_json::json!({"kind":"Acked"}))}}
                };
                match result {Ok(value)=>send(&mut write,&value).await?,Err(error)=>send(&mut write,&serde_json::json!({"kind":"Rejected","reason":error.to_string()})).await?,}
            }
            _=tick.tick(),if subscribed=> {
                let snapshot=client.snapshot(seat,generation).await?;
                let identity=(snapshot.view,snapshot.sequence);
                if last!=Some(identity){view.offer(snapshot)?;last=Some(identity);}
                if let Some(update)=view.next_update(){send(&mut write,&update).await?;}
            }
        }
    }
    Ok(())
}

async fn session(
    stream: tokio::net::UnixStream,
    client: LogicClient,
    seat: Ulid,
    generation: u64,
    capability: [u8; 32],
) -> Result<()> {
    let mut consumers = BTreeSet::new();
    let result = session_inner(
        stream,
        client.clone(),
        seat,
        generation,
        capability,
        &mut consumers,
    )
    .await;
    for consumer in consumers {
        let _ = client.bus.unsubscribe(&consumer).await;
    }
    result
}
