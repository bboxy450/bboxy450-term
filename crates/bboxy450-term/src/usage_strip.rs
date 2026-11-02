// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The native owner receives usage events; private/headless fixture owners never read accounts.
impl crate::App {
    pub(super) fn start_usage(&mut self) {
        let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) else {
            return;
        };
        let proxy = self.proxy.clone();
        self.usage_worker = Some(bboxy450_usage::poller::Worker::start(
            bbt_rt::handle(),
            home.clone(),
            bboxy450_usage::providers(),
            std::sync::Arc::new(bboxy450_usage::io::LiveReader { home }),
            std::sync::Arc::new(bboxy450_usage::io::LiveHttp::default()),
            std::sync::Arc::new(move |update| {
                let _ = proxy.send_event(crate::UiEvent::Usage(update));
            }),
            Default::default(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use bboxy450_usage::{Reading, machine::Machine, poller::Update};
    #[test]
    fn machine_and_provider_events_update_only_the_last_view() {
        let mut scene = crate::scene::Scene::live();
        scene.apply_usage(Update::Machine(Machine {
            cpu: Some(42),
            memory: Some(71),
            disk: Some(63),
            reason: None,
        }));
        scene.apply_usage(Update::Provider {
            id: "claude",
            reading: Some(Reading::current(58)),
        });
        scene.apply_usage(Update::Provider {
            id: "grok",
            reading: Some(Reading::error("Run `grok` to sign in")),
        });
        scene.apply_usage(Update::Provider {
            id: "codex",
            reading: None,
        });
        // A signed-out account is hidden, not shown as "grok —".
        assert_eq!(scene.usage.providers.len(), 1);
        assert!(scene.usage.accessible_label().contains("CPU 42%"));
        assert!(scene.usage.accessible_label().contains("Claude 58% used"));
        // A refresh in flight keeps the last value instead of blinking out.
        scene.apply_usage(Update::Provider {
            id: "claude",
            reading: Some(Reading::error("Refreshing usage")),
        });
        assert!(scene.usage.accessible_label().contains("Claude 58% used"));
        assert!(!scene.usage.accessible_label().contains("rok"));
        scene.apply_usage(Update::Provider {
            id: "claude",
            reading: None,
        });
        assert_eq!(scene.usage.providers.len(), 0);
        let mut prepared = crate::scene::Scene::live();
        let _ = prepared.layout([1600., 900.], false).unwrap();
        prepared.inherit_usage(&mut scene);
        assert!(scene.usage.machine.is_empty());
        assert!(prepared.usage.accessible_label().contains("CPU 42%"));
        let installed = prepared.layout([1600., 900.], false).unwrap();
        assert!(format!("{installed:?}").contains("CPU 42%"));
    }

    #[test]
    fn theme_button_label_names_the_saved_choice_and_the_next_one() {
        let mut scene = crate::scene::Scene::live();
        let mut preferences = bboxy450_term::settings::Preferences::default();
        for (saved, want) in [
            ("auto", "Theme: Auto (system is light)\nClick for Light"),
            ("light", "Theme: Light\nClick for Dark"),
            ("dark", "Theme: Dark\nClick for Auto"),
        ] {
            preferences.appearance.theme = saved.into();
            scene.set_appearance(&preferences, bbt_ui::Theme::Light);
            // Text paints one run per word; join them back into the visible label.
            let paints = format!("{:?}", scene.theme_tooltip([1600., 900.]));
            let shown: String = paints
                .split("text: \"")
                .skip(1)
                .filter_map(|run| run.split('"').next())
                .collect();
            assert_eq!(shown, want.replace('\n', ""), "{saved}");
        }
    }
}
