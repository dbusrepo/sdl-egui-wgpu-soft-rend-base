use std::cell::RefCell;
use std::rc::Rc;

use anyhow::{Context, Result};
use egui_wgpu_backend::wgpu::{self, PipelineCompilationOptions};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    AddressMode,
    BindGroup,
    BindGroupDescriptor,
    BindGroupEntry,
    BindGroupLayoutDescriptor,
    BindGroupLayoutEntry,
    BindingResource,
    BindingType,
    BlendState,
    Buffer,
    BufferAddress,
    BufferUsages,
    ColorTargetState,
    ColorWrites,
    Extent3d,
    FilterMode,
    FragmentState,
    LoadOp,
    MultisampleState,
    Operations,
    Origin3d,
    PipelineLayoutDescriptor,
    PrimitiveState,
    RenderPassColorAttachment,
    RenderPassDescriptor,
    RenderPipeline,
    RenderPipelineDescriptor,
    SamplerBindingType,
    SamplerDescriptor,
    ShaderModuleDescriptor,
    ShaderSource,
    ShaderStages,
    StoreOp,
    TexelCopyBufferLayout,
    TexelCopyTextureInfo,
    Texture,
    TextureAspect,
    TextureDescriptor,
    TextureDimension,
    TextureFormat,
    TextureSampleType,
    TextureUsages,
    TextureViewDescriptor,
    TextureViewDimension,
    VertexAttribute,
    VertexBufferLayout,
    VertexFormat,
    VertexState,
    VertexStepMode,
};

use crate::app::sdl_wgpu::{SdlWgpu, SdlWgpuConfiguration};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    uv:       [f32; 2],
}

impl Vertex {
    const fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            #[allow(clippy::as_conversions)]
            array_stride: size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                // position at location 0
                VertexAttribute {
                    offset:          0,
                    shader_location: 0,
                    format:          VertexFormat::Float32x2,
                },
                // uv at location 1
                VertexAttribute {
                    #[allow(clippy::as_conversions)]
                    offset: size_of::<[f32; 2]>() as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x2,
                },
            ],
        }
    }
}

// Full-screen quad (two triangles).
const VERTICES: &[Vertex] = &[
    Vertex { position: [-1.0, -1.0], uv: [0.0, 1.0] },
    Vertex { position: [1.0, -1.0], uv: [1.0, 1.0] },
    Vertex { position: [1.0, 1.0], uv: [1.0, 0.0] },
    Vertex { position: [-1.0, -1.0], uv: [0.0, 1.0] },
    Vertex { position: [1.0, 1.0], uv: [1.0, 0.0] },
    Vertex { position: [-1.0, 1.0], uv: [0.0, 0.0] },
];

// A minimal WGSL shader that draws a textured quad.
const QUAD_SHADER: &str = r"
    struct VertexOutput {
        @builtin(position) position: vec4<f32>,
        @location(0) uv: vec2<f32>,
    };

    @vertex
    fn vs_main(@location(0) position: vec2<f32>, @location(1) uv: vec2<f32>) -> VertexOutput {
        var out: VertexOutput;
        out.position = vec4<f32>(position, 0.0, 1.0);
        out.uv = uv;
        return out;
    }

    @group(0) @binding(0)
    var myTexture: texture_2d<f32>;
    @group(0) @binding(1)
    var mySampler: sampler;

    @fragment
    fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
        return textureSample(myTexture, mySampler, in.uv);
        // return vec4<f32>(1.0, 0.0, 0.0, 1.0);
    }
";

pub(super) struct ScreenQuad<'a> {
    sdl_wgpu:      Rc<RefCell<SdlWgpu<'a>>>,
    texture:       Texture,
    pipeline:      RenderPipeline,
    bind_group:    BindGroup,
    vertex_buffer: Buffer,
    num_vertices:  u32,
}

impl<'a> ScreenQuad<'a> {
    pub(super) fn new(sdl_wgpu: Rc<RefCell<SdlWgpu<'a>>>) -> Self {
        let SdlWgpuConfiguration { width, height, .. } = *sdl_wgpu.borrow().cfg.borrow();

        let screen_texture = sdl_wgpu.borrow_mut().device.create_texture(&TextureDescriptor {
            label:           Some("Screen Render Texture"),
            size:            Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count:    1,
            dimension:       TextureDimension::D2,
            format:          TextureFormat::Rgba8Unorm,
            usage:           TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats:    &[TextureFormat::Rgba8Unorm],
        });

        let screen_sampler = sdl_wgpu.borrow_mut().device.create_sampler(&SamplerDescriptor {
            label: Some("Screen Texture Sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            ..SamplerDescriptor::default()
        });

        let screen_bind_group_layout =
            sdl_wgpu.borrow_mut().device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label:   Some("Screen Bind Group Layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding:    0,
                        visibility: ShaderStages::FRAGMENT,
                        ty:         BindingType::Texture {
                            multisampled:   false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type:    TextureSampleType::Float { filterable: false },
                        },
                        count:      None,
                    },
                    BindGroupLayoutEntry {
                        binding:    1,
                        visibility: ShaderStages::FRAGMENT,
                        ty:         BindingType::Sampler(SamplerBindingType::NonFiltering),
                        count:      None,
                    },
                ],
            });

        let screen_texture_view = screen_texture.create_view(&TextureViewDescriptor::default());

        let screen_bind_group =
            sdl_wgpu.borrow_mut().device.create_bind_group(&BindGroupDescriptor {
                label:   Some("Screen Bind Group"),
                layout:  &screen_bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding:  0,
                        resource: BindingResource::TextureView(&screen_texture_view),
                    },
                    BindGroupEntry {
                        binding:  1,
                        resource: BindingResource::Sampler(&screen_sampler),
                    },
                ],
            });

        let screen_pipeline_layout =
            sdl_wgpu.borrow_mut().device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label:                Some("Screen Pipeline Layout"),
                bind_group_layouts:   &[&screen_bind_group_layout],
                push_constant_ranges: &[],
            });

        let screen_shader_module =
            sdl_wgpu.borrow_mut().device.create_shader_module(ShaderModuleDescriptor {
                label:  Some("Screen quad Shader"),
                source: ShaderSource::Wgsl(QUAD_SHADER.into()),
            });

        let screen_pipeline = {
            let sdl_wgpu = sdl_wgpu.borrow_mut();

            sdl_wgpu.device.create_render_pipeline(&RenderPipelineDescriptor {
                label:         Some("Screen Render Pipeline"),
                layout:        Some(&screen_pipeline_layout),
                vertex:        VertexState {
                    module:              &screen_shader_module,
                    entry_point:         Some("vs_main"),
                    buffers:             &[Vertex::desc()],
                    compilation_options: PipelineCompilationOptions::default(),
                },
                fragment:      Some(FragmentState {
                    module:              &screen_shader_module,
                    entry_point:         Some("fs_main"),
                    targets:             &[Some(ColorTargetState {
                        format:     sdl_wgpu.surface_configuration.format,
                        blend:      Some(BlendState::ALPHA_BLENDING),
                        write_mask: ColorWrites::ALL,
                    })],
                    compilation_options: PipelineCompilationOptions::default(),
                }),
                primitive:     PrimitiveState::default(),
                depth_stencil: None,
                multisample:   MultisampleState::default(),
                multiview:     None,
                cache:         None,
            })
        };

        let screen_vertex_buffer =
            sdl_wgpu.borrow_mut().device.create_buffer_init(&BufferInitDescriptor {
                label:    Some("Screen Vertex Buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage:    BufferUsages::VERTEX,
            });

        #[allow(clippy::cast_possible_truncation, clippy::as_conversions)]
        let screen_num_vertices = VERTICES.len() as u32;

        Self {
            sdl_wgpu,
            texture: screen_texture,
            pipeline: screen_pipeline,
            bind_group: screen_bind_group,
            vertex_buffer: screen_vertex_buffer,
            num_vertices: screen_num_vertices,
        }
    }

    pub(super) fn width(&self) -> u32 {
        self.sdl_wgpu.borrow().cfg.borrow().width
    }

    pub(super) fn height(&self) -> u32 {
        self.sdl_wgpu.borrow().cfg.borrow().height
    }

    fn update_texture(&self, pixel_data: &[u8]) -> Result<()> {
        let width = self.texture.width();
        let height = self.texture.height();

        let bytes_per_row = Some(width.checked_mul(4).with_context(|| {
            format!("Arithmetic overflow when computing bytes_per_row: 4 * {width}")
        })?);

        self.sdl_wgpu.borrow().queue.write_texture(
            TexelCopyTextureInfo {
                texture:   &self.texture,
                mip_level: 0,
                origin:    Origin3d::ZERO,
                aspect:    TextureAspect::All,
            },
            pixel_data,
            TexelCopyBufferLayout { offset: 0, bytes_per_row, rows_per_image: Some(height) },
            Extent3d { width, height, depth_or_array_layers: 1 },
        );

        Ok(())
    }

    // Renders the full-screen quad that displays the software texture.
    pub(super) fn render(&self, pixel_data: &[u8]) -> Result<()> {
        self.update_texture(pixel_data)?;

        let SdlWgpu { frame, encoder, .. } = &mut *self.sdl_wgpu.borrow_mut();

        #[allow(clippy::shadow_reuse)]
        let frame = frame.as_ref().context("Failed to get frame")?;

        let frame_view = frame.texture.create_view(&TextureViewDescriptor::default());

        let mut pass = encoder.as_mut().context("Failed to get the encoder")?.begin_render_pass(
            &RenderPassDescriptor {
                label:                    Some("texture quad render pass"),
                color_attachments:        &[Some(RenderPassColorAttachment {
                    view:           &frame_view,
                    resolve_target: None,
                    ops:            Operations {
                        // load: LoadOp::Clear(Color::BLUE),
                        load:  LoadOp::Load,
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes:         None,
                occlusion_query_set:      None,
            },
        );

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..self.num_vertices, 0..1);

        Ok(())
    }
}
