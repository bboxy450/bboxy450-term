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
    let diagnostics = prepare(&original, Some(&fonts))
        .err()
        .expect("original invalid family must refuse");
    assert_eq!(diagnostics[0].code, "font_family");
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
// keeps: spec svg-renderer — all unsupported paint is reported in one census.
#[test]
fn complete_diagnostics_and_inert_metadata_do_not_hide_paint() {
    let errors = prepare(
        &svg("<filter/><mask/><rect width=\"1\" height=\"1\" unknown-paint=\"red\"/>"),
        None,
    )
    .err()
    .unwrap();
    assert_eq!(errors.len(), 3);
    assert!(
        errors
            .iter()
            .all(|e| e.disposition == Disposition::Unsupported)
    );
    assert!(prepare(&svg("<metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><dc:title>credit</dc:title></metadata><rect width=\"1\" height=\"1\"/>"),None).is_ok());
}

// keeps: spec svg-renderer — foreign local names never become SVG paint.
#[test]
fn foreign_paint_attributes_are_explicitly_unsupported() {
    let input = svg(
        "<rect xmlns:f=\"urn:foreign\" f:fill=\"red\" f:transform=\"translate(3,4)\" width=\"1\" height=\"1\"/>",
    );
    let errors = prepare(&input, None)
        .err()
        .expect("foreign paint must fail admission");
    assert_eq!(errors.len(), 2);
    assert!(
        errors
            .iter()
            .all(|e| e.disposition == Disposition::Unsupported && e.detail.contains("urn:foreign"))
    );
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

// keeps: spec svg-renderer — an active FontSet requires a recorded matching identity even for shapes.
#[cfg(target_os = "macos")]
#[test]
fn unmarked_shapes_refuse_with_an_active_font_set() {
    let fonts = FontSet::load(&base().join("fonts")).unwrap();
    let error = prepare(&svg("<path d=\"M0 0h1v1Z\"/>"), Some(&fonts))
        .err()
        .expect("identity must be declared");
    assert_eq!(error[0].code, "font_identity_missing");
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
