// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Retained composer and pure draft control. Destinations are supplied by the
//! Atomic image markers stay in the same UTF-16 draft editor as ordinary text.
//! Inspired by Codex's image marker interaction (https://github.com/openai/codex).
//! Implemented independently.
use super::*;

#[derive(Debug, Clone)]
pub(super) struct ImageSpan {
    id: String,
    start: usize,
    end: usize,
}
pub(super) fn spans(view: &Composer) -> Vec<ImageSpan> {
    view.attach
        .iter()
        .filter(|a| a.kind == "image" && !a.title.is_empty())
        .filter_map(|a| {
            let at = view.text.find(&a.title)?;
            let start = view.text[..at].encode_utf16().count();
            Some(ImageSpan {
                id: a.id.clone(),
                start,
                end: start + a.title.encode_utf16().count(),
            })
        })
        .collect()
}
impl Controller {
    pub(super) fn edit_images(&mut self, command: EditCommand) -> bool {
        let destructive = matches!(
            command,
            EditCommand::Insert(_) | EditCommand::Commit(_) | EditCommand::Delete { .. }
        );
        let old_len = self.view.text.encode_utf16().count();
        let edit = self.body.edit();
        let [mut start, mut end] = edit.replacement.unwrap_or(self.body.selection());
        if destructive && (!self.body.composing() || matches!(command, EditCommand::Commit(_))) {
            if start == end
                && let EditCommand::Delete { backwards } = &command
            {
                if *backwards {
                    start = start.saturating_sub(1);
                } else {
                    end += 1;
                }
            }
            let mut expanded = false;
            for span in &self.image_spans {
                if start < span.end && end > span.start
                    || start == end && start > span.start && start < span.end
                {
                    start = start.min(span.start);
                    end = end.max(span.end);
                    expanded = true;
                }
            }
            if expanded {
                self.body.handle(
                    &mut self.view.text,
                    EditCommand::SetCaret {
                        at: start,
                        extend: false,
                    },
                );
                self.body.handle(
                    &mut self.view.text,
                    EditCommand::SetCaret {
                        at: end,
                        extend: true,
                    },
                );
            }
        }
        let changed = self.body.handle(&mut self.view.text, command);
        if changed && destructive {
            let delta = self.view.text.encode_utf16().count() as isize - old_len as isize;
            self.image_spans.retain_mut(|span| {
                if start < span.end && end > span.start
                    || start == end && start > span.start && start < span.end
                {
                    return false;
                }
                if span.start >= end {
                    span.start = span.start.saturating_add_signed(delta);
                    span.end = span.end.saturating_add_signed(delta);
                }
                true
            });
            self.view
                .attach
                .retain(|a| a.kind != "image" || self.image_spans.iter().any(|s| s.id == a.id));
        }
        changed
    }
    pub(super) fn attach_image(&mut self, mut image: Attachment) -> Outcome {
        if self.view.mode == "delegate" {
            return Outcome::Refused("delegation_requires_one_task");
        }
        if self.view.attach.len() >= 32 {
            return Outcome::Refused("attachment_count");
        }
        if self.composing() {
            return Outcome::Refused("image_paste_during_composition");
        }
        if image.id.is_empty() || self.view.attach.iter().any(|a| a.id == image.id) {
            return Outcome::Refused("image_identity_required");
        }
        while self
            .view
            .text
            .contains(&format!("[Image #{}]", self.next_image))
        {
            self.next_image += 1;
        }
        image.title = format!("[Image #{}]", self.next_image);
        self.next_image += 1;
        image.akind = "image".into();
        self.view.field = Field::Body;
        self.view.focus = true;
        self.view.attach_pick = None;
        self.view.kind_menu = false;
        self.edit_images(EditCommand::Insert(image.title.clone()));
        let end = self.body.edit().caret.unwrap_or_default();
        self.image_spans.push(ImageSpan {
            id: image.id.clone(),
            start: end - image.title.encode_utf16().count(),
            end,
        });
        self.view.attach.push(image);
        self.sync();
        Outcome::Changed
    }
    pub(super) fn remove_image_marker(&mut self, index: usize) {
        let image = &self.view.attach[index];
        if image.kind != "image" {
            return;
        }
        if let Some(index) = self.image_spans.iter().position(|s| s.id == image.id) {
            let span = self.image_spans.remove(index);
            self.body.handle(
                &mut self.view.text,
                EditCommand::SetCaret {
                    at: span.start,
                    extend: false,
                },
            );
            self.body.handle(
                &mut self.view.text,
                EditCommand::SetCaret {
                    at: span.end,
                    extend: true,
                },
            );
            self.body
                .handle(&mut self.view.text, EditCommand::Insert(String::new()));
            for other in &mut self.image_spans {
                if other.start >= span.end {
                    other.start -= span.end - span.start;
                    other.end -= span.end - span.start;
                }
            }
        }
    }
}
