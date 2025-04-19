use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;

use super::sdl_wgpu::SdlWgpu;

mod renderer;

use renderer::Renderer;

pub(super) struct EngineConfiguration {}

pub(super) struct Engine<'a> {
    cfg:      Rc<RefCell<EngineConfiguration>>,
    renderer: Renderer<'a>,
}

impl<'a> Engine<'a> {
    pub(super) fn new(
        cfg: Rc<RefCell<EngineConfiguration>>, sdl_wgpu: Rc<RefCell<SdlWgpu<'a>>>,
    ) -> Result<Self> {
        let renderer = Renderer::new(sdl_wgpu)?;
        Ok(Self { cfg, renderer })
    }

    pub(super) fn update(&mut self, _step_time: f64) -> Result<()> {
        Ok(())
    }

    pub(super) fn render(&mut self) -> Result<()> {
        self.renderer.render()
    }
}
