// Project integration: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// Fork source: https://github.com/Gonzih/kittens-old/tree/293e134526bb2deee96f1197d6268aee64495482
// Existing MIT OR Apache-2.0 terms and Kittens contributor notices apply.
// Credit bboxy450 integration accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(dead_code)]

async fn raw_runtime_bypass() {
    let (_sender, mut receiver) = tokio::sync::mpsc::channel::<u8>(1);
    tokio::select! {
        biased;
        _ = receiver.recv() => {}
        _ = core::future::pending::<()>() => {}
    }
    let handle = tokio::spawn(async {});
    drop(handle);
}

fn main() {}
