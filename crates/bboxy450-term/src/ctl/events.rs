//! Browse retained pages with the CLI's own stream cursor. Never attach or ACK.
use super::{Client, PAGE, TIMEOUT, TIMEOUT_PREFIX, Ulid, Value, json};
use std::{
    io::Write,
    time::{Duration, Instant},
};

pub(super) struct Progress {
    stream: String,
    after: u64,
}
impl Progress {
    pub(super) fn new(stream: String, after: u64) -> Self {
        Self { stream, after }
    }
    pub(super) fn error(&self, line: &mut Value) {
        line["stream"] = json!(self.stream);
        line["seq"] = json!(self.after);
        line["next_since"] = json!(self.after);
        let hint = line["error"]["hint"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        line["error"]["hint"] = json!(format!(
            "{hint}; events streams lists readable names; resume the selected stream at --since {}",
            self.after
        ));
    }
}

pub(super) fn tail(
    client: &mut Client,
    op: Ulid,
    progress: &mut Progress,
    subject: Option<&str>,
    timeout: Duration,
    out: &mut dyn Write,
) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    budget(client, deadline, op)?;
    let streams = client
        .event_streams()
        .map_err(|e| read_error(e, deadline, op))?;
    if !streams.contains(&progress.stream) {
        return Err("unknown retained stream; events streams lists readable names".into());
    }
    let mut count = 0u64;
    loop {
        budget(client, deadline, op)?;
        let start = progress.after;
        let (page, next, through) = client
            .events_page(&progress.stream, start, PAGE)
            .map_err(|e| read_error(e, deadline, op))?;
        if through < start || next.is_some_and(|n| n != through || n <= start) {
            return Err("invalid retained event page cursor".into());
        }
        for event in page {
            if event.seq <= progress.after || event.seq > through {
                return Err("invalid retained event sequence".into());
            }
            if subject.is_none_or(|pattern| bbt_mux::bus::matches(pattern, &event.subject)) {
                writeln!(
                    out,
                    "{}",
                    json!({"ok":true,"op":op,"stream":progress.stream,
                    "seq":event.seq,"event":event})
                )
                .map_err(|e| e.to_string())?;
                count += 1;
            }
            progress.after = event.seq;
        }
        // A filtered final page still advances past every scanned row.
        progress.after = through;
        out.flush().map_err(|e| e.to_string())?;
        if next.is_none() {
            writeln!(
                out,
                "{}",
                json!({"ok":true,"op":op,"stream":progress.stream,
                "seq":progress.after,"next_since":progress.after,"done":true,
                "data":{"stream":progress.stream,"next_since":progress.after,"events":count}})
            )
            .map_err(|e| e.to_string())?;
            return out.flush().map_err(|e| e.to_string());
        }
    }
}

fn exhausted(op: Ulid) -> String {
    format!("{TIMEOUT_PREFIX}{op}\nevents tail reached its read budget; resume from next_since")
}
fn read_error(error: String, deadline: Instant, op: Ulid) -> String {
    if Instant::now() >= deadline {
        exhausted(op)
    } else {
        error
    }
}
fn budget(client: &Client, deadline: Instant, op: Ulid) -> Result<(), String> {
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .filter(|d| !d.is_zero())
        .ok_or_else(|| exhausted(op))?
        .min(TIMEOUT);
    client
        .reader
        .get_ref()
        .set_read_timeout(Some(remaining))
        .map_err(|e| e.to_string())?;
    client
        .writer
        .set_write_timeout(Some(remaining))
        .map_err(|e| e.to_string())
}
