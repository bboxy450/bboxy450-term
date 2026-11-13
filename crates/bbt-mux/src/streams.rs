// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! A bounded acknowledged baseline and one latest pending viewport per viewer.

use crate::store::{Result, invalid};

pub use bbt_proto::streams::{Delta, Screen, Snapshot, Update, View};

#[derive(Default)]
pub struct ViewStream {
    baseline: Option<Snapshot>,
    sent: Option<Snapshot>,
    pending: Option<Snapshot>,
}
impl ViewStream {
    pub fn offer(&mut self, snapshot: Snapshot) -> Result<()> {
        let latest = self
            .pending
            .as_ref()
            .or(self.sent.as_ref())
            .or(self.baseline.as_ref());
        if let Some(latest) = latest {
            if latest.view.seat != snapshot.view.seat
                || latest.view.generation > snapshot.view.generation
            {
                return Err(invalid("stale or wrong-seat viewport"));
            }
            if latest.view == snapshot.view && snapshot.sequence < latest.sequence {
                return Err(invalid("stale viewport sequence"));
            }
        }
        self.pending = Some(snapshot);
        Ok(())
    }
    pub fn next_update(&mut self) -> Option<Update> {
        if self.sent.is_some() {
            return None;
        }
        let next = self.pending.take()?;
        let update = if let Some(base) = &self.baseline
            && base.view == next.view
            && base.screen.cols == next.screen.cols
            && base.screen.rows == next.screen.rows
        {
            Update::Delta {
                delta: Box::new(Delta {
                    view: next.view,
                    base: base.sequence,
                    sequence: next.sequence,
                    rows: next
                        .screen
                        .cells
                        .iter()
                        .enumerate()
                        .filter(|(i, row)| base.screen.cells[*i] != **row)
                        .map(|(i, row)| (i, row.clone()))
                        .collect(),
                    cursor: next.screen.cursor,
                    modes: next.screen.modes,
                    palette: next.screen.palette.clone(),
                    cursor_style: next.screen.cursor_style.clone(),
                    title: next.screen.title.clone(),
                    cwd: next.screen.cwd.clone(),
                    gap: next.screen.gap.clone(),
                    images: (base.screen.images != next.screen.images)
                        .then(|| next.screen.images.clone()),
                    image_data: next
                        .screen
                        .image_data
                        .iter()
                        .filter(|d| {
                            !base
                                .screen
                                .image_data
                                .iter()
                                .any(|b| b.id == d.id && b.png == d.png)
                        })
                        .cloned()
                        .collect(),
                }),
            }
        } else {
            Update::Snapshot {
                snapshot: Box::new(next.clone()),
            }
        };
        self.sent = Some(next);
        Some(update)
    }
    pub fn applied(&mut self, view: View, sequence: u64) -> Result<()> {
        if self
            .baseline
            .as_ref()
            .is_some_and(|b| b.view == view && b.sequence == sequence)
        {
            return Ok(());
        }
        if self
            .sent
            .as_ref()
            .is_none_or(|s| s.view != view || s.sequence != sequence)
        {
            return Err(invalid("ack does not match sent viewport"));
        }
        self.baseline = self.sent.take();
        Ok(())
    }
    pub fn reset(&mut self) {
        self.baseline = None;
        self.sent = None;
    }
}

/// Minimal GUI-side reference: deltas are applied to an exact, atomic baseline.
#[derive(Default)]
pub struct Replica {
    pub snapshot: Option<Snapshot>,
}
impl Replica {
    pub fn apply(&mut self, update: Update) -> Result<()> {
        match update {
            Update::Snapshot { snapshot } => {
                if self.snapshot.as_ref().is_some_and(|old| {
                    old.view.seat == snapshot.view.seat
                        && (old.view.generation > snapshot.view.generation
                            || (old.view == snapshot.view && old.sequence > snapshot.sequence))
                }) {
                    return Err(invalid("stale snapshot"));
                }
                if snapshot.screen.cells.len() != snapshot.screen.rows
                    || snapshot
                        .screen
                        .cells
                        .iter()
                        .any(|r| r.len() != snapshot.screen.cols)
                {
                    return Err(invalid("invalid snapshot dimensions"));
                }
                self.snapshot = Some(*snapshot);
            }
            Update::Delta { delta } => {
                let Delta {
                    view,
                    base,
                    sequence,
                    rows,
                    cursor,
                    modes,
                    palette,
                    cursor_style,
                    title,
                    cwd,
                    gap,
                    images,
                    image_data,
                } = *delta;
                let old = self
                    .snapshot
                    .as_ref()
                    .ok_or_else(|| invalid("snapshot required"))?;
                if old.view != view || old.sequence != base || sequence < base {
                    return Err(invalid("baseline mismatch; snapshot required"));
                }
                if rows
                    .iter()
                    .any(|(i, row)| *i >= old.screen.rows || row.len() != old.screen.cols)
                {
                    return Err(invalid("invalid delta rows"));
                }
                let mut next = old.clone();
                for (i, row) in rows {
                    next.screen.cells[i] = row;
                }
                next.sequence = sequence;
                next.screen.cursor = cursor;
                next.screen.modes = modes;
                next.screen.palette = palette;
                next.screen.cursor_style = cursor_style;
                next.screen.title = title;
                next.screen.cwd = cwd;
                next.screen.gap = gap;
                if let Some(images) = images {
                    next.screen.images = images;
                }
                // Pixels arrive once; keep the ones still shown, drop the rest.
                for data in image_data {
                    next.screen.image_data.retain(|d| d.id != data.id);
                    next.screen.image_data.push(data);
                }
                let shown = &next.screen.images;
                next.screen
                    .image_data
                    .retain(|d| shown.iter().any(|i| i.id == d.id));
                if shown
                    .iter()
                    .any(|i| !next.screen.image_data.iter().any(|d| d.id == i.id))
                {
                    return Err(invalid("delta shows an image without its pixels"));
                }
                self.snapshot = Some(next);
            }
        }
        Ok(())
    }
}

pub use bbt_supervisor::ring::HistoryGap;

#[cfg(test)]
mod kitty_tests {
    use super::*;
    use bbt_core::Term;
    fn shot(term: &Term, sequence: u64) -> Snapshot {
        Snapshot {
            view: View {
                seat: ulid::Ulid::from(7_u128),
                generation: 1,
                view_generation: ulid::Ulid::from(1_u128),
            },
            sequence,
            screen: Screen::from_term(term, String::new(), None, None),
        }
    }
    // State: one viewer, a terminal that shows a picture then types beside it.
    // Safety: the first update carries the pixels; the next delta carries the
    // placement set only when it changed and never the pixels again; the
    // replica always holds pixels for every placement it shows.
    #[test]
    fn deltas_send_picture_pixels_once_and_placements_on_change() {
        let mut term = Term::new(20, 5, 10);
        let png = bboxy450_kitty_graphics::Image::new(0, 4, 4, vec![9; 64]).png();
        term.advance(&bboxy450_kitty_graphics::encode_png(&png, 1, 2, 2));
        let mut stream = ViewStream::default();
        let mut replica = Replica::default();
        stream.offer(shot(&term, 1)).unwrap();
        let first = stream.next_update().unwrap();
        let Update::Snapshot { snapshot } = &first else {
            panic!("first update is a snapshot")
        };
        assert_eq!(snapshot.screen.images.len(), 1);
        assert_eq!(snapshot.screen.image_data.len(), 1);
        let view = snapshot.view;
        replica.apply(first).unwrap();
        stream.applied(view, 1).unwrap();
        term.advance(b"x");
        stream.offer(shot(&term, 2)).unwrap();
        let Update::Delta { delta } = stream.next_update().unwrap() else {
            panic!("second update is a delta")
        };
        assert!(
            delta.images.is_none(),
            "unchanged placements are not resent"
        );
        assert!(delta.image_data.is_empty(), "pixels travel once");
        replica
            .apply(Update::Delta {
                delta: delta.clone(),
            })
            .unwrap();
        stream.applied(delta.view, 2).unwrap();
        term.advance(b"\x1b_Ga=d,d=A\x1b\\");
        stream.offer(shot(&term, 3)).unwrap();
        let Update::Delta { delta } = stream.next_update().unwrap() else {
            panic!("third update is a delta")
        };
        assert_eq!(delta.images.as_deref(), Some(&[][..]));
        replica.apply(Update::Delta { delta }).unwrap();
        let screen = &replica.snapshot.as_ref().unwrap().screen;
        assert!(screen.images.is_empty() && screen.image_data.is_empty());
        // A delta that shows an image without its pixels is refused.
        let mut bad = stream.sent.take().unwrap();
        bad.sequence = 4;
        let _ = stream;
        let mut broken = Replica::default();
        broken
            .apply(Update::Snapshot {
                snapshot: Box::new(bad.clone()),
            })
            .unwrap();
        let delta = Delta {
            view: bad.view,
            base: 4,
            sequence: 5,
            rows: vec![],
            cursor: bad.screen.cursor,
            modes: bad.screen.modes,
            palette: bad.screen.palette.clone(),
            cursor_style: bad.screen.cursor_style.clone(),
            title: String::new(),
            cwd: None,
            gap: None,
            images: Some(vec![bbt_proto::streams::ScreenImage {
                id: 42,
                row: 0,
                col: 0,
                cols: 1,
                rows: 1,
                z: 0,
                source: [0, 0, 1, 1],
                width: 1,
                height: 1,
            }]),
            image_data: vec![],
        };
        assert!(
            broken
                .apply(Update::Delta {
                    delta: Box::new(delta)
                })
                .is_err()
        );
    }
}
