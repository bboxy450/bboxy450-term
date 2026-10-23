//! App placement capture. Reusable launcher control lives with its UI module.

pub use bbt_ui::modules::launcher::controller::{CaretMove, Command, Controller, Outcome};
use bbt_ui_model::input::Modifiers;

/// Resolve only a placement suffix, never a harness name or shell command.
/// The caller still admits the returned root against its actual item catalogue.
pub fn captured_placement<'a>(
    id: &'a str,
    modifiers: Modifiers,
    model: Option<&crate::model::Model>,
    stage: Option<&bbt_ui_model::Stage>,
) -> (
    &'a str,
    Option<bbt_proto::SwarmId>,
    bbt_proto::seat_state::Placement,
) {
    use bbt_proto::seat_state::{Placement, Side};
    let (root, explicit) = match id.rsplit_once(':') {
        Some((root, "tab")) => (root, Some(None)),
        Some((root, "right")) => (root, Some(Some(Side::Right))),
        Some((root, "bottom")) => (root, Some(Some(Side::Bottom))),
        _ => (id, None),
    };
    let stage = stage.or_else(|| model.map(|model| &model.ui.window.stage));
    let swarm = stage.and_then(|stage| stage.swarm);
    let side = explicit.unwrap_or_else(|| {
        modifiers.command.then_some(if modifiers.shift {
            Side::Bottom
        } else {
            Side::Right
        })
    });
    let anchor = model.zip(stage).and_then(|(model, stage)| {
        let target = stage.focused?;
        let tab = stage.tab?;
        let seat = model.ui.seats.get(&target)?;
        let (si, ti) = model.ui.tab_of(target)?;
        let owner = &model.ui.window.sidebar.swarms[si];
        (Some(owner.id) == swarm && owner.tabs[ti].id == tab).then_some((
            target,
            tab,
            seat.process_generation,
        ))
    });
    let placement = match side.zip(anchor) {
        Some((side, (target, tab, target_generation))) => Placement::Split {
            tab,
            target,
            target_generation,
            side,
        },
        None => Placement::NewTab {
            tab: bbt_proto::TabId::from_u128(bbt_mux::Ulid::new().0),
        },
    };
    (root, swarm, placement)
}
