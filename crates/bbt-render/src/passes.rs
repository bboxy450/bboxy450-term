//! Per-thread render-pass and copy tally for profiling. Each pass site bumps its kind; a host
//! drains the counts once per frame with `take`. Counting only increments: nothing drawn changes.
use std::cell::Cell;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub flat: u32,
    pub clear: u32,
    pub path: u32,
    pub group: u32,
    pub blur: u32,
    pub material: u32,
    pub cells: u32,
    pub copy: u32,
}
impl Counts {
    /// Render passes, excluding texture copies.
    pub fn passes(&self) -> u32 {
        self.flat + self.clear + self.path + self.group + self.blur + self.material + self.cells
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Kind {
    Flat,
    Clear,
    Path,
    Group,
    Blur,
    Material,
    Cells,
    Copy,
}

thread_local! {
    static COUNTS: Cell<Counts> = Cell::new(Counts::default());
}

pub fn count(kind: Kind) {
    COUNTS.with(|counts| {
        let mut c = counts.get();
        let n = match kind {
            Kind::Flat => &mut c.flat,
            Kind::Clear => &mut c.clear,
            Kind::Path => &mut c.path,
            Kind::Group => &mut c.group,
            Kind::Blur => &mut c.blur,
            Kind::Material => &mut c.material,
            Kind::Cells => &mut c.cells,
            Kind::Copy => &mut c.copy,
        };
        *n += 1;
        counts.set(c);
    });
}

/// This thread's counts since the last take.
pub fn take() -> Counts {
    COUNTS.with(|counts| counts.replace(Counts::default()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counts_accumulate_per_kind_and_take_resets() {
        take();
        for kind in [Kind::Flat, Kind::Flat, Kind::Path, Kind::Copy] {
            count(kind);
        }
        let c = take();
        assert_eq!((c.flat, c.path, c.copy, c.passes()), (2, 1, 1, 3));
        assert_eq!(take(), Counts::default());
    }
}
