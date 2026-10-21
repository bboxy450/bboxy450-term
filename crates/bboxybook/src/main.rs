use bboxybook::{Result,gallery,parity};
use std::{path::PathBuf,fs};
fn value(args:&[String],key:&str)->Option<String>{args.iter().position(|s|s==key).and_then(|i|args.get(i+1)).cloned()}
fn main(){if let Err(error)=run(){eprintln!("bboxybook: {error}");std::process::exit(1);}}
fn run()->Result<()>{
    let args:Vec<String>=std::env::args().skip(1).collect();let command=args.first().map(String::as_str).unwrap_or("gallery");
    if command=="manifest"{println!("{}",serde_json::to_string_pretty(&gallery::cases())?);return Ok(());}
    if command=="gallery"{return bboxybook::window::run();}
    let mut cases=if let Some(path)=value(&args,"--manifest"){serde_json::from_str(&fs::read_to_string(path)?)?}else{gallery::cases()};
    if let Some(component)=value(&args,"--component"){cases.retain(|c|c.component==component);}
    if let Some(limit)=value(&args,"--limit"){cases.truncate(limit.parse()?);}
    match command{
        "render"=>{let out=PathBuf::from(value(&args,"--out").unwrap_or_else(||"target/bboxybook".into()));parity::render(&out,&cases)?;let root=out.join("parity");fs::create_dir_all(&root)?;fs::write(root.join("index.html"),"<!doctype html><meta charset=utf-8><title>bboxybook renders</title><h1>Native matrix</h1><p>Run scripts/parity.sh for the web | native | diff review.</p>".to_owned()+&cases.iter().map(|c|format!("<p><a href='../{}'>{}</a></p>",c.relative(),c.relative())).collect::<String>())?;},
        "compare"=>parity::compare(&PathBuf::from(value(&args,"--out").unwrap_or_else(||"target/parity".into())),&cases)?,
        "sidebar"|"showcase"=>{let component=if command=="showcase"{format!("showcase-{}",value(&args,"--scene").unwrap_or_else(||"3".into()))}else{"sidebar".into()};cases.retain(|c|c.component==component);parity::render(&PathBuf::from("target/bboxybook"),&cases)?;},
        _=>return Err("usage: bboxybook [gallery | manifest | render --all [--out DIR] [--manifest JSON] [--component ID] | compare --out DIR]".into()),
    }Ok(())
}
