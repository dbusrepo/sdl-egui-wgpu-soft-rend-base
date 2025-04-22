use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;

mod frame_buffer;
mod screen_quad;

use frame_buffer::FrameBuffer;
use screen_quad::ScreenQuad;

use crate::app::sdl_wgpu::{SdlWgpu, SdlWgpuConfiguration};

pub(super) struct Renderer<'a> {
    screen_quad:  ScreenQuad<'a>,
    frame_buffer: FrameBuffer,
}

impl<'a> Renderer<'a> {
    pub(super) fn new(sdl_wgpu: Rc<RefCell<SdlWgpu<'a>>>) -> Result<Self> {
        let SdlWgpuConfiguration { width, height, .. } = *sdl_wgpu.borrow().cfg.borrow();

        let screen_quad = ScreenQuad::new(sdl_wgpu);

        let frame_buffer = FrameBuffer::new(width, height)?;

        Ok(Self { screen_quad, frame_buffer })
    }

    fn color_buffer(&self) -> &[u8] {
        self.frame_buffer.color.as_slice()
    }

    pub(super) fn render(&mut self) -> Result<()> {
        self.screen_quad.render(self.color_buffer())
    }
}
