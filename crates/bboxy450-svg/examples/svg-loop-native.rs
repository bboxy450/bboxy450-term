// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Private loop CLI. Native preparation is also used for negative inputs; those
//! inputs never reach Chrome or the independent shape interpreter.
#![allow(
    clippy::disallowed_methods,
    reason = "offline verification CLI owns synchronous file IO; no UI event loop"
)]
use bboxy450_svg::{FontSet, Layer, MAX_SVG_BYTES, RenderRequest, prepare};
use std::{error::Error, fs, io::Read, path::Path};

fn read_bounded(path: &str, maximum: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let file = fs::File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > maximum as u64 {
        return Err("input is not a bounded regular file".into());
    }
    let mut bytes = Vec::new();
    file.take(maximum as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > maximum {
        return Err("input grew past byte ceiling".into());
    }
    Ok(bytes)
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|op| op == "validate-view") && args.len() == 3 {
        let request: RenderRequest = serde_json::from_slice(&read_bounded(&args[2], 65_536)?)?;
        println!(
            "{}",
            serde_json::json!({"dimensions":request.dimensions()?})
        );
        return Ok(());
    }
    if args.get(1).is_some_and(|op| op == "decode-png") && args.len() == 4 {
        let limit: u64 = args[3].parse()?;
        let image = bbt_render::image::decode_png(
            &read_bounded(&args[2], bbt_render::image::MAX_PNG_BYTES)?,
            limit,
        )?;
        println!(
            "{}",
            serde_json::json!({"width":image.width,"height":image.height,"decoded_pixels":u64::from(image.width)*u64::from(image.height)})
        );
        return Ok(());
    }
    if args.len() < 4 {
        return Err(
            "usage: svg-loop-native prepare|vectors|text|image|render svg fonts|none [request.json output-prefix]"
                .into(),
        );
    }
    let bytes = read_bounded(&args[2], MAX_SVG_BYTES)?;
    let fonts = if args[3] == "none" {
        None
    } else {
        Some(FontSet::load(Path::new(&args[3]))?)
    };
    let scene = match prepare(&bytes, fonts.as_ref()) {
        Ok(scene) => scene,
        Err(diagnostics) => {
            println!(
                "{}",
                serde_json::json!({"status":"rejected","diagnostics":diagnostics})
            );
            return Err("native preparation rejected input".into());
        }
    };
    match args[1].as_str() {
        "prepare" => println!(
            "{}",
            serde_json::json!({"status":"prepared","view_box":scene.view_box(),"viewport":scene.viewport(),"decoded_pixels":scene.decoded_pixels(),"font_set":fonts.as_ref().map(FontSet::identity)})
        ),
        "vectors" | "render" | "text" | "image" if args.len() == 6 => {
            let request: RenderRequest = serde_json::from_slice(&read_bounded(&args[4], 65_536)?)?;
            request.dimensions()?;
            let mut renderer = bbt_render::chrome::ChromeRenderer::new(Vec::<
                bbt_render::chrome::icons::Image,
            >::new())?;
            let layer = match args[1].as_str() {
                "vectors" => Layer::Vectors,
                "text" => Layer::Text,
                "image" => Layer::Image,
                _ => Layer::Render,
            };
            let capture = scene.render(&mut renderer, request, layer)?;
            let png = format!("{}.png", args[5]);
            let mut encoder =
                png::Encoder::new(fs::File::create(&png)?, capture.width, capture.height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header()?.write_image_data(&capture.rgba)?;
            let evidence = serde_json::json!({"status":if args[1]=="vectors" {"rendered_vectors"} else {"rendered"},"viewport":capture.mapping.viewport,"view_transform":capture.mapping.transform,"layer":args[1],"width":capture.width,"height":capture.height,"request":request,"geometry":capture.geometry,"texts":capture.texts,"font_set":fonts.as_ref().map(FontSet::identity)});
            fs::write(
                format!("{}.json", args[5]),
                serde_json::to_vec_pretty(&evidence)?,
            )?;
            println!("{evidence}");
        }
        _ => return Err("unsupported native loop operation".into()),
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("svg-loop-native: {error}");
        std::process::exit(1);
    }
}
