// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The Kitty terminal graphics protocol, the subset that puts a picture in a
//! pane: transmit (direct, chunked, zlib), RGB, RGBA and PNG, display at the
//! cursor with cell extents, delete, and the query reply. No files, shared
//! memory, animation or Unicode placeholders.
//!
//! Protocol: https://sw.kovidgoyal.net/kitty/graphics-protocol/ (independent
//! implementation from the public specification; no code copied).
//!
//! A terminal owns one [`Graphics`]. It feeds every APC command through
//! [`Graphics::command`] with the cursor's row and column and gets back what
//! to write to the program and how the cursor moves; the terminal keeps the
//! placements on their rows as it scrolls.
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Read;

/// Decoded pixels. Bounded by [`MAX_IMAGE_PIXELS`]; larger transmissions are refused.
pub const MAX_IMAGE_PIXELS: u64 = 4096 * 4096;
/// Images kept per terminal; the oldest unreferenced one goes when a new one arrives.
pub const MAX_IMAGES: usize = 32;
/// Chunked transmissions in flight; a stale one is dropped when a new image starts.
pub const MAX_PENDING_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Image {
    pub id: u32,
    pub width: u32,
    pub height: u32,
    /// Straight-alpha RGBA, row-major.
    pub rgba: Vec<u8>,
    /// The wire form, encoded once and shared by every projection.
    #[serde(skip)]
    png: std::sync::OnceLock<std::sync::Arc<Vec<u8>>>,
}
impl Image {
    pub fn new(id: u32, width: u32, height: u32, rgba: Vec<u8>) -> Self {
        Self {
            id,
            width,
            height,
            rgba,
            png: Default::default(),
        }
    }
    /// PNG bytes for the wire; the GUI keeps its own textures.
    pub fn png(&self) -> std::sync::Arc<Vec<u8>> {
        self.png
            .get_or_init(|| {
                let mut bytes = Vec::new();
                {
                    let mut encoder = png::Encoder::new(&mut bytes, self.width, self.height);
                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);
                    if let Ok(mut writer) = encoder.write_header() {
                        let _ = writer.write_image_data(&self.rgba);
                    }
                }
                std::sync::Arc::new(bytes)
            })
            .clone()
    }
}

/// Where an image sits, in cells. `row` counts from the first row the
/// terminal ever showed, so a placement scrolls with its line; the terminal
/// projects it against its current scroll origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Placement {
    pub image: u32,
    pub placement: u32,
    pub row: u64,
    pub col: u32,
    pub cols: u32,
    pub rows: u32,
    pub z: i32,
    /// Source rectangle in image pixels; a zero size means the whole image.
    pub source: [u32; 4],
}

/// What the terminal does after a command: a reply to write, and whether the
/// cursor moved to the cell after the displayed image.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Outcome {
    pub reply: Option<String>,
    /// `(rows down, cols right)` from the placement's origin; the terminal
    /// clamps to its grid.
    pub cursor: Option<(u32, u32)>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
struct Pending {
    id: u32,
    format: u32,
    width: u32,
    height: u32,
    zlib: bool,
    data: Vec<u8>,
    /// The display part of a `T` command, applied when the last chunk lands.
    display: Option<Display>,
    quiet: u8,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
struct Display {
    placement: u32,
    cols: u32,
    rows: u32,
    z: i32,
    source: [u32; 4],
    move_cursor: bool,
}

/// One terminal's images and placements.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Graphics {
    images: BTreeMap<u32, Image>,
    placements: Vec<Placement>,
    pending: Option<Pending>,
    /// Insertion order for eviction.
    order: Vec<u32>,
    next_id: u32,
    /// Bumped by every change the picture can see; the projection compares it.
    pub revision: u64,
}

/// The control keys of one command. Unknown keys are ignored, as the
/// protocol asks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Command {
    pub keys: BTreeMap<char, i64>,
    pub payload: Vec<u8>,
}
impl Command {
    /// `key=value,key=value;payload`.
    pub fn parse(data: &[u8]) -> Option<Self> {
        let data = data.strip_prefix(b"G")?;
        let (control, payload) = match data.iter().position(|b| *b == b';') {
            Some(at) => (&data[..at], &data[at + 1..]),
            None => (data, &data[data.len()..]),
        };
        let mut keys = BTreeMap::new();
        for pair in control.split(|b| *b == b',').filter(|p| !p.is_empty()) {
            let (key, value) = pair.split_at(pair.iter().position(|b| *b == b'=')?);
            if key.len() != 1 {
                return None;
            }
            let value = std::str::from_utf8(&value[1..]).ok()?;
            // Single-letter flags are ASCII letters; numbers may be negative (z).
            let number = value
                .parse::<i64>()
                .ok()
                .or_else(|| (value.len() == 1).then(|| i64::from(value.as_bytes()[0])))?;
            keys.insert(char::from(key[0]), number);
        }
        Some(Self {
            keys,
            payload: payload.to_vec(),
        })
    }
    fn get(&self, key: char, default: i64) -> i64 {
        self.keys.get(&key).copied().unwrap_or(default)
    }
    fn flag(&self, key: char, default: u8) -> u8 {
        self.get(key, i64::from(default)) as u8
    }
}

impl Graphics {
    pub fn images(&self) -> impl Iterator<Item = &Image> {
        self.images.values()
    }
    pub fn image(&self, id: u32) -> Option<&Image> {
        self.images.get(&id)
    }
    pub fn placements(&self) -> &[Placement] {
        &self.placements
    }
    pub fn is_empty(&self) -> bool {
        self.placements.is_empty() && self.images.is_empty()
    }
    /// Applies one command issued with the cursor at `(row, col)`, `row` in
    /// the terminal's absolute row numbering.
    pub fn command(&mut self, data: &[u8], row: u64, col: u32) -> Outcome {
        let Some(command) = Command::parse(data) else {
            return Outcome::default();
        };
        let quiet = command.flag('q', 0);
        let action = command.flag('a', b't');
        let outcome = match action {
            b'q' => self.query(&command),
            b't' | b'T' => self.transmit(&command, action == b'T', row, col),
            b'p' => self.put(&command, row, col),
            b'd' => self.delete(&command),
            _ => Err("ENOTSUPPORTED"),
        };
        match outcome {
            Ok(mut outcome) => {
                if quiet >= 1 {
                    outcome.reply = None;
                }
                outcome
            }
            Err(code) => Outcome {
                reply: (quiet < 2).then(|| reply(&command, code)),
                cursor: None,
            },
        }
    }
    fn query(&mut self, command: &Command) -> Result<Outcome, &'static str> {
        // A query decodes the payload and answers OK without keeping anything.
        let id = command.get('i', 0) as u32;
        let probe = Pending {
            id,
            format: command.get('f', 32) as u32,
            width: command.get('s', 0) as u32,
            height: command.get('v', 0) as u32,
            zlib: command.get('o', 0) == i64::from(b'z'),
            data: decode_payload(&command.payload)?,
            display: None,
            quiet: 0,
        };
        decode(&probe)?;
        Ok(Outcome {
            reply: Some(reply(command, "OK")),
            cursor: None,
        })
    }
    fn transmit(
        &mut self,
        command: &Command,
        display: bool,
        row: u64,
        col: u32,
    ) -> Result<Outcome, &'static str> {
        if command.get('t', i64::from(b'd')) != i64::from(b'd') {
            return Err("EBADF");
        }
        let chunk = decode_payload(&command.payload)?;
        let more = command.get('m', 0) == 1;
        let pending = match self.pending.take() {
            // A continuation carries only `m` and the payload.
            Some(mut pending) if command.keys.keys().all(|k| matches!(k, 'm' | 'q')) => {
                pending.data.extend_from_slice(&chunk);
                pending
            }
            _ => {
                let mut id = command.get('i', 0) as u32;
                let number = command.get('I', 0) as u32;
                if id == 0 && number == 0 {
                    // An anonymous transmit gets a fresh id so a `T` can place it.
                    self.next_id = self.next_id.wrapping_add(1).max(1);
                    while self.images.contains_key(&self.next_id) {
                        self.next_id = self.next_id.wrapping_add(1).max(1);
                    }
                    id = self.next_id;
                } else if id == 0 {
                    return Err("EINVAL");
                }
                Pending {
                    id,
                    format: command.get('f', 32) as u32,
                    width: command.get('s', 0) as u32,
                    height: command.get('v', 0) as u32,
                    zlib: command.get('o', 0) == i64::from(b'z'),
                    data: chunk,
                    display: display.then(|| Display {
                        placement: command.get('p', 0) as u32,
                        cols: command.get('c', 0) as u32,
                        rows: command.get('r', 0) as u32,
                        z: command.get('z', 0) as i32,
                        source: [
                            command.get('x', 0) as u32,
                            command.get('y', 0) as u32,
                            command.get('w', 0) as u32,
                            command.get('h', 0) as u32,
                        ],
                        move_cursor: command.get('C', 0) == 0,
                    }),
                    quiet: command.flag('q', 0),
                }
            }
        };
        if pending.data.len() > MAX_PENDING_BYTES {
            return Err("EFBIG");
        }
        if more {
            self.pending = Some(pending);
            return Ok(Outcome::default());
        }
        let image = decode(&pending)?;
        let id = image.id;
        self.insert(image);
        let mut outcome = Outcome {
            reply: Some(format!("\x1b_Gi={id};OK\x1b\\")),
            cursor: None,
        };
        if pending.quiet >= 1 {
            outcome.reply = None;
        }
        if let Some(display) = pending.display {
            outcome.cursor = self.place(id, display, row, col)?;
        }
        Ok(outcome)
    }
    fn put(&mut self, command: &Command, row: u64, col: u32) -> Result<Outcome, &'static str> {
        let id = command.get('i', 0) as u32;
        if !self.images.contains_key(&id) {
            return Err("ENOENT");
        }
        let display = Display {
            placement: command.get('p', 0) as u32,
            cols: command.get('c', 0) as u32,
            rows: command.get('r', 0) as u32,
            z: command.get('z', 0) as i32,
            source: [
                command.get('x', 0) as u32,
                command.get('y', 0) as u32,
                command.get('w', 0) as u32,
                command.get('h', 0) as u32,
            ],
            move_cursor: command.get('C', 0) == 0,
        };
        let cursor = self.place(id, display, row, col)?;
        Ok(Outcome {
            reply: Some(reply(command, "OK")),
            cursor,
        })
    }
    fn place(
        &mut self,
        id: u32,
        display: Display,
        row: u64,
        col: u32,
    ) -> Result<Option<(u32, u32)>, &'static str> {
        let image = self.images.get(&id).ok_or("ENOENT")?;
        let [sx, sy, mut sw, mut sh] = display.source;
        if sw == 0 {
            sw = image.width.saturating_sub(sx);
        }
        if sh == 0 {
            sh = image.height.saturating_sub(sy);
        }
        if sw == 0 || sh == 0 || sx + sw > image.width || sy + sh > image.height {
            return Err("EINVAL");
        }
        // Without cell extents the terminal sizes the image from pixels when it
        // projects; zero means "natural size" to it.
        let placement = Placement {
            image: id,
            placement: display.placement,
            row,
            col,
            cols: display.cols,
            rows: display.rows,
            z: display.z,
            source: [sx, sy, sw, sh],
        };
        self.placements
            .retain(|p| !(p.image == id && p.placement == display.placement));
        self.placements.push(placement);
        self.revision += 1;
        Ok(display
            .move_cursor
            .then_some((placement.rows, placement.cols)))
    }
    fn delete(&mut self, command: &Command) -> Result<Outcome, &'static str> {
        let what = command.flag('d', b'a');
        let free = what.is_ascii_uppercase();
        let what = what.to_ascii_lowercase();
        let id = command.get('i', 0) as u32;
        let placement = command.get('p', 0) as u32;
        let before = self.placements.len();
        match what {
            b'a' => self.placements.clear(),
            b'i' => self
                .placements
                .retain(|p| !(p.image == id && (placement == 0 || p.placement == placement))),
            _ => return Err("ENOTSUPPORTED"),
        }
        if free {
            if what == b'a' {
                self.images.clear();
                self.order.clear();
            } else {
                self.images.remove(&id);
                self.order.retain(|i| *i != id);
            }
        }
        if before != self.placements.len() || free {
            self.revision += 1;
        }
        Ok(Outcome {
            reply: Some(reply(command, "OK")),
            cursor: None,
        })
    }
    fn insert(&mut self, image: Image) {
        let id = image.id;
        self.order.retain(|i| *i != id);
        self.order.push(id);
        self.images.insert(id, image);
        while self.images.len() > MAX_IMAGES {
            // Evict the oldest image that nothing shows, else the oldest.
            let victim = self
                .order
                .iter()
                .copied()
                .find(|i| !self.placements.iter().any(|p| p.image == *i))
                .unwrap_or(self.order[0]);
            self.order.retain(|i| *i != victim);
            self.images.remove(&victim);
            self.placements.retain(|p| p.image != victim);
        }
        self.revision += 1;
    }
    /// The terminal scrolled its content: placements whose rows fell behind
    /// `first_row` are gone for good (no scrollback images in this cut).
    pub fn forget_before(&mut self, first_row: u64) {
        let before = self.placements.len();
        self.placements
            .retain(|p| p.row + u64::from(p.rows.max(1)) > first_row);
        if before != self.placements.len() {
            self.revision += 1;
        }
    }
    /// The screen was cleared: every placement on rows at or after `row` goes.
    pub fn clear_from(&mut self, row: u64) {
        let before = self.placements.len();
        self.placements.retain(|p| p.row < row);
        if before != self.placements.len() {
            self.revision += 1;
        }
    }
    /// Rows `[from, to)` lost their text; pictures touching them go too.
    pub fn clear_rows(&mut self, from: u64, to: u64) {
        let before = self.placements.len();
        self.placements
            .retain(|p| p.row + u64::from(p.rows.max(1)) <= from || p.row >= to);
        if before != self.placements.len() {
            self.revision += 1;
        }
    }
    pub fn clear_placements(&mut self) {
        if !self.placements.is_empty() {
            self.placements.clear();
            self.revision += 1;
        }
    }
}

fn reply(command: &Command, code: &str) -> String {
    let id = command.get('i', 0);
    let number = command.get('I', 0);
    let mut keys = String::new();
    if id != 0 || number == 0 {
        keys.push_str(&format!("i={id}"));
    }
    if number != 0 {
        if !keys.is_empty() {
            keys.push(',');
        }
        keys.push_str(&format!("I={number}"));
    }
    if command.keys.contains_key(&'p') && code == "OK" {
        keys.push_str(&format!(",p={}", command.get('p', 0)));
    }
    format!("\x1b_G{keys};{code}\x1b\\")
}
fn decode_payload(payload: &[u8]) -> Result<Vec<u8>, &'static str> {
    if payload.is_empty() {
        return Ok(Vec::new());
    }
    base64::engine::general_purpose::STANDARD
        .decode(payload)
        .map_err(|_| "EINVAL")
}
fn decode(pending: &Pending) -> Result<Image, &'static str> {
    let mut data = pending.data.clone();
    if pending.zlib {
        let mut out = Vec::new();
        flate2::read::ZlibDecoder::new(&data[..])
            .take(MAX_IMAGE_PIXELS * 4)
            .read_to_end(&mut out)
            .map_err(|_| "EINVAL")?;
        data = out;
    }
    let (width, height, rgba) = match pending.format {
        100 => {
            let decoder = png::Decoder::new(std::io::Cursor::new(&data[..]));
            let mut reader = decoder.read_info().map_err(|_| "EINVAL")?;
            let info = reader.info();
            let (w, h) = (info.width, info.height);
            if u64::from(w) * u64::from(h) > MAX_IMAGE_PIXELS {
                return Err("EFBIG");
            }
            let mut buf = vec![0; reader.output_buffer_size().ok_or("EINVAL")?];
            let frame = reader.next_frame(&mut buf).map_err(|_| "EINVAL")?;
            let bytes = &buf[..frame.buffer_size()];
            let rgba = match (frame.color_type, frame.bit_depth) {
                (png::ColorType::Rgba, png::BitDepth::Eight) => bytes.to_vec(),
                (png::ColorType::Rgb, png::BitDepth::Eight) => bytes
                    .as_chunks::<3>().0.iter()
                    .flat_map(|p| [p[0], p[1], p[2], 255])
                    .collect(),
                (png::ColorType::Grayscale, png::BitDepth::Eight) => {
                    bytes.iter().flat_map(|g| [*g, *g, *g, 255]).collect()
                }
                (png::ColorType::GrayscaleAlpha, png::BitDepth::Eight) => bytes
                    .as_chunks::<2>().0.iter()
                    .flat_map(|p| [p[0], p[0], p[0], p[1]])
                    .collect(),
                _ => return Err("EINVAL"),
            };
            (w, h, rgba)
        }
        24 | 32 => {
            let (w, h) = (pending.width, pending.height);
            if w == 0 || h == 0 || u64::from(w) * u64::from(h) > MAX_IMAGE_PIXELS {
                return Err("EINVAL");
            }
            let channels = if pending.format == 24 { 3 } else { 4 };
            if data.len() != (w as usize) * (h as usize) * channels {
                return Err("EINVAL");
            }
            let rgba = if channels == 4 {
                data
            } else {
                data.as_chunks::<3>().0.iter()
                    .flat_map(|p| [p[0], p[1], p[2], 255])
                    .collect()
            };
            (w, h, rgba)
        }
        _ => return Err("EINVAL"),
    };
    Ok(Image::new(pending.id, width, height, rgba))
}

/// Bytes a program writes to show `png` at the cursor: one chunked `T` command.
pub fn encode_png(png: &[u8], id: u32, cols: u32, rows: u32) -> Vec<u8> {
    let payload = base64::engine::general_purpose::STANDARD.encode(png);
    let chunks: Vec<&str> = payload
        .as_bytes()
        .chunks(4096)
        .map(|c| std::str::from_utf8(c).unwrap())
        .collect();
    let mut out = Vec::new();
    for (i, chunk) in chunks.iter().enumerate() {
        let last = i + 1 == chunks.len();
        if i == 0 {
            out.extend_from_slice(
                format!(
                    "\x1b_Ga=T,f=100,i={id},c={cols},r={rows},q=1,m={};{chunk}\x1b\\",
                    u8::from(!last)
                )
                .as_bytes(),
            );
        } else {
            out.extend_from_slice(format!("\x1b_Gm={};{chunk}\x1b\\", u8::from(!last)).as_bytes());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rgba(w: u32, h: u32) -> Vec<u8> {
        (0..w * h).flat_map(|i| [i as u8, 0, 255, 255]).collect()
    }
    fn b64(bytes: &[u8]) -> String {
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }
    // State: no images, one program at row 10. Safety: a transmit-and-display
    // keeps one image and one placement, replies OK, moves the cursor by the
    // placement's extents; a delete removes it; a query keeps nothing.
    #[test]
    fn transmit_display_query_and_delete() {
        let mut g = Graphics::default();
        let data = format!("Ga=T,f=32,s=2,v=2,i=7,c=4,r=2;{}", b64(&rgba(2, 2)));
        let out = g.command(data.as_bytes(), 10, 3);
        assert_eq!(out.reply.as_deref(), Some("\x1b_Gi=7;OK\x1b\\"));
        assert_eq!(out.cursor, Some((2, 4)));
        assert_eq!(g.images().count(), 1);
        assert_eq!(
            g.placements(),
            &[Placement {
                image: 7,
                placement: 0,
                row: 10,
                col: 3,
                cols: 4,
                rows: 2,
                z: 0,
                source: [0, 0, 2, 2]
            }]
        );
        let q = g.command(
            format!("Ga=q,f=32,s=1,v=1,i=9;{}", b64(&rgba(1, 1))).as_bytes(),
            0,
            0,
        );
        assert_eq!(q.reply.as_deref(), Some("\x1b_Gi=9;OK\x1b\\"));
        assert_eq!(g.images().count(), 1);
        let bad = g.command(b"Ga=q,f=32,s=5,v=5,i=9;AAAA", 0, 0);
        assert_eq!(bad.reply.as_deref(), Some("\x1b_Gi=9;EINVAL\x1b\\"));
        let d = g.command(b"Ga=d,d=I,i=7", 0, 0);
        assert_eq!(d.reply.as_deref(), Some("\x1b_Gi=7;OK\x1b\\"));
        assert!(g.is_empty());
    }
    // State: a PNG split in three chunks. Safety: nothing shows until the last
    // chunk; the assembled image has the PNG's size; quiet=1 suppresses OK.
    #[test]
    fn chunked_png_with_zlib_and_quiet() {
        let image = Image::new(0, 3, 2, rgba(3, 2));
        let png = image.png();
        let mut g = Graphics::default();
        let payload = b64(&png);
        let (a, rest) = payload.split_at(8);
        let (b, c) = rest.split_at(8);
        assert_eq!(
            g.command(format!("Ga=T,f=100,i=3,q=1,m=1;{a}").as_bytes(), 0, 0),
            Outcome::default()
        );
        assert_eq!(
            g.command(format!("Gm=1;{b}").as_bytes(), 0, 0),
            Outcome::default()
        );
        assert!(g.placements().is_empty());
        let out = g.command(format!("Gm=0;{c}").as_bytes(), 4, 0);
        assert_eq!(out.reply, None);
        assert_eq!(out.cursor, Some((0, 0)));
        let got = g.image(3).unwrap();
        assert_eq!((got.width, got.height), (3, 2));
        assert_eq!(got.rgba, rgba(3, 2));
        // zlib-compressed raw RGB
        let raw: Vec<u8> = (0..6).flat_map(|i| [i, i, i]).collect();
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::Write::write_all(&mut z, &raw).unwrap();
        let z = z.finish().unwrap();
        let out = g.command(
            format!("Ga=t,f=24,o=z,s=3,v=2,i=4;{}", b64(&z)).as_bytes(),
            0,
            0,
        );
        assert_eq!(out.reply.as_deref(), Some("\x1b_Gi=4;OK\x1b\\"));
        assert_eq!(g.image(4).unwrap().rgba[4..8], [1, 1, 1, 255]);
        // The round trip through encode_png is one display.
        let mut h = Graphics::default();
        for cmd in String::from_utf8(encode_png(&png, 5, 10, 3))
            .unwrap()
            .split("\x1b\\")
            .filter(|s| !s.is_empty())
        {
            h.command(cmd.trim_start_matches("\x1b_").as_bytes(), 1, 1);
        }
        assert_eq!(h.placements().len(), 1);
        assert_eq!(h.placements()[0].cols, 10);
    }
    // State: 33 images. Safety: the cap evicts the oldest unplaced image first
    // and never an image still on screen while another is free.
    #[test]
    fn eviction_prefers_unplaced_images_and_scroll_forgets_rows() {
        let mut g = Graphics::default();
        let px = b64(&rgba(1, 1));
        g.command(format!("Ga=T,f=32,s=1,v=1,i=1;{px}").as_bytes(), 0, 0);
        for i in 2..=(MAX_IMAGES as u32 + 1) {
            g.command(format!("Ga=t,f=32,s=1,v=1,i={i};{px}").as_bytes(), 0, 0);
        }
        assert_eq!(g.images().count(), MAX_IMAGES);
        assert!(g.image(1).is_some(), "the placed image survives");
        assert!(g.image(2).is_none(), "the oldest unplaced image is evicted");
        // A put with the same placement id replaces; a new id adds.
        g.command(b"Ga=p,i=1,c=2,r=3", 20, 0);
        assert_eq!(g.placements().len(), 1);
        g.command(b"Ga=p,i=1,p=2,c=2,r=3", 0, 0);
        assert_eq!(g.placements().len(), 2);
        g.forget_before(3);
        assert_eq!(g.placements().len(), 1, "row 0 scrolled away; row 20 stays");
        assert_eq!(g.placements()[0].row, 20);
        g.clear_from(20);
        assert!(g.placements().is_empty());
        assert!(Command::parse(b"Xa=T").is_none());
        assert!(Command::parse(b"Gab=1").is_none());
    }
}
