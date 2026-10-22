//! Verification capture of the exact composition submitted to the surface.
//! The UI only queues a copy. GPU waits, PNG encoding and file IO run off UI.
use crate::log::Logger;
use std::{path::PathBuf, time::Duration};

pub fn save(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    path: PathBuf,
    logger: Logger,
) {
    let size = texture.size();
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
                    let mut rgba = Vec::with_capacity((size.width * size.height * 4) as usize);
                    for row in mapped.chunks_exact(stride as usize) {
                        rgba.extend_from_slice(&row[..(size.width * 4) as usize]);
                    }
                    drop(mapped);
                    buffer.unmap();
                    let mut bytes = Vec::new();
                    {
                        let mut encoder = png::Encoder::new(&mut bytes, size.width, size.height);
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
    });
}
