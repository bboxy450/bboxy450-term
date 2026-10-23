//! Bounded coverage cache: glyphs and minimal overlapping components, never lines.
use super::text_glyphs::{self as glyphs, Key, Mask};
use crate::{
    Result,
    atlas::{AtlasRect, GlyphAtlas},
};
use bbt_ui_model::draw::TextStyle;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
    pub atlas_size: u32,
    pub atlas_mask_pixels: usize,
    pub cached_mask_bytes: usize,
    pub glyph_entries: usize,
    pub overlap_entries: usize,
    pub atlas_entries: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub rebuilds: u64,
    pub component_requests: u64,
    pub overlap_requests: u64,
    pub max_component_glyphs: usize,
}
pub(super) struct Part {
    pub id: u64,
    /// Device-pixel offset from the original line bitmap's top-left.
    pub offset: [i32; 2],
    pub size: [u32; 2],
}
pub(super) struct Line {
    pub width: u32,
    pub height: u32,
    pub advance: f32,
    pub parts: Vec<Part>,
}
struct Entry {
    id: u64,
    mask: Arc<Mask>,
    last_use: u64,
    rect: Option<AtlasRect>,
}
pub(super) struct Cache {
    atlas: GlyphAtlas,
    dirty: Vec<AtlasRect>,
    entries: HashMap<Key, Entry>,
    ids: HashMap<u64, Key>,
    bytes: usize,
    byte_budget: usize,
    entry_budget: usize,
    initial_size: u32,
    atlas_limit: u32,
    clock: u64,
    next_id: u64,
    totals: Stats,
}
impl Cache {
    pub fn new() -> Result<Self> {
        Self::with_limits(512, 8192, 32 * 1024 * 1024, 32_768)
    }
    pub fn with_limits(
        initial_size: u32,
        atlas_limit: u32,
        byte_budget: usize,
        entry_budget: usize,
    ) -> Result<Self> {
        if byte_budget == 0 || entry_budget == 0 {
            return Err("invalid chrome text cache budget".into());
        }
        Ok(Self {
            atlas: GlyphAtlas::new(initial_size, atlas_limit)?,
            dirty: vec![],
            entries: HashMap::new(),
            ids: HashMap::new(),
            bytes: 0,
            byte_budget,
            entry_budget,
            initial_size,
            atlas_limit,
            clock: 0,
            next_id: 0,
            totals: Stats::default(),
        })
    }
    fn get(&mut self, key: Key) -> Result<(u64, Arc<Mask>)> {
        self.clock += 1;
        if let Some(entry) = self.entries.get_mut(&key) {
            self.totals.hits += 1;
            entry.last_use = self.clock;
            return Ok((entry.id, entry.mask.clone()));
        }
        self.totals.misses += 1;
        let mask = Arc::new(glyphs::rasterize(&key)?);
        self.next_id += 1;
        let id = self.next_id;
        self.bytes += mask.pixels.len();
        self.ids.insert(id, key.clone());
        self.entries.insert(
            key,
            Entry {
                id,
                mask: mask.clone(),
                last_use: self.clock,
                rect: None,
            },
        );
        Ok((id, mask))
    }
    pub fn line(
        &mut self,
        value: &str,
        style: TextStyle,
        max_width: Option<f32>,
        scale: f32,
        phase: f32,
    ) -> Result<Line> {
        let shape = glyphs::shape(value, style, max_width, scale, phase)?;
        let mut atoms = Vec::with_capacity(shape.glyphs.len());
        let mut bounds = Vec::with_capacity(shape.glyphs.len());
        for placed in &shape.glyphs {
            let atom = self.get(placed.key.clone())?;
            let b = atom.1.merge_bounds;
            bounds.push(if atom.1.empty() {
                [0; 4]
            } else {
                [
                    b[0] + placed.origin[0],
                    b[1] + placed.origin[1],
                    b[2] + placed.origin[0],
                    b[3] + placed.origin[1],
                ]
            });
            atoms.push(atom);
        }
        let mut parts = Vec::new();
        for group in glyphs::connected(&bounds) {
            self.totals.component_requests += 1;
            self.totals.max_component_glyphs = self.totals.max_component_glyphs.max(group.len());
            let (key, origin) = glyphs::component(&shape, &group);
            let (id, mask) = if group.len() == 1 {
                atoms[group[0]].clone()
            } else {
                self.totals.overlap_requests += 1;
                self.get(key)?
            };
            if mask.empty() {
                continue;
            }
            parts.push(Part {
                id,
                offset: [
                    origin[0] + mask.bounds[0],
                    shape.height as i32 - origin[1] - mask.bounds[3],
                ],
                size: [mask.width(), mask.height()],
            });
        }
        Ok(Line {
            width: shape.width,
            height: shape.height,
            advance: shape.advance,
            parts,
        })
    }

    fn oldest(&self, pinned: &HashSet<u64>, placed_only: bool) -> Option<u64> {
        self.entries
            .values()
            .filter(|e| !pinned.contains(&e.id) && (!placed_only || e.rect.is_some()))
            .min_by_key(|e| (e.last_use, e.id))
            .map(|e| e.id)
    }
    fn remove(&mut self, id: u64) -> bool {
        let key = self.ids.remove(&id).expect("cache id");
        let entry = self.entries.remove(&key).expect("cache entry");
        self.bytes -= entry.mask.pixels.len();
        self.totals.evictions += 1;
        entry.rect.is_some()
    }
    /// Resolve all text resources before any instances are bound to atlas coordinates.
    /// A repack requires fresh GPU storage, preserving earlier encoded passes.
    pub fn finish(&mut self, ids: &[u64]) -> Result<bool> {
        let pinned: HashSet<_> = ids.iter().copied().collect();
        let mut repack = false;
        while self.bytes > self.byte_budget || self.entries.len() > self.entry_budget {
            let Some(id) = self.oldest(&pinned, false) else {
                return Err("chrome text working set exceeds cache budget".into());
            };
            repack |= self.remove(id);
        }
        let mut ids: Vec<_> = pinned.iter().copied().collect();
        ids.sort_unstable();
        if !repack {
            for id in &ids {
                let key = self.ids.get(id).ok_or("missing pinned chrome glyph")?;
                let entry = self.entries.get_mut(key).unwrap();
                if entry.rect.is_some() {
                    continue;
                }
                match self
                    .atlas
                    .insert(entry.mask.width(), entry.mask.height(), &entry.mask.pixels)
                {
                    Ok(rect) => {
                        entry.rect = Some(rect);
                        self.dirty.push(rect);
                    }
                    Err(_) => {
                        repack = true;
                        break;
                    }
                }
            }
        }
        if !repack {
            return Ok(false);
        }
        loop {
            let mut retained: Vec<_> = self
                .entries
                .values()
                .filter(|e| e.rect.is_some() || pinned.contains(&e.id))
                .map(|e| (e.id, e.mask.clone()))
                .collect();
            retained.sort_by_key(|(id, m)| {
                (
                    std::cmp::Reverse(m.height()),
                    std::cmp::Reverse(m.width()),
                    *id,
                )
            });
            let mut atlas = GlyphAtlas::new(self.initial_size, self.atlas_limit)?;
            let mut rectangles = Vec::with_capacity(retained.len());
            let mut fits = true;
            for (id, mask) in retained {
                match atlas.insert(mask.width(), mask.height(), &mask.pixels) {
                    Ok(rect) => rectangles.push((id, rect)),
                    Err(_) => {
                        fits = false;
                        break;
                    }
                }
            }
            if fits {
                for e in self.entries.values_mut() {
                    e.rect = None;
                }
                for (id, rect) in rectangles {
                    let key = &self.ids[&id];
                    self.entries.get_mut(key).unwrap().rect = Some(rect);
                }
                self.atlas = atlas;
                self.dirty.clear();
                self.totals.rebuilds += 1;
                return Ok(true);
            }
            let Some(victim) = self.oldest(&pinned, true) else {
                return Err("chrome glyph working set exceeds atlas capacity".into());
            };
            self.remove(victim);
        }
    }
    pub fn rect(&self, id: u64) -> Result<AtlasRect> {
        self.ids
            .get(&id)
            .and_then(|key| self.entries.get(key))
            .and_then(|e| e.rect)
            .ok_or_else(|| "unresolved chrome glyph".into())
    }
    pub fn upload(&mut self) -> (&GlyphAtlas, &mut Vec<AtlasRect>) {
        (&self.atlas, &mut self.dirty)
    }
    pub fn stats(&self) -> Stats {
        let mut stats = self.totals;
        stats.atlas_size = self.atlas.size();
        stats.cached_mask_bytes = self.bytes;
        for (key, entry) in &self.entries {
            if key.glyphs.len() == 1 {
                stats.glyph_entries += 1;
            } else {
                stats.overlap_entries += 1;
            }
            if let Some(rect) = entry.rect {
                stats.atlas_entries += 1;
                stats.atlas_mask_pixels += usize::from(rect.width) * usize::from(rect.height);
            }
        }
        stats
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    #[test]
    fn reordered_text_reuses_cached_glyph_phases() {
        let mut cache = Cache::new().unwrap();
        for phase in [0., 0.25, 0.5, 0.75] {
            let a = cache
                .line("ABC", TextStyle::default(), None, 1., phase)
                .unwrap();
            cache
                .finish(&a.parts.iter().map(|p| p.id).collect::<Vec<_>>())
                .unwrap();
        }
        let first = cache.stats();
        let b = cache
            .line("CBA", TextStyle::default(), None, 1., 0.)
            .unwrap();
        cache
            .finish(&b.parts.iter().map(|p| p.id).collect::<Vec<_>>())
            .unwrap();
        let later = cache.stats();
        assert_eq!(first.glyph_entries, later.glyph_entries);
        assert_eq!(
            later.misses - first.misses,
            (later.overlap_entries - first.overlap_entries) as u64
        );
        assert!(later.hits > first.hits);
        assert_eq!(later.evictions, 0);
    }
    #[test]
    fn native_font_cache_moves_from_initializing_worker_to_renderer_thread() {
        fn assert_send<T: Send>() {}
        assert_send::<super::super::ChromeRenderer>();
        let mut cache = Cache::new().unwrap();
        let value = "AV fi · 許可 👩‍💻";
        let first = cache
            .line(value, TextStyle::default(), None, 1., 0.25)
            .unwrap();
        cache
            .finish(&first.parts.iter().map(|p| p.id).collect::<Vec<_>>())
            .unwrap();
        let expected = cache.atlas.pixels().to_vec();
        let misses = cache.stats().misses;
        let cache = std::thread::spawn(move || {
            let same = cache
                .line(value, TextStyle::default(), None, 1., 0.25)
                .unwrap();
            assert!(
                !cache
                    .finish(&same.parts.iter().map(|p| p.id).collect::<Vec<_>>())
                    .unwrap()
            );
            cache
        })
        .join()
        .unwrap();
        assert_eq!(cache.stats().misses, misses);
        assert_eq!(cache.atlas.pixels(), expected);
    }
    #[test]
    fn lru_evicts_unpinned_masks_and_repack_keeps_the_current_working_set() {
        let mut cache = Cache::with_limits(32, 128, 4096, 16).unwrap();
        let a = cache
            .line("ABC", TextStyle::default(), None, 1., 0.)
            .unwrap();
        let keep = a.parts[0].id;
        cache
            .finish(&a.parts.iter().map(|p| p.id).collect::<Vec<_>>())
            .unwrap();
        let old = cache.rect(keep).unwrap();
        for value in [
            "DEF", "GHI", "JKL", "MNO", "PQR", "STU", "VWX", "YZ1", "234", "567", "890",
        ] {
            let line = cache
                .line(value, TextStyle::default(), None, 1., 0.)
                .unwrap();
            let mut pins: Vec<_> = line.parts.iter().map(|p| p.id).collect();
            pins.push(keep);
            cache.finish(&pins).unwrap();
            let rect = cache.rect(keep).unwrap();
            let key = &cache.ids[&keep];
            let mask = &cache.entries[key].mask;
            for row in 0..u32::from(rect.height) {
                let at =
                    ((u32::from(rect.y) + row) * cache.atlas.size() + u32::from(rect.x)) as usize;
                assert_eq!(
                    &cache.atlas.pixels()[at..at + usize::from(rect.width)],
                    &mask.pixels[row as usize * usize::from(rect.width)
                        ..(row as usize + 1) * usize::from(rect.width)]
                );
            }
        }
        assert!(cache.stats().evictions > 0);
        assert!(cache.stats().rebuilds > 0);
        assert!(cache.stats().cached_mask_bytes <= 4096);
        assert!(cache.entries.len() <= 16);
        assert!(cache.rect(keep).is_ok());
        let _ = old;
    }
}
