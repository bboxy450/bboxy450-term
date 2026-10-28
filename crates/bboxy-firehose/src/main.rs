// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use std::io::{self, Write};
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::{Duration, Instant};

use bboxy_firehose::options::{Options, Rate};
use bboxy_firehose::{COLS, ROWS, Rng, SCENES, expected_hash, ordered_scenes, scene_bytes};

const HELP: &str = "bboxy450-term-firehose — embedded synthetic terminal output
  --list                  List the embedded scenes
  --scene NAME            Play one scene; otherwise shuffle all scenes
  --seed N                Seed for scene order and real pacing (default 1)
  --rate RATE             max, bytes/s, 64KiB/s, 8MiB/s, or 30lines/s
  --pace flood|real       Flood (default), or seeded bursts and pauses
  --loop [COUNT]          Repeat forever, or COUNT times
  --check                 Print frozen 80x24 final-screen FNV-1a hashes; no ANSI
  --help                  Show this help

All content is compiled in. No input, files or terminal-size queries at runtime.
--check ignores playback rate, pacing and loop count. See README for the schema.";

static SIGNAL: AtomicI32 = AtomicI32::new(0);
const RESTORE: &[u8] = b"\x1b[?2026l\x1b[?1049l\x1b[0m\x1b[r\x1b]8;;\x1b\\\x1b[?25h";

#[cfg(unix)]
extern "C" fn stop(signal: libc::c_int) {
    SIGNAL.store(signal, Ordering::Relaxed);
}

fn main() {
    #[cfg(unix)]
    // The handler only stores a lock-free atomic. All terminal restoration is
    // performed on the playback thread, outside the signal handler.
    unsafe {
        libc::signal(libc::SIGINT, stop as *const () as libc::sighandler_t);
        libc::signal(libc::SIGTERM, stop as *const () as libc::sighandler_t);
    }
    let result = match Options::parse(std::env::args().skip(1)) {
        Ok(options) => run(options),
        Err(error) => {
            eprintln!("bboxy450-term-firehose: {error}");
            std::process::exit(2);
        }
    };
    if let Err(error) = result {
        // A downstream pipe may deliberately stop consuming an infinite stream.
        if error.kind() != io::ErrorKind::BrokenPipe {
            eprintln!("bboxy450-term-firehose: {error}");
            std::process::exit(1);
        }
    }
    let signal = SIGNAL.load(Ordering::Relaxed);
    if signal != 0 {
        std::process::exit(128 + signal);
    }
}

fn run(options: Options) -> io::Result<()> {
    let mut out = io::stdout().lock();
    if options.help {
        return writeln!(out, "{HELP}");
    }
    if options.list {
        for scene in SCENES {
            writeln!(out, "{}\t{}", scene.name, scene.description)?;
        }
        return Ok(());
    }
    let scenes = ordered_scenes(options.scene.as_deref(), options.seed);
    if options.check {
        for scene in scenes {
            writeln!(
                out,
                "{} seed={} {}x{} fnv1a64={}",
                scene.name,
                options.seed,
                COLS,
                ROWS,
                expected_hash(scene.name).expect("every scene has a frozen hash")
            )?;
        }
        return Ok(());
    }
    let bytes: Vec<_> = scenes.into_iter().flat_map(scene_bytes).collect();
    let result = if options.rate == Rate::Max && !options.real {
        flood(&mut out, &bytes, options.loops)
    } else {
        paced(&mut out, &bytes, &options)
    };
    if SIGNAL.load(Ordering::Relaxed) != 0 {
        out.write_all(RESTORE)?;
        out.flush()?;
    }
    result
}

fn flood(out: &mut impl Write, cycle: &[u8], mut remaining: Option<u64>) -> io::Result<()> {
    // Batch complete cycles once, so max-rate playback is bounded memory and a
    // single write per MiB rather than a write per line or corpus entry.
    let cycles = (1024 * 1024 / cycle.len()).max(1) as u64;
    let cycles = remaining.map_or(cycles, |left| cycles.min(left));
    let batch = cycle.repeat(cycles as usize);
    loop {
        if SIGNAL.load(Ordering::Relaxed) != 0 {
            break;
        }
        let count = remaining.map_or(cycles, |left| cycles.min(left));
        if count == 0 {
            break;
        }
        out.write_all(&batch[..cycle.len() * count as usize])?;
        if let Some(left) = &mut remaining {
            *left -= count;
        }
    }
    out.flush()
}

#[allow(
    clippy::disallowed_methods,
    reason = "standalone PTY fixture intentionally sleeps for rate limiting; no UI thread"
)]
fn paced(out: &mut impl Write, bytes: &[u8], options: &Options) -> io::Result<()> {
    let started = Instant::now();
    paced_with_clock(
        out,
        bytes,
        options,
        || started.elapsed(),
        std::thread::sleep,
        || SIGNAL.load(Ordering::Relaxed) != 0,
    )
}

fn paced_with_clock(
    out: &mut impl Write,
    bytes: &[u8],
    options: &Options,
    mut elapsed: impl FnMut() -> Duration,
    mut pause: impl FnMut(Duration),
    mut stopped: impl FnMut() -> bool,
) -> io::Result<()> {
    let mut rng = Rng(options.seed);
    let mut emitted = 0u128;
    let mut lines = 0u128;
    let mut real_time = Duration::ZERO;
    let mut loops = 0u64;
    loop {
        let mut offset = 0;
        while offset < bytes.len() {
            if stopped() {
                return Ok(());
            }
            let max_bytes = match options.rate {
                Rate::Bytes(rate) => (rate / 100).clamp(1, 64 * 1024) as usize,
                _ => 4096,
            };
            let mut end = (offset + max_bytes).min(bytes.len());
            if options.real || matches!(options.rate, Rate::Lines(_)) {
                let mut breaks = if options.real {
                    1 + rng.next_u64() % 5
                } else {
                    1
                };
                for (i, byte) in bytes[offset..end].iter().enumerate() {
                    if *byte == b'\n' || (options.real && *byte == b'\r') {
                        breaks -= 1;
                        if breaks == 0 {
                            end = offset + i + 1;
                            break;
                        }
                    }
                }
            }
            let chunk = &bytes[offset..end];
            out.write_all(chunk)?;
            out.flush()?;
            emitted += chunk.len() as u128;
            lines += chunk.iter().filter(|byte| **byte == b'\n').count() as u128;
            if options.real {
                let n = rng.next_u64();
                real_time += Duration::from_millis(if n.is_multiple_of(9) {
                    150 + n % 250
                } else {
                    12 + n % 40
                });
            }
            let deadline = options.rate.elapsed_for(emitted, lines).max(real_time);
            while let Some(delay) = deadline.checked_sub(elapsed()) {
                if stopped() {
                    return Ok(());
                }
                pause(delay.min(Duration::from_millis(20)));
            }
            offset = end;
        }
        loops += 1;
        if options.loops.is_some_and(|count| loops >= count) {
            break;
        }
    }
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn render(bytes: &[u8], options: &Options) -> (Vec<u8>, Duration) {
        let elapsed = Cell::new(Duration::ZERO);
        let mut output = Vec::new();
        paced_with_clock(
            &mut output,
            bytes,
            options,
            || elapsed.get(),
            |delay| {
                // A zero-duration yield still lets a real monotonic clock
                // progress; give the virtual clock that same progress.
                elapsed.set(elapsed.get() + delay.max(Duration::from_nanos(1)));
            },
            || false,
        )
        .unwrap();
        (output, elapsed.get())
    }

    // keeps: spec 2026-10-22-t-terminal-fidelity §T1
    #[test]
    fn byte_and_line_rate_caps_keep_the_exact_stream() {
        let expected = scene_bytes(&SCENES[9]);
        let lines = expected.iter().filter(|byte| **byte == b'\n').count();
        assert!(lines > 0);
        for rate in [
            format!("{}B/s", expected.len() * 4),
            format!("{}lines/s", lines * 4),
        ] {
            let options =
                Options::parse(["--scene", "osc7501-codex", "--rate", &rate].map(str::to_owned))
                    .unwrap();
            let (output, elapsed) = render(&expected, &options);
            assert_eq!(output, expected);
            assert!(elapsed >= Duration::from_millis(250));
        }
    }

    // keeps: spec 2026-10-22-t-terminal-fidelity §T1
    #[test]
    fn real_pacing_preserves_scene_bytes() {
        let expected = scene_bytes(&SCENES[5]);
        let options = Options::parse(
            ["--scene", "alternate", "--pace", "real", "--seed", "42"].map(str::to_owned),
        )
        .unwrap();
        let (output, _) = render(&expected, &options);
        assert_eq!(output, expected);
    }
}
