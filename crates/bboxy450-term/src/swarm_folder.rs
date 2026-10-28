// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The native picker returns an intent; it never changes the process-wide cwd.
use crate::{App, UiEvent, UiProxy, live};
use bbt_proto::SwarmId;

impl App {
    pub(super) fn set_swarm_workdir(
        &mut self,
        swarm: SwarmId,
        path: Option<String>,
    ) -> Result<(), String> {
        if path
            .as_deref()
            .is_some_and(|path| !bbt_proto::gui::valid_workdir(path))
        {
            return Err("invalid_swarm_workdir: expected an absolute folder path".into());
        }
        if !self.live_state.as_ref().is_some_and(|model| {
            model
                .ui
                .window
                .sidebar
                .swarms
                .iter()
                .any(|row| row.id == swarm)
        }) {
            return Err("swarm_unavailable".into());
        }
        self.queue(live::UiMsg::SwarmWorkdir { swarm, path });
        // Choosing a folder in a left-hand swarm makes subsequent Cmd-T picks
        // target that same swarm, including one which has no harnesses yet.
        let first = self
            .live_state
            .as_ref()
            .and_then(|model| {
                model
                    .ui
                    .window
                    .sidebar
                    .swarms
                    .iter()
                    .find(|row| row.id == swarm)
            })
            .and_then(|row| row.tabs.first())
            .and_then(|tab| tab.tree.leaves().first().copied());
        if self.focus.stage.as_ref().and_then(|stage| stage.swarm) != Some(swarm) {
            if let Some(seat) = first {
                self.focus_action(crate::FocusAction::Activate(
                    seat,
                    bbt_ui_model::input::FocusOrigin::Row,
                ));
            } else {
                self.select_empty_swarm(swarm);
            }
        }
        self.request_redraw();
        Ok(())
    }
}

#[cfg(target_os = "macos")]
pub(super) fn choose(
    swarm: SwarmId,
    proxy: UiProxy,
    directory: Option<&str>,
) -> Result<(), String> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSModalResponseOK, NSOpenPanel};
    use objc2_foundation::{NSString, NSURL};
    let mtm = MainThreadMarker::new().ok_or("folder picker requires the window thread")?;
    let application = NSApplication::sharedApplication(mtm);
    let window = application
        .keyWindow()
        .or_else(|| application.mainWindow())
        .ok_or("folder picker requires an active terminal window")?;
    let panel = NSOpenPanel::openPanel(mtm);
    if let Some(directory) = directory {
        panel.setDirectoryURL(Some(&NSURL::fileURLWithPath(&NSString::from_str(
            directory,
        ))));
    }
    panel.setCanChooseDirectories(true);
    panel.setCanChooseFiles(false);
    panel.setAllowsMultipleSelection(false);
    panel.setTitle(Some(&NSString::from_str("Swarm working folder")));
    panel.setMessage(Some(&NSString::from_str(
        "New harnesses in this swarm start in this folder.",
    )));
    panel.setPrompt(Some(&NSString::from_str("Use folder")));
    let selected = panel.clone();
    let complete = block2::RcBlock::new(move |response| {
        let path = if response == NSModalResponseOK {
            selected
                .URLs()
                .firstObject()
                .and_then(|url| url.path())
                .map(|path| path.to_string())
        } else {
            None
        };
        let _ = proxy.send_event(UiEvent::SwarmFolderPicked { swarm, path });
    });
    // Keep the chooser attached to its terminal instead of opening a modeless
    // panel behind it. Origin/credit policy:
    // https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
    panel.beginSheetModalForWindow_completionHandler(&window, &complete);
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub(super) fn choose(_: SwarmId, _: UiProxy, _: Option<&str>) -> Result<(), String> {
    Err("Native folder selection is unavailable; use ctl ui swarm-workdir.".into())
}
