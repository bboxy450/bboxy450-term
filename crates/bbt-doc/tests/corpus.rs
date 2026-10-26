#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]
use std::{collections::BTreeMap, fs, io, path::Path};

fn record_sources(root: &Path) -> io::Result<Vec<(String, String)>> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, String)>) -> io::Result<()> {
        let mut entries = fs::read_dir(dir)?.collect::<io::Result<Vec<_>>>()?;
        entries.sort_by_key(|entry| entry.path());
        for entry in entries {
            let path = entry.path();
            let kind = entry.file_type()?;
            if kind.is_dir() {
                walk(root, &path, out)?;
            } else if kind.is_file() && path.extension().is_some_and(|ext| ext == "md") {
                out.push((
                    path.strip_prefix(root).unwrap().to_string_lossy().into(),
                    fs::read_to_string(&path)?,
                ));
            }
        }
        Ok(())
    }
    let mut sources = vec![];
    walk(root, &root.join(".bboxy"), &mut sources)?;
    Ok(sources)
}

// Read the real files rather than a baked fixture, including new untracked
// Markdown. Symlinked directories are not followed.
fn frontmatter_errors(root: &Path) -> io::Result<(usize, Vec<String>)> {
    let sources = record_sources(root)?;
    let mut errors = vec![];
    let mut numbers = BTreeMap::new();
    for (path, source) in &sources {
        let doc = bbt_doc::parse(source);
        if let Some(error) = &doc.frontmatter.error {
            errors.push(format!(
                "{path}:{}: invalid YAML frontmatter: {error}",
                doc.frontmatter.error_line.unwrap_or(1)
            ));
            continue;
        }
        if source.trim_start_matches('\u{feff}').lines().next() == Some("---") {
            for key in ["id", "type"] {
                if !doc.frontmatter.fields.contains_key(key) {
                    errors.push(format!("{path}:1: frontmatter missing `{key}:`"));
                }
            }
        }
        let line = source
            .lines()
            .position(|line| line.starts_with("number:"))
            .map_or(1, |n| n + 1);
        match doc.frontmatter.number() {
            None if !doc.frontmatter.fields.contains_key("number") => {
                errors.push(format!("{path}:1: frontmatter missing `number:`"))
            }
            None => errors.push(format!(
                "{path}:{line}: frontmatter `number:` must be a positive integer"
            )),
            Some(number) => {
                match numbers.entry(number) {
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        entry.insert(format!("{path}:{line}"));
                    }
                    std::collections::btree_map::Entry::Occupied(entry) => errors.push(format!(
                        "{path}:{line}: duplicate record number {number} (also {})",
                        entry.get()
                    )),
                }
                if doc.frontmatter.string("type").as_deref() == Some("decision") {
                    let stem = Path::new(path).file_stem().unwrap().to_string_lossy();
                    if !stem.starts_with(&format!("d{number}-")) {
                        errors.push(format!(
                            "{path}:{line}: decision filename must begin d{number}-"
                        ));
                    }
                }
            }
        }
    }
    Ok((sources.len(), errors))
}

fn corpus_root() -> std::path::PathBuf {
    std::env::var_os("BBOXY_DOC_CORPUS_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

// keeps: invariant frontmatter-parses
#[test]
fn real_bboxy_frontmatter_parses() {
    // A subprocess fixture can check actual mux-authored files without writing
    // into this repository. Ordinary corpus gates retain the real root.
    let root = corpus_root();
    let (count, errors) = frontmatter_errors(&root).unwrap();
    assert!(count > 0, "the real .bboxy corpus must not be empty");
    assert!(errors.is_empty(), "{}", errors.join("\n"));
    println!("frontmatter: {count} Markdown records, positive unique numbers, 0 errors");
}

// keeps: invariant frontmatter-parses
#[test]
fn corpus_gate_rejects_nested_malformed_frontmatter_with_path_and_source_line() {
    struct Temporary(std::path::PathBuf);
    impl Drop for Temporary {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root =
        Temporary(std::env::temp_dir().join(format!("bbt-doc-{}-{unique}", std::process::id())));
    let specs = root.0.join(".bboxy/specs");
    fs::create_dir_all(&specs).unwrap();
    fs::write(
        specs.join("broken.md"),
        "\u{feff}---\r\nid: fox\r\ntype: spec\r\ntitle: live seats: cockpit\r\nnumber: 1\r\n---\r\nKept body.",
    )
    .unwrap();
    fs::write(specs.join("open.md"), "---\ntitle: never closed").unwrap();
    fs::write(specs.join("plain.md"), "# A record also needs a number\n").unwrap();
    fs::write(
        specs.join("missing-id.md"),
        "---\ntype: spec\nnumber: 2\n---\nBody\n",
    )
    .unwrap();
    fs::write(
        specs.join("missing-type.md"),
        "---\nid: fox\nnumber: 3\n---\nBody\n",
    )
    .unwrap();
    let (count, errors) = frontmatter_errors(&root.0).unwrap();
    assert_eq!(count, 5);
    assert_eq!(errors.len(), 5);
    for expected in [
        ".bboxy/specs/broken.md:4: invalid YAML frontmatter:",
        ".bboxy/specs/open.md:1: invalid YAML frontmatter:",
        ".bboxy/specs/missing-id.md:1: frontmatter missing `id:`",
        ".bboxy/specs/missing-type.md:1: frontmatter missing `type:`",
        ".bboxy/specs/plain.md:1: frontmatter missing `number:`",
    ] {
        assert!(errors.iter().any(|e| e.starts_with(expected)), "{errors:?}");
    }
    fs::write(
        specs.join("broken.md"),
        "---\nid: fox\ntype: spec\nnumber: 1\ntitle: 'live seats: cockpit'\n---\nBody.",
    )
    .unwrap();
    for (index, name) in ["open.md", "missing-id.md", "missing-type.md", "plain.md"]
        .iter()
        .enumerate()
    {
        fs::write(
            specs.join(name),
            format!(
                "---\nid: fox\ntype: spec\nnumber: {}\n---\nBody\n",
                index + 2
            ),
        )
        .unwrap();
    }
    assert!(frontmatter_errors(&root.0).unwrap().1.is_empty());
}

struct Scratch(std::path::PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        loop {
            let next = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("bbt-records-{}-{next}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => {
                    fs::create_dir(path.join(".bboxy")).unwrap();
                    return Self(path);
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("create scratch corpus: {error}"),
            }
        }
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

// keeps: decision #48 — unique positive record handles and allocation
#[test]
fn corpus_gate_rejects_missing_duplicate_and_non_integer_numbers() {
    let root = Scratch::new();
    for (name, field) in [
        ("missing", ""),
        ("zero", "number: 0\n"),
        ("negative", "number: -1\n"),
        ("quoted", "number: '47'\n"),
        ("fraction", "number: 1.5\n"),
        ("first", "number: 47\n"),
        ("second", "number: 47\n"),
    ] {
        fs::write(
            root.0.join(".bboxy").join(format!("{name}.md")),
            format!("---\nid: {name}\ntype: note\n{field}---\nBody\n"),
        )
        .unwrap();
    }
    let (count, errors) = frontmatter_errors(&root.0).unwrap();
    assert_eq!(count, 7);
    assert_eq!(errors.len(), 6, "{errors:?}");
    for name in [
        "missing", "zero", "negative", "quoted", "fraction", "second",
    ] {
        assert!(
            errors
                .iter()
                .any(|error| error.starts_with(&format!(".bboxy/{name}.md:"))),
            "{errors:?}"
        );
    }
    assert!(errors.iter().any(|error|error.contains("duplicate record number 47") && error.contains("first.md")),"{errors:?}");
}

fn link_targets(doc: &bbt_doc::Markdown) -> Vec<String> {
    use bbt_doc::{Block, BlockKind, Inline};
    fn inlines(values: &[Inline], out: &mut Vec<String>) {
        for value in values {
            match value {
                Inline::WikiLink { target, .. } => out.push(target.clone()),
                Inline::Emphasis(values)
                | Inline::Strong(values)
                | Inline::Strike(values)
                | Inline::Link {
                    content: values, ..
                } => inlines(values, out),
                _ => (),
            }
        }
    }
    fn blocks(values: &[Block], out: &mut Vec<String>) {
        for block in values {
            match &block.kind {
                BlockKind::Paragraph(values)
                | BlockKind::Heading {
                    content: values, ..
                } => inlines(values, out),
                BlockKind::Quote(values) | BlockKind::Footnote { blocks: values, .. } => {
                    blocks(values, out)
                }
                BlockKind::List { items, .. } => {
                    for values in items {
                        blocks(values, out);
                    }
                }
                BlockKind::Table { header, rows, .. } => {
                    for values in header.iter().chain(rows.iter().flatten()) {
                        inlines(values, out);
                    }
                }
                _ => (),
            }
        }
    }
    fn metadata(value: &serde_yaml::Value, out: &mut Vec<String>) {
        match value {
            serde_yaml::Value::String(value) => {
                let mut rest = value.as_str();
                while let Some(start) = rest.find("[[") {
                    rest = &rest[start + 2..];
                    let Some(end) = rest.find("]]") else { break };
                    out.push(rest[..end].into());
                    rest = &rest[end + 2..];
                }
            }
            serde_yaml::Value::Sequence(values) => {
                for value in values {
                    metadata(value, out);
                }
            }
            serde_yaml::Value::Mapping(values) => {
                for value in values.values() {
                    metadata(value, out);
                }
            }
            _ => (),
        }
    }
    let mut out = vec![];
    blocks(&doc.blocks, &mut out);
    for value in doc.frontmatter.fields.values() {
        metadata(value, &mut out);
    }
    out
}

// keeps: decision #48 — unique positive record handles and allocation
#[test]
fn real_corpus_links_numeric_handles_and_legacy_ledger_anchors_resolve() {
    use bbt_doc::{
        BlockKind, Inline,
        links::{Record, resolve},
    };
    let sources = record_sources(&corpus_root()).unwrap();
    let docs: Vec<_> = sources
        .iter()
        .map(|(_, source)| bbt_doc::parse(source))
        .collect();
    let records: Vec<_> = sources
        .iter()
        .zip(&docs)
        .map(|((path, _), markdown)| Record { path, markdown })
        .collect();
    let mut count = 0;
    let mut errors = vec![];
    for record in &records {
        for target in link_targets(record.markdown) {
            let resolved = match resolve(records.iter().copied(), &target) {
                Ok(resolved) => resolved,
                Err(error) => {
                    errors.push(format!("{}: [[{target}]]: {error}", record.path));
                    continue;
                }
            };
            if resolved
                .record
                .path
                .ends_with("/2026-10-22-architecture-round-1.md")
                && let Some(anchor) = resolved.anchor.as_deref()
                && anchor.chars().all(|c| c.is_ascii_digit())
            {
                assert!(resolved.record.markdown.blocks.iter().any(|block|matches!(&block.kind,BlockKind::Heading{content,..} if Inline::plain_text(content)==anchor)),"ledger anchor #{anchor} missing");
            }
            count += 1;
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
    let resolved = resolve(records.iter().copied(), "#47").unwrap();
    assert!(
        resolved
            .record
            .path
            .ends_with("/decisions/d47-decision-nodes.md")
    );
    let view = resolved.record.markdown.document(resolved.record.path);
    assert_eq!(view.number, Some(47));
    assert_eq!(view.kind.as_deref(), Some("decision"));
    println!("record links: {count} resolved; #47 opens d47; legacy ledger anchors held");
}

// keeps: decision #48 — unique positive record handles and allocation
#[test]
fn next_number_reads_only_frontmatter_across_nested_and_spaced_paths() {
    use std::process::Command;
    let root = Scratch::new();
    fs::create_dir(root.0.join("scripts")).unwrap();
    let script = root.0.join("scripts/next-number.sh");
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/next-number.sh"),
        &script,
    )
    .unwrap();
    let run = || {
        let result = Command::new("sh")
            .arg(&script)
            .current_dir(std::env::temp_dir())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        String::from_utf8(result.stdout).unwrap()
    };
    assert_eq!(run(), "1\n");
    let nested = root.0.join(".bboxy/nested");
    fs::create_dir(&nested).unwrap();
    fs::write(
        nested.join("space record.md"),
        "\u{feff}---\r\nnumber: 47\r\n---\r\nnumber: 99999\r\n",
    )
    .unwrap();
    fs::write(
        root.0.join(".bboxy/other.md"),
        "---\nnumber: 208 # current max\n---\nBody\n",
    )
    .unwrap();
    assert_eq!(run(), "209\n");
}

// keeps: invariant licenses-present
#[test]
fn licence_map_fixtures_keep_escaped_aliases_and_spdx_refusals() {
    let output = std::process::Command::new("python3")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/test-licenses.py"))
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("multiple escaped links and pipes retain columns 0"),
        "{stdout}"
    );
    assert!(
        stdout.contains("linked row still enforces SPDX 1"),
        "{stdout}"
    );
}
