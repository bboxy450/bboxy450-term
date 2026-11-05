// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Read-only host provenance, independent of the selected storage plugin.
// Inspired by DataHub's search/entity/lineage split and Sigstore's separate
// lookup and verification operations. Independently implemented.
// https://docs.datahub.com/docs/features/feature-guides/mcp
// https://docs.sigstore.dev/logging/cli/
use serde_json::{Value, json};

pub(crate) fn handles(tool: &str) -> bool {
    matches!(
        tool,
        "provenance_search"
            | "provenance_get"
            | "provenance_lineage"
            | "provenance_history"
            | "provenance_verify"
    )
}

pub(crate) fn catalogue() -> Vec<Value> {
    let locator = json!({"type":"string","minLength":1,"maxLength":4096,"description":"Native record ID, #number, root-relative path/slug, artifact ref or recorded digest. For external-provider documents, fetch their checkpoint artifact ID with knowledge_fetch first."});
    let cursor = json!({"type":"string","maxLength":8192});
    let limit = json!({"type":"integer","minimum":1,"maximum":64});
    let entries = [
        (
            "provenance_search",
            "Find recorded entities in this workspace's native provenance snapshot by text, exact digest, producer seat or commit. Workers see their own records; Oracle sees this root. Results are bounded and cursor-bound to this actor/root/query/snapshot. Use provenance_get for checkpoint/publication state. Reads never rescan files, invoke Git or repair projections.",
            json!({"query":{"type":"string","maxLength":1024},"digest":{"type":"string","maxLength":128},"seat_id":{"type":"string"},"commit":{"type":"string","maxLength":256},"cursor":cursor,"limit":limit}),
            json!([]),
        ),
        (
            "provenance_get",
            "Explain one native entity: its recorded producer/claim, actual document signer metadata, committed checkpoint, direct provenance links and projection freshness. Recorded metadata is not a verification result. External artifact bytes and remote Git signatures are not fetched.",
            json!({"locator":locator}),
            json!(["locator"]),
        ),
        (
            "provenance_lineage",
            "Walk recorded input/output lineage upstream or downstream with cycle, depth and node bounds. Edges come from declared derived_from/used/evidence/revision/generated links, not proven causation. A truncated result includes a frontier for further bounded lookups. Hidden worker records are not exposed.",
            json!({"locator":locator,"direction":{"type":"string","enum":["upstream","downstream"]},"depth":{"type":"integer","minimum":1,"maximum":8},"limit":limit}),
            json!(["locator"]),
        ),
        (
            "provenance_history",
            "Page the canonical signed doc/artifact/task events for one entity, beyond the inline 32-entry history. Cursor pins the event head and actor/root/query; later appends are outside that snapshot. Returned rows verify signatures, stored columns and immediate predecessor links. This does not audit the entire historical chain or certify the signer's account authority.",
            json!({"locator":locator,"cursor":cursor,"limit":limit}),
            json!(["locator"]),
        ),
        (
            "provenance_verify",
            "Verify a cached native document signature and a selected canonical entity event (default: latest recorded observation/write anchor). Reports signature/column/local-link integrity separately from producer attribution, account-key trust, full-history audit and external artifact bytes. No Git/network/filesystem artifact hashing and no repair.",
            json!({"locator":locator,"sequence":{"type":["integer","string"],"minimum":1,"minLength":1,"maxLength":20,"pattern":"^[1-9][0-9]*$","description":"Canonical sequence; accepts the decimal string returned by provenance_history without loss of precision."}}),
            json!(["locator"]),
        ),
    ];
    entries.into_iter().map(|(name, description, properties, required)| json!({
        "name":name,"description":description,
        "inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false},
        "outputSchema":{"type":"object","additionalProperties":true},
        "annotations":{"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}
    })).collect()
}
