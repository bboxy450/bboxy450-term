// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Bounded, memory-only provenance for the grids an actual frame encoded.
use std::collections::BTreeMap;

use bbt_proto::{
    SeatId, View,
    gui::{UiRenderedScreen, UiScreenIngestion},
};

use crate::model::{Model, PreparedScreen};

struct Ingress {
    view: View,
    sequence: u64,
    after: u64,
    first: Option<(u64, View, u64)>,
}

#[derive(Default)]
pub struct Frames {
    submitted: u64,
    panes: BTreeMap<SeatId, Ingress>,
}

impl Frames {
    /// Called when the GUI owner takes a prepared publication, not when ctl
    /// subsequently asks for it. Metadata-only publications keep the old origin.
    pub fn ingest(&mut self, model: &Model) {
        self.panes.retain(|seat, _| {
            model
                .prepared_screens
                .get(seat)
                .is_some_and(|source| source.baseline.is_some())
        });
        for (&seat, source) in &model.prepared_screens {
            let Some((view, sequence)) = source.baseline else {
                continue;
            };
            if !self
                .panes
                .get(&seat)
                .is_some_and(|old| old.view == view && old.sequence == sequence)
            {
                self.panes.insert(
                    seat,
                    Ingress {
                        view,
                        sequence,
                        after: self.submitted,
                        first: None,
                    },
                );
            }
        }
    }

    /// Only a successful submit calls this. Preserve its actual source rather
    /// than repairing the first-frame evidence when a later frame catches up.
    pub fn produced(&mut self, frame: u64, model: &Model, visible: impl Iterator<Item = SeatId>) {
        self.submitted = frame;
        for seat in visible {
            if let Some(ingress) = self.panes.get_mut(&seat)
                && ingress.first.is_none()
                && let Some((view, sequence)) = model
                    .prepared_screens
                    .get(&seat)
                    .and_then(|source| source.baseline)
            {
                ingress.first = Some((frame, view, sequence));
            }
        }
    }

    pub fn read(
        &self,
        seat: SeatId,
        source: &PreparedScreen,
        remaining: &mut usize,
    ) -> Option<UiRenderedScreen> {
        let (view, sequence) = source.baseline?;
        let ingress = self.panes.get(&seat)?;
        let (first_frame_seq, first_drawn_view, first_drawn_sequence) = ingress.first?;
        let lines = bounded_lines(source, remaining);
        Some(UiRenderedScreen {
            view,
            sequence,
            lines,
            pixel_size: None, // the submit owner supplies its actual PreparedGrid dimensions
            ingested: UiScreenIngestion {
                view: ingress.view,
                sequence: ingress.sequence,
                after_frame_seq: ingress.after,
                first_frame_seq,
                first_drawn_view,
                first_drawn_sequence,
            },
        })
    }
}

fn bounded_lines(source: &PreparedScreen, remaining: &mut usize) -> Option<Vec<String>> {
    let mut lines = Vec::new();
    let mut bytes = 0;
    for row in &source.screen.cells {
        let mut line = String::new();
        for cell in row.iter().filter(|cell| {
            !cell.flags.intersects(
                bbt_core::term::cell::Flags::WIDE_CHAR_SPACER
                    | bbt_core::term::cell::Flags::LEADING_WIDE_CHAR_SPACER,
            )
        }) {
            for c in
                std::iter::once(cell.c).chain(cell.zerowidth().unwrap_or_default().iter().copied())
            {
                if bytes + line.len() + c.len_utf8() + 1 > *remaining {
                    return None;
                }
                line.push(c);
            }
        }
        line.truncate(line.trim_end().len());
        bytes += line.len() + 1;
        lines.push(line);
    }
    *remaining -= bytes;
    Some(lines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbt_mux::Ulid;
    use bbt_proto::Screen;
    use std::sync::Arc;

    fn model(sequence: u64) -> Model {
        let seat = SeatId::from_u128(1);
        let view = View {
            seat: Ulid::from(1u128),
            generation: 2,
            view_generation: Ulid::from(3u128),
        };
        let mut model = Model::default();
        model.prepared_screens.insert(
            seat,
            PreparedScreen {
                baseline: Some((view, sequence)),
                screen: Arc::new(Screen {
                    cols: 3,
                    rows: 1,
                    cells: vec![
                        "ABC"
                            .chars()
                            .map(|c| bbt_core::Cell {
                                c,
                                ..Default::default()
                            })
                            .collect(),
                    ],
                    cursor: (0, 0),
                    modes: 0,
                    palette: serde_json::Value::Null,
                    cursor_style: serde_json::Value::Null,
                    title: String::new(),
                    cwd: None,
                    gap: None,
                }),
            },
        );
        model
    }

    fn read(frames: &Frames, model: &Model, budget: usize) -> UiRenderedScreen {
        let seat = SeatId::from_u128(1);
        frames
            .read(seat, &model.prepared_screens[&seat], &mut { budget })
            .unwrap()
    }

    #[test]
    fn next_produced_frame_keeps_its_ingestion_and_actual_prepared_text() {
        let seat = SeatId::from_u128(1);
        let mut frames = Frames::default();
        frames.produced(4, &Model::default(), std::iter::empty());
        let old = model(7);
        frames.ingest(&old);
        frames.produced(5, &old, std::iter::once(seat));
        frames.ingest(&old); // unrelated metadata must not move the origin
        frames.produced(6, &old, std::iter::once(seat));
        let seen = read(&frames, &old, 10);
        assert_eq!(seen.lines, Some(vec!["ABC".into()]));
        assert_eq!(
            (seen.ingested.after_frame_seq, seen.ingested.first_frame_seq),
            (4, 5)
        );
        assert_eq!(seen.ingested.first_drawn_sequence, 7);
        let next = model(8);
        frames.ingest(&next);
        frames.produced(7, &model(9), std::iter::once(seat));
        let seen = read(&frames, &model(9), 10);
        assert_eq!(
            (seen.ingested.sequence, seen.ingested.first_drawn_sequence),
            (8, 9)
        );
    }

    #[test]
    fn late_catch_up_cannot_erase_the_first_frames_stale_source() {
        let seat = SeatId::from_u128(1);
        let mut frames = Frames::default();
        frames.ingest(&model(7));
        frames.produced(1, &model(5), std::iter::once(seat));
        frames.produced(2, &model(7), std::iter::once(seat));
        let seen = read(&frames, &model(7), 10);
        assert_eq!(seen.sequence, 7);
        assert_eq!(seen.ingested.sequence, 7);
        assert_eq!(seen.ingested.first_drawn_sequence, 5);
        assert_eq!(seen.ingested.first_frame_seq, 1);
    }

    #[test]
    fn unavailable_or_historical_sources_never_claim_live_frame_evidence() {
        let seat = SeatId::from_u128(1);
        let mut frames = Frames::default();
        let mut model = model(7);
        frames.ingest(&model);
        assert!(
            frames
                .read(seat, &model.prepared_screens[&seat], &mut 10)
                .is_none()
        );
        frames.produced(1, &model, std::iter::once(seat));
        assert!(read(&frames, &model, 2).lines.is_none());
        model.prepared_screens.get_mut(&seat).unwrap().baseline = None;
        frames.ingest(&model);
        assert!(
            frames
                .read(seat, &model.prepared_screens[&seat], &mut 10)
                .is_none()
        );
        assert!(frames.panes.is_empty());
    }
}
