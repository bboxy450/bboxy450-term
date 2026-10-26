//! Receiver-owned wake decisions. Storage, mute and native delivery stay separate.

use bbt_proto::{
    messaging::{MessageClass, MessageKind, WakeMode},
    registry::{AttentionPolicy, AttentionRule, Class, Role, SeatRecord},
};
use serde::Serialize;

use super::tree::{normalize, sink};
use crate::store::{Result, invalid};

pub const MAX_RULES: usize = 128;

/// Facts resolved by the service from the admitted principal, routing and thread root.
/// These are not policy fields or sender-supplied MCP arguments.
#[derive(Debug, Clone, Copy, Default)]
pub struct Facts {
    pub operator: bool,
    pub delegated: bool,
    pub own_thread_reply: bool,
    pub addressed_by_name: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Defaults {
    pub conversation: WakeMode,
    pub direct_conversation: WakeMode,
    pub named_conversation: WakeMode,
    pub event: WakeMode,
}

pub fn defaults(role: &Role) -> Defaults {
    let (conversation, direct_conversation, named_conversation) = if matches!(role.name.as_str(), "architect" | "creativity") {
        (WakeMode::Inbox, WakeMode::Wake, WakeMode::Wake)
    } else if role.class == Class::Oracle {
        (WakeMode::Wake, WakeMode::Wake, WakeMode::Wake)
    } else {
        (WakeMode::Digest, WakeMode::Wake, WakeMode::Wake)
    };
    Defaults {
        conversation,
        direct_conversation,
        named_conversation,
        event: WakeMode::Inbox,
    }
}

pub fn destination(value: &str) -> Result<String> {
    let path = normalize(value, false)?;
    if path.is_empty() {
        return Err(invalid("attention destination must not be empty"));
    }
    Ok(path)
}

pub fn validate(policy: &AttentionPolicy) -> Result<()> {
    if policy.rules.len() > MAX_RULES {
        return Err(invalid("attention policy exceeds 128 rules"));
    }
    for (i, rule) in policy.rules.iter().enumerate() {
        if destination(&rule.destination)? != rule.destination {
            return Err(invalid("attention policy destination is not canonical"));
        }
        if policy.rules[..i]
            .iter()
            .any(|old| old.destination == rule.destination && old.class == rule.class)
        {
            return Err(invalid("duplicate attention destination and class"));
        }
    }
    Ok(())
}

pub fn updated(
    policy: Option<&AttentionPolicy>,
    mut rule: AttentionRule,
) -> Result<AttentionPolicy> {
    let mut policy = policy.cloned().unwrap_or_default();
    validate(&policy)?;
    rule.destination = destination(&rule.destination)?;
    if let Some(old) = policy
        .rules
        .iter_mut()
        .find(|old| old.destination == rule.destination && old.class == rule.class)
    {
        *old = rule;
    } else {
        if policy.rules.len() == MAX_RULES {
            return Err(invalid("attention policy exceeds 128 rules"));
        }
        policy.rules.push(rule);
    }
    Ok(policy)
}

/// A class is derived from the signed kind; a decision never changes delivery or mute.
pub fn decide(
    receiver: &SeatRecord,
    destination_path: &str,
    kind: MessageKind,
    facts: Facts,
) -> Result<WakeMode> {
    let path = destination(destination_path)?;
    if let Some(policy) = &receiver.attention {
        validate(policy)?;
    }
    let class = kind.class();
    if class == MessageClass::Conversation
        && (facts.operator || facts.delegated || facts.own_thread_reply)
    {
        return Ok(WakeMode::Wake);
    }
    if let Some(rule) = receiver.attention.as_ref().and_then(|policy| {
        policy
            .rules
            .iter()
            .find(|rule| rule.destination == path && rule.class == class)
    }) {
        return Ok(rule.mode);
    }
    let defaults = defaults(&receiver.role);
    Ok(if class == MessageClass::Event {
        defaults.event
    } else if facts.addressed_by_name {
        defaults.named_conversation
    } else if sink(&path).is_ok_and(|(seat, _)| seat == receiver.id) {
        defaults.direct_conversation
    } else {
        defaults.conversation
    })
}
