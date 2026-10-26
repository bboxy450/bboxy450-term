// modified by bboxy450.
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use arrayvec::ArrayVec;
use bitflags::bitflags;
use serde::{Deserialize, Serialize};

use crate::grid::{self, GridCell};
use crate::index::Column;
use crate::vte::ansi::{Color, Hyperlink as VteHyperlink, NamedColor};

/// Maximum number of zerowidth characters to retain per grid cell.
///
/// This enforces an upper bound on memory usage per cell, to ensure
/// malicous applications cannot use this as a DoS vector.
const MAX_ZEROWIDTH_CHARS: usize = 9;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[derive(Serialize, Deserialize)]
    /// Cell rendition, underline, wrapping and wide-character layout flags.
    pub struct Flags: u16 {
        /// Swap foreground and background for rendering.
        const INVERSE                   = 0b0000_0000_0000_0001;
        /// Use bold rendition.
        const BOLD                      = 0b0000_0000_0000_0010;
        /// Use italic rendition.
        const ITALIC                    = 0b0000_0000_0000_0100;
        /// Combined bold and italic rendition.
        const BOLD_ITALIC               = 0b0000_0000_0000_0110;
        /// Use a single underline.
        const UNDERLINE                 = 0b0000_0000_0000_1000;
        /// This row continues into the following row.
        const WRAPLINE                  = 0b0000_0000_0001_0000;
        /// Leading cell of a full-width character.
        const WIDE_CHAR                 = 0b0000_0000_0010_0000;
        /// Trailing cell reserved by a full-width character.
        const WIDE_CHAR_SPACER          = 0b0000_0000_0100_0000;
        /// Use dim rendition.
        const DIM                       = 0b0000_0000_1000_0000;
        /// Combined dim and bold rendition.
        const DIM_BOLD                  = 0b0000_0000_1000_0010;
        /// Suppress the character while retaining its cell.
        const HIDDEN                    = 0b0000_0001_0000_0000;
        /// Use a strike-through line.
        const STRIKEOUT                 = 0b0000_0010_0000_0000;
        /// Spacer preceding a wrapped full-width character.
        const LEADING_WIDE_CHAR_SPACER  = 0b0000_0100_0000_0000;
        /// Use a double underline.
        const DOUBLE_UNDERLINE          = 0b0000_1000_0000_0000;
        /// Use a curled underline.
        const UNDERCURL                 = 0b0001_0000_0000_0000;
        /// Use a dotted underline.
        const DOTTED_UNDERLINE          = 0b0010_0000_0000_0000;
        /// Use a dashed underline.
        const DASHED_UNDERLINE          = 0b0100_0000_0000_0000;
        /// Union of supported underline-style flags.
        const ALL_UNDERLINES            = Self::UNDERLINE.bits() | Self::DOUBLE_UNDERLINE.bits()
                                        | Self::UNDERCURL.bits() | Self::DOTTED_UNDERLINE.bits()
                                        | Self::DASHED_UNDERLINE.bits();
    }
}

/// Counter for hyperlinks without explicit ID.
static HYPERLINK_ID_SUFFIX: AtomicU32 = AtomicU32::new(0);

/// Reference-counted hyperlink metadata shared by terminal cells.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hyperlink {
    inner: Arc<HyperlinkInner>,
}

impl Hyperlink {
    /// Store hyperlink metadata, deriving a local identifier when none is provided.
    pub fn new<T: ToString>(id: Option<T>, uri: String) -> Self {
        let inner = Arc::new(HyperlinkInner::new(id, uri));
        Self { inner }
    }

    /// Borrow the hyperlink's stable local identifier.
    pub fn id(&self) -> &str {
        &self.inner.id
    }

    /// Borrow the stored URI without opening or validating its destination.
    pub fn uri(&self) -> &str {
        &self.inner.uri
    }
}

impl From<VteHyperlink> for Hyperlink {
    fn from(value: VteHyperlink) -> Self {
        Self::new(value.id, value.uri)
    }
}

impl From<Hyperlink> for VteHyperlink {
    fn from(val: Hyperlink) -> Self {
        VteHyperlink {
            id: Some(val.id().to_owned()),
            uri: val.uri().to_owned(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct HyperlinkInner {
    /// Identifier for the given hyperlink.
    id: String,

    /// Resource identifier of the hyperlink.
    uri: String,
}

impl HyperlinkInner {
    /// Store hyperlink metadata, deriving a local identifier when none is provided.
    pub fn new<T: ToString>(id: Option<T>, uri: String) -> Self {
        let id = match id {
            Some(id) => id.to_string(),
            None => {
                let mut id = HYPERLINK_ID_SUFFIX
                    .fetch_add(1, Ordering::Relaxed)
                    .to_string();
                id.push_str("_alacritty");
                id
            }
        };

        Self { id, uri }
    }
}

/// Trait for determining if a reset should be performed.
pub trait ResetDiscriminant<T> {
    /// Value based on which equality for the reset will be determined.
    fn discriminant(&self) -> T;
}

impl<T: Copy> ResetDiscriminant<T> for T {
    fn discriminant(&self) -> T {
        *self
    }
}

impl ResetDiscriminant<Color> for Cell {
    fn discriminant(&self) -> Color {
        self.bg
    }
}

/// Dynamically allocated cell content.
///
/// This storage is reserved for cell attributes which are rarely set. This allows reducing the
/// allocation required ahead of time for every cell, with some additional overhead when the extra
/// storage is actually required.
#[derive(Default, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct CellExtra {
    zerowidth: ArrayVec<char, MAX_ZEROWIDTH_CHARS>,
    underline_color: Option<Color>,
    hyperlink: Option<Hyperlink>,
}

/// Content and attributes of a single cell in the terminal grid.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    /// Base Unicode character occupying this cell.
    pub c: char,
    /// Foreground color selection.
    pub fg: Color,
    /// Background color selection.
    pub bg: Color,
    /// Rendition and cell-layout flags.
    pub flags: Flags,
    /// Optional shared combining-character and hyperlink data.
    pub extra: Option<Arc<CellExtra>>,
}

impl Default for Cell {
    #[inline]
    fn default() -> Cell {
        Cell {
            c: ' ',
            bg: Color::Named(NamedColor::Background),
            fg: Color::Named(NamedColor::Foreground),
            flags: Flags::empty(),
            extra: None,
        }
    }
}

impl Cell {
    /// Conservative owned heap referenced by this cell, beyond its inline size.
    /// Shared attributes and links are counted for each cell reference so owners
    /// of copied row windows can enforce a bound without undercounting them.
    pub fn retained_extra_bytes(&self) -> usize {
        let Some(extra) = &self.extra else {
            return 0;
        };
        let mut bytes = std::mem::size_of::<CellExtra>() + 2 * std::mem::size_of::<usize>();
        if let Some(link) = &extra.hyperlink {
            bytes += std::mem::size_of::<HyperlinkInner>()
                + 2 * std::mem::size_of::<usize>()
                + link.inner.id.capacity()
                + link.inner.uri.capacity();
        }
        // Count every reference, including shared links. This overestimates
        // shared allocations instead of allowing a memory budget to undercount.
        bytes
    }

    /// Zerowidth characters stored in this cell.
    #[inline]
    pub fn zerowidth(&self) -> Option<&[char]> {
        self.extra.as_ref().map(|extra| extra.zerowidth.as_slice())
    }

    /// Write a new zerowidth character to this cell.
    #[inline]
    pub fn push_zerowidth(&mut self, character: char) {
        let extra = self.extra.get_or_insert(Default::default());
        let _ = Arc::make_mut(extra).zerowidth.try_push(character);
    }

    /// Remove all wide char data from a cell.
    #[inline(never)]
    pub fn clear_wide(&mut self) {
        self.flags.remove(Flags::WIDE_CHAR);
        if let Some(extra) = self.extra.as_mut() {
            Arc::make_mut(extra).zerowidth = ArrayVec::new();
        }
        self.c = ' ';
    }

    /// Set underline color on the cell.
    pub fn set_underline_color(&mut self, color: Option<Color>) {
        // If we reset color and we don't have zerowidth we should drop extra storage.
        if color.is_none()
            && self
                .extra
                .as_ref()
                .is_none_or(|extra| extra.zerowidth.is_empty() && extra.hyperlink.is_none())
        {
            self.extra = None;
        } else {
            let extra = self.extra.get_or_insert(Default::default());
            Arc::make_mut(extra).underline_color = color;
        }
    }

    /// Underline color stored in this cell.
    #[inline]
    pub fn underline_color(&self) -> Option<Color> {
        self.extra.as_ref()?.underline_color
    }

    /// Set hyperlink.
    pub fn set_hyperlink(&mut self, hyperlink: Option<Hyperlink>) {
        let should_drop = hyperlink.is_none()
            && self
                .extra
                .as_ref()
                .is_none_or(|extra| extra.zerowidth.is_empty() && extra.underline_color.is_none());

        if should_drop {
            self.extra = None;
        } else {
            let extra = self.extra.get_or_insert(Default::default());
            Arc::make_mut(extra).hyperlink = hyperlink;
        }
    }

    /// Hyperlink stored in this cell.
    #[inline]
    pub fn hyperlink(&self) -> Option<Hyperlink> {
        self.extra.as_ref()?.hyperlink.clone()
    }
}

impl GridCell for Cell {
    #[inline]
    fn is_empty(&self) -> bool {
        (self.c == ' ' || self.c == '\t')
            && self.bg == Color::Named(NamedColor::Background)
            && self.fg == Color::Named(NamedColor::Foreground)
            && !self.flags.intersects(
                Flags::INVERSE
                    | Flags::ALL_UNDERLINES
                    | Flags::STRIKEOUT
                    | Flags::WRAPLINE
                    | Flags::WIDE_CHAR_SPACER
                    | Flags::LEADING_WIDE_CHAR_SPACER,
            )
            && self.extra.as_ref().map(|extra| extra.zerowidth.is_empty()) != Some(false)
    }

    #[inline]
    fn flags(&self) -> &Flags {
        &self.flags
    }

    #[inline]
    fn flags_mut(&mut self) -> &mut Flags {
        &mut self.flags
    }

    #[inline]
    fn reset(&mut self, template: &Self) {
        *self = Cell {
            bg: template.bg,
            ..Cell::default()
        };
    }
}

impl From<Color> for Cell {
    #[inline]
    fn from(color: Color) -> Self {
        Self {
            bg: color,
            ..Cell::default()
        }
    }
}

/// Get the length of occupied cells in a line.
pub trait LineLength {
    /// Calculate the occupied line length.
    fn line_length(&self) -> Column;
}

impl LineLength for grid::Row<Cell> {
    fn line_length(&self) -> Column {
        let mut length = Column(0);

        if self[Column(self.len() - 1)].flags.contains(Flags::WRAPLINE) {
            return Column(self.len());
        }

        for (index, cell) in self[..].iter().rev().enumerate() {
            if cell.c != ' '
                || cell.extra.as_ref().map(|extra| extra.zerowidth.is_empty()) == Some(false)
            {
                length = Column(self.len() - index);
                break;
            }
        }

        length
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::mem;

    use crate::grid::Row;
    use crate::index::Column;

    // keeps: spec terminal-core §D1
    #[test]
    fn cell_size_is_below_cap() {
        // Expected cell size on 64-bit architectures.
        const EXPECTED_CELL_SIZE: usize = 24;

        // Ensure that cell size isn't growing by accident.
        assert!(mem::size_of::<Cell>() <= EXPECTED_CELL_SIZE);
    }

    // keeps: spec terminal-core §D1
    #[test]
    fn line_length_works() {
        let mut row = Row::<Cell>::new(10);
        row[Column(5)].c = 'a';

        assert_eq!(row.line_length(), Column(6));
    }

    // keeps: spec terminal-core §D1
    #[test]
    fn line_length_works_with_wrapline() {
        let mut row = Row::<Cell>::new(10);
        row[Column(9)].flags.insert(super::Flags::WRAPLINE);

        assert_eq!(row.line_length(), Column(10));
    }
}
