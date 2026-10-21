//! CoreText grayscale glyph rasterization. Shaping and colour glyphs are later work.

#[derive(Clone, Debug)]
pub struct FontConfig { pub family: String, pub size: f32, pub scale: u32 }

impl Default for FontConfig {
    fn default() -> Self { Self { family: "Menlo".into(), size: 13.0, scale: 1 } }
}

#[derive(Clone, Copy, Debug)]
pub struct FontMetrics { pub cell_width: u32, pub cell_height: u32, pub baseline: i16 }

pub struct RasterizedGlyph {
    pub left: i16, pub top: i16, pub width: u32, pub height: u32, pub mask: Vec<u8>,
}

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::{Font, register_font_file};

#[cfg(not(target_os = "macos"))]
pub struct Font;
#[cfg(not(target_os = "macos"))]
impl Font {
    pub fn new(_: &FontConfig) -> crate::Result<Self> { Err("CoreText requires macOS".into()) }
    pub fn metrics(&self) -> FontMetrics { unreachable!() }
    pub fn effective_name(&self) -> String { unreachable!() }
    pub fn rasterize(&self, _: char) -> crate::Result<Option<RasterizedGlyph>> { unreachable!() }
}
