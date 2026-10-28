// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Saved role preferences are acknowledged by the runtime before new UI launches.
use crate::*;

pub(super) fn control_key(field: &settings_controller::Field) -> Option<String> {
    use settings_controller::{Field, role_region};
    Some(match field {
        Field::DestinationAvailable(id) => format!("destination/{id}/available"),
        Field::RoleNew => "role/new".into(),
        Field::RoleAdd => "role/draft/add".into(),
        Field::RoleCancel => "role/draft/cancel".into(),
        Field::RoleDraftName => "role/draft/name".into(),
        Field::RoleDraftPrompt => "role/draft/prompt".into(),
        Field::RoleDraftColor => "role/draft/color".into(),
        Field::RoleDraftMark => "role/draft/mark".into(),
        Field::RoleDraftClass => "role/draft/class".into(),
        Field::RolePrompt(name) => role_region(name, "prompt"),
        Field::RoleClass(name) => role_region(name, "class"),
        Field::RoleDelete(name) => role_region(name, "delete"),
        Field::HarnessRole(name) => format!("harness/{name}/role"),
        Field::HarnessAvailableRole { harness, role } => {
            format!("harness/{harness}/available/{role}")
        }
        Field::Choice(index) => format!("settings/choice/{index}"),
        _ => return None,
    })
}

pub(super) fn configure_items(
    items: &mut [bbt_ui::modules::launcher::Item],
    catalog: &bbt_proto::gui::RoleCatalog,
) {
    for item in items {
        if !item.navigation
            && let Some(policy) = catalog.harnesses.get(&item.harness)
        {
            item.role = Some(policy.default_role.clone());
            if item.kind == "Harness" {
                let note = item
                    .sub
                    .find(" · this swarm")
                    .map(|at| item.sub[at..].to_owned())
                    .unwrap_or_default();
                item.sub = format!(
                    "{} by default{}{}",
                    policy.default_role,
                    if item.harness == "codex" {
                        " · never an oracle"
                    } else {
                        ""
                    },
                    note
                );
            }
        }
        if let Some(children) = &mut item.children {
            configure_items(children, catalog);
        }
    }
}

#[derive(Default)]
pub(super) struct Sync {
    pending: Option<(bbt_mux::Ulid, bbt_proto::gui::RoleCatalog, Instant)>,
    attempted: Option<bbt_proto::gui::RoleCatalog>,
    applied: Option<bbt_proto::gui::RoleCatalog>,
    pub error: Option<String>,
}
impl Sync {
    fn update(
        &mut self,
        desired: bbt_proto::gui::RoleCatalog,
        model: Option<&bboxy450_term::model::Model>,
        connected: bool,
        now: Instant,
    ) -> Option<live::UiMsg> {
        if !connected {
            self.disconnected();
            return None;
        }
        if let Some((op, _, deadline)) = &self.pending {
            let result =
                model.and_then(|model| model.operation_results.rows().find(|r| r.op == *op));
            if let Some(result) = result {
                let (_, catalog, _) = self.pending.take().expect("pending role settings");
                match &result.reply {
                    messaging::Reply::RolesConfigured => {
                        self.applied = Some(catalog);
                        self.error = None;
                    }
                    messaging::Reply::Refused(reason) | messaging::Reply::Unknown(reason) => {
                        self.error = Some(format!("Role settings not applied: {reason}"));
                    }
                    _ => self.error = Some("Unexpected role settings response".into()),
                }
            } else if now >= *deadline {
                self.pending = None;
                self.error = Some("Role settings outcome unknown; reload settings to retry".into());
            }
        }
        if self.pending.is_some()
            || self.applied.as_ref() == Some(&desired)
            || self.attempted.as_ref() == Some(&desired)
        {
            return None;
        }
        let id = bbt_mux::Ulid::new();
        self.attempted = Some(desired.clone());
        self.pending = Some((id, desired.clone(), now + Duration::from_secs(10)));
        self.error = None;
        Some(live::UiMsg::ConfigureRoles {
            id,
            catalog: desired,
        })
    }
    pub fn applied(&self, desired: &bbt_proto::gui::RoleCatalog) -> bool {
        self.applied.as_ref() == Some(desired)
    }
    pub fn settled(&self, desired: &bbt_proto::gui::RoleCatalog) -> bool {
        self.applied(desired) || (self.pending.is_none() && self.error.is_some())
    }
    pub fn wake_at(&self) -> Option<Instant> {
        self.pending.as_ref().map(|p| p.2)
    }
    pub fn disconnected(&mut self) {
        self.pending = None;
        self.attempted = None;
        self.applied = None;
    }
    pub fn retry(&mut self) {
        self.attempted = None;
        self.error = None;
    }
}

impl App {
    pub(super) fn sync_role_settings(&mut self) {
        if !self.preferences.loaded || !self.preferences.saved() {
            return;
        }
        let catalog = self.preferences.preferences.role_catalog();
        let connected = !self.backend_lost
            && self.live_commands.is_some()
            && self
                .live_state
                .as_ref()
                .is_some_and(|model| model.metadata_initialized);
        let before = (
            self.role_settings.wake_at(),
            self.role_settings.error.clone(),
        );
        let message = self.role_settings.update(
            catalog,
            self.live_state.as_deref(),
            connected,
            Instant::now(),
        );
        if let Some(message) = message {
            self.queue(message);
        }
        if before
            != (
                self.role_settings.wake_at(),
                self.role_settings.error.clone(),
            )
        {
            self.local_page_layout = None;
            self.request_redraw();
        }
    }
    pub(super) fn validate_role_edit(
        &self,
        patch: &settings_controller::Patch,
    ) -> Result<(), String> {
        let name = match patch {
            settings_controller::Patch::RoleDelete(name) => Some(name),
            settings_controller::Patch::RoleUpsert { name, role } => self
                .preferences
                .preferences
                .roles
                .get(name)
                .filter(|before| format!("{:?}", before.class).to_lowercase() != role.cls)
                .map(|_| name),
            _ => None,
        };
        if let Some(name) = name {
            if self.backend_lost
                || !self
                    .live_state
                    .as_ref()
                    .is_some_and(|model| model.metadata_initialized)
                || !self
                    .role_settings
                    .applied(&self.preferences.preferences.role_catalog())
            {
                return Err(
                    "Wait for connected role settings before deleting or reclassifying a role"
                        .into(),
                );
            }
            if self.live_state.as_ref().is_some_and(|model| {
                model
                    .gui_seats
                    .iter()
                    .any(|seat| seat.record.role.name == *name)
            }) {
                return Err(format!(
                    "Role {name} is in use; its class and definition must stay available"
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn role_sync_waits_for_its_own_durable_ack_and_retries_after_reconnect() {
        let catalog = bbt_proto::gui::RoleCatalog::builtin();
        let mut sync = Sync::default();
        let mut model = bboxy450_term::model::Model::default();
        let now = Instant::now();
        let Some(live::UiMsg::ConfigureRoles { id, .. }) =
            sync.update(catalog.clone(), Some(&model), true, now)
        else {
            panic!("catalog update not sent")
        };
        assert!(!sync.applied(&catalog));
        assert!(
            sync.update(catalog.clone(), Some(&model), true, now)
                .is_none()
        );
        Arc::make_mut(&mut model.operation_results)
            .push(bbt_mux::Ulid::new(), messaging::Reply::RolesConfigured);
        sync.update(catalog.clone(), Some(&model), true, now);
        assert!(!sync.applied(&catalog));
        Arc::make_mut(&mut model.operation_results).push(id, messaging::Reply::RolesConfigured);
        assert!(
            sync.update(catalog.clone(), Some(&model), true, now)
                .is_none()
        );
        assert!(sync.applied(&catalog));
        assert!(sync.wake_at().is_none());
        sync.update(catalog.clone(), Some(&model), false, now);
        assert!(!sync.applied(&catalog));
        assert!(matches!(
            sync.update(catalog, Some(&model), true, now),
            Some(live::UiMsg::ConfigureRoles { .. })
        ));
    }
    #[test]
    fn role_sync_timeout_is_visible_and_does_not_claim_authority_was_applied() {
        let catalog = bbt_proto::gui::RoleCatalog::builtin();
        let mut sync = Sync::default();
        let now = Instant::now();
        sync.update(catalog.clone(), None, true, now);
        sync.update(catalog.clone(), None, true, now + Duration::from_secs(11));
        assert!(!sync.applied(&catalog));
        assert!(sync.error.as_deref().unwrap().contains("unknown"));
        assert!(sync.wake_at().is_none());
        assert!(sync.update(catalog.clone(), None, true, now).is_none());
        sync.retry();
        assert!(sync.update(catalog, None, true, now).is_some());
    }
}
