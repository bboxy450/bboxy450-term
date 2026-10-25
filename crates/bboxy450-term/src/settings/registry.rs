//! G3/G4 presentation only. Production registry/trust and provider switching
//! are later backend cuts; these records never authorize or start a plugin.
use bbt_ui::modules::settings::{
    Records,
    knowledge::{Knowledge, Root},
    plugins::Plugin,
};
use std::sync::LazyLock;

// Wren's signed 4edc61ba public reference shapes. The page labels every one as
// example data, including its timings and process-health words.
static REFERENCE: LazyLock<Vec<Plugin>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("plugin-reference.json"))
        .expect("pinned public plugin fixtures")
});
pub(super) fn project(out: &mut Records, model: Option<&crate::model::Model>) {
    out.plugins = REFERENCE.clone();
    for plugin in &mut out.plugins {
        plugin.health.say = Some(plugin.health.say.as_ref().map_or_else(
            || "reference fixture".into(),
            |detail| format!("reference fixture · {detail}"),
        ));
    }
    out.plugin_notice = Some("Reference fixtures — these plugins and timings are examples. Plugin discovery, trust, enable and setup are unavailable in this build.".into());
    let snapshot = model.and_then(|model| model.e2e.as_ref());
    let (answering, read) = match snapshot {
        Some(snapshot) if snapshot.faults.is_empty() => ("logic · in process", String::new()),
        Some(snapshot) => (
            "bundled read unavailable",
            format!(" {}", snapshot.faults.join(" · ")),
        ),
        None => (
            "bundled read snapshot pending",
            " The current read snapshot is not available yet.".into(),
        ),
    };
    let provider = "bboxy450-term-data_source-bboxy";
    out.knowledge = Knowledge {
        roots: vec![Root {
            name: ".bboxy".into(),
            path: ".bboxy/ (current repository)".into(),
            provider: provider.into(),
            // The current read snapshot contains acceptance records, not an
            // inventory of the entire root. Never present it as a root count.
            records: None,
            answering: answering.into(),
            digests: None,
            refused: Some(format!(
                "Provider switching is unavailable in this build; the bundled provider remains active.{read}"
            )),
        }],
        offered: vec![provider.into()],
    };
}
