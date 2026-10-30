// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Original-operation lookup is a fresh read, never a replay of the effect or lookup.
use super::{Call, SeatActor, journal::Journal};
use crate::Ulid;
use serde::Deserialize;
use serde_json::{Value, json};

pub(crate) fn catalogue() -> Value {
    json!({"name":"operation_status",
        "description":"Read your original knowledge/work-item write outcome by original_op_id in your current root, including after restart. Missing/incomplete results remain unknown; never redispatch an unknown effect. This fresh lookup never admits an effect or returns raw arguments.",
        "inputSchema":{"type":"object","properties":{"original_op_id":{"type":"string","pattern":"^[0-7][0-9A-HJKMNP-TV-Z]{25}$"}},"required":["original_op_id"],"additionalProperties":false},
        "outputSchema":{"type":"object","additionalProperties":true},
        "annotations":{"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}})
}

pub(crate) fn read(actor: &SeatActor, call: &Call, journal: &Journal) -> Value {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Request {
        original_op_id: Ulid,
    }
    let request = match super::tools::parse::<Request>(&call.args) {
        Ok(request) if request.original_op_id != Ulid::nil() => request,
        _ => {
            return super::error(
                call.op_id,
                "invalid_arguments",
                "original_op_id requires a non-nil operation ID",
            );
        }
    };
    match journal.operation_status(actor, request.original_op_id) {
        Ok(value) if value["status"] == "forbidden" => super::error(
            call.op_id,
            "forbidden",
            "original operation is outside current seat/root write authority",
        ),
        Ok(value) => {
            super::knowledge::bounded(call.op_id, super::success(call.op_id, value), false)
        }
        Err(_) => super::error(
            call.op_id,
            "journal_unavailable",
            "original-operation ledger unavailable or incomplete; outcome remains unknown; do not redispatch",
        ),
    }
}
