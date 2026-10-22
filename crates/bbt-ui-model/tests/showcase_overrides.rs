#![allow(clippy::disallowed_methods, reason = "test/fixture IO, not the UI thread")]
//! The showcase is the source of the lab's look, so it must not keep a second copy of any Style value
//! (Oracle, decided). Every token-backed property lives in lab/web/lab.css; showcase.html keeps page
//! scaffolding only (position, flex, overflow, cursor, z-index, 100% sizes).

use std::path::Path;

const TOKEN_BACKED: &[&str] = &[
    "color",
    "background",
    "border",
    "outline",
    "box-shadow",
    "text-shadow",
    "padding",
    "margin",
    "gap",
    "row-gap",
    "column-gap",
    "font",
    "letter-spacing",
    "line-height",
    "opacity",
    "filter",
    "backdrop-filter",
    "transition",
    "animation",
];
const SIZED: &[&str] = &[
    "width",
    "height",
    "min-width",
    "min-height",
    "max-width",
    "max-height",
];
const FREE_SIZES: &[&str] = &["100%", "0", "auto", "none"];

fn violations(css: &str) -> Vec<String> {
    let css = strip_comments(css);
    let mut out = Vec::new();
    for block in css.split('}') {
        let Some((sel, body)) = block.split_once('{') else {
            continue;
        };
        let sel = sel.trim();
        if sel.starts_with('@') {
            continue;
        }
        for decl in body.split(';') {
            let Some((prop, value)) = decl.split_once(':') else {
                continue;
            };
            let (prop, value) = (
                prop.trim(),
                value.trim().trim_end_matches("!important").trim(),
            );
            let backed = prop.starts_with("--")
                || TOKEN_BACKED
                    .iter()
                    .any(|p| prop == *p || prop.starts_with(&format!("{p}-")))
                || (SIZED.contains(&prop) && !FREE_SIZES.contains(&value));
            if backed {
                out.push(format!("{sel} {{ {prop}: {value} }}"));
            }
        }
    }
    out
}

fn strip_comments(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find("/*") {
        out.push_str(&rest[..i]);
        rest = rest[i..].find("*/").map_or("", |j| &rest[i + j + 2..]);
    }
    out + rest
}

fn style_blocks(html: &str) -> String {
    let mut css = String::new();
    let mut rest = html;
    while let Some(i) = rest.find("<style") {
        let open = &rest[i..];
        let Some(start) = open.find('>') else { break };
        let end = open.find("</style>").unwrap_or(open.len());
        css.push_str(&open[start + 1..end]);
        css.push('\n');
        rest = &open[end..];
        if rest.is_empty() {
            break;
        }
        rest = &rest["</style>".len().min(rest.len())..];
    }
    // inline style="…" attributes count too
    for part in html.split("style=\"").skip(1) {
        if let Some(end) = part.find('"') {
            css.push_str(&format!("[style] {{ {} }}\n", &part[..end]));
        }
    }
    css
}

#[test]
fn showcase_sets_no_token_backed_property() {
    let web = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lab/web");
    let mut css = style_blocks(
        &std::fs::read_to_string(web.join("showcase.html")).expect("lab/web/showcase.html"),
    );
    if let Ok(sheet) = std::fs::read_to_string(web.join("showcase.css")) {
        css.push_str(&sheet);
    }
    let bad = violations(&css);
    assert!(
        bad.is_empty(),
        "showcase overrides a Style value; move it into its Style record and lab/web/lab.css:\n  {}",
        bad.join("\n  ")
    );
}

#[test]
fn the_guard_catches_an_override() {
    let html = "<style>\n  html, body { height: 100%; overflow: hidden; }\n  .space { border-radius: 11px; }\n  .x { height: 40px; --icon: 18px; }\n</style><div style=\"padding: 4px\"></div>";
    let bad = violations(&style_blocks(html));
    assert_eq!(bad.len(), 4, "{bad:?}");
}
