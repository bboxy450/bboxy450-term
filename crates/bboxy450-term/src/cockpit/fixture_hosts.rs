// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Keep scripted host transitions out of an isolated animation-clock fixture.

pub(super) fn enabled(smoke: bool, test_account: bool) -> bool {
    if smoke {
        return false;
    }
    #[cfg(debug_assertions)]
    if test_account
        && std::env::var_os("BBOXY_TEST_SCRIPTED_HOSTS").as_deref()
            == Some(std::ffi::OsStr::new("off"))
    {
        return false;
    }
    #[cfg(not(debug_assertions))]
    let _ = test_account;
    true
}
