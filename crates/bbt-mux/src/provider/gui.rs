use super::{Host, HostId, LinkState};

impl From<&bbt_proto::Where> for HostId {
    fn from(location: &bbt_proto::Where) -> Self {
        match location {
            bbt_proto::Where::Local => Self::Local,
            bbt_proto::Where::Ssh { host } => Self::Ssh { host: host.clone() },
            bbt_proto::Where::Cloud { provider, host } => Self::Cloud {
                provider: provider.clone(),
                host: host.clone(),
            },
        }
    }
}

impl From<Host> for bbt_proto::GuiHost {
    fn from(host: Host) -> Self {
        let location = match host.id {
            HostId::Local => bbt_proto::Where::Local,
            HostId::Ssh { host } => bbt_proto::Where::Ssh { host },
            HostId::Cloud { provider, host } => bbt_proto::Where::Cloud { provider, host },
        };
        let state = host.state.into();
        Self {
            location,
            state,
            provider: host.provider,
            label: host.label,
            region: host.region,
            address: None,
        }
    }
}

impl From<LinkState> for bbt_proto::HostState {
    fn from(state: LinkState) -> Self {
        match state {
            LinkState::Offline => bbt_proto::HostState::Offline,
            LinkState::Connecting => bbt_proto::HostState::Connecting,
            LinkState::Connected { rtt_ms } => bbt_proto::HostState::Connected { rtt_ms },
            LinkState::Degraded { rtt_ms } => bbt_proto::HostState::Degraded { rtt_ms },
            LinkState::Disconnected { reason } => bbt_proto::HostState::Disconnected { reason },
        }
    }
}
