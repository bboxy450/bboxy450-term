// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Bounded, deterministic code reading and search.
//!
//! Eight queries over one workspace root, each a plain function from JSON-shaped
//! input to JSON output: [`read`], [`search`], [`symbol`], [`files`], [`outline`],
//! [`den_check`], [`den_statechart`], [`contracts`].
//! Each query bounds its output. Batch queries report a bad element
//! beside the good ones. Nothing here keeps state or writes.
//!
//! The shapes come from measured use: specs/code-query.md.
#![expect(
    clippy::disallowed_methods,
    reason = "A standalone query tool reads files and runs git synchronously; no UI thread exists here."
)]
use regex::Regex;
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::{Component, Path, PathBuf};

mod formal;
mod search;
mod syntax;
pub use formal::{contracts, den_check, den_statechart};
pub use search::search;
pub use syntax::{Item, enclosing, items, items_of, supported};

/// A whole result stays under this many bytes.
pub const RESULT_BYTES: usize = 16 * 1024;
/// Lines one `read` request returns at most.
pub const READ_LINES: usize = 200;
/// A line is clipped to this many characters.
pub const LINE_CHARS: usize = 240;
/// Paths a `files` call lists at most.
pub const FILE_ROWS: usize = 200;
/// A file larger than this is searched but not parsed for items.
const PARSE_BYTES: usize = 2 * 1024 * 1024;

/// The workspace every path is relative to.
pub struct Root(PathBuf);

impl Root {
    pub fn new(path: impl AsRef<Path>) -> std::io::Result<Self> {
        path.as_ref().canonicalize().map(Self)
    }
    /// A relative path under the root. Absolute paths under the root are accepted;
    /// anything that leaves it is refused.
    fn resolve(&self, path: &str) -> Result<PathBuf, String> {
        let given = Path::new(path);
        let relative = if given.is_absolute() {
            given
                .strip_prefix(&self.0)
                .map_err(|_| "path is outside the workspace".to_owned())?
        } else {
            given
        };
        let mut clean = PathBuf::new();
        for part in relative.components() {
            match part {
                Component::Normal(name) => clean.push(name),
                Component::CurDir => {}
                _ => return Err("path is outside the workspace".into()),
            }
        }
        Ok(self.0.join(clean))
    }
    fn show(&self, path: &Path) -> String {
        path.strip_prefix(&self.0)
            .unwrap_or(path)
            .display()
            .to_string()
    }
    /// Every text file under `paths` in path order, honouring `.gitignore`
    /// and skipping hidden files unless `all` (rg `--hidden --no-ignore`).
    /// Missing paths are returned, not fatal.
    fn walk(
        &self,
        paths: &[String],
        glob: Option<&str>,
        depth: Option<usize>,
        all: bool,
    ) -> Result<(Vec<PathBuf>, Vec<String>), String> {
        let mut missing = vec![];
        let mut roots = vec![];
        let whole = [".".to_owned()];
        for path in if paths.is_empty() { &whole[..] } else { paths } {
            match self.resolve(path) {
                Ok(full) if full.exists() => roots.push(full),
                _ => missing.push(path.clone()),
            }
        }
        let mut found = vec![];
        let Some((first, rest)) = roots.split_first() else {
            return Ok((found, missing));
        };
        let mut builder = ignore::WalkBuilder::new(first);
        for root in rest {
            builder.add(root);
        }
        builder.max_depth(depth).sort_by_file_path(|a, b| a.cmp(b));
        if all {
            // Hidden and ignored files, never Git's own object store.
            builder
                .standard_filters(false)
                .filter_entry(|entry| entry.file_name() != ".git");
        }
        // The glob only narrows: given to the walker it would outrank .gitignore.
        let glob = match glob {
            Some(glob) => {
                let mut matcher = ignore::overrides::OverrideBuilder::new(&self.0);
                matcher
                    .add(glob)
                    .map_err(|e| format!("invalid glob: {e}"))?;
                Some(matcher.build().map_err(|e| format!("invalid glob: {e}"))?)
            }
            None => None,
        };
        for entry in builder.build().flatten() {
            if entry.file_type().is_some_and(|kind| kind.is_file())
                && glob
                    .as_ref()
                    .is_none_or(|glob| !glob.matched(entry.path(), false).is_ignore())
            {
                found.push(entry.into_path());
            }
        }
        found.sort();
        found.dedup();
        Ok((found, missing))
    }
}

/// Text of a file, or `None` for a binary or unreadable one.
fn text(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    if bytes[..bytes.len().min(8192)].contains(&0) {
        return None;
    }
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

fn clip(line: &str) -> String {
    match line.char_indices().nth(LINE_CHARS) {
        Some((end, _)) => format!("{}…", &line[..end]),
        None => line.to_owned(),
    }
}

/// Bytes a result may still spend. A producer that cannot afford its next
/// element stops and says so.
#[derive(Clone)]
struct Budget(usize);
impl Budget {
    fn new() -> Self {
        // Leave room for the envelope and the cut notice.
        Self(RESULT_BYTES - 1024)
    }
    fn spend(&mut self, value: &Value) -> bool {
        let cost = value.to_string().len() + 1;
        if cost > self.0 {
            return false;
        }
        self.0 -= cost;
        true
    }
}

fn parse<T: for<'a> Deserialize<'a>>(args: &Value) -> Result<T, String> {
    serde_json::from_value(args.clone()).map_err(|e| format!("invalid arguments: {e}"))
}

// ---------- read ----------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Read {
    requests: Vec<ReadOne>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadOne {
    path: String,
    #[serde(default)]
    start: Option<usize>,
    #[serde(default)]
    end: Option<usize>,
    /// Return the whole item that contains this line.
    #[serde(default)]
    around: Option<usize>,
    /// Read the file as of this git revision.
    #[serde(default)]
    rev: Option<String>,
}

/// Numbered lines of files: a range, the item around a line, or the start of the file.
pub fn read(root: &Root, args: &Value) -> Result<Value, String> {
    let args: Read = parse(args)?;
    let mut budget = Budget::new();
    let mut out = vec![];
    let mut cut = 0;
    for request in &args.requests {
        let one = read_one(root, request, budget.0)
            .unwrap_or_else(|error| json!({"path": request.path, "error": error}));
        if budget.spend(&one) {
            out.push(one);
        } else {
            cut += 1;
        }
    }
    Ok(with_cut(
        json!({"files": out}),
        cut,
        "requests did not fit; ask for them again in a smaller batch",
    ))
}

fn read_one(root: &Root, request: &ReadOne, room: usize) -> Result<Value, String> {
    let full = root.resolve(&request.path)?;
    let body = match &request.rev {
        Some(rev) => at_revision(root, rev, &root.show(&full))?,
        None => text(&full).ok_or_else(|| {
            if full.exists() {
                "not a text file".to_owned()
            } else {
                "no such file".to_owned()
            }
        })?,
    };
    let lines: Vec<&str> = body.lines().collect();
    let total = lines.len();
    let (mut start, mut end) = (
        request.start.unwrap_or(1).max(1),
        request.end.unwrap_or(usize::MAX),
    );
    let mut item = None;
    if let Some(line) = request.around {
        let found = items_of(&full, &body);
        match found
            .iter()
            .filter(|i| i.start <= line && line <= i.end)
            .min_by_key(|i| i.end - i.start)
        {
            Some(inner) => {
                (start, end) = (inner.start, inner.end);
                item = Some(inner.label(&found));
            }
            // No syntax tree: the same window a reader takes by hand.
            None => (start, end) = (line.saturating_sub(10).max(1), line + 40),
        }
    }
    let wanted = end.min(total);
    let end = wanted.min(start.saturating_add(READ_LINES - 1));
    let mut text = String::new();
    let mut last = start.saturating_sub(1);
    // The file's lines must fit what is left of the whole result.
    let room = room.saturating_sub(512);
    for number in start..=end {
        let line = format!("{number}\t{}\n", clip(lines[number - 1]));
        if text.len() + line.len() > room {
            break;
        }
        text.push_str(&line);
        last = number;
    }
    let mut value = json!({"path": root.show(&full), "lines": total, "start": start, "end": last, "text": text});
    if let Some(item) = item {
        value["item"] = json!(item);
    }
    if let Some(rev) = &request.rev {
        value["rev"] = json!(rev);
    }
    if last < wanted {
        value["next_start"] = json!(last + 1);
    }
    Ok(value)
}

fn at_revision(root: &Root, rev: &str, path: &str) -> Result<String, String> {
    if rev.starts_with('-') || rev.contains(':') || rev.chars().any(char::is_whitespace) {
        return Err("invalid revision".into());
    }
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(&root.0)
        .args(["show", &format!("{rev}:{path}")])
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| format!("git is unavailable: {e}"))?;
    if !output.status.success() {
        return Err("no such file at that revision".into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

// ---------- symbol ----------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Symbol {
    names: Vec<String>,
    /// fn, struct, enum, variant, trait, impl, mod, const, static, type, macro, union
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    paths: Vec<String>,
    /// Also list uses of each name.
    #[serde(default)]
    refs: bool,
}

/// Where names are defined in Rust sources, and optionally where they are used.
pub fn symbol(root: &Root, args: &Value) -> Result<Value, String> {
    let args: Symbol = parse(args)?;
    if args.names.is_empty() {
        return Err("give at least one name".into());
    }
    let words: Vec<Regex> = args
        .names
        .iter()
        .map(|name| Regex::new(&format!(r"\b{}\b", regex::escape(name))).map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    let (mut found, missing) = root.walk(&args.paths, None, None, false)?;
    found.retain(|path| supported(path));
    let mut budget = Budget::new();
    let mut symbols: Vec<Value> = args
        .names
        .iter()
        .map(|name| json!({"name": name, "definitions": [], "uses": [], "use_count": 0}))
        .collect();
    let mut cut = 0;
    // Definitions first, so a name with many uses still shows where it is defined.
    let mut sources = vec![];
    let mut skipped = vec![];
    for path in &found {
        let Some(body) = text(path) else { continue };
        if !args.names.iter().any(|name| body.contains(name.as_str())) {
            continue;
        }
        if body.len() > PARSE_BYTES {
            // Never silently: the name is in there, this tool just did not parse it.
            skipped
                .push(json!({"path": root.show(path), "reason": "too large to parse; use search"}));
            continue;
        }
        let found = items_of(path, &body);
        let mut defined = vec![vec![]; args.names.len()];
        for (index, name) in args.names.iter().enumerate() {
            for item in found.iter().filter(|i| i.named(name)) {
                if args.kind.as_deref().is_some_and(|kind| kind != item.kind) {
                    continue;
                }
                defined[index].push(item.start);
                let row = json!({"path": root.show(path), "start": item.start, "end": item.end,
                    "kind": item.kind, "signature": clip(&item.signature), "in": item.parent(&found)});
                if budget.spend(&row) {
                    symbols[index]["definitions"]
                        .as_array_mut()
                        .unwrap()
                        .push(row);
                } else {
                    cut += 1;
                }
            }
        }
        if args.refs {
            sources.push((path, body, found, defined));
        }
    }
    for (path, body, found, defined) in &sources {
        for (index, word) in words.iter().enumerate() {
            for (number, line) in body.lines().enumerate().map(|(i, l)| (i + 1, l)) {
                if defined[index].contains(&number) || !word.is_match(line) {
                    continue;
                }
                symbols[index]["use_count"] =
                    json!(symbols[index]["use_count"].as_u64().unwrap() + 1);
                let uses = symbols[index]["uses"].as_array().unwrap().len();
                let mut row =
                    json!({"path": root.show(path), "line": number, "text": clip(line.trim())});
                if let Some(label) = enclosing(found, number) {
                    row["item"] = json!(label);
                }
                // Each name gets a fair share of what is left.
                if uses < 50 / args.names.len().max(1) + 1 && budget.spend(&row) {
                    symbols[index]["uses"].as_array_mut().unwrap().push(row);
                } else {
                    cut += 1;
                }
            }
        }
    }
    if !args.refs {
        for symbol in &mut symbols {
            let symbol = symbol.as_object_mut().unwrap();
            symbol.remove("uses");
            symbol.remove("use_count");
        }
    }
    let mut value = json!({"symbols": symbols});
    if !skipped.is_empty() {
        value["skipped"] = json!(skipped);
    }
    if !missing.is_empty() {
        value["missing"] = json!(missing);
    }
    Ok(with_cut(
        value,
        cut,
        "more found than fit; narrow paths or ask for fewer names",
    ))
}

// ---------- files ----------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Files {
    #[serde(default)]
    paths: Vec<String>,
    #[serde(default)]
    glob: Option<String>,
    #[serde(default)]
    depth: Option<usize>,
    /// Also list hidden and gitignored files.
    #[serde(default)]
    all: bool,
}

/// Files under paths or matching a glob, with line counts.
pub fn files(root: &Root, args: &Value) -> Result<Value, String> {
    let args: Files = parse(args)?;
    let (found, missing) = root.walk(&args.paths, args.glob.as_deref(), args.depth, args.all)?;
    let mut budget = Budget::new();
    let (mut rows, mut lines_total, mut cut) = (vec![], 0usize, 0usize);
    for path in &found {
        let lines = text(path).map(|body| body.lines().count());
        lines_total += lines.unwrap_or(0);
        let row = json!({"path": root.show(path), "lines": lines});
        if rows.len() < FILE_ROWS && budget.spend(&row) {
            rows.push(row);
        } else {
            cut += 1;
        }
    }
    let mut value = json!({"total": found.len(), "lines": lines_total, "files": rows});
    if !missing.is_empty() {
        value["missing"] = json!(missing);
    }
    Ok(with_cut(
        value,
        cut,
        "more files than fit; narrow paths, add a glob or lower depth",
    ))
}

// ---------- outline ----------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Outline {
    paths: Vec<String>,
}

/// The items of Rust files: kind, name, signature and line range.
pub fn outline(root: &Root, args: &Value) -> Result<Value, String> {
    let args: Outline = parse(args)?;
    let mut budget = Budget::new();
    let (mut out, mut cut) = (vec![], 0usize);
    for given in &args.paths {
        let one = (|| {
            let full = root.resolve(given)?;
            let body = text(&full).ok_or("no such text file")?;
            if !supported(&full) {
                return Err(
                    "no outline for this file type (Rust, Den, Lean, Python, TypeScript, Go); use read or search"
                        .to_owned(),
                );
            }
            let found = items_of(&full, &body);
            let depth = |item: &Item| found.iter().filter(|o| o.contains(item)).count();
            let mut rows = vec![];
            let mut dropped = 0;
            for item in &found {
                let row = json!({"kind": item.kind, "name": item.name, "start": item.start, "end": item.end,
                    "depth": depth(item), "signature": clip(&item.signature)});
                if budget.spend(&row) {
                    rows.push(row);
                } else {
                    dropped += 1;
                }
            }
            let mut value = json!({"path": root.show(&full), "lines": body.lines().count(), "items": rows});
            if dropped > 0 {
                value["items_cut"] = json!(dropped);
            }
            Ok(value)
        })()
        .unwrap_or_else(|error: String| json!({"path": given, "error": error}));
        if one.get("items_cut").is_some() {
            cut += 1;
        }
        out.push(one);
    }
    Ok(with_cut(
        json!({"files": out}),
        cut,
        "items did not fit; ask for fewer files, or search within one",
    ))
}

fn with_cut(mut value: Value, cut: usize, how: &str) -> Value {
    if cut > 0 {
        value["cut"] = json!({"omitted": cut, "continue": how});
    }
    value
}

/// The tools as an MCP server lists them: name, description, input schema.
pub fn tools() -> Value {
    let mut tools = definitions();
    for tool in tools.as_array_mut().into_iter().flatten() {
        tool["inputSchema"]["properties"]["intent"] = json!({"type": "string",
            "description": "Say in a few words why you are making this call. Recorded for analysis; never changes the result."});
    }
    tools
}

fn definitions() -> Value {
    let path = json!({"type": "string", "description": "Path relative to the workspace root"});
    let paths = json!({"type": "array", "items": {"type": "string"}, "description": "Files or directories relative to the workspace root; default the whole workspace. Missing ones are reported, not fatal"});
    json!([
        {"name": "read", "description": "Read numbered lines of one or more files: a start..end range, the whole item around a line (around), or the first 200 lines. rev reads the file at a git revision. Use after search instead of sed -n.",
         "inputSchema": {"type": "object", "additionalProperties": false, "required": ["requests"], "properties": {"requests": {"type": "array", "minItems": 1, "items": {"type": "object", "additionalProperties": false, "required": ["path"], "properties": {
            "path": path, "start": {"type": "integer", "minimum": 1}, "end": {"type": "integer", "minimum": 1},
            "around": {"type": "integer", "minimum": 1, "description": "Return the whole function, impl or type containing this line"},
            "rev": {"type": "string", "description": "Git revision, e.g. HEAD~1 or origin/main"}}}}}}},
        {"name": "search", "description": "Search text with several regular expressions at once (Rust regex syntax; alternation is |). Returns compact path:line text with merged context and enclosing items. Bounded: default 50 hits, with totals per pattern. With ast: true the patterns are code shapes matched on the syntax tree by ast-grep, never inside comments or strings: `$X` is one node, `$$$ARGS` any run of nodes, e.g. `Command::new($P)` or `fn $F($$$A) -> Result<$T> { $$$B }`; a hit is the line a match starts on. Use instead of rg, grep, ast-grep and git grep; from python_execute: tools.bboxy_code.search(patterns=[...]).",
         "inputSchema": {"type": "object", "additionalProperties": false, "required": ["patterns"], "properties": {
            "patterns": {"type": "array", "minItems": 1, "items": {"type": "string"}, "description": "A line matching any pattern is a hit"},
            "paths": paths, "glob": {"type": "string", "description": "Only files matching this glob, e.g. *.rs or !tests/**"},
            "context": {"type": "integer", "minimum": 0, "maximum": 20, "description": "Lines to include after each hit (rg -A)"},
            "before": {"type": "integer", "minimum": 0, "maximum": 20, "description": "Lines to include before each hit (rg -B)"},
            "ignore_case": {"type": "boolean", "description": "Match regardless of case (rg -i)"},
            "all": {"type": "boolean", "description": "Also search hidden and gitignored files (rg --hidden --no-ignore)"},
            "limit": {"type": "integer", "minimum": 1, "maximum": 500},
            "mode": {"enum": ["lines", "files", "count"], "description": "lines: compact text; files: hit counts per file; count: totals only"},
            "fixed": {"type": "boolean", "description": "Take patterns literally"},
            "ast": {"type": "boolean", "description": "Take patterns as ast-grep code patterns (structural search); the language follows each file's extension. Not with fixed or ignore_case"}}}},
        {"name": "symbol", "description": "Find where names are defined, with signature and line range: Rust items in full, Den declarations (spec, event, invariant, theorem...), and Python, TypeScript, Go and Lean by syntax. refs also lists uses with their containing items. Use instead of grepping for 'fn name'.",
         "inputSchema": {"type": "object", "additionalProperties": false, "required": ["names"], "properties": {
            "names": {"type": "array", "minItems": 1, "items": {"type": "string"}}, "kind": {"type": "string"}, "paths": paths,
            "refs": {"type": "boolean", "description": "Also list uses of each name (at most 50 each, with the full count)"}}}},
        {"name": "files", "description": "List files under paths or matching a glob, with line counts. Honours .gitignore. Use instead of ls, find, rg --files and wc -l.",
         "inputSchema": {"type": "object", "additionalProperties": false, "properties": {
            "paths": paths, "glob": {"type": "string"}, "depth": {"type": "integer", "minimum": 1},
            "all": {"type": "boolean", "description": "Also list hidden and gitignored files (rg --files --hidden --no-ignore)"}}}},
        {"name": "outline", "description": "List a file's items in order: kind, name, signature, line range and nesting depth. Rust, Den, Lean, Python, TypeScript and Go. Use before reading a large file.",
         "inputSchema": {"type": "object", "additionalProperties": false, "required": ["paths"], "properties": {
            "paths": {"type": "array", "minItems": 1, "items": {"type": "string"}}}}},
        {"name": "den_check", "description": "Check Den specs: for each *.den file (directories are walked) whether it is sound, its records with reachable states, transitions and theorems, and every diagnostic with its repair. Read-only. Use after editing a .den file, or to see what the specs under a directory establish.",
         "inputSchema": {"type": "object", "additionalProperties": false, "required": ["paths"], "properties": {
            "paths": {"type": "array", "minItems": 1, "items": {"type": "string"}, "description": "*.den files or directories relative to the workspace root. Missing ones are reported, not fatal"}}}},
        {"name": "den_statechart", "description": "Draw a Den machine spec as a Mermaid statechart: every reachable state, every step that changes one labelled with its event and case, and the states where an invariant is false marked bad. Works on a spec whose invariants fail, so a counterexample can be seen. Read-only. Use to review a spec's behavior, or to show it to the operator.",
         "inputSchema": {"type": "object", "additionalProperties": false, "required": ["path"], "properties": {
            "path": {"type": "string", "description": "One *.den machine spec relative to the workspace root"},
            "max_states": {"type": "integer", "minimum": 1, "maximum": 64, "description": "Draw only when the spec has at most this many reachable states (default 32)"}}}},
        {"name": "contracts", "description": "What specifies, models or proves Rust code, from verification/*.toml and the tree's Den specs. For a path: the contracts whose sources touch it (property, models with expected verdicts, broken controls, tests), its crate's coverage row, or uncovered: true when nothing specifies it. For a name (contract, model or Den spec): the sources and tests it speaks for. Use before changing stateful Rust code, and to find the code behind a model.",
         "inputSchema": {"type": "object", "additionalProperties": false, "anyOf": [{"required": ["paths"]}, {"required": ["names"]}], "properties": {
            "paths": {"type": "array", "items": {"type": "string"}, "description": "Rust source files or directories relative to the workspace root"},
            "names": {"type": "array", "items": {"type": "string"}, "description": "Contract, model or Den spec names"},
            "detail": {"type": "boolean", "description": "Also return each contract's assumptions and sources (long)"}}}}
    ])
}

/// Run one tool by name. Every tool accepts `intent`, the caller's stated
/// reason for the call: it is for whoever studies the calls, and is ignored here.
pub fn call(root: &Root, tool: &str, args: &Value) -> Result<Value, String> {
    let mut args = args.clone();
    if let Some(fields) = args.as_object_mut() {
        fields.remove("intent");
    }
    let args = &args;
    match tool {
        "read" => read(root, args),
        "search" => search(root, args),
        "symbol" => symbol(root, args),
        "files" => files(root, args),
        "outline" => outline(root, args),
        "den_check" => den_check(root, args),
        "den_statechart" => den_statechart(root, args),
        "contracts" => contracts(root, args),
        other => Err(format!("unknown tool {other:?}")),
    }
}

const INSTRUCTIONS: &str = "Bounded, deterministic code reading and search for this workspace. \
search finds lines (several patterns at once, each hit names its containing item); read returns \
numbered lines or the whole item around a line; symbol finds Rust definitions and uses; files lists \
files with line counts; outline lists a file's items; den_check gives Den's verdict on .den files; den_statechart draws one spec's reachable states; \
contracts names what specifies, models or proves a Rust path. Every tool bounds its own \
output, so no head, cut or sed is needed. Paths are relative to the workspace root.";

/// Serve the queries as an MCP server: one JSON-RPC message per line, until the
/// input ends.
pub fn serve(root: &Root, input: impl std::io::BufRead, mut output: impl std::io::Write) {
    for line in input.lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(message) => answer(root, &message),
            Err(error) => Some(json!({"jsonrpc": "2.0", "id": null,
                "error": {"code": -32700, "message": format!("parse error: {error}")}})),
        };
        if let Some(reply) = reply
            && (writeln!(output, "{reply}").is_err() || output.flush().is_err())
        {
            break;
        }
    }
}

/// The reply to one message; a notification has none.
fn answer(root: &Root, message: &Value) -> Option<Value> {
    let id = message.get("id")?.clone();
    let result = match message["method"].as_str().unwrap_or_default() {
        "initialize" => Ok(json!({
            "protocolVersion": message["params"]["protocolVersion"].as_str().unwrap_or("2025-06-18"),
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "bboxy450-code", "version": env!("CARGO_PKG_VERSION")},
            "instructions": INSTRUCTIONS,
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools": tools()})),
        "tools/call" => {
            let name = message["params"]["name"].as_str().unwrap_or_default();
            let empty = json!({});
            let args = message["params"].get("arguments").unwrap_or(&empty);
            Ok(match call(root, name, args) {
                Ok(mut value) => {
                    let text = if name == "search" {
                        search::mcp_text(&mut value)
                    } else {
                        value.to_string()
                    };
                    json!({"content": [{"type": "text", "text": text}], "isError": false})
                }
                Err(error) => {
                    json!({"content": [{"type": "text", "text": error}], "isError": true})
                }
            })
        }
        other => {
            Err(json!({"code": -32601, "message": format!("method {other:?} is not supported")}))
        }
    };
    Some(match result {
        Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
        Err(error) => json!({"jsonrpc": "2.0", "id": id, "error": error}),
    })
}
