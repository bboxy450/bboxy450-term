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
