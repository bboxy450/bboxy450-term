// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "private unattended headless PNG fixtures"
)]
use bboxy450_canvas::{Action, Camera, Canvas, Content, load_log};
use bbt_ui::{self as ui, Theme, components as c};
use bbt_ui_model::{draw::Prim, layout::Rect};
struct Metrics;
impl bbt_ui_model::layout::TextMeasure for Metrics {
    fn width(&self, value: &str, style: &bbt_ui_model::layout::TextStyle) -> f32 {
        value.chars().count() as f32 * style.size * 0.5
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, style: &bbt_ui_model::layout::TextStyle) -> [f32; 2] {
        [style.size * 0.8, style.size * 0.2]
    }
}
fn render(
    canvas: &Canvas,
    camera: &Camera,
    viewport: Rect,
    theme: Theme,
) -> Result<bbt_ui_model::draw::DrawList, &'static str> {
    let element = ui::modules::canvas::render(canvas, camera, viewport, theme)?;
    let ctx = ui::Context::new(theme, ui::Density::Cozy);
    let layout = element.layout(viewport, ctx, &Metrics)?;
    let mut list = bbt_ui_model::draw::DrawList::default();
    for op in c::paint(&layout, ctx) {
        match op {
            c::Paint::Draw(draw) | c::Paint::Underlay(draw) => {
                list.primitives.extend(draw.primitives)
            }
            _ => panic!("static canvas uses ordinary component primitives"),
        }
    }
    list.validate()?;
    Ok(list)
}
fn assert_empty(list: &bbt_ui_model::draw::DrawList) {
    assert!(
        !list
            .primitives
            .iter()
            .any(|p| matches!(p, Prim::Path { .. } | Prim::Text { .. }))
    );
}
const LOG: &[u8] = include_bytes!("../../bboxy450-canvas/tests/fixtures/c0.log.jsonl");
fn fixture() -> Canvas {
    Canvas::replay(&load_log(LOG).unwrap()).unwrap()
}
fn camera() -> Camera {
    Camera {
        center: [400., 260.],
        zoom: 1.,
    }
}
fn viewport() -> Rect {
    Rect::new(0., 0., 800., 520.)
}
#[test]
fn canvas_draw_list_uses_house_themes_paths_text_frames_and_culling() {
    let c = fixture();
    let dark = render(&c, &camera(), viewport(), Theme::Dark).unwrap();
    let light = render(&c, &camera(), viewport(), Theme::Light).unwrap();
    assert_ne!(dark, light);
    dark.validate().unwrap();
    light.validate().unwrap();
    assert!(
        dark.primitives
            .iter()
            .any(|p| matches!(p,Prim::Path{stroke,..} if !stroke.dash.is_empty()))
    );
    for label in ["Canvas C0", "System", "Conversation", "Service", "Operator"] {
        assert!(
            dark.primitives
                .iter()
                .any(|p| matches!(p,Prim::Text{text,..} if text.as_ref() == label)),
            "{label}"
        );
    }
    assert_empty(
        &render(
            &c,
            &Camera {
                center: [1e12; 2],
                zoom: 1.,
            },
            viewport(),
            Theme::Dark,
        )
        .unwrap(),
    );
    assert_empty(
        &render(
            &c,
            &Camera {
                zoom: 1e-9,
                ..camera()
            },
            viewport(),
            Theme::Dark,
        )
        .unwrap(),
    );
    let collapsed = render(
        &c,
        &Camera {
            zoom: 0.2,
            ..camera()
        },
        viewport(),
        Theme::Dark,
    )
    .unwrap();
    assert!(
        !collapsed
            .primitives
            .iter()
            .any(|p| matches!(p,Prim::Text{text,..} if text.contains("One log")))
    );
    let collapsed_frames = render(
        &c,
        &Camera {
            zoom: 0.03,
            ..camera()
        },
        viewport(),
        Theme::Dark,
    )
    .unwrap();
    for title in ["System", "Conversation"] {
        assert!(
            collapsed_frames
                .primitives
                .iter()
                .any(|p| matches!(p, Prim::Text { text, .. } if text.as_ref() == title))
        );
    }
    assert!(
        render(
            &c,
            &Camera {
                zoom: 1e9,
                ..camera()
            },
            viewport(),
            Theme::Dark
        )
        .unwrap()
        .validate()
        .is_ok()
    );
    assert!(render(&c, &camera(), Rect::new(0., 0., f32::NAN, 1.), Theme::Dark).is_err());
}
#[test]
fn canvas_camera_relative_output_preserves_large_world_centers() {
    let original = fixture();
    let mut ops = load_log(LOG).unwrap();
    for op in &mut ops {
        if let Action::Add { element } = &mut op.action {
            match &mut element.content {
                Content::Shape { bounds, .. }
                | Content::Frame { bounds, .. }
                | Content::Text { bounds, .. } => {
                    bounds.origin[0] += 1e12;
                    bounds.origin[1] += 1e12;
                }
                _ => {}
            }
        }
    }
    let shifted = Canvas::replay(&ops).unwrap();
    assert_eq!(
        render(&original, &camera(), viewport(), Theme::Dark).unwrap(),
        render(
            &shifted,
            &Camera {
                center: [1e12 + 400., 1e12 + 260.],
                zoom: 1.
            },
            viewport(),
            Theme::Dark
        )
        .unwrap()
    );
}
#[test]
fn canvas_camera_gesture_scale_extremes_keep_finite_rendering() {
    let canvas = fixture();
    let mut camera = camera();
    for factor in [1e9, 1e-9, 1e-9, 1e9] {
        camera.zoom_at(factor, [0.; 2]).unwrap();
        for theme in [Theme::Dark, Theme::Light] {
            render(&canvas, &camera, viewport(), theme)
                .unwrap()
                .validate()
                .unwrap();
        }
    }
    assert!((camera.zoom - 1.).abs() < f64::EPSILON);
}
#[test]
fn canvas_sidebar_is_one_quiet_row_per_canvas_and_empty_section() {
    use ui::modules::sidebar::{CanvasRow, canvas_section};
    let ctx = ui::Context::new(Theme::Dark, ui::Density::Cozy);
    assert!(canvas_section("swarm", &[], true, ctx).is_none());
    let rows = [CanvasRow {
        id: "drawing".into(),
        title: "Kept drawing".into(),
        last_change: "2m ago".into(),
        last_author: "Canvas Executor".into(),
        tags: (1..=5).map(|i| format!("tag{i}")).collect(),
    }];
    fn names_and_text(node: &c::Layout, names: &mut Vec<String>, texts: &mut Vec<String>) {
        match &node.kind {
            c::LayoutKind::Named(name) => names.push(name.clone()),
            c::LayoutKind::Text { runs, .. } => {
                texts.extend(runs.iter().map(|run| run.text.clone()));
            }
            _ => {}
        }
        for child in &node.children {
            names_and_text(child, names, texts);
        }
    }
    for expanded in [true, false] {
        let element = canvas_section("swarm", &rows, expanded, ctx).unwrap();
        let layout = element
            .layout(Rect::new(0., 0., 300., 300.), ctx, &Metrics)
            .unwrap();
        let (mut names, mut texts) = (vec![], vec![]);
        names_and_text(&layout, &mut names, &mut texts);
        assert!(names.contains(&"canvas/swarm/toggle".into()));
        assert_eq!(
            names.contains(&"canvas/swarm/open/drawing".into()),
            expanded
        );
        assert_eq!(texts.iter().any(|s| s == "Kept drawing"), expanded);
        // The row shows the time of the last change; author and tags stay
        // out of the sidebar.
        assert_eq!(texts.iter().any(|s| s == "2m ago"), expanded);
        assert!(!texts.iter().any(|s| s.contains("Canvas Executor")));
        assert!(!texts.iter().any(|s| s.starts_with("tag")));
    }
}
#[test]
fn canvas_connector_follows_changed_endpoint_geometry() {
    let mut ops = load_log(LOG).unwrap();
    let canvas = Canvas::replay(&ops).unwrap();
    // A path's points are local to its own box; the absolute start is the
    // box origin plus the first point.
    let edge_start = |canvas: &Canvas| {
        let list = render(canvas, &camera(), viewport(), Theme::Dark).unwrap();
        list.primitives
            .iter()
            .find_map(|p| match p {
                Prim::Path { origin, points, .. } => match points[0] {
                    bbt_ui_model::draw::PathCommand::Move(p) => {
                        Some(bbt_ui_model::draw::PathCommand::Move([
                            (origin[0] + p[0]).round(),
                            (origin[1] + p[1]).round(),
                        ]))
                    }
                    other => Some(other),
                },
                _ => None,
            })
            .unwrap()
    };
    assert_eq!(
        edge_start(&canvas),
        bbt_ui_model::draw::PathCommand::Move([220., 142.])
    );
    let mut endpoint = canvas.get("box").unwrap().clone();
    if let Content::Shape { bounds, .. } = &mut endpoint.content {
        bounds.origin[0] += 40.;
    }
    ops.push(bboxy450_canvas::Op {
        author: ops[0].author.clone(),
        seq: ops.len() as u64 + 1,
        at: 0,
        action: Action::Change { element: endpoint },
    });
    assert_eq!(
        edge_start(&Canvas::replay(&ops).unwrap()),
        bbt_ui_model::draw::PathCommand::Move([260., 142.])
    );
}
#[cfg(target_os = "macos")]
fn golden(theme: Theme, name: &str) {
    use bbt_render::chrome::{ChromeRenderer, icons::Assets};
    use std::{fs::File, io::BufReader, path::Path};
    let c = fixture();
    let list = render(&c, &camera(), viewport(), theme).unwrap();
    let mut gpu =
        ChromeRenderer::new(Assets::default()).expect("headless renderer required; never skip");
    let pixels = gpu.render_to_rgba(&list, 800, 520, 1.).unwrap();
    assert_eq!(
        pixels,
        gpu.render_to_rgba(&list, 800, 520, 1.).unwrap(),
        "repeat render"
    );
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/goldens")
        .join(name);
    if std::env::var_os("UPDATE_GOLDENS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut encoder = png::Encoder::new(File::create(&path).unwrap(), 800, 520);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&pixels)
            .unwrap();
    }
    let decoder = png::Decoder::new(BufReader::new(
        File::open(&path).expect("missing canvas golden; UPDATE_GOLDENS=1 explicitly refreshes"),
    ));
    let mut reader = decoder.read_info().unwrap();
    let mut expected = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut expected).unwrap();
    assert_eq!(
        (info.width, info.height, info.color_type),
        (800, 520, png::ColorType::Rgba)
    );
    assert_eq!(
        pixels,
        &expected[..info.buffer_size()],
        "canvas golden {} differs",
        path.display()
    );
    assert_eq!(c.digest(), fixture().digest());
}
#[cfg(target_os = "macos")]
#[test]
fn canvas_golden_light() {
    golden(Theme::Light, "canvas-c0-light.png");
}
#[cfg(target_os = "macos")]
#[test]
fn canvas_golden_dark() {
    golden(Theme::Dark, "canvas-c0-dark.png");
}
