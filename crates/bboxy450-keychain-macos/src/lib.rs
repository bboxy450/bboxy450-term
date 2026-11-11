// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Adapted from bboxy450-term, crates/bboxy450-usage/src/io.rs::keychain:
// https://github.com/bboxy450/bboxy450-term. Moved behind the per-OS boundary.
#![cfg(target_os = "macos")]
#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

use core_foundation::{
    base::{CFType, TCFType},
    boolean::CFBoolean,
    data::CFData,
    dictionary::CFDictionary,
    string::{CFString, CFStringRef},
};
use security_framework_sys::{
    item::*,
    keychain::{SecKeychainGetUserInteractionAllowed, SecKeychainSetUserInteractionAllowed},
    keychain_item::SecItemCopyMatching,
};

#[link(name = "Security", kind = "framework")]
unsafe extern "C" {
    static kSecUseAuthenticationUIFail: CFStringRef;
}

/// Fixed errors that never retain secret bytes or OS error descriptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The returned data exceeds the caller's bound.
    TooLarge,
    /// The Keychain is unavailable or access requires interaction.
    Unavailable,
}

fn query(service: &str) -> CFDictionary<CFType, CFType> {
    unsafe {
        let s = |p| CFString::wrap_under_get_rule(p).into_CFType();
        CFDictionary::from_CFType_pairs(&[
            (s(kSecClass), s(kSecClassGenericPassword)),
            (s(kSecAttrService), CFString::new(service).into_CFType()),
            (s(kSecReturnData), CFBoolean::true_value().into_CFType()),
            (s(kSecUseAuthenticationUI), s(kSecUseAuthenticationUIFail)),
        ])
    }
}

fn status_has_data(status: i32, has_data: bool) -> Result<bool, Error> {
    match status {
        0 if has_data => Ok(true),
        -25300 => Ok(false), // errSecItemNotFound
        _ => Err(Error::Unavailable),
    }
}

fn copy_bounded(data: &[u8], max_bytes: u64) -> Result<Vec<u8>, Error> {
    if data.len() as u64 > max_bytes {
        return Err(Error::TooLarge);
    }
    Ok(data.to_vec())
}

/// Run `f` with this process's login-keychain user interaction disabled, then
/// restore the prior setting.
///
/// `kSecUseAuthenticationUIFail` covers the data-protection keychain only. The
/// login keychain's access-confirmation dialog ("wants to access key") is
/// governed by this process-wide switch, and that call blocks until answered.
fn without_interaction<T>(f: impl FnOnce() -> T) -> T {
    unsafe {
        let mut prior = 1;
        let known = SecKeychainGetUserInteractionAllowed(&mut prior) == 0;
        SecKeychainSetUserInteractionAllowed(0);
        let out = f();
        if !known || prior != 0 {
            SecKeychainSetUserInteractionAllowed(1);
        }
        out
    }
}

/// Read one generic-password service without authentication UI or an
/// access-confirmation dialog.
///
/// The caller owns the blocking IO slot and timeout. This function never writes,
/// unlocks, refreshes credentials or spawns a process. An item another
/// application owns reads as [`Error::Unavailable`] until that access is granted.
///
/// # Errors
/// Returns [`Error::Unavailable`] for denied/locked access or an invalid OS
/// result, and [`Error::TooLarge`] when the data exceeds `max_bytes`.
pub fn read_generic_password(service: &str, max_bytes: u64) -> Result<Option<Vec<u8>>, Error> {
    let query = query(service);
    unsafe {
        let mut result = std::ptr::null();
        let status =
            without_interaction(|| SecItemCopyMatching(query.as_concrete_TypeRef(), &mut result));
        if !status_has_data(status, !result.is_null())? {
            return Ok(None);
        }
        let data = CFData::wrap_under_create_rule(result.cast());
        copy_bounded(data.bytes(), max_bytes).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_forbids_authentication_ui_without_accessing_the_keychain() {
        let query = query("private-test-service");
        unsafe {
            let s = |p| CFString::wrap_under_get_rule(p).into_CFType();
            assert_eq!(
                *query.get(s(kSecUseAuthenticationUI)),
                s(kSecUseAuthenticationUIFail)
            );
            assert_eq!(*query.get(s(kSecClass)), s(kSecClassGenericPassword));
            assert_eq!(
                *query.get(s(kSecReturnData)),
                CFBoolean::true_value().into_CFType()
            );
            assert_eq!(
                *query.get(s(kSecAttrService)),
                CFString::new("private-test-service").into_CFType()
            );
        }
    }

    #[test]
    fn interaction_is_off_during_the_call_and_restored_after_it() {
        let read = || unsafe {
            let mut state = 2;
            assert_eq!(SecKeychainGetUserInteractionAllowed(&mut state), 0);
            state
        };
        let before = read();
        assert_eq!(without_interaction(read), 0);
        assert_eq!(read(), before);
        // A nested call restores the outer disabled state, not "allowed".
        assert_eq!(without_interaction(|| without_interaction(read) + read()), 0);
        assert_eq!(read(), before);
    }

    #[test]
    fn missing_locked_and_invalid_results_are_distinct() {
        assert_eq!(status_has_data(-25300, false), Ok(false));
        assert_eq!(status_has_data(0, true), Ok(true));
        assert_eq!(status_has_data(0, false), Err(Error::Unavailable));
        assert_eq!(status_has_data(-25308, false), Err(Error::Unavailable));
    }

    #[test]
    fn byte_limit_is_checked_before_copying() {
        assert_eq!(copy_bounded(b"ab", 2), Ok(b"ab".to_vec()));
        assert_eq!(copy_bounded(b"ab", 1), Err(Error::TooLarge));
    }
}
