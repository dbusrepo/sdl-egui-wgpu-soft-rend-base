use std::cell::RefCell;
use std::rc::Rc;

use anyhow::{Context, Result};
use egui::FullOutput;
use egui::epaint::textures::TexturesDelta;
use egui_sdl2_platform::platform::Platform;
use egui_wgpu_backend::RenderPass;
use egui_wgpu_backend::wgpu::{self};
use wgpu::TextureViewDescriptor;

use crate::app::sdl_wgpu::SdlWgpu;

pub(super) struct EguiRender<'a> {
    pub platform:  Rc<RefCell<Platform>>,
    pub sdl_wgpu:  Rc<RefCell<SdlWgpu<'a>>>,
    pub egui_pass: RenderPass,
    tdelta:        Option<TexturesDelta>,
}

impl<'a> EguiRender<'a> {
    pub(super) fn new(platform: Rc<RefCell<Platform>>, sdl_wgpu: Rc<RefCell<SdlWgpu<'a>>>) -> Self {
        let egui_pass =
            RenderPass::new(&sdl_wgpu.borrow().device, sdl_wgpu.borrow().surface_format, 1);

        Self { platform, sdl_wgpu, egui_pass, tdelta: None }
    }

    pub(super) fn render(&mut self) -> Result<()> {
        let mut platform = self.platform.borrow_mut();
        let mut sdl_wgpu = self.sdl_wgpu.borrow_mut();

        // Stop drawing the egui frame and get the full output
        let full_output = platform.end_frame(&mut sdl_wgpu.video)?;

        // Get the paint jobs
        let paint_jobs = platform.tessellate(&full_output);

        // // Upload all the resources to the egui render pass
        let screen_descriptor = egui_wgpu_backend::ScreenDescriptor {
            physical_width:  sdl_wgpu.surface_configuration.width,
            physical_height: sdl_wgpu.surface_configuration.height,
            // The sdl scale factor remains constant, so we shall set it to 1
            scale_factor:    1.0,
        };

        // Add the textures to the egui render pass
        let FullOutput { textures_delta, .. } = full_output;

        self.egui_pass.add_textures(&sdl_wgpu.device, &sdl_wgpu.queue, &textures_delta)?;

        self.tdelta = Some(textures_delta);

        self.egui_pass.update_buffers(
            &sdl_wgpu.device,
            &sdl_wgpu.queue,
            &paint_jobs,
            &screen_descriptor,
        );

        let SdlWgpu { frame, encoder, .. } = &mut *sdl_wgpu;

        let frame = frame.as_ref().context("Failed to get frame")?;

        let frame_view = frame.texture.create_view(&TextureViewDescriptor::default());

        self.egui_pass.execute(
            encoder.as_mut().context("Failed to get the encoder")?,
            &frame_view,
            &paint_jobs,
            &screen_descriptor,
            None,
        )?;

        Ok(())
    }

    pub(super) fn clean(&mut self) -> Result<()> {
        self.egui_pass.remove_textures(self.tdelta.take().context("No textures delta")?)?;
        Ok(())
    }
}
