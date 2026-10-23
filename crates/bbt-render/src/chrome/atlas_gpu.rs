//! Device-owned atlas storage. Pixel coordinates survive CPU shelf growth.
use crate::atlas::{AtlasRect, GlyphAtlas};

pub(super) struct AtlasTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub size: u32,
    pub generation: u64,
    pub writes: u64,
    pub bytes: u64,
}
impl AtlasTexture {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: u32,
        pixels: &[u8],
        format: wgpu::TextureFormat,
        bpp: u32,
        generation: u64,
    ) -> Self {
        let texture = super::upload(device, queue, size, pixels, format, bpp);
        let view = texture.create_view(&Default::default());
        Self {
            texture,
            view,
            size,
            generation,
            writes: 1,
            bytes: pixels.len() as u64,
        }
    }
    fn write_glyph(&mut self, queue: &wgpu::Queue, atlas: &GlyphAtlas, rect: AtlasRect) {
        let x = u32::from(rect.x);
        let y = u32::from(rect.y);
        let width = u32::from(rect.width);
        let height = u32::from(rect.height);
        let start = (y * self.size + x) as usize;
        let end = start + ((height - 1) * self.size + width) as usize;
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            &atlas.pixels()[start..end],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.size),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.writes += 1;
        self.bytes += u64::from(width) * u64::from(height);
    }
}
pub(super) fn sync_glyphs(
    gpu: &mut Option<AtlasTexture>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    cpu: &GlyphAtlas,
    dirty: &mut Vec<AtlasRect>,
    reset: bool,
) {
    if reset || gpu.as_ref().is_none_or(|g| g.size != cpu.size()) {
        let generation = gpu.as_ref().map_or(1, |g| g.generation + 1);
        *gpu = Some(AtlasTexture::new(
            device,
            queue,
            cpu.size(),
            cpu.pixels(),
            wgpu::TextureFormat::R8Unorm,
            1,
            generation,
        ));
        // New storage contains the complete CPU atlas, including all pending
        // inserts and transparent guard texels. Existing bind groups retain
        // an old generation until their encoded GPU work has finished.
        dirty.clear();
    } else {
        let gpu = gpu.as_mut().unwrap();
        for rect in dirty.drain(..) {
            gpu.write_glyph(queue, cpu, rect);
        }
    }
}
