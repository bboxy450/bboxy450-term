// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use serde_json::{Value, json};

pub(crate) fn handles(tool: &str) -> bool {
    matches!(
        tool,
        "browser_open" | "browser_cdp" | "browser_screenshot" | "browser_close"
    )
}

pub(crate) fn catalogue() -> impl Iterator<Item = Value> {
    [
        ("browser_open", "Open a URL in your seat's offscreen Chromium, starting it if needed. Wait for the main document load.",
            json!({"url":{"type":"string"}}), json!(["url"])),
        ("browser_cdp", "Run a Chrome DevTools Protocol command in your open browser and drain its bounded event buffer.",
            json!({"method":{"type":"string","minLength":1,"maxLength":128},"params":{"type":"object"},"timeout_ms":{"type":"integer","minimum":1,"maximum":8000}}), json!(["method"])),
        ("browser_screenshot", "Capture an open browser's viewport or full page as a PNG artifact in your workspace.",
            json!({"full_page":{"type":"boolean"}}), json!([])),
        ("browser_close", "Close your offscreen Chromium and remove its private profile.",
            json!({}), json!([])),
    ].into_iter().map(|(name, description, properties, required)| {
        json!({
            "name":name,"description":description,
            "inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false},
            "outputSchema":{"type":"object","additionalProperties":true},
            "annotations":{"readOnlyHint":false,"destructiveHint":false,"idempotentHint":false,"openWorldHint":true}
        })
    })
}
