#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bbt_mux::logic::SupervisorConnection;
use bbt_mux::provider::{
    Connections, Host, HostId, LinkState, LocalProvider, Provider, ProviderEvent, ProviderKind,
};
use bbt_mux::registry::Location;
use bbt_mux::{Action, Event, Reply, SeatInfo};

#[derive(Default)]
struct Calls {
    requests: Vec<HostId>,
    waits: Vec<(HostId, Duration)>,
}
struct ProbeLink {
    host: HostId,
    calls: Arc<Mutex<Calls>>,
}
impl SupervisorConnection for ProbeLink {
    fn request(&mut self, _: Action) -> bbt_mux::store::Result<Reply> {
        self.calls.lock().unwrap().requests.push(self.host.clone());
        Ok(Reply::Seats {
            seats: Vec::<SeatInfo>::new(),
        })
    }
    fn receive(&mut self, wait: Duration) -> bbt_mux::store::Result<Option<Event>> {
        self.calls
            .lock()
            .unwrap()
            .waits
            .push((self.host.clone(), wait));
        Ok(None)
    }
}
struct ProbeProvider {
    host: HostId,
    id: String,
    calls: Arc<Mutex<Calls>>,
}
impl Provider for ProbeProvider {
    fn id(&self) -> String {
        self.id.clone()
    }
    fn kind(&self) -> ProviderKind {
        ProviderKind::Ssh
    }
    fn hosts(&self) -> Vec<Host> {
        vec![Host {
            id: self.host.clone(),
            provider: self.id(),
            label: "probe".into(),
            region: None,
            state: LinkState::Connected { rtt_ms: 0 },
        }]
    }
    fn link(&mut self, host: HostId) -> bbt_mux::store::Result<Box<dyn SupervisorConnection>> {
        assert_eq!(host, self.host);
        Ok(Box::new(ProbeLink {
            host,
            calls: self.calls.clone(),
        }))
    }
    fn poll(&mut self) -> Vec<ProviderEvent> {
        vec![]
    }
}
#[test]
fn location_namespaces_select_one_link_and_only_the_local_link_waits() {
    let calls = Arc::new(Mutex::new(Calls::default()));
    let ssh = HostId::from(&Location::Ssh {
        host: "same-host".into(),
    });
    let cloud = HostId::from(&Location::Cloud {
        provider: "fixture-cloud".into(),
        host: "same-host".into(),
    });
    assert_ne!(ssh, cloud);
    let mut connections = Connections::open(vec![
        Box::new(LocalProvider::new(ProbeLink {
            host: HostId::Local,
            calls: calls.clone(),
        })),
        Box::new(ProbeProvider {
            host: ssh.clone(),
            id: "ssh".into(),
            calls: calls.clone(),
        }),
        Box::new(ProbeProvider {
            host: cloud.clone(),
            id: "cloud".into(),
            calls: calls.clone(),
        }),
    ])
    .unwrap();
    for host in [&HostId::Local, &ssh, &cloud] {
        connections.request(host, Action::ListOwnedSeats).unwrap();
    }
    assert_eq!(
        calls.lock().unwrap().requests,
        vec![HostId::Local, ssh.clone(), cloud.clone()]
    );
    assert!(connections.receive().unwrap().is_empty());
    let waits = &calls.lock().unwrap().waits;
    assert_eq!(waits[0], (HostId::Local, Duration::from_millis(10)));
    assert_eq!(waits.len(), 3);
    assert!(waits[1..].iter().all(|(_, wait)| *wait == Duration::ZERO));
}
#[test]
fn local_provider_preserves_the_existing_link_and_rejects_unknown_hosts() {
    let calls = Arc::new(Mutex::new(Calls::default()));
    let mut provider = LocalProvider::new(ProbeLink {
        host: HostId::Local,
        calls: calls.clone(),
    });
    assert_eq!(provider.kind(), ProviderKind::Local);
    assert_eq!(provider.hosts()[0].label, "this Mac");
    assert!(provider.poll().is_empty());
    assert!(
        provider
            .link(HostId::Ssh {
                host: "missing".into()
            })
            .is_err()
    );
    let mut link = provider.link(HostId::Local).unwrap();
    link.request(Action::ListOwnedSeats).unwrap();
    assert_eq!(calls.lock().unwrap().requests, vec![HostId::Local]);
    assert!(provider.link(HostId::Local).is_err());
}
