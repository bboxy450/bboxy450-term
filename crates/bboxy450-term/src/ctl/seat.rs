//! Seats: listing, input, screens, capture and lifecycle: `ctl seat …`.
use super::*;

/// This noun's rows of [`super::VERBS`], in parse order.
pub(super) const VERBS: &[super::Verb] = &[
    ("seat", "ls", "ListSeats"),
    ("seat", "spawn", "Spawn"),
    ("seat", "screen", "Screen"),
    ("seat", "type", "Input"),
    ("seat", "key", "Input"),
    ("seat", "paste", "Input"),
    ("seat", "wait", "Screen"),
    ("seat", "resize", "Resize"),
    ("seat", "watch", "EventsRead"),
    ("seat", "rename", "RenameSeat"),
    ("seat", "focus", "FocusSeat"),
    ("seat", "restart", "RestartSeat"),
    ("seat", "close", "CloseSeat"),
    ("seat", "undo-close", "UndoCloseSeat"),
    ("seat", "capture", "Capture"),
];

/// This noun's `Command` variants, spliced into the flat enum by `commands` in `ctl.rs`.
macro_rules! seat_commands {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)* "seat": [
            Spawn {
                harness: String,
                name: Option<String>,
                cwd: Option<PathBuf>,
                swarm: Option<Ulid>,
                /// A fresh tab; without it the backend places the seat as the launcher's default.
                new_tab: bool,
                /// A role preset (#262); the backend resolves and guards it as the launcher does.
                role: Option<String>,
            },
            /// An explicit placement is captured by the attached GUI, never guessed here.
            SpawnAt(Box<bbt_proto::gui::UiSpawn>),
            Seats,
            Capture {
                path: PathBuf,
                seat: Option<String>,
            },
            /// `seat screen` (ctl-cli §1, decision #270): what the operator would see in its pane.
            Screen {
                seat: String,
                human: bool,
                /// `--scrollback A..B`: the history rows of that capture range (#27).
                scrollback: Option<(u64, u64)>,
                cells: bool,
            },
            /// `seat watch <seat> [--timeout MS]`: its state from the retained stream, its screen when
            /// the screen's sequence moves; event lines, then a done line.
            Watch {
                seat: String,
                timeout: Duration,
            },
            /// `seat rename <seat> <name>`.
            Rename {
                seat: String,
                name: String,
            },
            /// `seat restart <seat>`: the seat's next process generation.
            Restart {
                seat: String,
            },
            /// `seat focus <seat>`: the window focuses it.
            Focus {
                seat: String,
            },
            /// `seat resize <seat> <cols>x<rows>`.
            Resize {
                seat: String,
                cols: u16,
                rows: u16,
            },
            /// `seat type`, `seat key`, `seat paste` (ctl-cli rev. 2): bytes for the window's one
            /// input path, from literal text, named keys, or a bracketed paste.
            Type {
                seat: String,
                text: String,
            },
            Key {
                seat: String,
                keys: Vec<String>,
            },
            Paste {
                seat: String,
                source: String,
            },
            /// `seat wait`: until the seat is in a state or its screen matches, or the timeout.
            Wait {
                seat: String,
                state: Option<String>,
                text: Option<String>,
                subagent: Option<String>,
                grade: Option<String>,
                /// `--on-screen`: the seat has a pane in the window's current frame (ctl-cli §1a).
                on_screen: bool,
                /// `--frame-seq N`: the window has produced frame N.
                frame_seq: Option<u64>,
                timeout: Duration,
            },
            /// `app endpoint`: the run directory, socket path and generation; never the capability.
            Close {
                seat: String,
            },
            UndoClose {
                seat: String,
            },
        ]] $($head)*);
    };
}
pub(super) use seat_commands;

/// `ctl seat …` and its pre-grammar aliases, parsed into this noun's commands; `None`
/// when the words are not this noun's.
pub(super) fn parse(words: &[&str], cx: &super::ParseCx) -> Result<Option<Command>, String> {
    let flags = cx.flags;
    let flag = |name: &str| flags.get(name).cloned();
    let swarm = cx.swarm;
    let human_flag = cx.human_flag;
    let on_screen_flag = cx.on_screen_flag;
    let cells_flag = cx.cells_flag;
    Ok(Some(match *words {
    ["spawn", harness] => Command::Spawn {
        harness: harness.into(),
        name: flag("name"),
        cwd: flag("cwd").map(PathBuf::from),
        swarm,
        role: flag("role"),
        new_tab: match flag("placement").as_deref() {
            None => false,
            Some("new-tab") => true,
            Some(other) => {
                return Err(format!(
                    "--placement: only new-tab is supported, not {other}"
                ));
            }
        },
    },
    ["seats"] => Command::Seats,
    ["seat", "screen", seat] => Command::Screen {
        seat: seat.into(),
        human: human_flag,
        scrollback: flag("scrollback")
            .map(|range| {
                range
                    .split_once("..")
                    .and_then(|(a, b)| Some((a.parse::<u64>().ok()?, b.parse::<u64>().ok()?)))
                    .filter(|(a, b)| a <= b && b - a <= SCROLLBACK_SEQUENCES)
                    .ok_or(format!(
                        "{USAGE_PREFIX}--scrollback FROM..TO, capture sequences, at most {SCROLLBACK_SEQUENCES} apart, not {range}"
                    ))
            })
            .transpose()?,
        cells: cells_flag,
    },
    ["seat", "rename", seat, name] => Command::Rename {
        seat: seat.into(),
        name: name.into(),
    },
    ["seat", "focus", seat] => Command::Focus { seat: seat.into() },
    ["seat", "restart", seat] => Command::Restart { seat: seat.into() },
    ["seat", "close", seat] => Command::Close { seat: seat.into() },
    ["seat", "undo-close", seat] => Command::UndoClose { seat: seat.into() },
    ["seat", "watch", seat] => Command::Watch {
        seat: seat.into(),
        timeout: Duration::from_millis(
            flag("timeout")
                .map(|v| {
                    v.parse::<u64>()
                        .ok()
                        .filter(|ms| (1..=3_600_000).contains(ms))
                        .ok_or(format!("{USAGE_PREFIX}--timeout: 1 to 3600000 ms, not {v}"))
                })
                .transpose()?
                .unwrap_or(30_000),
        ),
    },
    ["seat", "resize", seat, size] => {
        let (cols, rows) = size
            .split_once('x')
            .and_then(|(c, r)| Some((c.parse::<u16>().ok()?, r.parse::<u16>().ok()?)))
            .ok_or(format!("{USAGE_PREFIX}seat resize: COLSxROWS, not {size}"))?;
        Command::Resize {
            seat: seat.into(),
            cols,
            rows,
        }
    }
    ["seat", "type", seat, text] => Command::Type {
        seat: seat.into(),
        text: text.into(),
    },
    ["seat", "key", seat, ref keys @ ..] if !keys.is_empty() => Command::Key {
        seat: seat.into(),
        keys: keys.iter().map(|k| k.to_string()).collect(),
    },
    ["seat", "paste", seat, source] => Command::Paste {
        seat: seat.into(),
        source: source.into(),
    },
    ["seat", "wait", seat] => {
        let (state, text) = (flag("state"), flag("text"));
        let frame_seq = flag("frame-seq")
            .map(|v| {
                v.parse::<u64>()
                    .map_err(|_| format!("{USAGE_PREFIX}--frame-seq: a frame number, not {v}"))
            })
            .transpose()?;
        let conditions = [
            state.is_some(),
            text.is_some(),
            on_screen_flag,
            frame_seq.is_some(),
        ];
        if conditions.iter().filter(|set| **set).count() != 1 {
            return Err(format!(
                "seat wait: one of --state, --text, --on-screen or --frame-seq\n{USAGE}"
            ));
        }
        if (flag("subagent").is_some() || flag("grade").is_some()) && state.is_none() {
            return Err(format!("{USAGE_PREFIX}--subagent and --grade require --state"));
        }
        if flag("grade").as_deref().is_some_and(|grade| grade != "reported") {
            return Err(format!("{USAGE_PREFIX}--grade: reported"));
        }
        Command::Wait {
            seat: seat.into(),
            state,
            text,
            subagent: flag("subagent"),
            grade: flag("grade"),
            on_screen: on_screen_flag,
            frame_seq,
            timeout: Duration::from_millis(
                flag("timeout")
                    .map(|v| {
                        v.parse::<u64>()
                            .ok()
                            .filter(|ms| (1..=3_600_000).contains(ms))
                            .ok_or(format!("--timeout: 1 to 3600000 ms, not {v}"))
                    })
                    .transpose()?
                    .unwrap_or(30_000),
            ),
        }
    }
    ["seat", "ls"] => Command::Seats,
    ["seat", "spawn", harness] => {
        if let Some(place) = flag("place").or(flag("placement")) {
            use bbt_proto::gui::{SpawnPlace, UiSpawn};
            let place = match place.as_str() {
                "right" => SpawnPlace::Right,
                "down" => SpawnPlace::Down,
                "tab" | "new-tab" => SpawnPlace::Tab,
                "space" => SpawnPlace::Space,
                _ => return Err(format!("{USAGE_PREFIX}--place: expected right|down|tab|space")),
            };
            let pick = UiSpawn {
                harness: harness.into(), name: flag("name"), role: flag("role"), swarm, place,
                cwd: std::path::absolute(flag("cwd").unwrap_or_else(|| ".".into()))
                    .map_err(|e| format!("{USAGE_PREFIX}--cwd: {e}"))?.to_string_lossy().into_owned(),
            };
            let request = bbt_proto::gui::UiRequest::Spawn(Box::new(pick));
            request.validate().map_err(|e| format!("{USAGE_PREFIX}{e}"))?;
            let bbt_proto::gui::UiRequest::Spawn(pick) = request else { unreachable!() };
            Command::SpawnAt(pick)
        } else {
            // The no-placement backend lane also serves bare Session callers.
            Command::Spawn {harness:harness.into(),name:flag("name"),cwd:flag("cwd").map(PathBuf::from),swarm,role:flag("role"),new_tab:false}
        }
    },
    ["seat", "capture", target, path] => Command::Capture {
        path: std::path::absolute(path).map_err(|e| format!("capture {path}: {e}"))?,
        seat: (target != "window").then(|| target.to_string()),
    },
    ["capture", path] => Command::Capture {
        path: std::path::absolute(path).map_err(|e| format!("capture {path}: {e}"))?,
        seat: flag("seat"),
    },
        _ => return Ok(None),
    }))
}

/// The flat (pre-grammar) form of this noun's commands; the others are noun-verb only.
pub(super) fn run(client: &mut Client, command: Command) -> Result<Vec<Value>, String> {
    Ok(match command {
        Command::Spawn {
            harness,
            name,
            cwd,
            swarm,
            new_tab,
            role,
        } => {
            // The launcher's defaults (live.rs `spawn_spec`): the first seat's swarm, else a
            // new one; the backend validates the spec as it does for the GUI.
            let swarm = match swarm {
                Some(swarm) => swarm,
                None => client
                    .seats()?
                    .first()
                    .map_or_else(Ulid::new, |s| s.record.swarm),
            };
            let cwd = match cwd {
                Some(cwd) => cwd,
                None => std::env::current_dir().map_err(|e| e.to_string())?,
            };
            let seat = client.spawn(SpawnSpec {
                swarm: SwarmId::from_u128(swarm.0),
                harness_kind: harness.clone(),
                name,
                role,
                cwd: cwd.to_string_lossy().into_owned(),
                location: Where::Local,
                resume: None,
                placement: new_tab.then(|| bbt_proto::seat_state::Placement::NewTab {
                    tab: bbt_proto::TabId::from_u128(Ulid::new().0),
                }),
            })?;
            vec![json!({ "kind": "spawned", "seat": seat, "harness": harness, "swarm": swarm })]
        }
        Command::Seats => client.seats()?.iter().map(seat_line).collect(),
        Command::Capture { path, seat } => {
            let seat = match seat {
                Some(seat) => Some(resolve(&client.seats()?, &seat)?.record.id),
                None => None,
            };
            // A stale frame says so, so a judge never takes it for the current one.
            let (path, frame) = client.capture(&path, seat)?;
            vec![json!({ "kind": "captured", "path": path, "seat": seat,
                "stale": frame.stale, "occluded": frame.occluded,
                "frame_presented_at": frame.presented_at_ms, "offscreen":frame.offscreen,
                "frame_seq":frame.frame_seq,"rendered_at_ms":frame.rendered_at_ms })]
        }
        Command::Screen {
            seat,
            human,
            scrollback,
            cells,
        } => {
            let seat = resolve(&client.seats()?, &seat)?.record.id;
            let (op, mut screen) = client.screen_with(seat, cells)?;
            if let Some((from, to)) = scrollback {
                // The history rows of the range: its capture records replayed into a fresh
                // terminal the seat's size, gaps marked where the ring has none (#27).
                let records = client.history(seat, from, to)?;
                screen.rows = replay_rows(&records, screen.cols, screen.rows.len());
                screen.cells = None;
            }
            if human {
                let mut lines: Vec<Value> = screen.rows.into_iter().map(Value::String).collect();
                lines.push(Value::String(format!(
                    "-- cursor {},{} · {}",
                    screen.cursor.0, screen.cursor.1, screen.title
                )));
                lines
            } else {
                vec![
                    json!({ "ok": true, "op": op, "noun": "seat", "verb": "screen", "data": screen }),
                ]
            }
        }
        Command::Resize { seat, cols, rows } => {
            let seat = resolve(&client.seats()?, &seat)?.record.id;
            let (_, screen) = client.screen(seat)?;
            let op = client.resize(seat, screen.generation, cols, rows)?;
            vec![
                json!({ "ok": true, "op": op, "noun": "seat", "verb": "resize",
                "data": { "seat": seat, "generation": screen.generation, "cols": cols, "rows": rows } }),
            ]
        }
        Command::Type { seat, text } => input(client, &seat, "type", |_| Ok(text.into_bytes()))?,
        Command::Key { seat, keys } => input(client, &seat, "key", |modes| {
            let mut bytes = vec![];
            for key in &keys {
                bytes.extend(key_bytes(key, modes)?);
            }
            Ok(bytes)
        })?,
        Command::Paste { seat, source } => {
            let mut text = vec![];
            if source == "-" {
                std::io::stdin().read_to_end(&mut text)
            } else {
                std::fs::File::open(&source).and_then(|mut f| f.read_to_end(&mut text))
            }
            .map_err(|e| format!("seat paste {source}: {e}"))?;
            paste(client, &seat, &text)?
        }
        Command::Wait {
            seat,
            on_screen,
            frame_seq,
            timeout,
            ..
        } if on_screen || frame_seq.is_some() => {
            // Window truth, not the model: the owner's latest produced layout (ctl-cli §1a).
            let seat = resolve(&client.seats()?, &seat)?.record.id;
            let op = Ulid::new();
            let deadline = std::time::Instant::now() + timeout;
            loop {
                let layout = match client.ui(bbt_proto::gui::UiRequest::Layout) {
                    Ok(layout) => Some(layout),
                    Err(reason) if reason.starts_with("ui_not_ready") => None,
                    Err(reason) => return Err(reason),
                };
                if let Some(layout) = &layout {
                    let pane = layout.panes.iter().find(|pane| pane.seat == seat);
                    let met = match frame_seq {
                        Some(n) => layout.frame_seq >= n,
                        None => pane.is_some(),
                    };
                    if met {
                        break vec![json!({ "ok": true, "op": op, "done": true,
                            "data": { "seat": seat, "frame_seq": layout.frame_seq,
                                "pane": pane.map(|p| json!({ "rect": p.rect, "cols": p.cols,
                                    "rows": p.rows, "focused": p.focused })) } })];
                    }
                }
                if std::time::Instant::now() >= deadline {
                    let seen = layout.map_or("no produced frame yet".into(), |l| {
                        format!("the window is at frame {}", l.frame_seq)
                    });
                    return Err(format!(
                        "{TIMEOUT_PREFIX}{op}\nwaited {} ms; {seen}",
                        timeout.as_millis()
                    ));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
        Command::Wait {
            seat,
            state,
            text,
            subagent,
            grade,
            timeout,
            ..
        } => {
            let predicate = WaitPredicate::new(state, text, subagent, grade)?;
            let started = std::time::Instant::now();
            wait_for_seat(client, &seat, &predicate, timeout, || started.elapsed())?
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
        capture_seat,
        out,
        noun,
        verb,
        id,
        human,
        ..
    } = cx;
    let data = match command {
        Command::SpawnAt(pick) => {
            let harness = pick.harness.clone();
            let layout = client.ui(bbt_proto::gui::UiRequest::Spawn(pick))?;
            let placed = layout
                .spawned
                .as_ref()
                .ok_or("ui_frame_refused: spawn reply has no placement receipt")?;
            json!({"seat":placed.seat,"swarm":placed.swarm,"harness":harness,
                "spawn_op":placed.op,"placement":placed.placement,"layout_revision":placed.layout_revision,
                "applied_layout_revision":placed.applied_layout_revision,
                "frame_seq":layout.frame_seq,"layout":layout})
        }
        Command::Seats => serde_json::to_value(client.seats()?).map_err(|e| e.to_string())?,
        Command::Screen { .. }
        | Command::Resize { .. }
        | Command::Type { .. }
        | Command::Key { .. }
        | Command::Paste { .. }
        | Command::Wait { .. } => {
            // Already in the §2 shape (and --human for screen).
            for line in run(&mut client, command)? {
                match line {
                    Value::String(text) => writeln!(out, "{text}"),
                    line => writeln!(out, "{line}"),
                }
                .map_err(|e| e.to_string())?;
            }
            return Ok(());
        }
        Command::Rename { seat, name } => {
            let seat = resolve(&client.seats()?, &seat)?.record.id;
            let op = client.mint();
            client.call(
                Some(op),
                GuiAction::RenameSeat {
                    id: op,
                    seat,
                    name: name.clone(),
                },
                |r| matches!(r, GuiReply::Ok { op: o } if *o == op),
            )?;
            json!({ "seat": seat, "name": name })
        }
        Command::Restart { seat } => {
            let seat = resolve(&client.seats()?, &seat)?.record.clone();
            let op = client.mint();
            client.call(
                Some(op),
                GuiAction::RestartSeat {
                    id: op,
                    seat: seat.id,
                    generation: seat.generation,
                },
                |r| matches!(r, GuiReply::Ok { op: o } if *o == op),
            )?;
            json!({ "seat": seat.id, "from_generation": seat.generation })
        }
        command @ (Command::Close { .. } | Command::UndoClose { .. }) => {
            let (seat, undo) = match command {
                Command::Close { seat } => (seat, false),
                Command::UndoClose { seat } => (seat, true),
                _ => unreachable!(),
            };
            let seat = resolve(&client.seats()?, &seat)?.record.id;
            let op = client.mint();
            let reply = client.call(
                Some(op),
                if undo {
                    GuiAction::UndoCloseSeat { id: op, seat }
                } else {
                    GuiAction::CloseSeat { id: op, seat }
                },
                |r| matches!(r,GuiReply::Ui {op:o,..} if *o==op),
            )?;
            let GuiReply::Ui { layout, .. } = reply else {
                unreachable!()
            };
            if layout.frame_seq == 0 {
                return Err(
                    "close_frame_incomplete: the GUI owner returned no produced frame".into(),
                );
            }
            json!({"seat":seat,"relayed":true,"applied":true,"action":if undo {"undo-close"} else {"close"},"frame_seq":layout.frame_seq,"layout":layout})
        }
        Command::Capture { path, seat } => {
            let seat = match seat {
                Some(seat) => Some(resolve(&client.seats()?, &seat)?.record.id),
                None => None,
            };
            *capture_seat = seat;
            let (path, frame) = client.capture(&path, seat)?;
            json!({ "path": path, "seat": seat,
                "stale": frame.stale, "occluded": frame.occluded,
                "frame_presented_at": frame.presented_at_ms, "offscreen":frame.offscreen,
                "frame_seq":frame.frame_seq,"rendered_at_ms":frame.rendered_at_ms })
        }
        Command::Focus { seat } => {
            let seat = resolve(&client.seats()?, &seat)?.record.id;
            let op = client.mint();
            let reply = client.call(
                Some(op),
                GuiAction::FocusSeat { id: op, seat },
                |r| matches!(r, GuiReply::Focused { op: o, .. } if *o == op),
            )?;
            let GuiReply::Focused { frame, .. } = reply else {
                unreachable!()
            };
            if frame.seat.0.0 != seat.0 {
                return Err("focus_superseded: GUI acknowledged a different seat".into());
            }
            json!({"seat":seat,"applied":true,"generation":frame.generation,"frame_seq":frame.frame_seq})
        }
        Command::Watch { seat, timeout } => {
            // State: the seat's retained stream paged by our own cursor, never the window's
            // lane. Screen: polled, a line only when its capture sequence moves (#272).
            let seat = resolve(&client.seats()?, &seat)?.record.id;
            let stream = format!("seat-{seat}");
            let deadline = std::time::Instant::now() + timeout;
            let (mut after, mut state, mut seen, mut events) = (0, None, None, 0u64);
            loop {
                while let (page, Some(next)) = client.events_read(&stream, after, 128)? {
                    state = page
                        .into_iter()
                        .rev()
                        .find(|e| e.subject.ends_with("/state"))
                        .or(state);
                    after = next;
                }
                let (page, _) = client.events_read(&stream, after, 128)?;
                if let Some(last) = page.last() {
                    after = last.seq;
                }
                for event in page.into_iter().filter(|e| e.subject.ends_with("/state")) {
                    state = Some(event);
                }
                if let Some(event) = state.take() {
                    events += 1;
                    writeln!(
                        out,
                        "{}",
                        json!({ "ok": true, "op": id, "event": {
                        "kind": "state", "seat": seat, "seq": event.seq, "state": event.payload } })
                    )
                    .map_err(|e| e.to_string())?;
                }
                let (_, screen) = client.screen(seat)?;
                if seen != Some(screen.sequence) {
                    seen = Some(screen.sequence);
                    events += 1;
                    writeln!(
                        out,
                        "{}",
                        json!({ "ok": true, "op": id, "event": {
                        "kind": "screen", "seat": seat, "sequence": screen.sequence,
                        "cursor": screen.cursor, "rows": screen.rows } })
                    )
                    .map_err(|e| e.to_string())?;
                }
                out.flush().map_err(|e| e.to_string())?;
                if std::time::Instant::now() >= deadline {
                    break;
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            writeln!(
                out,
                "{}",
                json!({ "ok": true, "op": id, "done": true,
                "data": { "seat": seat, "events": events } })
            )
            .map_err(|e| e.to_string())?;
            return Ok(());
        }
        command => super::flat_reply(&mut client, command, verb)?,
    };
    super::print_reply(out, noun, verb, id, data, human)
}

/// One parse sample per verb of this noun's table (the verb gate in ctl.rs's tests).
#[cfg(test)]
pub(super) const PARSE_SAMPLES: &[&str] = &[
    "seat ls",
    "seat spawn zsh",
    "seat screen s",
    "seat type s x",
    "seat key s enter",
    "seat paste s -",
    "seat wait s --state idle",
    "seat capture window /tmp/x.png",
    "seat watch s",
    "seat rename s n",
    "seat close s",
    "seat undo-close s",
    "seat focus s",
    "seat restart s",
    "seat resize s 80x24",
];
