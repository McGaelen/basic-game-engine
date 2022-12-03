use std::borrow::Cow;
use std::mem::size_of;
use tobj::{LoadOptions, Material, Model};
use wgpu::{Backends, DeviceDescriptor, Instance, PowerPreference, RequestAdapterOptions, Features, Limits, SurfaceConfiguration, TextureUsages, PresentMode, CompositeAlphaMode, TextureViewDescriptor, BufferDescriptor, BufferAddress, BufferUsages, CommandEncoderDescriptor, Label, RenderPassDescriptor, RenderPassColorAttachment, Operations, LoadOp, Color, RenderPipelineDescriptor, PipelineLayout, MultisampleState, VertexState, ShaderModule, ShaderModuleDescriptor, ShaderSource, PrimitiveState, VertexBufferLayout, VertexAttribute, VertexFormat, VertexStepMode};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use winit::window::CursorIcon::Default;
use winit::window::Window;

pub struct GraphicsState {
  pub surface: wgpu::Surface, // The surface for the window we're rendering onto
  pub config: SurfaceConfiguration, // The surface's config (size, vsync, format)
  pub device: wgpu::Device, // The gpu
  pub queue: wgpu::Queue, // Where commands are submitted to

  pub models: Vec<Model>,
  pub materials: Vec<Material>,
}

impl GraphicsState {
  pub async fn new(window: &Window) -> Self {
    let size = window.inner_size();

    let instance = Instance::new(Backends::all());
    let surface = unsafe { instance.create_surface(window) };
    let adapter = instance.request_adapter(
      &RequestAdapterOptions {
        power_preference: PowerPreference::HighPerformance,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false
      }
    ).await.unwrap();

    let (device, queue) = adapter.request_device(
      &DeviceDescriptor {
        features: Features::empty(),
        limits: if cfg!(target_arch = "wasm32") {
          Limits::downlevel_webgl2_defaults()
        } else {
          Limits::default()
        },
        label: None
      },
      None
    ).await.unwrap();

    let config = SurfaceConfiguration {
      usage: TextureUsages::RENDER_ATTACHMENT,
      format: surface.get_supported_formats(&adapter)[0],
      width: size.width,
      height: size.height,
      present_mode: PresentMode::Fifo,
      alpha_mode: CompositeAlphaMode::Auto
    };
    surface.configure(&device, &config);

    let obj = tobj::load_obj(
      "assets/teslacyberv3.0.obj",
      &LoadOptions {
        single_index: true,
        triangulate: true,
        ..LoadOptions::default()
      }
    ).unwrap();

    let models = obj.0;
    let materials = obj.1.unwrap();

    GraphicsState {
      surface,
      device,
      queue,
      config,
      models,
      materials
    }
  }

  pub fn resize(&mut self, new_width: u32, new_height: u32) {
    if new_width > 0 && new_height > 0 {
      self.config.width = new_width;
      self.config.height = new_height;
      self.surface.configure(&self.device, &self.config)
    }
  }

  // pub fn input(&mut self, event: &WindowEvent) -> bool {
  //   todo!()
  // }

  // pub fn update(&mut self) {
  //   todo!()
  // }

  pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
    let output = self.surface.get_current_texture()?;

    let view = output.texture.create_view(&TextureViewDescriptor::default());

    let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
      label: Some("my-command-encoder")
    });



    let buffer = self.device.create_buffer_init(&BufferInitDescriptor {
      label: Some("my-buffer"),
      usage: BufferUsages::VERTEX,
      contents: bytemuck::cast_slice(&self.models[0].mesh.positions[..])
    });

    let buffer_layout = VertexBufferLayout {
      array_stride: size_of::<[f32; 3]>() as BufferAddress,
      step_mode: VertexStepMode::Vertex,
      attributes: &[
        VertexAttribute {
          format: VertexFormat::Float32x3, // represents a vec3 in the shader code
          shader_location: 0, // maps to the shader's @location
          offset: 0 // Offset from the previous VertexAttribute - but we only have one, so it's zero.
        }
      ]
    };

    let shader_module = self.device.create_shader_module(ShaderModuleDescriptor {
      label: Some("my-shader"),
      source: ShaderSource::Wgsl(Cow::Borrowed(
"
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(1 - i32(in_vertex_index)) * 0.5;
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    return out;
}
"
      ))
    });

    let render_pipeline = self.device.create_render_pipeline(&RenderPipelineDescriptor {
      label: Some("my-render-pipeline"),
      depth_stencil: None,
      layout: None,
      fragment: None,
      multisample: MultisampleState::default(),
      multiview: None,
      vertex: VertexState {
        buffers: &[buffer_layout],
        module: &shader_module,
        entry_point: "vertex-entry"
      },
      primitive: PrimitiveState::default()
    });

    { // we have this new scope so that `encoder` can be given back (it is borrowed here)
      let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("my-render-pass"),
        color_attachments: &[Some(RenderPassColorAttachment {
          view: &view,
          ops: Operations {
            load: LoadOp::Clear(Color {
              r: 0.1,
              g: 0.2,
              b: 0.3,
              a: 1.0
            }),
            store: true
          },
          resolve_target: None
        })],
        depth_stencil_attachment: None
      });

      render_pass.set_vertex_buffer(0, buffer.slice(..));
      render_pass.set_pipeline(&render_pipeline);
      render_pass.draw(0..((self.models[0].mesh.positions.len() / 3) as u32), 0..1);
    }

    // here's where we move `encoder` - which is why we have the scope above.
    self.queue.submit(std::iter::once(encoder.finish()));
    output.present();

    Ok(())
  }
}

// fn convert_to_2d_array
