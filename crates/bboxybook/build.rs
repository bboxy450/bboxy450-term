#![allow(clippy::disallowed_methods, reason = "build script: compile-time IO, not the UI thread")]
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
    let mut code = String::from("pub const BAKED_ICONS: &[(&str,u32,u32,&[u8])] = &[\n");
    let mut count = 0;
    for c in pattern.captures_iter(&text) {
        let svg = format!("{}</svg>", &c[2])
            .replace("<svg ", "<svg xmlns=\"http://www.w3.org/2000/svg\" ")
            .replace("currentColor", "#ffffff");
        let tree = resvg::usvg::Tree::from_str(&svg, &Default::default()).expect("lab icon SVG");
        let mut pix = resvg::tiny_skia::Pixmap::new(64, 64).unwrap();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(
                64.0 / tree.size().width(),
                64.0 / tree.size().height(),
            ),
            &mut pix.as_mut(),
        );
        let mut data = pix.data().to_vec();
        for p in data.as_chunks_mut::<4>().0 {
            let a = u32::from(p[3]);
            for c in &mut p[..3] {
                *c = (u32::from(*c) * 255).checked_div(a).unwrap_or(0).min(255) as u8;
            }
        }
        let path = out.join(format!("icon-{}.rgba", &c[1]));
        fs::write(&path, data).unwrap();
        code.push_str(&format!(
            "({:?},64,64,include_bytes!({:?})),\n",
            &c[1], path
        ));
        count += 1;
    }
    assert!(count >= 15, "lab SVG dictionary disappeared");
    code.push_str("];\n");
    fs::write(out.join("icons.rs"), code).unwrap();
}
