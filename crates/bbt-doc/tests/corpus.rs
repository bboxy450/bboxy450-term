#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]
use std::{fs, io, path::Path};

// Read the real files rather than a baked fixture, including new untracked
// Markdown. Symlinked directories are not followed.
fn frontmatter_errors(root: &Path) -> io::Result<(usize, Vec<String>)> {
    fn walk(
        root: &Path,
        dir: &Path,
        count: &mut usize,
        errors: &mut Vec<String>,
    ) -> io::Result<()> {
        let mut entries = fs::read_dir(dir)?.collect::<io::Result<Vec<_>>>()?;
        entries.sort_by_key(|entry| entry.path());
        for entry in entries {
            let path = entry.path();
            let kind = entry.file_type()?;
            if kind.is_dir() {
                walk(root, &path, count, errors)?;
            } else if kind.is_file() && path.extension().is_some_and(|ext| ext == "md") {
                *count += 1;
                let doc = bbt_doc::parse(&fs::read_to_string(&path)?);
                if let Some(error) = &doc.frontmatter.error {
                    errors.push(format!(
                        "{}:{}: invalid YAML frontmatter: {error}",
                        path.strip_prefix(root).unwrap().display(),
                        doc.frontmatter.error_line.unwrap_or(1)
                    ));
                }
            }
        }
        Ok(())
    }
    let (mut count, mut errors) = (0, vec![]);
    walk(root, &root.join(".bboxy"), &mut count, &mut errors)?;
    Ok((count, errors))
}

#[test]
fn real_bboxy_frontmatter_parses() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let (count, errors) = frontmatter_errors(&root).unwrap();
    assert!(count > 0, "the real .bboxy corpus must not be empty");
    assert!(errors.is_empty(), "{}", errors.join("\n"));
    println!("frontmatter: {count} Markdown files, 0 parse errors");
}

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
        "\u{feff}---\r\nid: fox\r\ntype: spec\r\ntitle: live seats: cockpit\r\n---\r\nKept body.",
    )
    .unwrap();
    fs::write(specs.join("open.md"), "---\ntitle: never closed").unwrap();
    fs::write(specs.join("plain.md"), "# No frontmatter is valid\n").unwrap();
    let (count, errors) = frontmatter_errors(&root.0).unwrap();
    assert_eq!(count, 3);
    assert_eq!(errors.len(), 2);
    assert!(errors[0].starts_with(".bboxy/specs/broken.md:4: invalid YAML frontmatter:"));
    assert!(errors[1].starts_with(".bboxy/specs/open.md:1: invalid YAML frontmatter:"));
    fs::write(
        specs.join("broken.md"),
        "---\ntitle: 'live seats: cockpit'\n---\nBody.",
    )
    .unwrap();
    fs::write(specs.join("open.md"), "---\ntitle: now closed\n---\n").unwrap();
    assert!(frontmatter_errors(&root.0).unwrap().1.is_empty());
}
