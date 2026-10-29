// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The operator identity hold and the connection's issuer label.
use super::*;

/// Public signing state; custody and secret key material never cross the GUI socket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GuiIdentityState {
    LocalDev,
    Pending,
    Certified,
    Held,
}

/// The compact identity note kept in the app's immutable model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuiIdentity {
    pub state: GuiIdentityState,
    pub reason: Option<String>,
    /// Internal verified test-run context, never inferred from display text.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "private_test_run"
    )]
    pub private_test_run: Option<Ulid>,
}

fn private_test_run<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Ulid>, D::Error> {
    let id = Option::<Ulid>::deserialize(d)?;
    if id == Some(Ulid::nil()) {
        return Err(serde::de::Error::custom("private test run must be nonnil"));
    }
    Ok(id)
}

/// This noun's GuiAction variants, spliced into the flat enum by `flat_enum` in `gui/mod.rs`.
macro_rules! identity_actions {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            /// The hold's two operator controls; target identity is derived in logic.
            Identity(crate::identity::IdentityCall),
            /// Who issues this connection's operator requests (decision #269): `ctl --as LABEL`.
            /// Attribution only, recorded on the envelopes the operator principal signs; it grants
            /// nothing.
            Issuer {
                label: String,
            },
        ] $($head)*);
    };
}
pub(super) use identity_actions;

/// This noun's GuiReply variants, spliced into the flat enum by `flat_enum` in `gui/mod.rs`.
macro_rules! identity_replies {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            Identity(GuiIdentity),
            /// A correlated control response, separate from the asynchronous identity note.
            IdentityControl {
                op: Ulid,
                response: crate::identity::IdentityControl,
            },
        ] $($head)*);
    };
}
pub(super) use identity_replies;
