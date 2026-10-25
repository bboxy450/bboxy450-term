//! Every capture fixture owns its supervisor journal outside its seat roots.
pub fn capture_config(root: &std::path::Path) -> bbt_mux::Config {
    bbt_mux::Config {
        journal_directory: Some(root.join("supervisor-journal")),
        ..Default::default()
    }
}

/// Successful leaf pages, independent of how many share one transport reply.
#[allow(dead_code)] // This common module is also used by non-replay fixtures.
pub fn ring_histories(reply: &bbt_mux::Reply) -> Vec<&bbt_mux::ring::History> {
    match reply {
        bbt_mux::Reply::RingHistory { history } => vec![history],
        bbt_mux::Reply::RingHistoryBatch { results } => results
            .iter()
            .filter_map(|row| row.result.as_deref().ok())
            .collect(),
        _ => vec![],
    }
}
