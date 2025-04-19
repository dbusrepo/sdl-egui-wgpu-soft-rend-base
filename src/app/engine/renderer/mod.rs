use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;

use super::super::sdl_wgpu::{SdlWgpu, SdlWgpuConfiguration};

pub(super) struct Renderer<'a> {
    sdl_wgpu: Rc<RefCell<SdlWgpu<'a>>>,
}

impl<'a> Renderer<'a> {
    pub(super) fn new(sdl_wgpu: Rc<RefCell<SdlWgpu<'a>>>) -> Result<Self> {
        Ok(Self { sdl_wgpu })
    }

    pub(super) fn render(&mut self) -> Result<()> {
        Ok(())
    }
}
