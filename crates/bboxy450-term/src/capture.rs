//! Verification capture of the exact composition submitted to the surface.
//! The UI only queues a copy. GPU waits, PNG encoding and file IO run off UI.
use crate::log::Logger;
use std::{path::PathBuf, time::Duration};

pub fn save(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    path: PathBuf,
    crop: Option<Crop>,
    logger: Logger,
    written: impl FnOnce(Result<(), String>) + Send + 'static,
) {
    let size = texture.size();
    let crop = crop.map_or(
        Crop {
            x: 0,
            y: 0,
            width: size.width,
            height: size.height,
        },
        |crop| crop.within(size.width, size.height),
    );
    let stride = (size.width * 4).div_ceil(256) * 256;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("verification capture"),
        size: u64::from(stride) * u64::from(size.height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(size.height),
            },
        },
        size,
    );
    let submission = queue.submit([encoder.finish()]);
    let (done, received) = tokio::sync::oneshot::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = done.send(result);
        });
    let device = device.clone();
    bbt_rt::handle().spawn(async move {
        let result = async {
            bbt_rt::handle()
                .spawn_blocking(move || {
                    bbt_rt::assert_not_ui_thread();
                    device.poll(wgpu::PollType::Wait {
                        submission_index: Some(submission),
                        timeout: Some(Duration::from_secs(20)),
                    })
                })
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;
            received
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;
            let png = bbt_rt::handle()
                .spawn_blocking(move || -> Result<Vec<u8>, String> {
                    bbt_rt::assert_not_ui_thread();
                    let mapped = buffer
                        .slice(..)
                        .get_mapped_range()
                        .map_err(|e| e.to_string())?;
                    let mut rgba = Vec::with_capacity((crop.width * crop.height * 4) as usize);
                    let (from, to) = ((crop.x * 4) as usize, ((crop.x + crop.width) * 4) as usize);
                    for row in mapped
                        .chunks_exact(stride as usize)
                        .skip(crop.y as usize)
                        .take(crop.height as usize)
                    {
                        rgba.extend_from_slice(&row[from..to]);
                    }
                    drop(mapped);
                    buffer.unmap();
                    let mut bytes = Vec::new();
                    {
                        let mut encoder = png::Encoder::new(&mut bytes, crop.width, crop.height);
                        encoder.set_color(png::ColorType::Rgba);
                        encoder.set_depth(png::BitDepth::Eight);
                        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
                        writer.write_image_data(&rgba).map_err(|e| e.to_string())?;
                    }
                    Ok(bytes)
                })
                .await
                .map_err(|e| e.to_string())??;
            tokio::fs::write(&path, png)
                .await
                .map_err(|e| e.to_string())?;
            Ok::<(), String>(())
        }
        .await;
        logger.write(format!("capture {}: {result:?}", path.display()));
        written(result);
    });
}

/// A pixel rectangle of the captured frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Crop {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
impl Crop {
    /// Clamped to a `width`×`height` frame, at least one pixel.
    fn within(self, width: u32, height: u32) -> Self {
        let x = self.x.min(width.saturating_sub(1));
        let y = self.y.min(height.saturating_sub(1));
        Self {
            x,
            y,
            width: self.width.clamp(1, width - x),
            height: self.height.clamp(1, height - y),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Crop;
    #[test]
    fn a_crop_stays_inside_the_frame() {
        let crop = |x, y, width, height| Crop { x, y, width, height };
        assert_eq!(crop(10, 20, 30, 40).within(100, 100), crop(10, 20, 30, 40));
        assert_eq!(crop(90, 95, 30, 40).within(100, 100), crop(90, 95, 10, 5));
        assert_eq!(crop(500, 500, 0, 0).within(100, 100), crop(99, 99, 1, 1));
    }
}
