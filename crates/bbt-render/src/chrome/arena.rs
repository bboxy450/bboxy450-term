// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Per-owner buffer arenas: one growable buffer per binding kind, a fresh aligned slice per
//! draw. Slices are staged in a CPU shadow and uploaded by `flush` in one write per arena,
//! which the owner of the submit calls right before submitting (wgpu 30's write_buffer makes a
//! staging buffer per call). The write lands at that submit, before its commands, so slices
//! never overlap within a submission: the cursor rewinds only in `begin_frame`, at a submission
//! boundary. A frame left unflushed would draw stale data, so `begin_frame` and drop
//! debug-assert that it was flushed. Before the first `begin_frame`, or past the cap, each draw
//! gets its own buffer, as before arenas existed.
use std::num::NonZeroU64;

const FIRST: u64 = 64 * 1024;
const CAP: u64 = 16 * 1024 * 1024;

pub(crate) struct Slice {
    buffer: wgpu::Buffer,
    offset: u64,
    /// None binds the whole buffer, exactly as the per-draw buffers did.
    size: Option<NonZeroU64>,
}
impl Slice {
    pub fn binding(&self) -> wgpu::BindingResource<'_> {
        wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer: &self.buffer,
            offset: self.offset,
            size: self.size,
        })
    }
}

struct Arena {
    usage: wgpu::BufferUsages,
    label: &'static str,
    buffer: Option<wgpu::Buffer>,
    cursor: u64,
    /// Bytes of the current buffer from `base` to `cursor`, not yet written.
    shadow: Vec<u8>,
    base: u64,
}
impl Arena {
    fn new(usage: wgpu::BufferUsages, label: &'static str) -> Self {
        Self {
            usage,
            label,
            buffer: None,
            cursor: 0,
            shadow: vec![],
            base: 0,
        }
    }
    fn pending(&self) -> bool {
        !self.shadow.is_empty()
    }
    fn flush(&mut self, context: &crate::device::DeviceContext) {
        if let Some(buffer) = self.buffer.as_ref().filter(|_| self.pending()) {
            context.queue.write_buffer(buffer, self.base, &self.shadow);
        }
        self.shadow.clear();
        self.base = self.cursor;
    }
    fn rewind(&mut self) {
        self.shadow.clear();
        self.cursor = 0;
        self.base = 0;
    }
    fn slice(
        &mut self,
        context: &crate::device::DeviceContext,
        bytes: &[u8],
        align: u64,
        armed: bool,
    ) -> Slice {
        let size = bytes.len() as u64;
        let fallback = || {
            let buffer = wgpu::util::DeviceExt::create_buffer_init(
                &context.device,
                &wgpu::util::BufferInitDescriptor {
                    label: Some(self.label),
                    contents: bytes,
                    usage: self.usage,
                },
            );
            Slice {
                buffer,
                offset: 0,
                size: None,
            }
        };
        if !armed || size == 0 || !size.is_multiple_of(wgpu::COPY_BUFFER_ALIGNMENT) || size > CAP {
            return fallback();
        }
        let offset = self.cursor.next_multiple_of(align);
        let capacity = self.buffer.as_ref().map_or(0, wgpu::Buffer::size);
        if offset + size > capacity {
            let grown = (capacity * 2).max(FIRST).max(size.next_power_of_two());
            if grown > CAP {
                return fallback();
            }
            // Earlier slices of this frame keep the old buffer alive through their bind groups;
            // their staged bytes go to that buffer before it is replaced.
            self.flush(context);
            self.buffer = Some(context.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(self.label),
                size: grown,
                usage: self.usage | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.rewind();
            return self.slice(context, bytes, align, armed);
        }
        let buffer = self.buffer.clone().expect("arena buffer");
        // Alignment padding, then the slice, contiguous from base.
        self.shadow.resize((offset - self.base) as usize, 0);
        self.shadow.extend_from_slice(bytes);
        self.cursor = offset + size;
        Slice {
            buffer,
            offset,
            size: NonZeroU64::new(size),
        }
    }
}

/// A uniform and a storage arena, rewound together at each submission boundary.
pub(crate) struct Arenas {
    uniform: Arena,
    storage: Arena,
    armed: bool,
    /// Tests compare arena slices with the per-draw fallback; always true otherwise.
    enabled: bool,
}
impl Arenas {
    pub fn new(label: &'static str) -> Self {
        Self {
            uniform: Arena::new(wgpu::BufferUsages::UNIFORM, label),
            storage: Arena::new(wgpu::BufferUsages::STORAGE, label),
            armed: false,
            enabled: true,
        }
    }
    /// Call only where no command referencing an earlier slice is still being encoded.
    pub fn begin_frame(&mut self) {
        debug_assert!(
            !self.pending(),
            "arena slices were never flushed: call flush() before submitting"
        );
        self.armed = self.enabled;
        self.uniform.rewind();
        self.storage.rewind();
    }
    /// Upload this frame's staged slices; call right before submitting the commands using them.
    pub fn flush(&mut self, context: &crate::device::DeviceContext) {
        self.uniform.flush(context);
        self.storage.flush(context);
    }
    fn pending(&self) -> bool {
        self.uniform.pending() || self.storage.pending()
    }
    pub fn uniform(&mut self, context: &crate::device::DeviceContext, bytes: &[u8]) -> Slice {
        let align = u64::from(context.device.limits().min_uniform_buffer_offset_alignment);
        self.uniform.slice(context, bytes, align, self.armed)
    }
    #[cfg(test)]
    pub fn fallback_only(&mut self) {
        self.enabled = false;
        self.armed = false;
    }
    #[cfg(test)]
    pub fn capacity(&self) -> [u64; 2] {
        [&self.uniform, &self.storage].map(|a| a.buffer.as_ref().map_or(0, wgpu::Buffer::size))
    }
    pub fn storage(&mut self, context: &crate::device::DeviceContext, bytes: &[u8]) -> Slice {
        let align = u64::from(context.device.limits().min_storage_buffer_offset_alignment);
        self.storage.slice(context, bytes, align, self.armed)
    }
}
impl Drop for Arenas {
    fn drop(&mut self) {
        debug_assert!(
            std::thread::panicking() || !self.pending(),
            "arena slices were never flushed: call flush() before submitting"
        );
    }
}
