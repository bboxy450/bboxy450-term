// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "immutable local test fixtures; no UI event loop"
)]
use crate::*;
use std::{fs, path::PathBuf};

fn base() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn svg(body: &str) -> Vec<u8> {
    format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 64 64\">{body}</svg>")
        .into_bytes()
}
fn error_code(input: &[u8]) -> &'static str {
    prepare(input, None).err().expect("must refuse")[0].code
}
// keeps: spec svg-renderer — parser acceptance uses every frozen original source.
#[cfg(target_os = "macos")]
#[test]
fn frozen_positive_sources_prepare_with_the_actual_manifest_faces() {
    let fonts = FontSet::load(&base().join("fonts")).unwrap();
    for directory in ["corpus", "fixtures"] {
        let manifest: serde_json::Value = serde_json::from_slice(
            &fs::read(base().join(directory).join("manifest.json")).unwrap(),
        )
        .unwrap();
        for case in manifest["cases"].as_array().unwrap() {
            let path = base().join(directory).join(case["svg"].as_str().unwrap());
            let bytes = fs::read(&path).unwrap();
            let result = prepare(&bytes, Some(&fonts));
            assert!(result.is_ok(), "{}: {:?}", path.display(), result.err());
        }
    }
}
// keeps: spec svg-renderer — dangerous and unsupported inputs never paint partially.
#[cfg(target_os = "macos")]
#[test]
fn frozen_refusals_are_classified_natively_without_a_reference_renderer() {
    let fonts = FontSet::load(&base().join("fonts")).unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(base().join("fixtures/manifest.json")).unwrap()).unwrap();
    for case in manifest["negatives"].as_array().unwrap() {
        let path = base().join("fixtures").join(case["svg"].as_str().unwrap());
        if case["expected"] == "rendered" {
            let result = prepare(&fs::read(&path).unwrap(), Some(&fonts));
            assert!(result.is_ok(), "{}: {:?}", path.display(), result.err());
            continue;
        }
        let diagnostics = prepare(&fs::read(&path).unwrap(), Some(&fonts))
            .err()
            .unwrap_or_else(|| panic!("accepted {}", path.display()));
        let expected = if case["expected"] == "unsupported" {
            Disposition::Unsupported
        } else {
            Disposition::Refused
        };
        assert_eq!(
            diagnostics[0].disposition,
            expected,
            "{}: {:?}",
            path.display(),
            diagnostics
        );
    }
    let original = fs::read(base().join("fixtures/refusals/invalid-family.svg")).unwrap();
    use sha2::Digest;
    assert_eq!(
        format!("{:x}", sha2::Sha256::digest(&original)),
        "1043e461bee4f58c04a78760b44db0e9421c4f904cc5619a3826bdedbf2041f1"
    );
    // An unknown family renders with the bundled face instead of refusing.
    assert!(prepare(&original, Some(&fonts)).is_ok());
    let mismatched = String::from_utf8(original)
        .unwrap()
        .replace(fonts.identity(), &format!("sha256:{}", "0".repeat(64)));
    let diagnostics = prepare(mismatched.as_bytes(), Some(&fonts))
        .err()
        .expect("identity mismatch refuses independently of family syntax");
    assert_eq!(diagnostics[0].code, "font_identity_mismatch");
}
// keeps: spec svg-renderer — exact byte/node/depth/expanded-command bounds.
#[test]
fn preparation_bounds_have_accepted_and_rejected_sides() {
    let prefix = svg("<!---->");
    let extra = MAX_SVG_BYTES - prefix.len();
    let accepted = String::from_utf8(prefix)
        .unwrap()
        .replace("<!---->", &format!("<!--{}-->", "x".repeat(extra)));
    assert_eq!(accepted.len(), MAX_SVG_BYTES);
    assert!(prepare(accepted.as_bytes(), None).is_ok());
    assert_eq!(error_code(&(accepted + " ").into_bytes()), "svg_bytes");
    let nodes = svg(&"<g/>".repeat(MAX_XML_NODES - 1));
    assert!(prepare(&nodes, None).is_ok());
    assert!(prepare(&svg(&"<g/>".repeat(MAX_XML_NODES)), None).is_err());
    let nested = |count: usize| {
        svg(&format!(
            "{}<rect width=\"1\" height=\"1\"/>{}",
            "<g>".repeat(count - 2),
            "</g>".repeat(count - 2)
        ))
    };
    assert!(prepare(&nested(MAX_DEPTH), None).is_ok());
    assert_eq!(error_code(&nested(MAX_DEPTH + 1)), "depth");
    let path = format!("<path d=\"M0 0{}\"/>", "L1 1".repeat(8191));
    let scene = path.repeat(MAX_SCENE_COMMANDS / 8192);
    assert!(prepare(&svg(&scene), None).is_ok());
    assert_eq!(
        error_code(&svg(&(scene + "<path d=\"M0 0\"/>"))),
        "scene_commands"
    );
    let cycle = svg("<defs><g id=\"a\"><use href=\"#a\"/></g></defs><use href=\"#a\"/>");
    assert_eq!(error_code(&cycle), "reference_cycle");
}
// keeps: spec svg-renderer — permissive admission: what a browser draws, we draw;
// unknown elements/attributes/properties are ignored, never refused.
#[test]
fn unknown_markup_is_ignored_and_active_content_still_refused() {
    assert!(
        prepare(
            &svg(
                "<filter/><mask/><animate/><rect width=\"1\" height=\"1\" unknown-paint=\"red\"/>"
            ),
            None
        )
        .is_ok()
    );
    assert!(prepare(&svg("<rect xmlns:f=\"urn:foreign\" f:fill=\"red\" width=\"1\" height=\"1\" style=\"cursor:pointer;fill:red !important\"/>"), None).is_ok());
    assert!(prepare(&svg("<metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><dc:title>credit</dc:title></metadata><rect width=\"1\" height=\"1\"/>"),None).is_ok());
    assert_eq!(error_code(&svg("<script>1</script>")), "active_content");
    assert_eq!(
        error_code(&svg("<rect onclick=\"x()\" width=\"1\" height=\"1\"/>")),
        "active_content"
    );
    assert_eq!(
        error_code(&svg(
            "<image href=\"https://x.test/a.png\" width=\"1\" height=\"1\"/>"
        )),
        "external_reference"
    );
    assert_eq!(
        error_code(&svg(
            "<rect fill=\"url(https://x.test/p)\" width=\"1\" height=\"1\"/>"
        )),
        "external_reference"
    );
}

// keeps: spec svg-renderer — hand-written basic shapes, CSS classes, currentColor,
// radial gradients and percentage sizes paint at the expected pixels.
#[cfg(target_os = "macos")]
#[test]
fn hand_written_shapes_and_styles_paint() {
    let input = br##"<svg xmlns="http://www.w3.org/2000/svg" width="100%" height="100%" viewBox="0 0 40 20">
      <style>.hot { fill: #ff0000 } #cold { fill: #0000ff }</style>
      <defs><radialGradient id="r"><stop offset="0" stop-color="#00ff00"/></radialGradient></defs>
      <circle class="hot" cx="5" cy="5" r="4"/>
      <ellipse id="cold" cx="15" cy="5" rx="4" ry="3"/>
      <rect x="22" y="1" width="8" height="8" rx="2" fill="url(#r)"/>
      <polygon points="32,1 39,1 39,9" style="color:#ff00ff" fill="currentColor"/>
      <polyline points="1,15 39,15" stroke="black" stroke-width="2" fill="none"/>
    </svg>"##;
    let scene = prepare(input, None).unwrap();
    assert_eq!(scene.viewport(), [40., 20.]);
    let mut renderer = bbt_render::chrome::ChromeRenderer::new(Vec::new()).unwrap();
    let capture = scene
        .render(
            &mut renderer,
            RenderRequest {
                camera: [0., 0., 40., 20.],
                scale: 1.,
            },
            Layer::Render,
        )
        .unwrap();
    let pixel = |x: usize, y: usize| {
        let i = 4 * (y * capture.width as usize + x);
        [capture.rgba[i], capture.rgba[i + 1], capture.rgba[i + 2]]
    };
    assert_eq!(pixel(5, 5), [255, 0, 0], "circle via class rule");
    assert_eq!(pixel(1, 1), [255, 255, 255], "circle corner stays empty");
    assert_eq!(pixel(15, 5), [0, 0, 255], "ellipse via id rule");
    assert_eq!(
        pixel(26, 5),
        [0, 255, 0],
        "radial gradient falls back to its stop"
    );
    assert_ne!(pixel(22, 1), [0, 255, 0], "rounded corner is cut");
    assert_eq!(pixel(38, 2), [255, 0, 255], "polygon in currentColor");
    assert_eq!(pixel(20, 15), [0, 0, 0], "polyline stroke");
}

// keeps: spec svg-renderer — a declared identity requires an active verified FontSet even for outlined content.
#[test]
fn marked_shapes_refuse_without_an_active_font_set() {
    let plain = svg("<path d=\"M0 0h1v1Z\"/>");
    assert!(prepare(&plain, None).is_ok());
    let marked = String::from_utf8(plain).unwrap().replace(
        "viewBox=",
        "data-bboxy-font-set=\"sha256:unverified\" viewBox=",
    );
    assert_eq!(error_code(marked.as_bytes()), "font_unavailable");
}

// keeps: spec svg-renderer — unmarked hand-written SVG, text included, renders with the
// active FontSet; only a declared identity naming another set is refused.
#[cfg(target_os = "macos")]
#[test]
fn unmarked_svg_renders_with_the_active_font_set() {
    let fonts = FontSet::load(&base().join("fonts")).unwrap();
    assert!(prepare(&svg("<path d=\"M0 0h1v1Z\"/>"), Some(&fonts)).is_ok());
    let text = svg(
        "<text x=\"2\" y=\"20\" font-family=\"Helvetica, sans-serif\" font-weight=\"bold\" font-size=\"1em\">Hello</text>",
    );
    assert!(prepare(&text, Some(&fonts)).is_ok());
}

// keeps: spec svg-renderer — exact fractional cameras show paint to the integer
// root edge, then clip it even when authored paint extends past that edge.
#[cfg(target_os = "macos")]
#[test]
fn fractional_camera_preserves_the_integer_root_clip() {
    let scene = prepare(b"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 4 3.25\"><rect width=\"8\" height=\"8\" fill=\"red\"/></svg>",None).unwrap();
    let mut renderer = bbt_render::chrome::ChromeRenderer::new(Vec::new()).unwrap();
    let request = RenderRequest {
        camera: [0.333, 3.333, 2., 2.],
        scale: 2.,
    };
    let capture = scene.render(&mut renderer, request, Layer::Render).unwrap();
    assert_eq!(capture.mapping.viewport, [0., 0., 4., 4.]);
    assert_eq!(capture.mapping.camera, request.camera);
    assert!(capture.mapping.root_clip);
    assert_eq!([capture.width, capture.height], [4, 4]);
    assert_eq!(&capture.rgba[..4], &[255, 0, 0, 255]);
    assert_eq!(&capture.rgba[32..36], &[255, 255, 255, 255]);
}

#[cfg(target_os = "macos")]
#[test]
fn retained_picture_repaints_a_deep_zoom_through_the_visible_rounded_clip() {
    use bbt_ui_model::{
        draw::{DrawList, ImageAsset, Prim},
        layout::Rect,
    };
    let scene = prepare(&svg(r#"<rect width="64" height="64" fill="red"/>"#), None).unwrap();
    let asset = ImageAsset {
        key: "svg-test".into(),
        width: 64,
        height: 64,
        retained: Some(scene.retained_image()),
        revision: Some("artifact:test:3".into()),
    };
    let mut list = DrawList::default();
    list.clip(Rect::new(6.25, 8.25, 19.75, 19.75), [5.; 4]);
    list.push(Prim::Image {
        rect: Rect::new(-20_000., -20_000., 80_000., 80_000.),
        image: asset.source(),
        radius: 0.,
    });
    list.unclip();

    let mut renderer = bbt_render::chrome::ChromeRenderer::new(Vec::new()).unwrap();
    for scale in [1, 2] {
        let width = 40 * scale;
        let rgba = renderer
            .render_to_rgba(&list, width, 40 * scale, scale as f32)
            .unwrap();
        let pixel = |x: u32, y: u32| {
            let i = (4 * (y * scale * width + x * scale)) as usize;
            &rgba[i..i + 4]
        };
        let red = &[255, 0, 0, 255];
        assert_eq!(pixel(14, 14), red);
        assert_ne!(pixel(6, 8), red, "rounded edge must clip");
        assert_ne!(pixel(26, 15), red, "right edge must clip");
        assert_ne!(pixel(15, 28), red, "bottom edge must clip");
    }
}
