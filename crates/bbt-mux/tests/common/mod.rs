//! Every capture fixture owns its supervisor journal outside its seat roots.
pub fn capture_config(root: &std::path::Path) -> bbt_mux::Config {
    bbt_mux::Config {
        journal_directory: Some(root.join("supervisor-journal")),
        ..Default::default()
    }
}
