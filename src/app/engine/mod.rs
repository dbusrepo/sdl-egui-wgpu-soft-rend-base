use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;

use super::sdl_wgpu::SdlWgpu;

mod renderer;
mod world;

use renderer::Renderer;
use world::World;

pub(super) struct EngineConfiguration {}

pub(super) struct Engine<'a> {
    cfg:      Rc<RefCell<EngineConfiguration>>,
    world:    World,
    renderer: Renderer<'a>,
}

impl<'a> Engine<'a> {
    pub(super) fn new(
        cfg: Rc<RefCell<EngineConfiguration>>,
        sdl_wgpu: Rc<RefCell<SdlWgpu<'a>>>,
    ) -> Result<Self> {
        let world = World::new()?;
        let renderer = Renderer::new(sdl_wgpu)?;
        Ok(Self { cfg, world, renderer })
    }

    pub(super) fn update(&mut self, step_time: f64) -> Result<()> {
        self.world.update(step_time)
    }

    pub(super) fn render(&mut self) -> Result<()> {
        self.renderer.render()
    }
}
