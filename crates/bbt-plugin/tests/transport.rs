// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Transport contract authority and bounded, literal declared fields.
use bbt_plugin::{
    contracts::Contract,
    policy::{RoleClass, Tier},
};

#[test]
fn transport_contract_owns_wrapper_schema_and_host_authority() {
    let contract = Contract::named("transport").expect("transport contract must be supported");
    assert_eq!(contract.methods(), &["wrap"]);
    assert_eq!(contract.version(), "1.0");
    assert_eq!(contract.tier(), Tier::Transport);
    assert_eq!(contract.role("wrap").unwrap(), RoleClass::HostOnly);
    assert!(contract.method_schema("wrap", false).is_some());
    assert!(contract.method_schema("wrap", true).is_some());
}

#[test]
fn transport_fields_use_only_the_trusted_names_and_defaults() {
    use bbt_plugin::transport::{Field, resolve_fields, validate_fields};
    use std::collections::BTreeMap;
    let fields = vec![
        Field {
            name: "route".into(),
            label: "Route".into(),
            default: None,
        },
        Field {
            name: "prefix".into(),
            label: "Prefix".into(),
            default: Some("default".into()),
        },
    ];
    assert_eq!(
        resolve_fields(&fields, &BTreeMap::new()).unwrap_err().code,
        "missing_transport_field"
    );
    let value = "literal ; $(must_not_expand) 'quote'";
    let resolved =
        resolve_fields(&fields, &BTreeMap::from([("route".into(), value.into())])).unwrap();
    assert_eq!(resolved["route"], value);
    assert_eq!(resolved["prefix"], "default");
    assert_eq!(
        resolve_fields(
            &fields,
            &BTreeMap::from([("root".into(), "/elsewhere".into())])
        )
        .unwrap_err()
        .code,
        "unknown_transport_field"
    );
    let mut duplicate = fields.clone();
    duplicate[1].name = "route".into();
    assert_eq!(
        validate_fields(&duplicate).unwrap_err().code,
        "invalid_transport_fields"
    );
    let mut invalid = fields;
    invalid[1].default = Some("a\0b".into());
    assert!(validate_fields(&invalid).is_err());
}
