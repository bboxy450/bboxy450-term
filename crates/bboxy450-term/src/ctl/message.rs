//! Messages between seats: `ctl message …`.
use super::*;

/// This noun's rows of [`super::VERBS`], in parse order.
pub(super) const VERBS: &[super::Verb] = &[
    ("message", "send", "Messaging"),
    ("message", "post", "Messaging"),
    ("message", "inbox", "Messaging"),
    ("message", "receipts", "Messaging"),
    ("message", "ack", "Messaging"),
];

/// This noun's `Command` variants, spliced into the flat enum by `commands` in `ctl.rs`.
macro_rules! message_commands {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)* "message": [
            Dm {
                seat: String,
                text: String,
            },
            Post {
                destination: String,
                text: String,
                swarm: Option<Ulid>,
            },
            Inbox {
                since: u64,
            },
            Receipts {
                message: Ulid,
            },
            /// `message send (<destination|seat> | --reply-to <id>) <text>` (#264 routing).
            Send {
                to: Option<String>,
                text: String,
                reply_to: Option<Ulid>,
                swarm: Option<Ulid>,
            },
            /// `message ack <id>`: the operator acknowledges a message addressed to it.
            Ack {
                message: Ulid,
            },
        ]] $($head)*);
    };
}
pub(super) use message_commands;

/// `ctl message …` and its pre-grammar aliases, parsed into this noun's commands; `None`
/// when the words are not this noun's.
pub(super) fn parse(words: &[&str], cx: &super::ParseCx) -> Result<Option<Command>, String> {
    let flags = cx.flags;
    let flag = |name: &str| flags.get(name).cloned();
    let ulid = |v: &String| v.parse::<Ulid>().map_err(|_| format!("not an id: {v}"));
    let swarm = cx.swarm;
    Ok(Some(match *words {
        ["dm", seat, text] => Command::Dm {
            seat: seat.into(),
            text: text.into(),
        },
        ["post", destination, text] => Command::Post {
            destination: destination.into(),
            text: text.into(),
            swarm,
        },
        ["inbox"] => Command::Inbox {
            since: flag("since")
                .map(|v| {
                    v.parse()
                        .map_err(|_| format!("--since: not a sequence: {v}"))
                })
                .transpose()?
                .unwrap_or(0),
        },
        ["receipts", message] => Command::Receipts {
            message: ulid(&message.to_owned())?,
        },
        ["message", "send", to, text] => Command::Send {
            to: Some(to.into()),
            text: text.into(),
            reply_to: flag("reply-to").as_ref().map(ulid).transpose()?,
            swarm,
        },
        ["message", "send", text] if flags.contains_key("reply-to") => Command::Send {
            to: None,
            text: text.into(),
            reply_to: flag("reply-to").as_ref().map(ulid).transpose()?,
            swarm,
        },
        ["message", "post", destination, text] => Command::Post {
            destination: destination.into(),
            text: text.into(),
            swarm,
        },
        ["message", "inbox"] => Command::Inbox {
            since: flag("since")
                .map(|v| {
                    v.parse()
                        .map_err(|_| format!("--since: not a sequence: {v}"))
                })
                .transpose()?
                .unwrap_or(0),
        },
        ["message", "receipts", message] => Command::Receipts {
            message: ulid(&message.to_owned())?,
        },
        ["message", "ack", message] => Command::Ack {
            message: ulid(&message.to_owned())?,
        },
        _ => return Ok(None),
    }))
}

/// The flat (pre-grammar) form of this noun's commands; the others are noun-verb only.
pub(super) fn run(client: &mut Client, command: Command) -> Result<Vec<Value>, String> {
    let to_value = |v: &dyn erased::Serialize| v.to_json();
    Ok(match command {
        Command::Dm { seat, text } => {
            let seats = client.seats()?;
            let seat = resolve(&seats, &seat)?;
            let message = SendRequest::text(format!("seats/{}/inbox", seat.record.id), text);
            let id = message.id;
            let response = client.messaging(seat.record.swarm, Request::Send { message })?;
            vec![
                json!({ "kind": "sent", "message": id, "seat": seat.record.id, "response": to_value(&response) }),
            ]
        }
        Command::Post {
            destination,
            text,
            swarm,
        } => {
            let swarm = match swarm {
                Some(swarm) => swarm,
                None => *swarms(&client.seats()?)
                    .first()
                    .ok_or("no seats: pass --swarm")?,
            };
            let message = SendRequest::text(destination.clone(), text);
            let id = message.id;
            let response = client.messaging(swarm, Request::Send { message })?;
            vec![
                json!({ "kind": "sent", "message": id, "destination": destination, "response": to_value(&response) }),
            ]
        }
        Command::Inbox { since } => {
            // The operator's inbox: an operator List with no channel lists what is addressed
            // to this device's operator sink (decision #38 (e)), one swarm at a time (#38 (f)).
            let mut out = vec![];
            for swarm in swarms(&client.seats()?) {
                let mut after = since;
                loop {
                    let request = Request::List {
                        thread: None,
                        channel: None,
                        after,
                        limit: PAGE,
                    };
                    let Response::Messages { page } = client.messaging(swarm, request)? else {
                        return Err("inbox: unexpected messaging response".into());
                    };
                    for message in &page.items {
                        out.push(json!({ "kind": "message", "swarm": swarm, "message": to_value(message) }));
                    }
                    match page.next {
                        Some(next) if next > after && !page.items.is_empty() => after = next,
                        _ => break,
                    }
                }
            }
            out
        }
        Command::Receipts { message } => {
            // A message lives in one swarm (decision #38 (f)); ask each until one answers.
            let mut last = String::from("no seats, so no swarm to ask");
            for swarm in swarms(&client.seats()?) {
                let request = Request::Receipts {
                    message,
                    after: 0,
                    limit: PAGE,
                };
                match client.messaging(swarm, request) {
                    Ok(Response::Receipts { page }) => {
                        return Ok(page
                            .items
                            .iter()
                            .map(|r| json!({ "kind": "receipt", "message": message, "swarm": swarm, "receipt": to_value(r) }))
                            .collect());
                    }
                    Ok(_) => return Err("receipts: unexpected messaging response".into()),
                    Err(reason) => last = reason,
                }
            }
            return Err(format!("receipts for {message}: {last}"));
        }
        _ => {
            return Err(no_flat_form());
        }
    })
}

/// This noun's noun-verb commands, run on the admitted connection; prints the one reply.
pub(super) fn run_verb(cx: super::VerbCx<'_>, command: Command) -> Result<(), String> {
    let super::VerbCx {
        mut client,
        out,
        noun,
        verb,
        id,
        human,
        ..
    } = cx;
    let data = match command {
        Command::Send {
            to,
            text,
            reply_to,
            swarm,
        } => {
            let seats = client.seats()?;
            // A seat name or id addresses its inbox; anything else is a destination path.
            let (destination, seat_swarm) = match to.as_deref() {
                Some(to) if !to.contains('/') => {
                    let seat = resolve(&seats, to)?;
                    (
                        format!("seats/{}/inbox", seat.record.id),
                        Some(seat.record.swarm),
                    )
                }
                Some(path) => (path.to_owned(), None),
                None => (String::new(), None),
            };
            let swarm = swarm
                .or(seat_swarm)
                .or_else(|| swarms(&seats).first().copied())
                .ok_or("no seats: pass --swarm")?;
            let mut message = SendRequest::text(destination, text);
            message.id = id;
            message.reply_to = reply_to;
            serde_json::to_value(client.messaging(swarm, Request::Send { message })?)
                .map_err(|e| e.to_string())?
        }
        Command::Ack { message } => {
            let mut last = String::from("no seats, so no swarm to ask");
            let mut answer = None;
            for swarm in swarms(&client.seats()?) {
                match client.messaging(
                    swarm,
                    Request::Acknowledge {
                        message,
                        agent: None,
                    },
                ) {
                    Ok(response) => {
                        answer = Some(response);
                        break;
                    }
                    Err(reason) => last = reason,
                }
            }
            serde_json::to_value(answer.ok_or(format!("ack {message}: {last}"))?)
                .map_err(|e| e.to_string())?
        }
        command => super::flat_reply(&mut client, command, verb)?,
    };
    super::print_reply(out, noun, verb, id, data, human)
}
