//! GUI socket IO, immutable model publication and worker-only preparation.
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

use crate::{UiEvent, log::Logger};
use bboxy450_term::{
    cockpit::{self, Session},
    grids::Grids,
    model::{self, Model, PanePreparer},
};
use bbt_mux::{
    Ulid,
    store::{Result, invalid},
};
use bbt_proto::{
    GuiAction, GuiIdentity, GuiReply, GuiSeat, ProcessStatus, SeatId, SpawnSpec, SwarmId, Update,
    Where,
};
use bbt_render::{cells::terminal, device::DeviceContext};
use bbt_rt::Latest;
use tokio::sync::{mpsc, oneshot};
use winit::event_loop::EventLoopProxy;

pub struct RenderContext {
    pub context: Arc<DeviceContext>,
    pub scale: f32,
}
pub use bboxy450_term::cockpit::UiMsg;

pub fn start(
    updates: Arc<Latest<Arc<Model>>>,
    proxy: EventLoopProxy<UiEvent>,
    logger: Logger,
    run_dir: Option<PathBuf>,
    rendering: oneshot::Receiver<RenderContext>,
) -> mpsc::Sender<UiMsg> {
    let (sender, commands) = mpsc::channel(64);
    bbt_rt::handle().spawn(async move {
        let result = run(updates, commands, logger.clone(), run_dir, rendering).await;
        if let Err(error) = &result {
            logger.write(format!("cockpit stopped: {error}"));
        }
        let _ = proxy.send_event(UiEvent::BackendStopped(result.map_err(|e| e.to_string())));
    });
    sender
}

fn publish(
    seats: &BTreeMap<Ulid, GuiSeat>,
    identity: &GuiIdentity,
    grids: &Grids,
    previous: &mut Arc<Model>,
    preparer: &mut PanePreparer,
    updates: &Latest<Arc<Model>>,
) -> Result<()> {
    let seats: Vec<_> = seats.values().cloned().collect();
    let records: Vec<_> = seats.iter().map(|seat| seat.record.clone()).collect();
    let mut next = model::build(&records, grids, previous);
    next.apply_seat_states(&seats);
    next.identity = Some(identity.clone());
    preparer.prepare(&mut next)?;
    *previous = Arc::new(next);
    updates.publish(previous.clone());
    Ok(())
}

async fn run(
    updates: Arc<Latest<Arc<Model>>>,
    mut commands: mpsc::Receiver<UiMsg>,
    logger: Logger,
    run_dir: Option<PathBuf>,
    rendering: oneshot::Receiver<RenderContext>,
) -> Result<()> {
    let config = cockpit::Config::discover(run_dir)?;
    let cwd = config.repository.clone();
    let mut session = Session::start(config).await?;
    logger.write(format!(
        "cockpit ready: supervisor={} logic={}",
        session.supervisor.id().unwrap_or(0),
        session.logic.id().unwrap_or(0)
    ));
    let mut seats: BTreeMap<_, _> = session
        .list()
        .await?
        .into_iter()
        .map(|seat| (seat.record.id, seat))
        .collect();
    let mut identity = session.endpoint.identity.clone();
    let swarm = seats
        .values()
        .next()
        .map_or_else(Ulid::new, |seat| seat.record.swarm);
    let rendering = rendering
        .await
        .map_err(|_| invalid("renderer bootstrap stopped"))?;
    let mut options = terminal::Options::default();
    options.font.scale = rendering.scale.round().max(1.0) as u32;
    let mut preparer = PanePreparer::with_options(rendering.context, options)?;
    let mut grids = Grids::new();
    let mut model = Arc::new(Model::default());
    publish(
        &seats,
        &identity,
        &grids,
        &mut model,
        &mut preparer,
        &updates,
    )?;
    let result:Result<()>=async {
        loop {
            tokio::select! {
                command=commands.recv()=>match command {
                    Some(UiMsg::SpawnZsh)=>session.send(GuiAction::Spawn {spec:SpawnSpec {
                        swarm:SwarmId::from_u128(swarm.0),harness_kind:"zsh".into(),name:None,role:None,
                        cwd:cwd.to_string_lossy().into_owned(),location:Where::Local,resume:None,
                    }}).await?,
                    Some(UiMsg::Input {seat,id,bytes})=>session.send(GuiAction::Input {seat,id,bytes}).await?,
                    Some(UiMsg::Resize {seat,cols,rows})=>session.send(GuiAction::Resize {seat,cols,rows}).await?,
                    Some(UiMsg::Watched(panes))=> {
                        for action in grids.set_watched(&panes) {session.send(action).await?;}
                        publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;
                    }
                    None=>break,
                },
                event=session.event()=>match event? {
                    cockpit::Event::Reply(reply)=>match *reply {
                        GuiReply::Identity(next)=> {
                            if identity!=next {
                                identity=next;
                                logger.write(format!("identity status: {:?}",identity.state));
                                publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;
                            }
                        }
                        GuiReply::Seats(next)=> {
                            seats=next.into_iter().map(|seat|(seat.record.id,seat)).collect();
                            publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;
                        }
                        GuiReply::Seat(seat)=> {
                            if seats.get(&seat.record.id).is_some_and(|old|old==&seat||old.state.generation>seat.state.generation) {continue;}
                            let id=SeatId::from_u128(seat.record.id.0);
                            if matches!(seat.state.process,ProcessStatus::Exited {..}|ProcessStatus::Retained) {
                                grids.forget(id);
                                session.send(GuiAction::Unwatch {seat:id}).await?;
                            }
                            logger.write(format!("seat {} activity={:?} process={:?}",seat.record.id,seat.state.activity,seat.state.process));
                            seats.insert(seat.record.id,seat);
                            publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;
                        }
                        GuiReply::Update {seat,update}=> {
                            let (view,sequence)=match &update {
                                Update::Snapshot {snapshot}=>(snapshot.view,snapshot.sequence),
                                Update::Delta {delta}=>(delta.view,delta.sequence),
                            };
                            match grids.apply(seat,&update) {
                                Ok(changed)=> {
                                    if changed.is_some() {publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;}
                                    // A valid byte-equivalent update still needs
                                    // its ack; an unwatched in-flight one does not.
                                    if grids.screen(seat).is_some() {
                                        session.send(GuiAction::Applied {seat,view,sequence}).await?;
                                    }
                                }
                                Err(error)=>logger.write(format!("grid update rejected (no ack): {error}")),
                            }
                        }
                        GuiReply::Ok {op}=>logger.write(format!("GUI operation {op} accepted")),
                        GuiReply::Error {op,reason}=>logger.write(format!("GUI operation {op} rejected: {reason}")),
                        GuiReply::History {..}=>logger.write("history reply deferred to C1c"),
                    },
                    cockpit::Event::Exited {process,status}=>logger.write(format!("{process} exited: {status}")),
                    cockpit::Event::Disconnected(reason)=>logger.write(format!("GUI disconnected: {reason}")),
                }
            }
        }
        Ok(())
    }.await;
    let stopped = session.shutdown().await;
    if stopped.is_ok() {
        logger.write("cockpit children reaped after orderly shutdown");
    }
    result.and(stopped)
}
