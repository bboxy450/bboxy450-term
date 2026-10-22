//! Terminal snapshots and rendering, independent of the parser and app process.
//! Font discovery/rasterization belongs on the preparing worker, never the UI thread.
use crate::{Color, FontConfig, Result};

mod gpu;
mod sprites;
pub use gpu::{Image, TerminalRenderer};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Underline {
    #[default]
    None,
    Single,
    Double,
    Curly,
    Dotted,
    Dashed,
}

#[derive(Clone, Debug)]
pub struct Cell {
    /// One parser cell's base character and combining scalars; spacer cells are empty.
    pub text: String,
    pub width: u8,
    pub foreground: Color,
    pub background: Color,
    pub underline_color: Color,
    pub bold: bool,
    pub italic: bool,
    pub dim: bool,
    pub hidden: bool,
    pub underline: Underline,
    pub strike: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            text: " ".into(),
            width: 1,
            foreground: [232, 237, 247, 255],
            background: [15, 20, 29, 255],
            underline_color: [232, 237, 247, 255],
            bold: false,
            italic: false,
            dim: false,
            hidden: false,
            underline: Underline::None,
            strike: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CursorShape {
    #[default]
    Block,
    Bar,
    Underline,
}

#[derive(Clone, Copy, Debug)]
pub struct Cursor {
    pub column: u16,
    pub row: u16,
    pub shape: CursorShape,
    pub focused: bool,
    pub color: Color,
}

#[derive(Clone, Debug)]
pub struct Grid {
    pub columns: u16,
    pub rows: u16,
    pub cells: Vec<Cell>,
    pub cursor: Option<Cursor>,
}

impl Grid {
    pub fn validate(&self) -> Result<()> {
        if self.columns == 0
            || self.rows == 0
            || self.cells.len() != usize::from(self.columns) * usize::from(self.rows)
        {
            return Err("terminal cell count must match nonempty grid".into());
        }
        if self.cells.iter().any(|cell| cell.width > 2) {
            return Err("terminal cell width must be 0, 1 or 2".into());
        }
        if self
            .cursor
            .is_some_and(|c| c.column >= self.columns || c.row >= self.rows)
        {
            return Err("terminal cursor is outside the grid".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
pub struct Options {
    pub font: FontConfig,
    /// Logical line height multiplier; None uses rounded primary font metrics.
    pub line_height: Option<f32>,
    pub thicken: bool,
}
