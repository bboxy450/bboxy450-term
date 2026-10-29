// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Opaque host-signed portable intent; native providers retain their own representation.
use crate::knowledge::{WriteContext, data_source as d};
use base64::{Engine, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signature, Signer};
use serde_json::{Value, json};

fn error(code: &str, message: impl ToString) -> d::CommandError {
    d::CommandError::new(code, message.to_string())
}
fn payload(request: &d::WriteRequest, context: &WriteContext<'_>) -> Value {
    json!({"version":1,"request":request,"authorship":context.author.public})
}
// Adapted from bboxy450-term, crates/bbt-mux/src/index.rs, Index::write_inner:
// https://github.com/bboxy450/bboxy450-term/blob/main/crates/bbt-mux/src/index.rs
// Changes: DSSE signs a portable intent under a distinct domain, rather than document digests.
fn pae(payload: &Value) -> d::Result<Vec<u8>> {
    let bytes = serde_json::to_vec(payload).map_err(|e| error("invalid_request", e))?;
    let domain = "bboxy/data-source-write/v1";
    let mut pae = format!("DSSEv1 {} {domain} {} ", domain.len(), bytes.len()).into_bytes();
    pae.extend(bytes);
    Ok(pae)
}
pub(super) fn sign(request: &d::WriteRequest, context: &WriteContext<'_>) -> d::Result<String> {
    let payload = payload(request, context);
    let signature = STANDARD.encode(context.author.key.sign(&pae(&payload)?).to_bytes());
    serde_json::to_string(&json!({"payload":payload,"signature":signature}))
        .map_err(|e| error("invalid_request", e))
}
pub(super) fn verify(
    proof: &str,
    request: &d::WriteRequest,
    context: &WriteContext<'_>,
) -> d::Result<()> {
    let value: Value =
        serde_json::from_str(proof).map_err(|e| error("signed_record_mismatch", e))?;
    if value["payload"] != payload(request, context) {
        return Err(error(
            "signed_record_mismatch",
            "signed intent differs from admitted request",
        ));
    }
    let sig = value["signature"]
        .as_str()
        .ok_or_else(|| error("invalid_signature", "signature missing"))?;
    let bytes = STANDARD
        .decode(sig)
        .map_err(|e| error("invalid_signature", e))?;
    let signature = Signature::from_slice(&bytes).map_err(|e| error("invalid_signature", e))?;
    context
        .author
        .key
        .verifying_key()
        .verify_strict(&pae(&value["payload"])?, &signature)
        .map_err(|e| error("invalid_signature", e))
}
pub(super) fn matches(request: &d::WriteRequest, record: &d::Record) -> d::Result<()> {
    let expected_id = request.frontmatter["id"].as_str();
    if record.parse_error.is_some()
        || expected_id != Some(record.reference.id.as_str())
        || record.body != request.body
        || record.etag.is_empty()
        || record.reference.digest.is_empty()
        || request.reference.as_ref().is_some_and(|old| {
            old.id != record.reference.id || old.external_id != record.reference.external_id
        })
        || request.frontmatter.as_object().is_none_or(|fields| {
            fields
                .iter()
                .any(|(key, value)| record.frontmatter.get(key) != Some(value))
        })
    {
        return Err(error(
            "signed_record_mismatch",
            "stored portable fields differ from the host intent",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    // keeps: #350 signed-writer bridge — private key stays in the host and tampering is refused.
    #[test]
    fn intent_signature_covers_author_request_and_body() {
        let key = ed25519_dalek::SigningKey::from_bytes(&[13; 32]);
        let context = WriteContext::seat(crate::Ulid::new(), Some(7), &key);
        let mut request = d::WriteRequest {
            reference: None,
            path: Some("notes/one.md".into()),
            frontmatter: json!({"id":crate::Ulid::new(),"type":"note"}),
            body: "original".into(),
            etag: None,
        };
        let proof = sign(&request, &context).unwrap();
        verify(&proof, &request, &context).unwrap();
        request.body = "changed".into();
        assert!(verify(&proof, &request, &context).is_err());
        request.body = "original".into();
        let other = WriteContext::seat(crate::Ulid::new(), Some(7), &key);
        assert!(verify(&proof, &request, &other).is_err());
    }
}
