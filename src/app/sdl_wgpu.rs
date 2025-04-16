#![allow(unused_results)]

use std::cell::RefCell;
use std::rc::Rc;

use anyhow::{Context, Result, anyhow};
use egui_sdl2_platform::sdl2;
use egui_wgpu_backend::wgpu::{self, Features, Limits};
use sdl2::video::Window;
use sdl2::{Sdl, VideoSubsystem};
use wgpu::{
    CommandEncoder,
    CommandEncoderDescriptor,
    Device,
    DeviceDescriptor,
    PowerPreference,
    PresentMode,
    Queue,
    RequestAdapterOptions,
    Surface,
    SurfaceConfiguration,
    SurfaceTexture,
    TextureFormat,
    TextureViewDescriptor,
};

#[derive(Debug, Clone)]
pub(super) struct SdlWgpuConfig {
    pub title:      &'static str,
    pub width:      u32,
    pub height:     u32,
    pub fullscreen: bool,
    pub vsync:      bool,
}

pub(super) struct SdlWgpu<'a> {
    pub cfg:                   Rc<RefCell<SdlWgpuConfig>>,
    pub frame:                 Option<SurfaceTexture>,
    pub encoder:               Option<CommandEncoder>,
    pub surface:               Surface<'a>,
    pub surface_configuration: SurfaceConfiguration,
    pub surface_format:        TextureFormat,
    pub queue:                 Queue,
    pub device:                Device,
    pub window:                Window,
    pub video:                 VideoSubsystem,
    pub context:               Sdl,
}

impl SdlWgpu<'_> {
    pub(super) fn new(cfg: Rc<RefCell<SdlWgpuConfig>>) -> Result<Self> {
        let SdlWgpuConfig { title, width, height, fullscreen, vsync } = *cfg.borrow();

        let context = sdl2::init().map_err(|e| anyhow!("Failed to create sdl context: {}", e))?;

        let video = context
            .video()
            .map_err(|e| anyhow::anyhow!("Failed to initialize sdl video subsystem: {}", e))?;

        let mut window_builder = video.window(title, width, height);

        if fullscreen {
            window_builder.fullscreen();
            // window_builder.fullscreen_desktop();
        } else {
            window_builder.position_centered();
        }

        let window = window_builder.allow_highdpi().metal_view().build()?;

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

        let surface = {
            #[allow(unsafe_code)]
            unsafe {
                instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(&window)?)?
            }
        };

        let adapter_opt = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
            power_preference:       PowerPreference::None,
            force_fallback_adapter: false,
            compatible_surface:     Some(&surface),
        }));

        let adapter = adapter_opt.context("Failed to find wgpu adapter")?;

        let (device, queue) = match pollster::block_on(adapter.request_device(
            &DeviceDescriptor {
                label: Some("device"),
                required_features: Features::default(),
                required_limits: Limits::default(),
                ..Default::default()
            },
            None,
        )) {
            Ok(a) => a,
            Err(e) => return Err(anyhow!("{}", e.to_string())),
        };

        let surface_format = surface
            .get_capabilities(&adapter)
            .formats
            .first()
            .copied()
            .context("No surface formats")?;

        let present_mode = if vsync { PresentMode::Fifo } else { PresentMode::Immediate };

        let surface_configuration = SurfaceConfiguration {
            present_mode,
            // present_mode: wgpu::PresentMode::AutoVsync,
            format: surface_format,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            view_formats: vec![TextureFormat::Bgra8UnormSrgb],
            ..surface
                .get_default_config(&adapter, width, height)
                .context("Failed to get SurfaceConfiguration default config")?
        };

        surface.configure(&device, &surface_configuration);

        Ok(Self {
            cfg,
            context,
            window,
            video,
            surface,
            surface_format,
            surface_configuration,
            device,
            queue,
            frame: None,
            encoder: None,
        })
    }

    pub(super) fn init_render(&mut self) -> Result<()> {
        let frame = self
            .surface
            .get_current_texture()
            .map_err(|e| anyhow!("Failed to get current texture: {}", e))?;

        // self.frame_view = Some(frame.texture.create_view(&TextureViewDescriptor::default()));

        self.frame = Some(frame);

        self.encoder = Some(self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Main Command Encoder"),
        }));

        Ok(())
    }

    pub(super) fn clear(&mut self) -> Result<()> {
        let frame = self.frame.as_ref().context("Failed to get frame")?;

        let frame_view = frame.texture.create_view(&TextureViewDescriptor::default());

        let color = [0.0, 0.0, 0.0, 1.0];

        let encoder = self.encoder.as_mut().context("Failed to get the encoder")?;

        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view:           &frame_view,
                resolve_target: None,
                ops:            wgpu::Operations {
                    load:  wgpu::LoadOp::Clear(wgpu::Color {
                        r: color[0],
                        g: color[1],
                        b: color[2],
                        a: color[3],
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            label: None,
            ..Default::default()
        });

        Ok(())
    }

    pub(super) fn present(&mut self) {
        if let Some(encoder) = self.encoder.take() {
            let command_buffer = encoder.finish();
            self.queue.submit(Some(command_buffer));
        }
        if let Some(frame) = self.frame.take() {
            frame.present();
        }
    }
}
