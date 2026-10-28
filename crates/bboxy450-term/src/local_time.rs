// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Discover the system zone on IO; rendering only formats its retained rules.
use jiff::{Timestamp, tz::TimeZone};
use std::sync::OnceLock;

static SYSTEM: OnceLock<TimeZone> = OnceLock::new();

/// Call from the IO startup owner, before publishing messages to the UI.
pub fn initialize() {
    SYSTEM.get_or_init(TimeZone::system);
}

fn in_zone(milliseconds: u64, zone: &TimeZone) -> Option<String> {
    if zone.is_unknown() {
        return None;
    }
    let instant = Timestamp::from_millisecond(i64::try_from(milliseconds).ok()?).ok()?;
    let civil = instant.to_zoned(zone.clone());
    Some(format!("{:02}:{:02}", civil.hour(), civil.minute()))
}

/// Keep the original UTC instant in records; only the display label is local.
pub fn hm(milliseconds: u64) -> String {
    SYSTEM
        .get()
        .and_then(|zone| in_zone(milliseconds, zone))
        .unwrap_or_else(|| "—".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn historical_local_times_follow_dst_and_half_hour_offsets() {
        let zone = TimeZone::get("America/Los_Angeles").unwrap();
        for (instant, expected) in [
            ("2026-01-15T18:06:00Z", "10:06"),
            ("2026-06-15T18:06:00Z", "11:06"),
            ("2026-03-08T09:59:00Z", "01:59"),
            ("2026-03-08T10:00:00Z", "03:00"),
            ("2026-11-01T08:59:00Z", "01:59"),
            ("2026-11-01T09:00:00Z", "01:00"),
        ] {
            let ts: Timestamp = instant.parse().unwrap();
            assert_eq!(
                in_zone(ts.as_millisecond() as u64, &zone).as_deref(),
                Some(expected),
                "{instant}"
            );
        }
        let ts: Timestamp = "2026-10-28T18:06:00Z".parse().unwrap();
        assert_eq!(
            in_zone(
                ts.as_millisecond() as u64,
                &TimeZone::get("Asia/Kolkata").unwrap()
            )
            .as_deref(),
            Some("23:36")
        );
        assert_eq!(in_zone(u64::MAX, &zone), None);
        assert_eq!(in_zone(0, &TimeZone::unknown()), None);
    }

    #[test]
    #[allow(
        clippy::disallowed_methods,
        reason = "private process isolates system timezone environment"
    )]
    fn startup_uses_the_machine_zone_without_changing_the_parent_environment() {
        const CHILD: &str = "BBOXY_LOCAL_TIME_TEST_EXPECTED";
        let instant: Timestamp = "2026-10-28T18:06:00Z".parse().unwrap();
        if let Ok(expected) = std::env::var(CHILD) {
            initialize();
            assert_eq!(hm(instant.as_millisecond() as u64), expected);
            return;
        }
        for (zone, expected) in [("America/Los_Angeles", "11:06"), ("Asia/Kolkata", "23:36")] {
            let home = tempfile::tempdir().unwrap();
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "local_time::tests::startup_uses_the_machine_zone_without_changing_the_parent_environment", "--nocapture"])
                .env("HOME", home.path())
                .env("TZ", zone)
                .env(CHILD, expected)
                .output().unwrap();
            assert!(
                output.status.success(),
                "{zone}: {}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}
