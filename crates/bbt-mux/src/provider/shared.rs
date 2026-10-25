//! Logical hosts sharing one admitted local supervisor and its journal.
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, TryLockError},
    time::Duration,
};

use super::{
    HostId, LocalProvider, Provider,
    fake::{Clock, FakeProvider, Fixture},
};
use crate::{
    Action, Event, Reply, Ulid,
    store::{Result, invalid},
};
use bbt_supervisor::{SupervisorConnection, client::PendingEvents};

struct Shared {
    connection: Box<dyn SupervisorConnection>,
    owners: HashMap<Ulid, HostId>,
    queues: HashMap<HostId, PendingEvents>,
}

/// IO-only logical links; cloning this handle neither admits another lease nor
/// creates another process, journal, connection or polling thread.
#[derive(Clone)]
pub struct LocalLinks(Arc<Mutex<Shared>>);
impl LocalLinks {
    pub fn new(
        connection: impl SupervisorConnection + 'static,
        owners: impl IntoIterator<Item = (Ulid, HostId)>,
    ) -> Self {
        Self(Arc::new(Mutex::new(Shared {
            connection: Box::new(connection),
            owners: owners.into_iter().collect(),
            queues: HashMap::new(),
        })))
    }
    pub fn link(&self, host: HostId) -> impl SupervisorConnection + 'static {
        LogicalLink {
            shared: self.clone(),
            host,
        }
    }
}

struct LogicalLink {
    shared: LocalLinks,
    host: HostId,
}
fn action_seat(action: &Action) -> Option<Ulid> {
    match action {
        Action::AcquireLogic { .. } | Action::ListOwnedSeats | Action::ReadRingBatch { .. } => None,
        Action::Spawn { spec } => Some(spec.seat),
        Action::Attach { seat, .. }
        | Action::AttachRing { seat, .. }
        | Action::InputStatus { seat, .. } => Some(*seat),
        Action::RingRecovery { attachment }
        | Action::ReadRing { attachment, .. }
        | Action::PublishCheckpoint { attachment, .. }
        | Action::Input { attachment, .. }
        | Action::InputContext { attachment }
        | Action::GuardedInput { attachment, .. }
        | Action::Resize { attachment, .. }
        | Action::Kill { attachment }
        | Action::ScheduleClose { attachment, .. }
        | Action::UndoClose { attachment, .. } => Some(attachment.seat),
    }
}
impl SupervisorConnection for LogicalLink {
    fn request(&mut self, action: Action) -> Result<Reply> {
        bbt_rt::assert_not_ui_thread();
        let mut shared = match self.shared.0.try_lock() {
            Ok(shared) => shared,
            Err(TryLockError::WouldBlock) => {
                return Err(crate::Failure {
                    code: crate::FailureCode::Busy,
                    message: "shared supervisor stream occupied; request not sent".into(),
                }
                .into());
            }
            Err(TryLockError::Poisoned(_)) => return Err(invalid("host router stopped")),
        };
        if matches!(action, Action::AcquireLogic { .. }) {
            return Err(invalid("logical host links are already admitted"));
        }
        if let Action::ReadRingBatch { reads, .. } = &action
            && reads.iter().any(|read| {
                let owner = shared.owners.get(&read.attachment.seat);
                owner.is_some_and(|owner| owner != &self.host)
                    || (owner.is_none() && self.host != HostId::Local)
            })
        {
            // Logic groups by host. A mixed group is a routing bug, not a
            // capture-member refusal; forward none of it across the boundary.
            return Err(invalid("ring batch crosses supervisor host boundary"));
        }
        if let Some(seat) = action_seat(&action) {
            let owner = shared.owners.get(&seat);
            if owner.is_some_and(|owner| owner != &self.host)
                || (owner.is_none()
                    && self.host != HostId::Local
                    && !matches!(action, Action::Spawn { .. }))
            {
                return Err(invalid("seat is assigned to a different supervisor host"));
            }
            if matches!(action, Action::Spawn { .. }) {
                shared.owners.insert(seat, self.host.clone());
            }
        }
        let ring = match &action {
            Action::AttachRing {
                seat, generation, ..
            } => Some((*seat, *generation)),
            _ => None,
        };
        let reply = shared.connection.request(action)?;
        if let (
            Some((seat, generation)),
            Reply::Attached {
                first_available_sequence,
                ..
            },
        ) = (ring, &reply)
        {
            shared
                .queues
                .entry(self.host.clone())
                .or_default()
                .confirm_ring(seat, generation, *first_available_sequence)?;
        }
        match reply {
            Reply::Seats { seats } => Ok(Reply::Seats {
                seats: seats
                    .into_iter()
                    .filter(|seat| {
                        shared.owners.get(&seat.seat).unwrap_or(&HostId::Local) == &self.host
                    })
                    .collect(),
            }),
            reply => Ok(reply),
        }
    }
    fn receive(&mut self, timeout: Duration) -> Result<Option<Event>> {
        bbt_rt::assert_not_ui_thread();
        let mut shared = match self.shared.0.try_lock() {
            Ok(shared) => shared,
            Err(TryLockError::WouldBlock) => return Ok(None),
            Err(TryLockError::Poisoned(_)) => return Err(invalid("host router stopped")),
        };
        if let Some(event) = shared
            .queues
            .entry(self.host.clone())
            .or_default()
            .pop_front()
        {
            return Ok(Some(event));
        }
        if let Some(event) = shared.connection.receive(timeout)? {
            let host = shared
                .owners
                .get(&event.seat())
                .cloned()
                .unwrap_or(HostId::Local);
            shared.queues.entry(host).or_default().push(event)?;
        }
        Ok(shared
            .queues
            .entry(self.host.clone())
            .or_default()
            .pop_front())
    }
}

/// Enable only fixture-defined remote hosts over the existing admitted local
/// link. Durable ownership is supplied by logic before startup recovery.
pub fn fixture_providers(
    connection: impl SupervisorConnection + 'static,
    owners: impl IntoIterator<Item = (Ulid, HostId)>,
    clock: Arc<dyn Clock>,
) -> Result<Vec<Box<dyn Provider>>> {
    let links = LocalLinks::new(connection, owners);
    let mut providers: Vec<Box<dyn Provider>> =
        vec![Box::new(LocalProvider::new(links.link(HostId::Local)))];
    for config in Fixture::bundled()?.into_providers() {
        let links = links.clone();
        providers.push(Box::new(FakeProvider::new(
            config,
            clock.clone(),
            move |host| Ok(Box::new(links.link(host.id.clone()))),
        )?));
    }
    Ok(providers)
}
