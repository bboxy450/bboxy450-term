//! Scripted local links: fixture metadata is never used to open a network socket.
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use bbt_supervisor::SupervisorConnection;
use serde::Deserialize;

use super::{Host, HostId, LinkState, Provider, ProviderEvent, ProviderId, ProviderKind};
use crate::store::{Result, invalid};
use crate::{Action, Event, Reply};

pub const BUNDLED_FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/fixtures/providers.json"
));
/// A transition log: a trailing `/state` would coalesce different hosts and
/// intermediate transitions on the existing bus.
pub const STATE_SUBJECT: &str = "account/providers/transition";

/// A monotonic clock shared by a provider and all of its links. Tests supply a
/// manually advanced clock, so scripts and latency never require a timer thread.
pub trait Clock: Send + Sync {
    fn now_ms(&self) -> u64;
}
pub struct RealClock(Instant);
impl Default for RealClock {
    fn default() -> Self {
        Self(Instant::now())
    }
}
impl Clock for RealClock {
    fn now_ms(&self) -> u64 {
        u64::try_from(self.0.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    providers: Vec<FakeConfig>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FakeConfig {
    id: ProviderId,
    kind: ProviderKind,
    hosts: Vec<HostScript>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct HostScript {
    id: HostId,
    label: String,
    address: Option<String>,
    region: Option<String>,
    script: Vec<Transition>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Transition {
    at_ms: u64,
    state: LinkState,
}
impl Fixture {
    pub fn bundled() -> Result<Self> {
        Self::parse(BUNDLED_FIXTURE)
    }
    pub fn parse(json: &str) -> Result<Self> {
        let fixture: Self = serde_json::from_str(json)?;
        Self::validate(&fixture.providers)?;
        Ok(fixture)
    }
    fn validate(configs: &[FakeConfig]) -> Result<()> {
        let mut providers = HashSet::new();
        let mut hosts = HashSet::new();
        if configs.is_empty() {
            return Err(invalid("provider fixture is empty"));
        }
        for provider in configs {
            if provider.id.is_empty()
                || !providers.insert(&provider.id)
                || provider.hosts.is_empty()
            {
                return Err(invalid("empty or duplicate fixture provider"));
            }
            for host in &provider.hosts {
                let matches = match (&provider.kind, &host.id) {
                    (ProviderKind::Ssh, HostId::Ssh { host: name }) => {
                        !name.is_empty() && host.address.as_ref().is_some_and(|a| a.contains('@'))
                    }
                    (ProviderKind::Cloud { name }, HostId::Cloud { provider, host: id }) => {
                        !name.is_empty()
                            && name == provider
                            && !id.is_empty()
                            && host.region.as_ref().is_some_and(|r| !r.is_empty())
                    }
                    _ => false,
                };
                if !matches || host.label.is_empty() || !hosts.insert(&host.id) {
                    return Err(invalid("invalid or duplicate fixture host"));
                }
                if !host
                    .script
                    .first()
                    .is_some_and(|step| step.at_ms == 0 && step.state == LinkState::Connecting)
                    || host.script.windows(2).any(|s| s[0].at_ms >= s[1].at_ms)
                {
                    return Err(invalid(
                        "fixture script must start Connecting at zero and advance",
                    ));
                }
            }
        }
        Ok(())
    }
    pub fn into_providers(self) -> Vec<FakeConfig> {
        self.providers
    }
}

#[derive(Debug)]
pub struct LinkDown {
    pub host: HostId,
    pub state: LinkState,
}
impl std::fmt::Display for LinkDown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LinkDown: {:?}: {:?}", self.host, self.state)
    }
}
impl std::error::Error for LinkDown {}
pub fn is_link_down(error: &crate::store::Error) -> bool {
    error.downcast_ref::<LinkDown>().is_some()
}

#[derive(Clone)]
struct Script {
    host: HostId,
    start_ms: u64,
    clock: Arc<dyn Clock>,
    steps: Arc<[Transition]>,
}
impl Script {
    fn elapsed(&self) -> u64 {
        self.clock.now_ms().saturating_sub(self.start_ms)
    }
    fn state(&self) -> LinkState {
        let elapsed = self.elapsed();
        let i = self.steps.partition_point(|step| step.at_ms <= elapsed);
        self.steps[i.saturating_sub(1)].state.clone()
    }
    fn latency(&self) -> Result<u64> {
        match self.state() {
            LinkState::Connected { rtt_ms } | LinkState::Degraded { rtt_ms } => {
                Ok(u64::from(rtt_ms))
            }
            state => Err(Box::new(LinkDown {
                host: self.host.clone(),
                state,
            })),
        }
    }
}
struct ScriptedHost {
    host: Host,
    script: Script,
    emitted: usize,
    linked: bool,
    address: Option<String>,
}
type Connector = Box<dyn FnMut(&Host) -> Result<Box<dyn SupervisorConnection>> + Send>;

pub struct FakeProvider {
    id: ProviderId,
    kind: ProviderKind,
    hosts: Vec<ScriptedHost>,
    connect: Connector,
}
impl FakeProvider {
    /// The caller supplies fresh admitted *local* links, with their own journals.
    /// The factory receives fixture metadata for labels only; this module does
    /// not create sockets, processes, credentials or polling threads.
    pub fn new(
        config: FakeConfig,
        clock: Arc<dyn Clock>,
        connect: impl FnMut(&Host) -> Result<Box<dyn SupervisorConnection>> + Send + 'static,
    ) -> Result<Self> {
        Fixture::validate(std::slice::from_ref(&config))?;
        let start_ms = clock.now_ms();
        let hosts = config
            .hosts
            .into_iter()
            .map(|entry| ScriptedHost {
                host: Host {
                    id: entry.id.clone(),
                    provider: config.id.clone(),
                    label: entry.label,
                    region: entry.region,
                    state: LinkState::Connecting,
                },
                script: Script {
                    host: entry.id,
                    start_ms,
                    clock: clock.clone(),
                    steps: entry.script.into(),
                },
                emitted: 0,
                linked: false,
                address: entry.address,
            })
            .collect();
        Ok(Self {
            id: config.id,
            kind: config.kind,
            hosts,
            connect: Box::new(connect),
        })
    }
    pub fn address(&self, id: &HostId) -> Option<&str> {
        self.hosts
            .iter()
            .find(|host| &host.host.id == id)?
            .address
            .as_deref()
    }
}
impl Provider for FakeProvider {
    fn id(&self) -> ProviderId {
        self.id.clone()
    }
    fn kind(&self) -> ProviderKind {
        self.kind.clone()
    }
    fn hosts(&self) -> Vec<Host> {
        self.hosts
            .iter()
            .map(|host| Host {
                state: host.script.state(),
                ..host.host.clone()
            })
            .collect()
    }
    fn link(&mut self, id: HostId) -> Result<Box<dyn SupervisorConnection>> {
        let host = self
            .hosts
            .iter_mut()
            .find(|h| h.host.id == id)
            .ok_or_else(|| invalid("unknown fixture host"))?;
        if host.linked {
            return Err(invalid("fixture host already linked"));
        }
        let inner = (self.connect)(&host.host)?;
        host.linked = true;
        Ok(Box::new(FakeLink {
            inner,
            script: host.script.clone(),
            events: VecDeque::new(),
        }))
    }
    fn poll(&mut self) -> Vec<ProviderEvent> {
        let mut events = Vec::new();
        for host in &mut self.hosts {
            let elapsed = host.script.elapsed();
            while let Some(step) = host.script.steps.get(host.emitted)
                && step.at_ms <= elapsed
            {
                events.push(ProviderEvent::HostState {
                    host: host.host.id.clone(),
                    state: step.state.clone(),
                });
                host.emitted += 1;
            }
        }
        events
    }
}

pub struct FakeLink {
    inner: Box<dyn SupervisorConnection>,
    script: Script,
    events: VecDeque<(u64, Event)>,
}
impl SupervisorConnection for FakeLink {
    fn request(&mut self, action: Action) -> Result<Reply> {
        self.script.latency()?;
        self.inner.request(action)
    }
    fn receive(&mut self, _timeout: Duration) -> Result<Option<Event>> {
        let Ok(rtt) = self.script.latency() else {
            return Ok(None);
        };
        let now = self.script.clock.now_ms();
        // Backpressure the local link instead of dropping queued event bytes.
        if self.events.len() < 128
            && let Some(event) = self.inner.receive(Duration::ZERO)?
        {
            self.events.push_back((now.saturating_add(rtt), event));
        }
        if self.events.front().is_some_and(|(due, _)| *due <= now) {
            Ok(self.events.pop_front().map(|(_, event)| event))
        } else {
            Ok(None)
        }
    }
}
