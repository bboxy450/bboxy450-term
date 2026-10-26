#![allow(clippy::disallowed_methods, reason = "test/fixture IO, not the UI thread")]
//! Isolate raster from layout with explicit shared baselines. No production
//! component consumes these browser measurements. Run the adjacent JS capture,
//! then `cargo run -p bboxy450-book --example chrome-text-probe -- OUT`.
use bbt_render::chrome::{ChromeRenderer, text};
use bbt_ui_model::{
    draw::{Color, DrawList, TextStyle, TypeScale},
    layout::Rect,
};
use serde_json::Value;
use std::{fs, path::PathBuf};

fn main() -> bboxybook::Result<()> {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "target/bboxybook/chrome-text".into()),
    );
    let mut renderer = ChromeRenderer::new(vec![])?;
    let mut results = vec![];
    for prefix in ["", "palette-", "phase-", "italic-"] {
        for scale in [1u32, 2] {
            let cases: Vec<Value> = serde_json::from_slice(&fs::read(
                out.join(format!("{prefix}cases-{scale}x.json")),
            )?)?;
            let (w, h) = (1360 * scale, (cases.len() as u32).div_ceil(4) * 48 * scale);
            let mut pixels = vec![255u8; (w * h * 4) as usize];
            let mut list = DrawList::default();
            for case in &cases {
                let n = |key: &str| case[key].as_f64().unwrap() as f32;
                let cell = &case["cell"];
                let f = |key: &str| cell[key].as_f64().unwrap() as f32;
                let style = TextStyle {
                    scale: TypeScale::Custom(n("size")),
                    weight: n("weight") as u16,
                    mono: case["mono"] == true,
                    italic: case["italic"] == true,
                    ..Default::default()
                };
                let text = case["text"].as_str().unwrap();
                let sf = scale as f32;
                let px = n("x") * sf;
                let mask = text::rasterize_at(text, style, None, sf, px.rem_euclid(1.))?;
                let baseline = (mask.descent * sf).ceil() + 2.;
                let top = (n("baseline") * sf - mask.height as f32 + baseline).round() as i32;
                let left = px.floor() as i32 - 2;
                let dark = case["dark"] == true;
                let parse = |value: Option<&str>, fallback| -> bboxybook::Result<Color> {
                    let Some(value) = value else {
                        return Ok(fallback);
                    };
                    let rgb = u32::from_str_radix(value.trim_start_matches('#'), 16)?;
                    Ok(Color([
                        ((rgb >> 16) & 255) as f32 / 255.,
                        ((rgb >> 8) & 255) as f32 / 255.,
                        (rgb & 255) as f32 / 255.,
                        1.,
                    ]))
                };
                let background = parse(
                    case["bg"].as_str(),
                    if dark { Color::BLACK } else { Color::WHITE },
                )?;
                let foreground = parse(
                    case["fg"].as_str(),
                    if dark { Color::WHITE } else { Color::BLACK },
                )?;
                list.rect(Rect::new(f("x"), f("y"), f("w"), f("h")), 0., background);
                // The experiment supplies the common baseline. Renderer placement
                // remains the production Prim::Text -> phase-aware atlas path.
                let origin_y =
                    (top as f32 - (style.pixels() * 1.4 * sf - mask.height as f32) * 0.5) / sf;
                list.text(
                    Rect::new(n("x"), origin_y, 330., style.pixels() * 1.4),
                    text,
                    style,
                    foreground,
                );
                let (cx, cy, cw, ch) = (
                    (f("x") * sf) as u32,
                    (f("y") * sf) as u32,
                    (f("w") * sf) as u32,
                    (f("h") * sf) as u32,
                );
                for y in cy..cy + ch {
                    for x in cx..cx + cw {
                        let mx = x as i32 - left;
                        let my = y as i32 - top;
                        let a = if mx >= 0
                            && my >= 0
                            && mx < mask.width as i32
                            && my < mask.height as i32
                        {
                            mask.mask[my as usize * mask.width as usize + mx as usize]
                        } else {
                            0
                        };
                        let at = ((y * w + x) * 4) as usize;
                        for channel in 0..3 {
                            pixels[at + channel] = (foreground.0[channel] * f32::from(a)
                                + background.0[channel] * (255. - f32::from(a)))
                            .round() as u8;
                        }
                        pixels[at + 3] = 255;
                    }
                }
            }
            bbt_render::write_png(
                out.join(format!("{prefix}native-mask-{scale}x.png")),
                &pixels,
                w,
                h,
            )?;
            let gpu = renderer.render_to_rgba(&list, w, h, scale as f32)?;
            bbt_render::write_png(
                out.join(format!("{prefix}native-gpu-{scale}x.png")),
                &gpu,
                w,
                h,
            )?;
            let (_, _, web) = bboxybook::read_png(&out.join(format!("{prefix}web-{scale}x.png")))?;
            for (mode, actual) in [("mask", &pixels), ("gpu", &gpu)] {
                let difference = bbt_render::chrome::diff::compare(&web, actual)?;
                bbt_render::write_png(
                    out.join(format!("{prefix}diff-{mode}-{scale}x.png")),
                    &difference.rgba,
                    w,
                    h,
                )?;
                let changed = web
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .zip(actual.as_chunks::<4>().0)
                    .filter(|(a, b)| a.iter().zip(b.iter()).any(|(a, b)| a.abs_diff(*b) > 8))
                    .count();
                let result = serde_json::json!({"case":prefix,"scale":scale,"mode":mode,"cases":cases.len(),"width":w,"height":h,"changed":changed,"fraction":difference.changed_fraction,"mean_error":difference.mean_error});
                println!("{result}");
                results.push(result);
            }
        }
    }
    fs::write(
        out.join("summary.json"),
        serde_json::to_vec_pretty(&results)?,
    )?;
    Ok(())
}
