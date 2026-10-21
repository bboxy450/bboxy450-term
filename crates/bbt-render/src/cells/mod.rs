//! Packed glyph instances; colours and flags retain separate fields.
use bytemuck::{Pod, Zeroable};

/// The upstream layout is 36 bytes with four f32 UVs. Pixel UVs fit in u16,
/// retaining all fields in the brief's 32 bytes without a C/GL dependency.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct InstanceData {
    pub cell: [u16; 2],
    pub bearing: [i16; 2],
    pub size: [i16; 2],
    pub atlas: [u16; 4],
    pub foreground: [u8; 4],
    pub background: [u8; 4],
    pub flags: u32,
}
const _: () = assert!(std::mem::size_of::<InstanceData>() == 32);

impl InstanceData {
    pub const ATTRIBUTES: [wgpu::VertexAttribute; 7] = wgpu::vertex_attr_array![
        0 => Uint16x2, 1 => Sint16x2, 2 => Sint16x2, 3 => Uint16x4,
        4 => Unorm8x4, 5 => Unorm8x4, 6 => Uint32
    ];
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout { array_stride: 32, step_mode: wgpu::VertexStepMode::Instance, attributes: &Self::ATTRIBUTES }
    }
}
