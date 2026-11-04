// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Seat attribution for plugin calls. The logic owner scopes a seat's
//! validated ATC session attribution around the blocking IO it runs for that
//! seat; every plugin `tools/call` made inside the scope carries it in request
//! `_meta` ([`bbt_plugin::policy::ATTRIBUTION_META`]). Outside a scope (the
//! operator, the host, hooks, checks) a call carries none. In memory only.
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::{Value, json};

/// One seat's validated attribution.
pub type Attribution = Option<Arc<BTreeMap<String, String>>>;

thread_local! {
    static CURRENT: RefCell<Attribution> = const { RefCell::new(None) };
}

/// Validate `env` again on the host; `None` when nothing remains.
pub fn admit(env: &BTreeMap<String, String>) -> Attribution {
    let env =
        bbt_plugin::policy::attribution_env(env.iter().map(|(k, v)| (k.as_str(), v.as_str())));
    (!env.is_empty()).then(|| Arc::new(env))
}

/// Run `f` with `attribution` current on this thread, then restore the
/// previous one, also when `f` unwinds.
pub fn scope<T>(attribution: Attribution, f: impl FnOnce() -> T) -> T {
    struct Restore(Attribution);
    impl Drop for Restore {
        fn drop(&mut self) {
            let previous = self.0.take();
            CURRENT.with(|current| *current.borrow_mut() = previous);
        }
    }
    let _restore = Restore(CURRENT.with(|current| current.replace(attribution)));
    f()
}

/// The attribution current on this thread.
pub fn current() -> Attribution {
    CURRENT.with(|current| current.borrow().clone())
}

/// `tools/call` params for `name`, with `attribution` (read with
/// [`current`] on the calling thread) in `_meta`.
pub fn tools_call(name: &str, args: Value, attribution: &Attribution) -> Value {
    let mut params = json!({"name": name, "arguments": args});
    if let Some(meta) = attribution
        .as_deref()
        .and_then(bbt_plugin::policy::attribution_meta)
    {
        params["_meta"] = meta;
    }
    params
}

#[cfg(test)]
mod tests {
    use super::*;

    // keeps: seat attribution — only a scoped seat call carries `_meta`; the
    // scope ends with its call, also on unwind.
    #[test]
    fn only_a_scoped_call_carries_attribution_and_the_scope_ends_with_it() {
        let lent = BTreeMap::from([
            ("ATC_SESSION_ID".to_owned(), "xses-a".to_owned()),
            ("ATC_TOKEN".to_owned(), "secret".to_owned()),
        ]);
        let call = || tools_call("t", json!({"x": 1}), &current());
        assert!(call().get("_meta").is_none());
        assert_eq!(
            scope(admit(&lent), call),
            json!({"name":"t","arguments":{"x":1},
                   "_meta":{"bboxy450/attribution":{"env":{"ATC_SESSION_ID":"xses-a"}}}})
        );
        assert!(call().get("_meta").is_none());
        let unwound = std::panic::catch_unwind(|| scope(admit(&lent), || panic!("call failed")));
        assert!(unwound.is_err());
        assert!(current().is_none());
        assert!(admit(&BTreeMap::from([("ATC_TOKEN".to_owned(), "t".to_owned())])).is_none());
    }
}
