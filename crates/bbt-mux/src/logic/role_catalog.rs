// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Derived runtime cache of operator Settings; TOML remains the authoring home.
use crate::registry::{Role, SeatRecord};
use crate::store::{Result, Store, invalid, sqlite::SqliteStore};
use bbt_proto::gui::RoleCatalog;
use serde::{Deserialize, Serialize};

const KEY: &str = "roles/catalog/v1";
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Saved {
    version: u16,
    catalog: RoleCatalog,
}
#[derive(Clone)]
pub(super) struct CatalogStore(SqliteStore);
impl CatalogStore {
    pub(super) fn new(store: SqliteStore) -> Self {
        Self(store)
    }
    /// A malformed cache is an explicit role-admission refusal, never authority
    /// inferred from a display name or a silent builtin fallback.
    pub(super) fn load(&self) -> Result<RoleCatalog> {
        let catalog = match self.0.get(KEY)? {
            None => RoleCatalog::builtin(),
            Some(bytes) => {
                if bytes.len() > 768 * 1024 {
                    return Err(invalid("role_catalog_cache_too_large"));
                }
                let saved: Saved = serde_json::from_slice(&bytes)?;
                if saved.version != 1 {
                    return Err(invalid("unsupported_role_catalog_cache_version"));
                }
                saved.catalog
            }
        };
        catalog.validate().map_err(invalid)?;
        // Missing cache bytes are compatible with pre-catalog builtin seats,
        // but cannot silently orphan a retained custom role after data loss.
        validate_replacement(
            &catalog,
            &crate::registry::Registry::new(self.0.clone()).list()?,
        )?;
        Ok(catalog)
    }
    /// Runs on the bounded IO job. While it commits, Worker rejects only new
    /// role-dependent spawn admissions; unrelated reads, output and messaging run.
    pub(super) fn save(&self, catalog: RoleCatalog) -> Result<RoleCatalog> {
        catalog.validate().map_err(invalid)?;
        let records = crate::registry::Registry::new(self.0.clone()).list()?;
        validate_replacement(&catalog, &records)?;
        self.0.put(
            KEY,
            &serde_json::to_vec(&Saved {
                version: 1,
                catalog: catalog.clone(),
            })?,
        )?;
        Ok(catalog)
    }
}
/// There is no complete operator role-change/grant-reissue control yet. Never
/// grant a changed class to an existing name by editing its catalog alone.
fn validate_replacement(catalog: &RoleCatalog, records: &[SeatRecord]) -> Result<()> {
    for record in records {
        let definition = catalog
            .roles
            .get(&record.role.name)
            .ok_or_else(|| invalid(format!("role_in_use: {}", record.role.name)))?;
        if definition.class != record.role.class {
            return Err(invalid(format!(
                "role_in_use_class_change_requires_grant_reissue: {}",
                record.role.name
            )));
        }
    }
    Ok(())
}
// Existing seats retain their assigned role when launch availability changes.
// New spawns additionally pass resolve(), which enforces the current allowed list.
pub(super) fn validate_assignment(catalog: &RoleCatalog, kind: &str, role: &Role) -> Result<()> {
    crate::adapters::validate_role(kind, role)?;
    let definition = catalog
        .roles
        .get(&role.name)
        .ok_or_else(|| invalid("unknown_configured_role"))?;
    if definition.class != role.class {
        return Err(invalid("configured_role_class_mismatch"));
    }
    Ok(())
}
pub(super) fn resolve(
    catalog: &RoleCatalog,
    kind: &str,
    requested: Option<&str>,
) -> Result<(Role, String)> {
    let policy = catalog
        .harnesses
        .get(kind)
        .ok_or_else(|| invalid("harness_roles_not_configured"))?;
    let name = requested.unwrap_or(&policy.default_role);
    let definition = catalog
        .roles
        .get(name)
        .ok_or_else(|| invalid("unknown_role_preset"))?;
    let role = Role {
        name: name.into(),
        class: definition.class,
    };
    validate_assignment(catalog, kind, &role)?;
    if !policy.available_roles.iter().any(|value| value == name) {
        return Err(invalid("role_not_available_for_harness"));
    }
    Ok((role, definition.prompt.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Ulid, registry::Class};
    fn configured() -> RoleCatalog {
        let mut catalog = RoleCatalog::builtin();
        let mut role = catalog.roles["executor"].clone();
        role.prompt = "Review all changes.\nCite exact evidence.".into();
        catalog.roles.insert("Reviewer".into(), role);
        let policy = catalog.harnesses.get_mut("codex").unwrap();
        policy.available_roles.push("Reviewer".into());
        policy.default_role = "Reviewer".into();
        catalog
    }
    #[test]
    fn resolution_uses_configured_class_prompt_default_and_availability() {
        let mut catalog = configured();
        catalog.validate().unwrap();
        let (role, prompt) = resolve(&catalog, "codex", None).unwrap();
        assert_eq!(
            role,
            Role {
                name: "Reviewer".into(),
                class: Class::Worker
            }
        );
        assert_eq!(prompt, "Review all changes.\nCite exact evidence.");
        let mut oracle = catalog.roles["oracle"].clone();
        oracle.prompt = "Coordinate the swarm.".into();
        catalog.roles.insert("Coordinator".into(), oracle);
        catalog
            .harnesses
            .get_mut("claude")
            .unwrap()
            .available_roles
            .push("Coordinator".into());
        let (role, prompt) = resolve(&catalog, "claude", Some("Coordinator")).unwrap();
        assert_eq!(role.class, Class::Oracle);
        assert_eq!(prompt, "Coordinate the swarm.");
        assert_eq!(
            resolve(&catalog, "codex", Some("Coordinator"))
                .unwrap_err()
                .to_string(),
            "role_not_available_for_harness"
        );
        catalog
            .harnesses
            .get_mut("codex")
            .unwrap()
            .available_roles
            .push("Coordinator".into());
        catalog.harnesses.get_mut("codex").unwrap().default_role = "Coordinator".into();
        catalog.validate().unwrap();
        let (role, prompt) = resolve(&catalog, "codex", None).unwrap();
        assert_eq!(role.class, Class::Oracle);
        assert_eq!(prompt, "Coordinate the swarm.");
        validate_assignment(&catalog, "codex", &role).unwrap();
        assert_eq!(
            resolve(&catalog, "claude", Some("Reviewer"))
                .unwrap_err()
                .to_string(),
            "role_not_available_for_harness"
        );
        assert!(resolve(&catalog, "zsh", None).unwrap().1.is_empty());
    }
    #[test]
    fn retained_assignments_keep_their_class_when_availability_changes() {
        let mut catalog = configured();
        let oracle = Role {
            name: "oracle".into(),
            class: Class::Oracle,
        };
        for kind in ["claude", "codex", "pi", "omp", "zsh", "custom"] {
            validate_assignment(&catalog, kind, &oracle).unwrap();
        }
        let executor = Role {
            name: "executor".into(),
            class: Class::Worker,
        };
        // Explicit operator-assigned/promoted shells retain their signed role.
        // New choices are governed by the configured availability list.
        validate_assignment(&catalog, "zsh", &executor).unwrap();
        validate_assignment(&catalog, "zsh", &oracle).unwrap();
        assert_eq!(
            resolve(&catalog, "zsh", Some("executor")).unwrap().0,
            executor
        );
        validate_assignment(
            &catalog,
            "zsh",
            &Role {
                name: "shell".into(),
                class: Class::Worker,
            },
        )
        .unwrap();
        let reviewer = resolve(&catalog, "codex", None).unwrap().0;
        let policy = catalog.harnesses.get_mut("codex").unwrap();
        policy.available_roles.retain(|name| name != "Reviewer");
        policy.default_role = "executor".into();
        catalog.validate().unwrap();
        validate_assignment(&catalog, "codex", &reviewer).unwrap();
        assert_eq!(
            resolve(&catalog, "codex", Some("Reviewer"))
                .unwrap_err()
                .to_string(),
            "role_not_available_for_harness"
        );
    }
    #[test]
    fn in_use_roles_cannot_be_deleted_or_reclassified_without_grant_reissue() {
        let mut catalog = configured();
        let mut record = SeatRecord::new(Ulid::new(), Ulid::new(), "fixture", "codex");
        record.role = resolve(&catalog, "codex", None).unwrap().0;
        validate_replacement(&catalog, &[record.clone()]).unwrap();
        catalog.roles.get_mut("Reviewer").unwrap().class = Class::Oracle;
        assert!(
            validate_replacement(&catalog, &[record.clone()])
                .unwrap_err()
                .to_string()
                .starts_with("role_in_use_class_change_requires_grant_reissue")
        );
        catalog.roles.remove("Reviewer");
        assert!(
            validate_replacement(&catalog, &[record])
                .unwrap_err()
                .to_string()
                .starts_with("role_in_use:")
        );
    }
    #[test]
    fn cached_custom_roles_reload_and_malformed_cache_never_falls_back() {
        let directory = tempfile::tempdir().unwrap();
        let store = SqliteStore::open(directory.path()).unwrap();
        let cache = CatalogStore::new(store.clone());
        let catalog = configured();
        cache.save(catalog.clone()).unwrap();
        let reopened = CatalogStore::new(SqliteStore::open(directory.path()).unwrap());
        assert_eq!(reopened.load().unwrap(), catalog);
        assert_eq!(
            resolve(&reopened.load().unwrap(), "codex", None).unwrap().1,
            catalog.roles["Reviewer"].prompt
        );
        let mut seat = SeatRecord::new(Ulid::new(), Ulid::new(), "fixture", "codex");
        seat.role = resolve(&catalog, "codex", None).unwrap().0;
        crate::registry::Registry::new(store.clone())
            .insert(&seat)
            .unwrap();
        assert!(
            cache
                .save(RoleCatalog::builtin())
                .unwrap_err()
                .to_string()
                .starts_with("role_in_use:")
        );
        assert_eq!(reopened.load().unwrap(), catalog);
        let mut reclassified = catalog.clone();
        reclassified.roles.get_mut("Reviewer").unwrap().class = Class::Oracle;
        reclassified
            .harnesses
            .get_mut("codex")
            .unwrap()
            .available_roles
            .retain(|name| name != "Reviewer");
        reclassified
            .harnesses
            .get_mut("codex")
            .unwrap()
            .default_role = "executor".into();
        reclassified
            .harnesses
            .get_mut("claude")
            .unwrap()
            .available_roles
            .push("Reviewer".into());
        reclassified.validate().unwrap();
        assert!(
            cache
                .save(reclassified)
                .unwrap_err()
                .to_string()
                .starts_with("role_in_use_class_change_requires_grant_reissue:")
        );
        assert_eq!(reopened.load().unwrap(), catalog);
        store.put(KEY, b"not JSON").unwrap();
        assert!(reopened.load().is_err());
        store
            .put(
                KEY,
                &serde_json::to_vec(&Saved {
                    version: 2,
                    catalog,
                })
                .unwrap(),
            )
            .unwrap();
        assert_eq!(
            reopened.load().unwrap_err().to_string(),
            "unsupported_role_catalog_cache_version"
        );
    }
}
