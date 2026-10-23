//! Providers yield supervisor links. The local provider adds no transport.
pub mod fake;
mod gui;
pub mod shared;
use std::{collections::HashMap, time::Duration};

use bbt_supervisor::SupervisorConnection;
use serde::{Deserialize, Serialize};

use crate::{
    Action, Event, Reply, SeatInfo,
    registry::Location,
    store::{Result, invalid},
};

pub type ProviderId = String;

/// The same namespace as SeatRecord.location, without changing its wire form.
/// A cloud host and an ssh host with the same label cannot alias one link.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HostId {
    Local,
    Ssh { host: String },
    Cloud { provider: String, host: String },
}
impl From<&Location> for HostId {
    fn from(location: &Location) -> Self {
        match location {
            Location::Local => Self::Local,
            Location::Ssh { host } => Self::Ssh { host: host.clone() },
            Location::Cloud { provider, host } => Self::Cloud {
                provider: provider.clone(),
                host: host.clone(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderKind {
    Local,
    Ssh,
    Cloud { name: String },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkState {
    Offline,
    Connecting,
    Connected { rtt_ms: u32 },
    Degraded { rtt_ms: u32 },
    Disconnected { reason: String },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Host {
    pub id: HostId,
    pub provider: ProviderId,
    pub label: String,
    pub region: Option<String>,
    pub state: LinkState,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderEvent {
    HostState { host: HostId, state: LinkState },
}
pub trait Provider: Send {
    fn id(&self) -> ProviderId;
    fn kind(&self) -> ProviderKind;
    fn hosts(&self) -> Vec<Host>;
    /// Public display metadata, never a credential or a network dial target.
    fn address(&self, _host: &HostId) -> Option<String> {
        None
    }
    fn link(&mut self, host: HostId) -> Result<Box<dyn SupervisorConnection>>;
    /// Called by logic; providers own no polling thread.
    fn poll(&mut self) -> Vec<ProviderEvent>;
}

pub struct LocalProvider {
    link: Option<Box<dyn SupervisorConnection>>,
}
impl LocalProvider {
    pub fn new(link: impl SupervisorConnection + 'static) -> Self {
        Self {
            link: Some(Box::new(link)),
        }
    }
}
impl Provider for LocalProvider {
    fn id(&self) -> ProviderId {
        "local".into()
    }
    fn kind(&self) -> ProviderKind {
        ProviderKind::Local
    }
    fn hosts(&self) -> Vec<Host> {
        vec![Host {
            id: HostId::Local,
            provider: self.id(),
            label: "this Mac".into(),
            region: None,
            state: LinkState::Connected { rtt_ms: 0 },
        }]
    }
    fn link(&mut self, host: HostId) -> Result<Box<dyn SupervisorConnection>> {
        if host != HostId::Local {
            return Err(invalid("unknown local supervisor host"));
        }
        // The existing admitted client/lease is moved exactly once into the map.
        self.link
            .take()
            .ok_or_else(|| invalid("local supervisor link already connected"))
    }
    fn poll(&mut self) -> Vec<ProviderEvent> {
        Vec::new()
    }
}

/// One link per host, owned and polled by the existing logic worker.
pub struct Connections {
    providers: Vec<Box<dyn Provider>>,
    links: HashMap<HostId, Box<dyn SupervisorConnection>>,
}
impl Connections {
    pub fn open(mut providers: Vec<Box<dyn Provider>>) -> Result<Self> {
        let mut links = HashMap::new();
        let mut ids = std::collections::HashSet::new();
        for provider in &mut providers {
            let id = provider.id();
            if id.is_empty() || !ids.insert(id.clone()) {
                return Err(invalid("duplicate or empty provider id"));
            }
            for host in provider.hosts() {
                if host.provider != id || links.contains_key(&host.id) {
                    return Err(invalid("duplicate host or wrong provider owner"));
                }
                let link = provider.link(host.id.clone())?;
                links.insert(host.id, link);
            }
        }
        if !links.contains_key(&HostId::Local) {
            return Err(invalid("local supervisor host is required"));
        }
        Ok(Self { providers, links })
    }
    pub fn request(&mut self, host: &HostId, action: Action) -> Result<Reply> {
        self.links
            .get_mut(host)
            .ok_or_else(|| invalid("unknown supervisor host"))?
            .request(action)
    }
    pub fn host_ids(&self) -> Vec<HostId> {
        let mut hosts: Vec<_> = self
            .links
            .keys()
            .filter(|h| **h != HostId::Local)
            .cloned()
            .collect();
        hosts.sort();
        hosts.insert(0, HostId::Local);
        hosts
    }
    pub fn owned_seats(&mut self) -> Result<Vec<(HostId, SeatInfo)>> {
        let mut owned = Vec::new();
        for host in self.host_ids() {
            let reply = match self.request(&host, Action::ListOwnedSeats) {
                Err(error) if host != HostId::Local && fake::is_link_down(&error) => continue,
                result => result?,
            };
            if let Reply::Seats { seats } = reply {
                owned.extend(seats.into_iter().map(|seat| (host.clone(), seat)));
            }
        }
        Ok(owned)
    }
    /// Preserve one local event and its 10 ms wait per old worker pass. Other
    /// links each get a nonblocking turn, so an idle remote cannot add a wait.
    pub fn receive(&mut self) -> Result<Vec<(HostId, Event)>> {
        let mut events = Vec::new();
        for host in self.host_ids() {
            if let Some(event) = self.receive_from(&host)? {
                events.push((host, event));
            }
        }
        Ok(events)
    }
    pub fn receive_from(&mut self, host: &HostId) -> Result<Option<Event>> {
        let wait = if *host == HostId::Local {
            Duration::from_millis(10)
        } else {
            Duration::ZERO
        };
        self.links
            .get_mut(host)
            .ok_or_else(|| invalid("unknown supervisor host"))?
            .receive(wait)
    }
    /// Poll notifications while replay work is already ready to run.
    pub fn receive_ready_from(&mut self, host: &HostId) -> Result<Option<Event>> {
        self.links
            .get_mut(host)
            .ok_or_else(|| invalid("unknown supervisor host"))?
            .receive(Duration::ZERO)
    }
    pub fn poll(&mut self) -> Vec<ProviderEvent> {
        self.providers
            .iter_mut()
            .flat_map(|provider| provider.poll())
            .collect()
    }
    pub fn hosts(&self) -> Vec<Host> {
        self.providers
            .iter()
            .flat_map(|provider| provider.hosts())
            .collect()
    }
    pub fn gui_hosts(&self) -> Vec<bbt_proto::GuiHost> {
        self.providers
            .iter()
            .flat_map(|provider| {
                provider.hosts().into_iter().map(move |host| {
                    let address = provider.address(&host.id);
                    let mut view: bbt_proto::GuiHost = host.into();
                    view.address = address;
                    view
                })
            })
            .collect()
    }
}
