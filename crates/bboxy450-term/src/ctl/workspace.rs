// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! A pending workspace has admitted no effect. Retry that one typed condition
//! with the original operation and request, within the connection's budget.
use super::*;
use std::time::Instant;

impl Client {
    pub(super) fn workspace_scope(
        &mut self,
        swarm: Option<Ulid>,
    ) -> Result<Option<bbt_proto::gui::SwarmScope>, String> {
        let Some(swarm) = swarm else { return Ok(None) };
        let op = Ulid::new();
        match self.call(
            Some(op),
            GuiAction::SwarmGet { id: op, swarm },
            |reply| matches!(reply, GuiReply::Swarm { op: id, .. } if *id == op),
        )? {
            GuiReply::Swarm { record, .. } => Ok(Some(bbt_proto::gui::SwarmScope {
                swarm,
                revision: record.map_or(0, |record| record.revision),
            })),
            _ => unreachable!(),
        }
    }
}

fn pending(result: &Result<GuiReply, String>) -> bool {
    match result {
        Err(reason) => reason.starts_with("workspace_pending:"),
        Ok(GuiReply::Knowledge { response, .. }) => serde_json::from_value::<
            bbt_mux::knowledge::data_source::Result<Value>,
        >(response.clone())
        .is_ok_and(|result| result.is_err_and(|error| error.code == "workspace_pending")),
        _ => false,
    }
}

fn time_left(deadline: Instant) -> Result<Duration, String> {
    let left = deadline.saturating_duration_since(Instant::now());
    if left.is_zero() {
        return Err("unknown: timeout: workspace result not received; inspect the original operation before retrying".into());
    }
    Ok(left.max(Duration::from_millis(1)))
}

fn closed_peer(stream: &UnixStream) -> bool {
    use std::os::fd::AsRawFd;
    let mut poll = libc::pollfd {
        fd: stream.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // A hangup is permanent: the sole reader can drain queued bytes and EOF
    // without waiting. Darwin can refuse timeout changes after this close.
    // SAFETY: poll points to one initialized descriptor for this synchronous call.
    unsafe { libc::poll(&mut poll, 1, 0) > 0 && poll.revents & libc::POLLHUP != 0 }
}

pub(super) fn send(
    client: &mut Client,
    action: GuiAction,
    deadline: Instant,
) -> Result<(), String> {
    let mut frame = serde_json::to_vec(&GuiRequest {
        version: bbt_proto::gui::VERSION,
        capability: client.capability,
        action,
    })
    .map_err(|e| e.to_string())?;
    frame.push(b'\n');
    let mut written = 0;
    while written < frame.len() {
        client
            .writer
            .set_write_timeout(Some(time_left(deadline)?))
            .map_err(|e| format!("gui.sock write deadline: {e}"))?;
        match client.writer.write(&frame[written..]) {
            Ok(0) => return Err("gui.sock write: connection stopped accepting the request".into()),
            Ok(count) => written += count,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => (),
            Err(error) => return Err(format!("gui.sock write: {error}")),
        }
    }
    Ok(())
}

pub(super) fn reply(client: &mut Client, deadline: Instant) -> Result<GuiReply, String> {
    let mut line = Vec::new();
    loop {
        if let Err(error) = client
            .reader
            .get_ref()
            .set_read_timeout(Some(time_left(deadline)?))
            && (error.raw_os_error() != Some(libc::EINVAL) || !closed_peer(client.reader.get_ref()))
        {
            return Err(format!("gui.sock read deadline: {error}"));
        }
        let bytes = client
            .reader
            .fill_buf()
            .map_err(|e| format!("gui.sock read: {e}"))?;
        if bytes.is_empty() {
            if line.is_empty() {
                return Err(
                    "gui.sock closed the connection (version or capability refused)".into(),
                );
            }
            break;
        }
        let count = bytes
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(bytes.len(), |at| at + 1)
            .min(FRAME_LIMIT + 1 - line.len());
        line.extend_from_slice(&bytes[..count]);
        client.reader.consume(count);
        if line.len() > FRAME_LIMIT {
            return Err("gui.sock frame over the GUI reply limit".into());
        }
        if line.last() == Some(&b'\n') {
            break;
        }
    }
    serde_json::from_slice(&line).map_err(|e| format!("gui.sock reply: {e}"))
}

impl Client {
    pub(super) fn call_workspace(
        &mut self,
        op: Ulid,
        action: GuiAction,
        mut answer: impl FnMut(&GuiReply) -> bool,
    ) -> Result<GuiReply, String> {
        let read = self
            .reader
            .get_ref()
            .read_timeout()
            .map_err(|e| e.to_string())?;
        let write = self.writer.write_timeout().map_err(|e| e.to_string())?;
        let budget = [read, write].into_iter().flatten().min().unwrap_or(TIMEOUT);
        let deadline = Instant::now() + budget;
        let result = loop {
            let result =
                self.call_with_deadline(Some(op), action.clone(), &mut answer, Some(deadline));
            if !pending(&result) {
                break result;
            }
            let left = deadline.saturating_duration_since(Instant::now());
            let pause = Duration::from_millis(100).min(left);
            std::thread::sleep(pause);
            if Instant::now() >= deadline {
                break result;
            }
        };
        // A completed response remains authoritative even if the peer closed
        // immediately afterward. The next call reports a closed connection.
        let _ = self.reader.get_ref().set_read_timeout(read);
        let _ = self.writer.set_write_timeout(write);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;

    fn fixture() -> (tempfile::TempDir, Client, UnixStream) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gui.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let client = Client::connect(&Endpoint {
            path,
            capability: [0; 32],
        })
        .unwrap();
        let (door, _) = listener.accept().unwrap();
        door.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        (dir, client, door)
    }
    fn read(reader: &mut BufReader<UnixStream>) -> GuiRequest {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap()
    }
    fn send(door: &mut UnixStream, reply: &GuiReply) {
        serde_json::to_writer(&mut *door, reply).unwrap();
        door.write_all(b"\n").unwrap();
    }
    #[test]
    fn plugin_and_knowledge_controls_capture_swarm_revision_and_retry_only_pending() {
        use bbt_proto::gui::{KnowledgeRequest, KnowledgeStatus, PluginRequest, SwarmScope};
        for (noun, verb) in [
            ("plugin", "list"),
            ("knowledge", "status"),
            ("knowledge", "select"),
        ] {
            let (_dir, mut client, mut door) = fixture();
            let swarm = Ulid::new();
            let op = Ulid::new();
            client.op = Some(op);
            let expected = Some(SwarmScope { swarm, revision: 7 });
            let server = std::thread::spawn(move || {
                let mut reader = BufReader::new(door.try_clone().unwrap());
                let first = read(&mut reader);
                let GuiAction::SwarmGet { id, swarm: actual } = first.action else {
                    panic!("capture workspace first")
                };
                assert_eq!(actual, swarm);
                send(&mut door, &serde_json::from_value(json!({"kind":"Swarm","body":{"op":id,"record":{"id":swarm,"name":null,"workdir":null,"revision":7}}})).unwrap());
                if verb == "select" {
                    let preflight = read(&mut reader);
                    let GuiAction::KnowledgeControl {
                        id,
                        scope,
                        request: KnowledgeRequest::Status,
                    } = preflight.action
                    else {
                        panic!("provider selection needs a status preflight")
                    };
                    assert_ne!(id, op);
                    assert_eq!(scope, expected);
                    let status = serde_json::from_value(json!({"roots":[{"id":"root","path":"/private/project/.bboxy","provider":{"id":"native","name":"native","sha256":null},"revision":3,"records":null,"process":null,"refusal":null}],"offered":[{"id":"gitkb","name":"GitKB","sha256":"a".repeat(64)}]})).unwrap();
                    send(&mut door, &GuiReply::KnowledgeControl { op: id, status });
                }
                let admitted = read(&mut reader);
                let response = match &admitted.action {
                    GuiAction::Plugin {
                        id,
                        scope,
                        request: PluginRequest::List,
                    } => {
                        assert_eq!(*id, op);
                        assert_eq!(*scope, expected);
                        GuiReply::Plugin {
                            op,
                            entries: vec![],
                        }
                    }
                    GuiAction::KnowledgeControl {
                        id,
                        scope,
                        request: KnowledgeRequest::Status,
                    } => {
                        assert_eq!(*id, op);
                        assert_eq!(*scope, expected);
                        GuiReply::KnowledgeControl {
                            op,
                            status: KnowledgeStatus {
                                roots: vec![],
                                offered: vec![],
                            },
                        }
                    }
                    GuiAction::KnowledgeControl {
                        id,
                        scope,
                        request:
                            KnowledgeRequest::Select {
                                root,
                                provider,
                                revision,
                                ..
                            },
                    } => {
                        assert_eq!(*id, op);
                        assert_eq!(*scope, expected);
                        assert_eq!(root, "root");
                        assert_eq!(provider, "gitkb");
                        assert_eq!(*revision, 3);
                        GuiReply::KnowledgeControl {
                            op,
                            status: KnowledgeStatus {
                                roots: vec![],
                                offered: vec![],
                            },
                        }
                    }
                    _ => panic!("unexpected workspace control"),
                };
                send(
                    &mut door,
                    &GuiReply::Error {
                        op,
                        reason: "workspace_pending: preparing root".into(),
                    },
                );
                assert_eq!(
                    serde_json::to_value(read(&mut reader)).unwrap(),
                    serde_json::to_value(admitted).unwrap()
                );
                send(&mut door, &response);
            });
            let mut args = vec![noun.into(), verb.into()];
            if verb == "select" {
                args.extend(["root".into(), "gitkb".into()]);
            }
            args.extend(["--swarm".into(), swarm.to_string()]);
            let parsed = parse_full(&args).unwrap();
            let mut tail = None;
            let mut capture_seat = None;
            let mut message_refusal = None;
            with_stdout(|out| {
                let cx = VerbCx {
                    client,
                    id: op,
                    human: false,
                    endpoint_file: PathBuf::new(),
                    tail: &mut tail,
                    capture_seat: &mut capture_seat,
                    message_refusal: &mut message_refusal,
                    out,
                    noun,
                    verb,
                };
                if noun == "plugin" {
                    plugin::run_verb(cx, parsed.command)
                } else {
                    knowledge::run_verb(cx, parsed.command)
                }
            })
            .unwrap();
            server.join().unwrap();
        }
    }
    #[test]
    fn pending_workspace_reuses_the_exact_task_operation_and_request() {
        let (_dir, mut client, mut door) = fixture();
        let op = Ulid::new();
        client.op = Some(op);
        let server = std::thread::spawn(move || {
            let mut reader = BufReader::new(door.try_clone().unwrap());
            let first = read(&mut reader);
            send(
                &mut door,
                &GuiReply::Error {
                    op,
                    reason: "workspace_pending: reconciling".into(),
                },
            );
            let second = read(&mut reader);
            assert_eq!(
                serde_json::to_value(first).unwrap(),
                serde_json::to_value(second).unwrap()
            );
            send(
                &mut door,
                &GuiReply::Task {
                    op,
                    response: TaskResponse::Applied,
                },
            );
        });
        assert!(matches!(
            client
                .task(TaskRequest::Accept {
                    task_path: ".bboxy/tasks/original.md".into()
                })
                .unwrap(),
            TaskResponse::Applied
        ));
        server.join().unwrap();
    }
    #[test]
    fn pending_record_write_keeps_its_original_operation_and_payload() {
        let (_dir, mut client, mut door) = fixture();
        let op = Ulid::new();
        client.op = Some(op);
        let server = std::thread::spawn(move || {
            let mut reader = BufReader::new(door.try_clone().unwrap());
            let first = read(&mut reader);
            let response =
                serde_json::to_value(bbt_mux::knowledge::data_source::Result::<Value>::Err(
                    bbt_mux::knowledge::data_source::CommandError::new(
                        "workspace_pending",
                        "preparing",
                    ),
                ))
                .unwrap();
            send(&mut door, &GuiReply::Knowledge { op, response });
            let second = read(&mut reader);
            assert_eq!(
                serde_json::to_value(first).unwrap(),
                serde_json::to_value(second).unwrap()
            );
            send(
                &mut door,
                &GuiReply::Knowledge {
                    op,
                    response: json!({"Ok":{"written":true}}),
                },
            );
        });
        assert_eq!(
            client
                .record_write(json!({"path":"notes/e2e.md","body":"original"}))
                .unwrap(),
            json!({"written":true})
        );
        server.join().unwrap();
    }
    #[test]
    fn a_closed_peer_keeps_its_final_reply_readable() {
        let (_dir, mut client, mut door) = fixture();
        let op = Ulid::new();
        send(
            &mut door,
            &GuiReply::Task {
                op,
                response: TaskResponse::Applied,
            },
        );
        drop(door);
        assert!(closed_peer(client.reader.get_ref()));
        assert!(matches!(
            reply(&mut client, Instant::now() + Duration::from_secs(1)).unwrap(),
            GuiReply::Task { op: received, response: TaskResponse::Applied } if received == op
        ));
    }
    #[test]
    fn a_closed_peer_cannot_turn_a_truncated_reply_into_success() {
        let (_dir, mut client, mut door) = fixture();
        door.write_all(b"{\"incomplete\":").unwrap();
        drop(door);
        assert!(
            reply(&mut client, Instant::now() + Duration::from_secs(1))
                .unwrap_err()
                .starts_with("gui.sock reply:")
        );
    }
    #[test]
    fn unknown_and_permanent_refusals_are_never_redispatched() {
        for (reason, portable) in [
            ("unknown: outcome unconfirmed", false),
            ("rejected: workspace_pending is not a retry code", false),
            ("op_id_reused: different root", false),
            (
                "unknown: workspace_pending cannot disguise an unknown effect",
                true,
            ),
        ] {
            let (_dir, mut client, mut door) = fixture();
            let op = Ulid::new();
            client.op = Some(op);
            let expected = reason.to_owned();
            let server = std::thread::spawn(move || {
                let mut reader = BufReader::new(door.try_clone().unwrap());
                read(&mut reader);
                let reply = if portable {
                    GuiReply::Knowledge {
                        op,
                        response: serde_json::to_value(bbt_mux::knowledge::data_source::Result::<
                            Value,
                        >::Err(
                            bbt_mux::knowledge::data_source::CommandError::new(
                                "unknown",
                                expected.strip_prefix("unknown: ").unwrap(),
                            ),
                        ))
                        .unwrap(),
                    }
                } else {
                    GuiReply::Error {
                        op,
                        reason: expected,
                    }
                };
                send(&mut door, &reply);
                door.set_read_timeout(Some(Duration::from_millis(150)))
                    .unwrap();
                let mut next = String::new();
                assert!(
                    reader.read_line(&mut next).is_err(),
                    "refused effect was redispatched"
                );
            });
            assert_eq!(
                client
                    .record_write(json!({"body":"must run once"}))
                    .unwrap_err(),
                reason
            );
            server.join().unwrap();
        }
    }
    #[test]
    fn pending_retry_respects_the_existing_socket_budget_and_restores_it() {
        let (_dir, mut client, mut door) = fixture();
        let budget = Duration::from_millis(100);
        client
            .reader
            .get_ref()
            .set_read_timeout(Some(budget))
            .unwrap();
        let op = Ulid::new();
        client.op = Some(op);
        let server = std::thread::spawn(move || {
            let mut reader = BufReader::new(door.try_clone().unwrap());
            read(&mut reader);
            send(
                &mut door,
                &GuiReply::Error {
                    op,
                    reason: "workspace_pending: held".into(),
                },
            );
            door.set_read_timeout(Some(Duration::from_millis(150)))
                .unwrap();
            let mut next = String::new();
            assert!(reader.read_line(&mut next).is_err());
        });
        let started = Instant::now();
        assert!(
            client
                .record_write(json!({}))
                .unwrap_err()
                .starts_with("workspace_pending:")
        );
        assert!(started.elapsed() < Duration::from_secs(1));
        assert_eq!(
            client.reader.get_ref().read_timeout().unwrap(),
            Some(budget)
        );
        server.join().unwrap();
    }

    #[test]
    fn a_failed_write_cannot_promote_another_operations_pending_reply() {
        let (_dir, mut client, mut door) = fixture();
        let op = Ulid::new();
        send(
            &mut door,
            &GuiReply::Error {
                op: Ulid::new(),
                reason: "workspace_pending: unrelated operation".into(),
            },
        );
        client.writer.shutdown(std::net::Shutdown::Write).unwrap();
        let result = client.call_with_deadline(
            Some(op),
            GuiAction::RecordWrite {
                id: op,
                scope: None,
                args: json!({"body":"must not retry"}),
            },
            |reply| matches!(reply, GuiReply::Knowledge { op: reply_op, .. } if *reply_op == op),
            Some(Instant::now() + Duration::from_secs(1)),
        );
        assert!(
            !pending(&result),
            "a failed transport cannot authorize replay"
        );
        let error = result.unwrap_err();
        assert!(error.contains("gui.sock write"), "{error}");
        assert!(!error.starts_with("workspace_pending:"), "{error}");
    }

    #[test]
    fn partial_reply_progress_cannot_extend_the_absolute_workspace_deadline() {
        let (_dir, mut client, mut door) = fixture();
        client
            .reader
            .get_ref()
            .set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        let server = std::thread::spawn(move || {
            let mut reader = BufReader::new(door.try_clone().unwrap());
            read(&mut reader);
            // Each piece arrives within the socket timeout, but the complete
            // frame takes longer than the request's total budget.
            for byte in b"{\"kind\":\"Knowledge\",\"body\":{}}\n" {
                if door.write_all(&[*byte]).is_err() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(30));
            }
        });
        let began = Instant::now();
        assert!(client.record_write(json!({})).is_err());
        assert!(began.elapsed() < Duration::from_millis(600));
        assert_eq!(
            client.reader.get_ref().read_timeout().unwrap(),
            Some(Duration::from_millis(100))
        );
        drop(client);
        server.join().unwrap();
    }
}
