use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;

mod frame_buffer;

use frame_buffer::FrameBuffer;

use crate::app::screen_quad::ScreenQuad;

pub(super) struct Renderer<'a> {
    screen_quad:  ScreenQuad<'a>,
    frame_buffer: FrameBuffer,
}

impl<'a> Renderer<'a> {
    pub(super) fn new(screen_quad: ScreenQuad<'a>) -> Result<Self> {
        let frame_buffer = FrameBuffer::new(screen_quad.width(), screen_quad.height())?;

        Ok(Self { screen_quad, frame_buffer })
    }

    fn color_buffer(&self) -> &[u8] {
        self.frame_buffer.color.as_slice()
    }

    pub(super) fn render(&mut self) -> Result<()> {
        self.screen_quad.render(self.color_buffer())
    }
}
