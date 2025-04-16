use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;

use super::sdl_wgpu::SdlWgpu;

pub(super) struct Engine<'a> {
    sdl_wgpu: Rc<RefCell<SdlWgpu<'a>>>,
}

impl<'a> Engine<'a> {
    pub(super) fn new(sdl_wgpu: Rc<RefCell<SdlWgpu<'a>>>) -> Result<Self> {
        Ok(Self { sdl_wgpu })
    }

    pub(super) fn update(&mut self, _step_time: f64) -> Result<()> {
        Ok(())
    }

    pub(super) fn render(&self) -> Result<()> {
        self.sdl_wgpu.borrow_mut().clear()
    }
}
