//! Test-account retained-read masks. Capture bytes and live replay are untouched.
use crate::{
    Ulid,
    store::{Result, invalid, sqlite::SqliteStore},
};
use bbt_proto::gui::HistoryRecord;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

const RANGES: usize = 8;
pub(crate) const INTERVAL: u64 = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Missing {
    pub from: u64,
    pub to: u64,
}

#[derive(Clone)]
pub(crate) struct Masks(SqliteStore);
impl Masks {
    pub fn new(store: SqliteStore) -> Self {
        Self(store)
    }
    fn key(seat: Ulid, generation: u64) -> String {
        format!("debug/test-history-gap/{seat}/{generation}")
    }
    pub fn ranges(&self, seat: Ulid, generation: u64) -> Result<Vec<Missing>> {
        let bytes: Option<Vec<u8>> = self
            .0
            .connection()?
            .query_row(
                "SELECT value FROM kv WHERE key=?1",
                [Self::key(seat, generation)],
                |r| r.get(0),
            )
            .optional()?;
        let ranges: Vec<Missing> = bytes
            .map(|b| serde_json::from_slice(&b))
            .transpose()?
            .unwrap_or_default();
        if ranges
            .windows(2)
            .any(|w| w[0].to.saturating_add(1) >= w[1].from)
            || ranges.len() > RANGES
            || ranges
                .iter()
                .any(|m| m.from == 0 || m.to < m.from || m.to - m.from >= INTERVAL)
        {
            return Err(invalid("ring_gap_mask_invalid"));
        }
        Ok(ranges)
    }
    /// Called only after the logic owner verifies the actual test-account chain.
    pub fn install(&self, seat: Ulid, generation: u64, from: u64, to: u64) -> Result<()> {
        if seat == Ulid::nil() || generation == 0 || from == 0 || to < from || to - from >= INTERVAL
        {
            return Err(invalid(
                "ring_gap_invalid_range: 1..4096 retained capture sequences",
            ));
        }
        let mut db = self.0.connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let key = Self::key(seat, generation);
        let bytes: Option<Vec<u8>> = tx
            .query_row("SELECT value FROM kv WHERE key=?1", [&key], |r| r.get(0))
            .optional()?;
        let mut ranges: Vec<Missing> = bytes
            .map(|b| serde_json::from_slice(&b))
            .transpose()?
            .unwrap_or_default();
        let missing = Missing { from, to };
        if ranges.contains(&missing) {
            return Ok(());
        }
        if ranges.len() >= RANGES {
            return Err(invalid(
                "ring_gap_fixture_limit: eight intervals per generation",
            ));
        }
        if ranges
            .iter()
            .any(|m| m.to.saturating_add(1) >= from && to.saturating_add(1) >= m.from)
        {
            return Err(invalid("ring_gap_overlap: intervals must be separate"));
        }
        ranges.push(missing);
        ranges.sort_by_key(|m| m.from);
        tx.execute(
            "INSERT OR REPLACE INTO kv(key,value) VALUES(?1,?2)",
            params![key, serde_json::to_vec(&ranges)?],
        )?;
        tx.commit()?;
        Ok(())
    }
}

/// Replace masked output/control records with one explicit gap, even when a
/// missing interval spans the input page boundary. Advance next beyond it, so
/// the next page cannot repeat the marker or silently reintroduce masked bytes.
pub(crate) fn project(
    records: &mut Vec<HistoryRecord>,
    next: &mut Option<u64>,
    to: u64,
    masks: &[Missing],
) {
    if masks.is_empty() {
        return;
    }
    let mut result = Vec::with_capacity(records.len());
    let mut covered = None;
    for record in records.drain(..) {
        match record {
            HistoryRecord::Bytes { sequence, bytes } => {
                if covered.is_some_and(|end| sequence <= end) {
                    continue;
                }
                if let Some(missing) = masks
                    .iter()
                    .find(|m| m.from <= sequence && sequence <= m.to)
                {
                    let end = missing.to.min(to);
                    result.push(HistoryRecord::Gap {
                        from: sequence,
                        to: end,
                        reason: "test-account retained-history mask; original ring bytes retained"
                            .into(),
                    });
                    covered = Some(end);
                } else {
                    result.push(HistoryRecord::Bytes { sequence, bytes });
                    covered = Some(sequence);
                }
            }
            HistoryRecord::Gap {
                mut from,
                to,
                reason,
            } => {
                if covered.is_some_and(|end| to <= end) {
                    continue;
                }
                if let Some(end) = covered {
                    from = from.max(end.saturating_add(1));
                }
                result.push(HistoryRecord::Gap { from, to, reason });
                covered = Some(to);
            }
        }
    }
    if let Some(end) = covered {
        if end >= to {
            *next = None;
        } else if next.is_some_and(|cursor| cursor <= end) {
            *next = Some(end + 1);
        }
    }
    *records = result;
}

#[cfg(test)]
mod tests {
    use super::*;
    fn bytes(from: u64, to: u64) -> Vec<HistoryRecord> {
        (from..=to)
            .map(|sequence| HistoryRecord::Bytes {
                sequence,
                bytes: vec![sequence as u8],
            })
            .collect()
    }
    #[test]
    fn mask_is_durable_generation_bound_and_does_not_duplicate_a_cross_page_gap() {
        let root = tempfile::tempdir().unwrap();
        let seat = Ulid::new();
        let mask = Masks::new(SqliteStore::open(root.path()).unwrap());
        mask.install(seat, 1, 3, 5).unwrap();
        mask.install(seat, 1, 3, 5).unwrap();
        let reopened = Masks::new(SqliteStore::open(root.path()).unwrap());
        assert!(reopened.ranges(seat, 2).unwrap().is_empty());
        let ranges = reopened.ranges(seat, 1).unwrap();
        assert_eq!(ranges.len(), 1);
        let mut first = bytes(1, 3);
        let mut next = Some(4);
        project(&mut first, &mut next, 8, &ranges);
        assert_eq!(next, Some(6));
        assert!(matches!(
            &first[2],
            HistoryRecord::Gap { from: 3, to: 5, .. }
        ));
        let mut second = bytes(6, 8);
        project(&mut second, &mut None, 8, &ranges);
        assert_eq!(second, bytes(6, 8));
        assert_eq!(
            first
                .iter()
                .chain(&second)
                .filter(|r| matches!(r, HistoryRecord::Gap { .. }))
                .count(),
            1
        );
    }
    #[test]
    fn invalid_oversized_overlapping_and_excess_intervals_are_refused() {
        let root = tempfile::tempdir().unwrap();
        let masks = Masks::new(SqliteStore::open(root.path()).unwrap());
        let seat = Ulid::new();
        for (from, to) in [(0, 1), (2, 1), (1, u64::MAX), (1, 4097)] {
            assert!(masks.install(seat, 1, from, to).is_err());
        }
        for i in 0..8 {
            masks.install(seat, 1, 1 + i * 4, 2 + i * 4).unwrap();
        }
        assert!(masks.install(seat, 1, 100, 101).is_err());
        assert!(masks.install(seat, 2, 1, 2).is_ok());
        assert!(masks.install(seat, 2, 2, 3).is_err());
        let mut record = bytes(2, 2);
        let mut next = Some(3);
        project(&mut record, &mut next, 2, &[Missing { from: 1, to: 4 }]);
        assert!(matches!(
            &record[0],
            HistoryRecord::Gap { from: 2, to: 2, .. }
        ));
        assert_eq!(next, None);
    }

    #[test]
    fn paged_masks_preserve_both_sides_and_never_skip_unmasked_records() {
        let masks = [Missing { from: 5, to: 12 }, Missing { from: 23, to: 25 }];
        for limit in 1..=7 {
            let mut at = 1;
            let mut result = Vec::new();
            loop {
                let last = (at + limit - 1).min(40);
                let mut page = bytes(at, last);
                let mut next = (last < 40).then_some(last + 1);
                project(&mut page, &mut next, 40, &masks);
                result.extend(page);
                match next {
                    Some(next) => {
                        assert!(next > at);
                        at = next;
                    }
                    None => break,
                }
            }
            let kept: Vec<_> = result
                .iter()
                .filter_map(|r| match r {
                    HistoryRecord::Bytes { sequence, bytes } => Some((*sequence, bytes.clone())),
                    _ => None,
                })
                .collect();
            assert_eq!(
                kept,
                (1..=40)
                    .filter(|n| !(5..=12).contains(n) && !(23..=25).contains(n))
                    .map(|n| (n, vec![n as u8]))
                    .collect::<Vec<_>>()
            );
            let gaps: Vec<_> = result
                .iter()
                .filter_map(|r| match r {
                    HistoryRecord::Gap { from, to, .. } => Some((*from, *to)),
                    _ => None,
                })
                .collect();
            assert_eq!(gaps, vec![(5, 12), (23, 25)]);
        }
    }
}
