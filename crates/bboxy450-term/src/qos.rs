// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Process entry classes. UI threads outrank logic; tools retain their own role.
pub fn startup(role: &str) {
    match role {
        "supervisor" | "logic" | "mcp" => bbt_rt::qos::initiated_thread(),
        "desktop" | "ui" => bbt_rt::qos::interactive_thread(),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(target_os = "macos")]
    fn measurement_flags_do_not_promote_tool_threads() {
        std::thread::spawn(|| {
            bbt_rt::qos::with_default_qos(|| {
                for role in ["", "--p1-headless", "--stall-test", "--headless", "plugin"] {
                    super::startup(role);
                    assert_eq!(bbt_rt::qos::requested_class(), 0x15, "{role}");
                }
            });
        })
        .join()
        .unwrap();
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn startup_role_ceilings() {
        for (role, expected) in [
            ("supervisor", 0x19),
            ("logic", 0x19),
            ("mcp", 0x19),
            ("desktop", 0x21),
            ("ui", 0x21),
        ] {
            std::thread::spawn(move || {
                super::startup(role);
                assert_eq!(bbt_rt::qos::requested_class(), expected, "{role}");
            })
            .join()
            .unwrap();
        }
    }
}
