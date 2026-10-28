// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Pure projection of confirmed backend attention. No preference file owns this policy.
use crate::{cockpit::UiMsg, model::Model};
use bbt_proto::{
    SeatId,
    gui::AttentionEdit,
    messaging::{MessageClass, WakeMode},
};
use bbt_ui::modules::{
    seat_row::Deferred,
    settings::messaging::{Defaults, Messaging, Rule, Seat},
};

/// Strip a selected read back to the bounded ordinary observation.
pub fn observation(value: Option<&serde_json::Value>) -> Option<serde_json::Value> {
    let value=value?;
    let mut observed=serde_json::Map::new();
    for key in ["deferred","deferred_error"] {
        if let Some(value)=value.get(key).filter(|value| !value.is_null()) { observed.insert(key.into(),value.clone()); }
    }
    (!observed.is_empty()).then_some(serde_json::Value::Object(observed))
}

pub fn deferred(value: Option<&serde_json::Value>) -> Option<Deferred> {
    let value = value?.get("deferred")?;
    let n = value.get("count")?.as_u64()?;
    let millis = value.get("next_at_ms")?.as_u64()?;
    (n > 0).then(|| Deferred {
        n,
        // The backend supplies the deadline. Formatting never advances its clock.
        at: format!(
            "{:02}:{:02} UTC",
            millis / 3_600_000 % 24,
            millis / 60_000 % 60
        ),
    })
}

pub fn view(model: Option<&Model>, focused: Option<SeatId>) -> Messaging {
    let Some(seat) = model.and_then(|model| {
        model
            .gui_seats
            .iter()
            .find(|seat| Some(SeatId::from_u128(seat.record.id.0)) == focused)
    }) else {
        return Messaging::default();
    };
    let mut out = Messaging {
        seat: Seat {
            id: seat.record.id.to_string(),
            generation: seat.record.generation,
            name: seat.record.name.clone(),
            role: seat.record.role.name.clone(),
        },
        muted: seat.record.muted,
        ..Default::default()
    };
    let selected=model.and_then(|model|model.attention_read.as_ref())
        .filter(|read|read.record.id==seat.record.id && read.record.generation==seat.record.generation && read.record.role==seat.record.role);
    let Some(value) = selected.and_then(|read|read.attention.as_ref()) else {
        out.error = Some("Messaging policy is not loaded for this seat.".into());
        out.seat.id.clear();
        return out;
    };
    match serde_json::from_value::<Defaults>(value["defaults"].clone()) {
        Ok(defaults) => out.defaults = defaults,
        Err(_) => {
            out.error = Some("The backend supplied an invalid messaging policy.".into());
            out.seat.id.clear();
            return out;
        }
    }
    out.channels = value["channels"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|path| path.as_str()?.strip_prefix("channels/"))
        .map(|path| format!("#{path}"))
        .collect();
    for rule in seat
        .record
        .attention
        .as_ref()
        .into_iter()
        .flat_map(|policy| &policy.rules)
    {
        let destination = if rule.destination == format!("seats/{}/inbox", seat.record.id) {
            "direct".into()
        } else if let Some(channel) = rule.destination.strip_prefix("channels/") {
            format!("#{channel}")
        } else {
            rule.destination.clone()
        };
        out.rules.push(Rule {
            destination,
            class: serde_json::to_value(rule.class)
                .unwrap()
                .as_str()
                .unwrap()
                .into(),
            mode: serde_json::to_value(rule.mode)
                .unwrap()
                .as_str()
                .unwrap()
                .into(),
        });
    }
    out.deferred = deferred(seat.attention.as_ref());
    out.error = seat.attention.as_ref().and_then(|observed|observed["deferred_error"].as_str()).map(str::to_owned);
    if value["channels_complete"] == false {
        out.error = Some("More channels are available; this view shows the first 128.".into());
    }
    out
}

pub fn command(view: &Messaging, rule: Rule) -> Result<UiMsg, String> {
    let seat = view
        .seat
        .id
        .parse()
        .map_err(|_| "Messaging has no current seat")?;
    if view.seat.generation == 0 {
        return Err("Messaging has no current generation".into());
    }
    let class = match rule.class.as_str() {
        "conversation" => MessageClass::Conversation,
        "event" => MessageClass::Event,
        _ => return Err("Invalid messaging class".into()),
    };
    let mode = match rule.mode.as_str() {
        "wake" => Some(WakeMode::Wake),
        "inbox" => Some(WakeMode::Inbox),
        "digest" => Some(WakeMode::Digest),
        "default" => None,
        _ => return Err("Invalid messaging mode".into()),
    };
    Ok(UiMsg::SetAttention {
        id: bbt_mux::Ulid::new(),
        seat,
        edit: AttentionEdit {
            generation: view.seat.generation,
            destination: rule.destination,
            class,
            mode,
        },
    })
}
