#![allow(dead_code)]

use anyhow::{Context, Result};

pub(super) struct FrameBuffer {
    pub width:  u32,
    pub height: u32,
    pub color:  Vec<u8>,
    pub depth:  Vec<f32>,
}

impl FrameBuffer {
    pub(super) fn new(width: u32, height: u32) -> Result<Self> {
        #[allow(clippy::as_conversions)]
        let width_usize = width as usize;
        #[allow(clippy::as_conversions)]
        let height_usize = height as usize;

        let num_pixels = width_usize
            .checked_mul(height_usize)
            .context("Overflow calculating frame buffer size")?;

        let color_buffer_size =
            num_pixels.checked_mul(4).context("Overflow calculating color buffer size")?;

        let depth_buffer_size = num_pixels;

        let color_buffer: Vec<u8> = vec![0; color_buffer_size];
        let depth_buffer: Vec<f32> = vec![1000.0; depth_buffer_size];

        Ok(Self { color: color_buffer, depth: depth_buffer, width, height })
    }
}
