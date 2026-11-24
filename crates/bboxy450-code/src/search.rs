// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use crate::{
    Budget, FILE_ROWS, Item, PARSE_BYTES, Root, clip, enclosing, items_of, parse, supported,
    with_cut,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Search {
    patterns: Vec<String>,
    #[serde(default)]
    paths: Vec<String>,
    glob: Option<String>,
    #[serde(default)]
    context: usize,
    #[serde(default)]
    before: usize,
    #[serde(default)]
    ignore_case: bool,
    #[serde(default)]
    all: bool,
    limit: Option<usize>,
    #[serde(default)]
    mode: Mode,
    #[serde(default)]
    fixed: bool,
}

#[derive(Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Mode {
    #[default]
    Lines,
    Files,
    Count,
}

#[derive(Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
enum Event {
    Begin {
        path: RgPath,
    },
    Match(Line),
    Context(Line),
    End {
        binary_offset: Option<usize>,
        stats: Stats,
    },
    Summary(serde::de::IgnoredAny),
}

#[derive(Deserialize)]
struct RgPath {
    text: Option<PathBuf>,
    bytes: Option<String>,
}

#[derive(Deserialize)]
struct Line {
    line_number: usize,
    lines: LineText,
}

#[derive(Deserialize)]
struct LineText {
    text: String,
}

#[derive(Deserialize)]
struct Stats {
    matched_lines: usize,
}

fn rg_binary() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("BBOXY_CODE_RG").map(PathBuf::from) {
        return path
            .is_absolute()
            .then_some(path)
            .ok_or_else(|| "BBOXY_CODE_RG must be an absolute path to ripgrep".into());
    }
    if let Ok(exe) = std::env::current_exe() {
        let sibling = exe.with_file_name(format!("rg{}", std::env::consts::EXE_SUFFIX));
        if sibling.is_file() {
            return Ok(sibling);
        }
    }
    let path = if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "/opt/homebrew/bin/rg"
    } else if cfg!(target_os = "macos") {
        "/usr/local/bin/rg"
    } else if cfg!(unix) {
        "/usr/bin/rg"
    } else {
        return Err("ripgrep is unavailable; set BBOXY_CODE_RG to its absolute path".into());
    };
    Ok(PathBuf::from(path))
}

fn rg(
    program: &Path,
    root: &Root,
    args: &Search,
    patterns: &[String],
    files: &[PathBuf],
    context: (usize, usize),
    mut event: impl FnMut(Event),
) -> Result<(), String> {
    let mut command = Command::new(program);
    command
        .current_dir(&root.0)
        .args([
            "--no-config",
            "--json",
            "--threads",
            "1",
            "--no-mmap",
            "--encoding",
            "utf-8",
            "--crlf",
        ])
        .arg("--before-context")
        .arg(context.0.to_string())
        .arg("--after-context")
        .arg(context.1.to_string());
    if args.fixed {
        command.arg("--fixed-strings");
    }
    if args.ignore_case {
        command.arg("--ignore-case");
    }
    for pattern in patterns {
        command.arg("-e").arg(pattern);
    }
    command.arg("--");
    if files.is_empty() {
        // Still let rg validate the patterns when no paths survived the walk.
        command.arg("-");
    } else {
        command.args(files);
    }
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("ripgrep at {} is unavailable: {e}", program.display()))?;
    let stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    std::thread::scope(|scope| {
        let errors = scope.spawn(move || {
            let mut head = Vec::new();
            (&mut stderr).take(800).read_to_end(&mut head)?;
            io::copy(&mut stderr, &mut io::sink())?;
            Ok::<_, io::Error>(head)
        });
        let result = (|| -> Result<(), String> {
            for line in BufReader::new(stdout).lines() {
                let line = line.map_err(|e| format!("read ripgrep output: {e}"))?;
                event(
                    serde_json::from_str(&line)
                        .map_err(|e| format!("invalid ripgrep JSON: {e}"))?,
                );
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = child.kill();
        }
        let status = child.wait().map_err(|e| format!("wait for ripgrep: {e}"))?;
        let errors = errors
            .join()
            .unwrap()
            .map_err(|e| format!("read ripgrep errors: {e}"))?;
        result?;
        match status.code() {
            Some(0 | 1) => Ok(()),
            _ => {
                let detail = String::from_utf8_lossy(&errors);
                let kind = if detail.contains("regex parse error") {
                    "invalid pattern"
                } else {
                    "ripgrep search failed"
                };
                Err(format!("{kind} ({status}): {}", detail.trim()))
            }
        }
    })
}

struct Preview {
    path: String,
    source: Option<PathBuf>,
    items: Option<Vec<Item>>,
    leading: Vec<(usize, String)>,
    text: String,
    budget: Budget,
    hits: usize,
    last_line: usize,
    through: usize,
    full: bool,
}

impl Preview {
    fn new(root: &Root, path: RgPath, budget: &Budget) -> Self {
        let name = match &path.text {
            Some(path) => root.show(path),
            None => format!("[path base64 {}]", path.bytes.unwrap_or_default()),
        };
        Self {
            path: name,
            source: path.text,
            items: None,
            leading: vec![],
            text: String::new(),
            budget: budget.clone(),
            hits: 0,
            last_line: 0,
            through: 0,
            full: false,
        }
    }

    fn push(&mut self, line: Line, matched: bool, slots: usize, context: (usize, usize)) {
        use std::fmt::Write as _;
        let (before, after) = context;
        let number = line.line_number;
        if self.full || (self.hits == slots && (matched || number > self.through)) {
            return;
        }
        let line = line
            .lines
            .text
            .strip_suffix("\n")
            .map_or(line.lines.text.as_str(), |line| {
                line.strip_suffix("\r").unwrap_or(line)
            });
        let path = self.path.replace('\r', "\\r").replace('\n', "\\n");
        let mut row = format!("{path}:{number}: {}", clip(line));
        if !matched && number > self.through {
            // Lead-in context belongs to the next visible hit. Keep it pending
            // until that hit fits, so a cut cannot leave a context-only block.
            self.leading
                .retain(|(n, _)| *n > number.saturating_sub(before));
            self.leading.push((number, row));
            return;
        }

        let mut addition = String::new();
        if matched {
            let items = self
                .items
                .get_or_insert_with(|| self.source.as_deref().map_or_else(Vec::new, parse_items));
            if let Some(item) = enclosing(items, number) {
                let _ = write!(row, " [{}]", clip(&item));
            }
            let start = number.saturating_sub(before);
            let mut leading = self
                .leading
                .drain(..)
                .filter(|(n, _)| *n >= start)
                .peekable();
            let first = leading.peek().map_or(number, |(n, _)| *n);
            if context != (0, 0) && self.last_line > 0 && first > self.last_line + 1 {
                addition.push_str("--\n");
            }
            for (_, row) in leading {
                let _ = writeln!(addition, "{row}");
            }
        }
        let _ = writeln!(addition, "{row}");
        if !self.budget.spend(&json!(addition)) {
            self.full = true;
            return;
        }
        self.text.push_str(&addition);
        self.last_line = number;
        if matched {
            self.through = number + after;
            self.hits += 1;
        }
    }
}

fn parse_items(path: &Path) -> Vec<Item> {
    if !supported(path) {
        return vec![];
    }
    let mut source = Vec::new();
    match std::fs::File::open(path)
        .and_then(|file| file.take(PARSE_BYTES as u64 + 1).read_to_end(&mut source))
    {
        Ok(size) if size <= PARSE_BYTES => items_of(path, &String::from_utf8_lossy(&source)),
        _ => vec![],
    }
}

/// Lines matching any pattern, with ripgrep's merged context and enclosing items.
pub fn search(root: &Root, args: &Value) -> Result<Value, String> {
    let args: Search = parse(args)?;
    if args.patterns.is_empty() {
        return Err("give at least one pattern".into());
    }
    if !args.fixed && args.patterns.iter().any(|pattern| pattern.contains("\\|")) {
        return Err("pattern uses the grep spelling \\|; alternation here is |, or pass each alternative as its own pattern".into());
    }

    let (files, missing) = root.walk(&args.paths, args.glob.as_deref(), None, args.all)?;
    let program = rg_binary()?;
    let mut budget = Budget::new();
    let pattern_rows = |counts: &[usize]| {
        args.patterns
            .iter()
            .zip(counts)
            .map(|(pattern, hits)| json!({"pattern": pattern, "hits": hits}))
            .collect::<Vec<_>>()
    };
    let mut counts = vec![usize::MAX; args.patterns.len()];
    if !budget.spend(&json!({"patterns": pattern_rows(&counts), "missing": missing})) {
        return Err("search patterns and missing paths exceed the result budget".into());
    }

    counts.fill(0);
    let limit = args.limit.unwrap_or(50).clamp(1, 500);
    let context = if args.mode == Mode::Lines {
        (args.before.min(20), args.context.min(20))
    } else {
        (0, 0)
    };
    let mut total = 0;
    let mut shown = 0;
    let mut dropped_files = 0;
    let mut preview = None;
    let mut full = false;
    let mut lines = String::new();
    let mut rows = Vec::new();

    // Bound each argv independently of the size of the selected tree.
    let mut groups = Vec::new();
    let mut start = 0;
    let mut bytes = 0;
    for (index, path) in files.iter().enumerate() {
        let cost = path.as_os_str().len() + size_of::<usize>() + 1;
        if bytes + cost > 32 * 1024 && index > start {
            groups.push(&files[start..index]);
            start = index;
            bytes = 0;
        }
        bytes += cost;
    }
    groups.push(&files[start..]);

    for files in groups {
        // A combined rg match counts the union. Each pattern needs its own pass
        // to count lines where alternative matches overlap or mask each other.
        if args.patterns.len() > 1 {
            for (pattern, count) in args.patterns.iter().zip(&mut counts) {
                rg(
                    &program,
                    root,
                    &args,
                    std::slice::from_ref(pattern),
                    files,
                    (0, 0),
                    |event| {
                        if let Event::End {
                            binary_offset: None,
                            stats,
                        } = event
                        {
                            *count += stats.matched_lines;
                        }
                    },
                )?;
            }
        }
        rg(
            &program,
            root,
            &args,
            &args.patterns,
            files,
            context,
            |event| match event {
                Event::Begin { path }
                    if args.mode == Mode::Files
                        || (args.mode == Mode::Lines && !full && shown < limit) =>
                {
                    preview = Some(Preview::new(root, path, &budget));
                }
                Event::Match(line) if args.mode == Mode::Lines => {
                    if let Some(preview) = &mut preview {
                        preview.push(line, true, limit - shown, context);
                    }
                }
                Event::Context(line) if args.mode == Mode::Lines => {
                    if let Some(preview) = &mut preview {
                        preview.push(line, false, limit - shown, context);
                    }
                }
                Event::End {
                    binary_offset,
                    stats,
                } => {
                    let file = preview.take();
                    if binary_offset.is_some() {
                        return;
                    }
                    total += stats.matched_lines;
                    if let Some(file) = file {
                        if args.mode == Mode::Files {
                            let row = json!({"path": file.path, "hits": stats.matched_lines});
                            if rows.len() < FILE_ROWS && budget.spend(&row) {
                                rows.push(row);
                            } else {
                                dropped_files += 1;
                            }
                        } else {
                            if !lines.is_empty() && !file.text.is_empty() && context != (0, 0) {
                                lines.push_str("--\n");
                            }
                            lines.push_str(&file.text);
                            shown += file.hits;
                            budget = file.budget;
                            full = file.full;
                        }
                    }
                }
                _ => {}
            },
        )?;
    }

    if counts.len() == 1 {
        counts[0] = total;
    }
    let mut value = json!({"total": total, "patterns": pattern_rows(&counts)});
    let cut = match args.mode {
        Mode::Lines => {
            value["text"] = json!(lines);
            total - shown
        }
        Mode::Files => {
            value["files"] = json!(rows);
            dropped_files
        }
        Mode::Count => 0,
    };
    if !missing.is_empty() {
        value["missing"] = json!(missing);
    }
    if full {
        value["preview_budget_exhausted"] = json!(true);
    }
    Ok(with_cut(
        value,
        cut,
        "more matched than fit; narrow paths or patterns, raise limit, or use mode files or count",
    ))
}

pub(super) fn mcp_text(value: &mut Value) -> String {
    if let Some(Value::String(mut text)) = value.as_object_mut().and_then(|obj| obj.remove("text"))
    {
        text.push_str(&value.to_string());
        text
    } else {
        value.to_string()
    }
}
