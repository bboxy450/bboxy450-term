//! Document fixture host. Public document records and parent constraints only.
use crate::{
    Result,
    gallery::{self, Case},
    seat_rows,
};
use bbt_ui::{components as c, modules::document_modal::DocumentModal};
use bbt_ui_model::{draw::DrawCtx, layout::Rect};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};
pub mod repository;

pub const STATES: &[&str] = &[
    "epic",
    "spec",
    "task",
    "bug",
    "note",
    "invariant",
    "artifact",
    "with a diagram",
    "signed",
    "unsigned",
    "superseded",
    "broken frontmatter",
    "as modal",
    "modal · plain markdown",
    "title in body",
    "different first heading",
];
pub fn cases() -> Vec<Case> {
    let mut all = vec![];
    for state in STATES {
        for density in gallery::DENSITIES {
            for theme in gallery::THEMES {
                all.push(Case {
                    component: "documents".into(),
                    state: (*state).into(),
                    slug: state.replace(" · ", "-").replace(' ', "-"),
                    density: gallery::density_name(density).into(),
                    theme: gallery::theme_name(theme).into(),
                    page: "documents.html".into(),
                    selector: ".card > .body".into(),
                    width: 1246,
                    height: 900,
                    checker: false,
                    fixture: Some(json!({"layer":"documents"})),
                });
            }
        }
    }
    let source: Vec<_> = all
        .iter()
        .filter(|c| c.state == "as modal")
        .cloned()
        .map(|mut c| {
            c.slug = "as-modal-source".into();
            c.fixture = Some(json!({"layer":"documents","diagram_source":true}));
            c
        })
        .collect();
    all.extend(source);
    all
}
pub fn legacy_cases() -> Vec<Case> {
    gallery::cases()
        .into_iter()
        .filter(|c| c.component == "document_modal")
        .map(|mut c| {
            c.component.push_str("-v2");
            c.fixture = Some(json!({"layer":"documents","reference":"baseline"}));
            c
        })
        .collect()
}
pub fn layout_case(case: &Case) -> Result<Vec<c::Layout>> {
    let f = case.fixture.as_ref().ok_or("Document fixture missing")?;
    let ctx = crate::components::context(case);
    let metrics = crate::settings::Metrics::default();
    let crop = &f["backdrop"]["crop"];
    f["frames"]
        .as_array()
        .ok_or("Document frames missing")?
        .iter()
        .map(|frame| {
            let view: DocumentModal = serde_json::from_value(frame["view"].clone())?;
            let host = &frame["host"];
            let available = [seat_rows::number(host, "w")?, seat_rows::number(host, "h")?];
            let markdown = bbt_doc::parse(&view.document.body);
            let element = view.element_with_body(ctx, &metrics, available, &|width| {
                markdown.body_element_with_title(
                    Some(if view.document.title.is_empty() {
                        &view.document.path
                    } else {
                        &view.document.title
                    }),
                    ctx,
                    &metrics,
                    width,
                    &view.diagrams,
                )
            });
            let width = if view.modal {
                980_f32.min(view.viewport[0] * 0.94).min(available[0])
            } else {
                available[0]
            };
            element
                .layout(
                    Rect::new(
                        seat_rows::number(host, "x")?
                            + seat_rows::number(crop, "x")?
                            + if view.modal {
                                (available[0] - width) / 2.
                            } else {
                                0.
                            },
                        seat_rows::number(host, "y")? + seat_rows::number(crop, "y")?,
                        width,
                        if view.modal { available[1] } else { 0. },
                    ),
                    ctx,
                    &metrics,
                )
                .map_err(Into::into)
        })
        .collect()
}
pub fn render_case(
    renderer: &mut bbt_render::chrome::ChromeRenderer,
    case: &Case,
) -> Result<Vec<u8>> {
    let f = case.fixture.as_ref().ok_or("Document fixture missing")?;
    let bg = &f["backdrop"];
    let (w, h, mut pixels) = crate::read_png(Path::new(
        bg["path"].as_str().ok_or("Document backdrop missing")?,
    ))?;
    let ctx = crate::components::context(case);
    for (frame, layout) in f["frames"]
        .as_array()
        .ok_or("Document frames missing")?
        .iter()
        .zip(layout_case(case)?)
    {
        let mut clips = vec![];
        if !frame["clip"].is_null() {
            let r = &frame["clip"]["rect"];
            let origin = &bg["crop"];
            let radii: Vec<f32> = frame["clip"]["radii"]
                .as_array()
                .ok_or("Document clip radii missing")?
                .iter()
                .map(|n| n.as_f64().map(|n| n as f32).ok_or("invalid clip radius"))
                .collect::<std::result::Result<_, _>>()?;
            clips.push(c::Clip {
                rect: Rect::new(
                    seat_rows::number(r, "x")? + seat_rows::number(origin, "x")?,
                    seat_rows::number(r, "y")? + seat_rows::number(origin, "y")?,
                    seat_rows::number(r, "w")?,
                    seat_rows::number(r, "h")?,
                ),
                radii: radii.try_into().map_err(|_| "four clip radii required")?,
            });
        }
        pixels = repository::page_pixels(
            renderer,
            c::paint_with_ancestor_clips(&layout, DrawCtx::new(ctx.theme, ctx.density), &clips),
            pixels,
            [w, h],
            ctx,
        )?;
    }
    let x = seat_rows::number(&bg["crop"], "x")?.round() as u32;
    let y = seat_rows::number(&bg["crop"], "y")?.round() as u32;
    if x + case.width > w || y + case.height > h {
        return Err("Document crop outside backdrop".into());
    }
    let mut out = vec![];
    for row in y..y + case.height {
        let i = ((row * w + x) * 4) as usize;
        out.extend_from_slice(&pixels[i..i + case.width as usize * 4]);
    }
    Ok(out)
}
struct DiagramRegion {
    key: String,
    native: [f64; 4],
    web: [f64; 4],
}
fn in_diagram_body(key: &str, region: &DiagramRegion) -> bool {
    key.strip_prefix(&region.key).is_some_and(|suffix| {
        suffix == "/canvas" || suffix.starts_with("/node") || suffix.starts_with("/label")
    })
}
// Comparison projection only, never a layout input. Each side removes its own
// engine-owned canvas height, retaining header, padding and following gaps.
fn chrome_rect(
    key: &str,
    mut rect: [f64; 4],
    regions: &[DiagramRegion],
    native: bool,
    modal: bool,
) -> [f64; 4] {
    let raw = rect;
    for region in regions {
        let canvas = if native { region.native } else { region.web };
        let flow = key.starts_with("block") || key == "provenance";
        if flow && raw[1] >= canvas[1] + canvas[3] - 0.5 {
            rect[1] -= canvas[3];
        }
        let container = matches!(key, "view" | "main" | "body" | "rail")
            || (!modal && key == "document")
            || key == region.key
            || key == format!("{}/figure", region.key)
            || key == format!("{}/drawing", region.key);
        if container && raw[1] <= canvas[1] + 0.5 && raw[1] + raw[3] >= canvas[1] + canvas[3] - 0.5
        {
            rect[3] -= canvas[3];
        }
    }
    rect
}
pub fn has_diagram_body(case: &Case) -> bool {
    case.fixture
        .as_ref()
        .and_then(|f| f["frames"].as_array())
        .is_some_and(|frames| {
            frames.iter().any(|f| {
                f["parts"].as_array().is_some_and(|parts| {
                    parts
                        .iter()
                        .any(|p| p["key"].as_str().is_some_and(|k| k.ends_with("/canvas")))
                })
            })
        })
}
pub fn snapshots(cases: &[Case]) -> Result<Vec<Value>> {
    let mut all = vec![];
    for case in cases.iter().filter(|c| {
        c.fixture
            .as_ref()
            .is_some_and(|f| f["layer"] == "documents")
    }) {
        let f = case.fixture.as_ref().unwrap();
        let crop = &f["backdrop"]["crop"];
        let offset = [seat_rows::number(crop, "x")?, seat_rows::number(crop, "y")?];
        let mut parts = vec![];
        for (frame, layout) in f["frames"]
            .as_array()
            .ok_or("Document frames missing")?
            .iter()
            .zip(layout_case(case)?)
        {
            let mut names = BTreeMap::new();
            seat_rows::named(&layout, &mut names);
            let diagnostics = frame["parts"]
                .as_array()
                .ok_or("Document diagnostics missing")?;
            let mut regions = vec![];
            for p in diagnostics {
                let key = p["key"].as_str().ok_or("Document part key missing")?;
                if let Some(prefix) = key.strip_suffix("/canvas")
                    && let Some(n) = names.get(key)
                {
                    let r = &p["rect"];
                    regions.push(DiagramRegion {
                        key: prefix.into(),
                        native: [
                            f64::from(n.rect.x - offset[0]),
                            f64::from(n.rect.y - offset[1]),
                            f64::from(n.rect.w),
                            f64::from(n.rect.h),
                        ],
                        web: [
                            f64::from(seat_rows::number(r, "x")?),
                            f64::from(seat_rows::number(r, "y")?),
                            f64::from(seat_rows::number(r, "w")?),
                            f64::from(seat_rows::number(r, "h")?),
                        ],
                    });
                }
            }
            for p in frame["parts"]
                .as_array()
                .ok_or("Document diagnostics missing")?
            {
                let key = p["key"].as_str().ok_or("Document part key missing")?;
                if regions.iter().any(|r| in_diagram_body(key, r)) {
                    continue;
                }
                let raw_native = names.get(key).map(|n| {
                    [
                        f64::from(n.rect.x - offset[0]),
                        f64::from(n.rect.y - offset[1]),
                        f64::from(n.rect.w),
                        f64::from(n.rect.h),
                    ]
                });
                let r = &p["rect"];
                let raw_web = [
                    f64::from(seat_rows::number(r, "x")?),
                    f64::from(seat_rows::number(r, "y")?),
                    f64::from(seat_rows::number(r, "w")?),
                    f64::from(seat_rows::number(r, "h")?),
                ];
                let modal = frame["view"]["modal"].as_bool().unwrap_or(false);
                let native = raw_native.map(|r| chrome_rect(key, r, &regions, true, modal));
                let web = chrome_rect(key, raw_web, &regions, false, modal);
                let text = p["kind"] == "text";
                let native_baseline = names
                    .get(key)
                    .and_then(|n| seat_rows::text_baseline(n))
                    .map(|y| {
                        f64::from(y - offset[1]) + native.unwrap()[1] - raw_native.unwrap()[1]
                    });
                parts.push(json!({"key":key,"kind":if text{"text"}else{"box"},"native":native,"web":web,"raw_native":raw_native,"raw_web":raw_web,"diagram_body_excluded":!regions.is_empty(),"baseline":if text{json!({"native":native_baseline,"web":p["font_diagnostics"]["baseline"].as_f64().map(|b|b+web[1])})}else{Value::Null},"children":[]}));
            }
        }
        all.push(json!({"key":case.relative(),"tree":{"kind":"box","native":[0.,0.,case.width as f32,case.height as f32],"web":[0.,0.,case.width as f32,case.height as f32],"children":parts}}));
    }
    Ok(all)
}
pub fn render(out: &Path) -> Result<()> {
    crate::materials::render_gallery(out, &cases())
}

#[cfg(test)]
mod diagram_policy_tests {
    use super::*;

    #[test]
    fn engine_body_height_is_excluded_but_chrome_padding_and_following_gaps_stay_gated() {
        let regions = [DiagramRegion {
            key: "block2".into(),
            native: [10., 50., 500., 160.],
            web: [10., 50., 450., 120.],
        }];
        let native = chrome_rect(
            "block2/figure",
            [2., 20., 576., 200.],
            &regions,
            true,
            false,
        );
        let web = chrome_rect(
            "block2/figure",
            [2., 20., 576., 160.],
            &regions,
            false,
            false,
        );
        assert_eq!(native, web);
        assert_eq!(
            chrome_rect("block3", [2., 230., 576., 30.], &regions, true, false),
            chrome_rect("block3", [2., 190., 576., 30.], &regions, false, false)
        );
        assert_ne!(
            chrome_rect(
                "block2/figure",
                [2., 20., 576., 201.],
                &regions,
                true,
                false
            ),
            web,
            "one wrong chrome pixel still fails"
        );
        assert_eq!(
            chrome_rect("block2/head", [2., 20., 576., 30.], &regions, true, false),
            [2., 20., 576., 30.]
        );
        assert_eq!(
            chrome_rect("rail/h1", [900., 230., 200., 30.], &regions, true, false),
            [900., 230., 200., 30.],
            "parallel rail content never follows the Markdown flow"
        );
        assert!(in_diagram_body("block2/node0", &regions[0]));
        assert!(!in_diagram_body("block2/head", &regions[0]));
        assert!(!in_diagram_body("block20/node0", &regions[0]));
    }
}
