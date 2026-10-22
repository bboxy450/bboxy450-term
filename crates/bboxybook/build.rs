#![allow(
    clippy::disallowed_methods,
    reason = "build script: compile-time IO, not the UI thread"
)]
use std::{env, fs, path::PathBuf};
fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let source = root.join("lab/web/components.js");
    println!("cargo:rerun-if-changed={}", source.display());
    println!(
        "cargo:rerun-if-changed={}",
        root.join("lab/web/bboxy.png").display()
    );
    let text = fs::read_to_string(source).unwrap();
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let pattern = regex::Regex::new(r"(?m)^\s*(\w+):\s*'(<svg[^\n]+)</svg>',?\s*$").unwrap();
    let mut code = String::from("pub const BAKED_ICONS: &[(&str,&str)] = &[\n");
    let mut count = 0;
    for c in pattern.captures_iter(&text) {
        let svg = format!("{}</svg>", &c[2])
            .replace("<svg ", "<svg xmlns=\"http://www.w3.org/2000/svg\" ")
            .replace("currentColor", "#ffffff");
        code.push_str(&format!("({:?},{:?}),\n", &c[1], svg));
        count += 1;
    }
    assert!(count >= 15, "lab SVG dictionary disappeared");
    code.push_str("];\n");
    fs::write(out.join("icons.rs"), code).unwrap();
}
