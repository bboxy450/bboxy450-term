use bboxybook::{Result, gallery, parity};
use std::{fs, path::PathBuf};
fn value(args: &[String], key: &str) -> Option<String> {
    args.iter()
        .position(|s| s == key)
        .and_then(|i| args.get(i + 1))
        .cloned()
}
fn main() {
    if let Err(error) = run() {
        eprintln!("bboxybook: {error}");
        std::process::exit(1);
    }
}
fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("gallery");
    if command == "panes" {
        return bboxybook::panes::render(&PathBuf::from(
            value(&args, "--out").unwrap_or_else(|| "target/bboxybook/panes".into()),
        ));
    }
    if command == "swarms" {
        return bboxybook::swarms::render(&PathBuf::from(
            value(&args, "--out").unwrap_or_else(|| "target/bboxybook/swarms".into()),
        ));
    }
    if command == "tabs" {
        return bboxybook::tabs::render(&PathBuf::from(
            value(&args, "--out").unwrap_or_else(|| "target/bboxybook/tabs".into()),
        ));
    }
    if command == "seat-rows" {
        return bboxybook::seat_rows::render(&PathBuf::from(
            value(&args, "--out").unwrap_or_else(|| "target/bboxybook/seat-rows".into()),
        ));
    }
    if command == "overlays" {
        if args.iter().any(|s| s == "--native-only") {
            return bboxybook::overlays::render_native_gallery(&PathBuf::from(
                value(&args, "--out").unwrap_or_else(|| "target/bboxybook/overlays/gallery".into()),
            ));
        }
        return bboxybook::overlays::render(&PathBuf::from(
            value(&args, "--out").unwrap_or_else(|| "target/bboxybook/overlays".into()),
        ));
    }
    if command == "widgets" {
        return bboxybook::widgets::render(&PathBuf::from(
            value(&args, "--out").unwrap_or_else(|| "target/bboxybook/widgets".into()),
        ));
    }
    if command == "components" {
        return bboxybook::components::render(&PathBuf::from(
            value(&args, "--out").unwrap_or_else(|| "target/bboxybook/components".into()),
        ));
    }
    if command == "atoms" {
        return bboxybook::atoms::render(&PathBuf::from(
            value(&args, "--out").unwrap_or_else(|| "target/bboxybook/atoms".into()),
        ));
    }
    if command == "materials" {
        return bboxybook::materials::render(&PathBuf::from(
            value(&args, "--out").unwrap_or_else(|| "target/bboxybook/materials".into()),
        ));
    }
    if command == "styles" {
        return bboxybook::styles::render(&PathBuf::from(
            value(&args, "--out").unwrap_or_else(|| "target/bboxybook/style".into()),
        ));
    }
    if command == "manifest" {
        println!("{}", serde_json::to_string_pretty(&gallery::cases())?);
        return Ok(());
    }
    if command == "gallery" {
        return bboxybook::window::run(args.iter().any(|a| a == "--smoke"));
    }
    let mut cases = if let Some(path) = value(&args, "--manifest") {
        serde_json::from_str(&fs::read_to_string(path)?)?
    } else {
        gallery::cases()
    };
    if let Some(component) = value(&args, "--component") {
        cases.retain(|c| c.component == component);
    }
    if let Some(limit) = value(&args, "--limit") {
        cases.truncate(limit.parse()?);
    }
    match command{
        "render"=>{
            let explicit=value(&args,"--out");
            let out=PathBuf::from(explicit.clone().unwrap_or_else(||"target/bboxybook".into()));
            let report=out.join("parity");
            if explicit.is_none() {
                let shared=PathBuf::from("target/parity");
                if shared.join("cases.json").exists() {
                    let reference:Vec<gallery::Case>=serde_json::from_str(&fs::read_to_string(shared.join("cases.json"))?)?;
                    for case in &mut cases {if let Some(r)=reference.iter().find(|r|r.relative()==case.relative()){case.width=r.width;case.height=r.height;case.fixture=r.fixture.clone();}}
                    println!("Using existing target/parity web captures; scripts/parity.sh refreshes them.");
                    parity::render(&out,&cases)?;parity::compare_paths(&report,&cases,"../../parity/web","..")?;
                    if shared.join("capture.json").exists(){fs::copy(shared.join("capture.json"),report.join("capture.json"))?;}
                } else {
                    fs::create_dir_all(&report)?;
                    fs::write(report.join("requested.json"),serde_json::to_vec_pretty(&cases)?)?;
                    let status=std::process::Command::new("node").arg("crates/bboxybook/capture.mjs").arg(&report).arg(report.join("requested.json")).status()?;
                    if !status.success(){return Err("web capture failed; inspect capture.json".into());}
                    cases=serde_json::from_str(&fs::read_to_string(report.join("cases.json"))?)?;
                    parity::render(&out,&cases)?;parity::compare_paths(&report,&cases,"web","..")?;
                }
            } else { parity::render(&out,&cases)?; }
        },
        "compare"=>parity::compare_paths(&PathBuf::from(value(&args,"--out").unwrap_or_else(||"target/parity".into())),&cases,&value(&args,"--web-root").unwrap_or_else(||"web".into()),&value(&args,"--native-root").unwrap_or_else(||"native".into()))?,
        "sidebar"|"showcase"=>{let component=if command=="showcase"{format!("showcase-{}",value(&args,"--scene").unwrap_or_else(||"3".into()))}else{"sidebar".into()};cases.retain(|c|c.component==component);parity::render(&PathBuf::from("target/bboxybook"),&cases)?;},
        _=>return Err("usage: bboxybook [gallery | panes | swarms | tabs | seat-rows | components | widgets | overlays | atoms | materials | styles | manifest | render --all [--out DIR] [--manifest JSON] [--component ID] | compare --out DIR]".into()),
    }
    Ok(())
}
