//! Portable provider records. Transport and custody stay in logic.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum HostState {
    Offline,
    Connecting,
    Connected { rtt_ms: u32 },
    Degraded { rtt_ms: u32 },
    Disconnected { reason: String },
}
impl HostState {
    pub fn connected(&self) -> bool {
        matches!(self, Self::Connected { .. } | Self::Degraded { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuiHost {
    #[serde(with = "crate::seat_state::location")]
    pub location: crate::Where,
    pub provider: String,
    pub label: String,
    pub address: Option<String>,
    pub region: Option<String>,
    pub state: HostState,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn host_states_and_namespaced_locations_round_trip_without_mux_types() {
        for location in [
            crate::Where::Local,
            crate::Where::Ssh {
                host: "build-box".into(),
            },
            crate::Where::Cloud {
                provider: "nebula".into(),
                host: "worker-east".into(),
            },
        ] {
            for state in [
                HostState::Offline,
                HostState::Connecting,
                HostState::Connected { rtt_ms: 40 },
                HostState::Degraded { rtt_ms: 80 },
                HostState::Disconnected {
                    reason: "fixture link dropped".into(),
                },
            ] {
                let host = GuiHost {
                    location: location.clone(),
                    provider: "fixture".into(),
                    label: "Build box".into(),
                    address: Some("agent@build-box.invalid".into()),
                    region: None,
                    state,
                };
                let value = serde_json::to_value(crate::GuiReply::HostState { host: host.clone() })
                    .unwrap();
                let crate::GuiReply::HostState { host: decoded } =
                    serde_json::from_value(value).unwrap()
                else {
                    panic!("host reply kind changed")
                };
                assert_eq!(decoded, host);
            }
        }
        let value = serde_json::to_value(crate::GuiAction::ListHosts).unwrap();
        assert!(matches!(
            serde_json::from_value::<crate::GuiAction>(value).unwrap(),
            crate::GuiAction::ListHosts
        ));
    }
}
