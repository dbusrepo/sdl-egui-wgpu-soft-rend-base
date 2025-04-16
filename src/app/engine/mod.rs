use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;

use super::sdl_wgpu::SdlWgpu;

#[derive(Debug, Clone)]
pub(super) struct EngineConfig {}

pub(super) struct Engine<'a> {
    cfg:      Rc<RefCell<EngineConfig>>,
    sdl_wgpu: Rc<RefCell<SdlWgpu<'a>>>,
}

impl<'a> Engine<'a> {
    pub(super) fn new(
        cfg: Rc<RefCell<EngineConfig>>, sdl_wgpu: Rc<RefCell<SdlWgpu<'a>>>,
    ) -> Result<Self> {
        Ok(Self { cfg, sdl_wgpu })
    }

    pub(super) fn update(&mut self, _step_time: f64) -> Result<()> {
        Ok(())
    }

    pub(super) fn render(&self) -> Result<()> {
        self.sdl_wgpu.borrow_mut().clear()
    }
}
