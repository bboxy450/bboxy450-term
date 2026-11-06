// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_proto::messaging::Evidence;

#[test]
fn delivery_evidence_retains_old_spellings_and_reads_future_evidence_honestly() {
    for (text, evidence) in [
        ("sender-stored", Evidence::SenderStored),
        ("recipient-stored", Evidence::RecipientStored),
        ("native-queued", Evidence::NativeQueued),
        ("native-written", Evidence::NativeWritten),
        ("native-unknown", Evidence::NativeUnknown),
        (
            "application-acknowledged",
            Evidence::ApplicationAcknowledged,
        ),
        ("native-steered", Evidence::NativeSteered),
        ("hinted", Evidence::Hinted),
    ] {
        assert_eq!(serde_json::to_value(evidence).unwrap(), text);
        assert_eq!(
            serde_json::from_value::<Evidence>(serde_json::json!(text)).unwrap(),
            evidence
        );
    }
    assert_eq!(
        serde_json::from_value::<Evidence>(serde_json::json!("future-delivery-evidence")).unwrap(),
        Evidence::NativeUnknown
    );
}
