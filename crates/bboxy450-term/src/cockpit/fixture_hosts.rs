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
