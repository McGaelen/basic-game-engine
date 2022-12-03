use std::thread;
use std::time::{Duration, SystemTime};

use super::event::Event;
use super::eventqueue::EventQueue;
use super::renderer::Renderer;
use super::game_window::GameWindow;

use std::{ffi::CString};
use imgui::{Condition, Window};
use sdl2::video::{VkInstance};
use sdl2::keyboard::Keycode;
use sdl2::event::Event as SdlEvent;
use vulkano::format::{ClearValue, Format};
use vulkano::image::view::ImageView;
use vulkano::image::{StorageImage, ImageDimensions};
use vulkano::{
  instance::{Instance, InstanceCreateInfo, InstanceExtensions},
  device::{physical::{PhysicalDevice}, Device, QueueCreateInfo, DeviceCreateInfo}, Version, VulkanObject, Handle, swapchain::{Surface, SurfaceApi}
};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo};
use vulkano::pipeline::graphics::vertex_input::{BuffersDefinition, Vertex};
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::render_pass::{RenderPass, RenderPassCreateInfo, SubpassDescription, AttachmentReference, Framebuffer, FramebufferCreateInfo};

const FRAME_DURATION: Duration = Duration::from_nanos(33_333_333);

pub type MainLoopFn = fn(engine: &mut Engine) -> Result<(), String>;

pub struct Engine {
  pub renderer: Renderer,
  pub game_window: GameWindow,
  pub event_queue: Vec<Event>,
  task: MainLoopFn,
}

impl Engine {
  pub fn run(task: MainLoopFn) {
    let (renderer, game_window) = Engine::init();

    let mut engine = Engine {
      renderer,
      game_window,
      event_queue: Vec::new(),
      task,
    };
    engine.main_loop();
  }

  fn init() -> (Renderer, GameWindow) {
    // Initialize SDL
    let sdl_context = sdl2::init().expect("Failed to initialize sdl2.");
    let event_pump = sdl_context.event_pump().unwrap();
    let video_subsystem = sdl_context.video().expect("Failed to create video subsystem.");

    // Create a window
    let window = video_subsystem.window("Vulkan Test App", 800, 600)
        .vulkan()
        .build()
        .expect("Failed to create game window.");

    // Add Vulkan extensions that allow it to render into a window
    let instance_extensions_strings: Vec<CString> = window
        .vulkan_instance_extensions()
        .unwrap()
        .iter()
        .map(|&v| CString::new(v).unwrap())
        .collect();
    let enabled_extensions = InstanceExtensions::from(instance_extensions_strings.iter().map(AsRef::as_ref));

    // Create Vulkan instance
    let instance = Instance::new(InstanceCreateInfo {
      application_name: Some("Vulkan Test App".to_string()),
      enabled_extensions,
      engine_version: Version::V1_2,
      ..Default::default()
    }).expect("Failed to create Vulkan instance");

    // Create surface for Vulkan to render to inside the window
    let surface_handle = window
        .vulkan_create_surface(instance.internal_object().as_raw() as VkInstance)
        .expect("Failed to create surface handle.");

    let surface = unsafe {
      Surface::from_raw_surface(
        instance.clone(),
        Handle::from_raw(surface_handle),
        SurfaceApi::Win32,
        window.context()
      )
    };

    // Take the first physical device we find
    let physical_device = PhysicalDevice::enumerate(&instance).next()
        .expect("No devices available that support Vulkan.");

    // Find all queue families on the physical device that support graphics.
    // Then create a QueueCreateInfo for each of them.
    let queue_create_infos: Vec<QueueCreateInfo> = physical_device.queue_families()
        .into_iter()
        .filter(|q| q.supports_graphics() || q.explicitly_supports_transfers())
        .map(|q| QueueCreateInfo::family(q))
        .collect();

    // Initialize the device by telling Vulkan which queue families we want to use on the device.
    let (device, mut queues) = Device::new(
      physical_device,
      DeviceCreateInfo {
        queue_create_infos,
        ..Default::default()
      },
    ).expect("failed to create device");

    let gfx_queue = queues
        .find(|q| q.family().supports_graphics())
        .expect("No graphics queue available.");

    let transfer_queue = queues
        .find(|q| !q.family().supports_graphics() && q.family().explicitly_supports_transfers())
        .unwrap_or(gfx_queue.clone());

    (
        Renderer { device, gfx_queue, transfer_queue, },
        GameWindow { sdl_context, event_pump, video_subsystem, window, surface, },
    )
  }

  fn main_loop(&mut self) {
    'running: loop {
      let start = SystemTime::now();

      for event in self.game_window.event_pump.poll_iter() {
        match event {
          SdlEvent::Quit {..} | SdlEvent::KeyDown { keycode: Some(Keycode::Escape), .. } => {
            break 'running
          },
          SdlEvent::KeyDown {keycode: Some(Keycode::A), ..} => {
            self.event_queue.push(Event {
              task: || {
                println!("doing the thingy")
              },
              name: "event".to_string(),
              frames: 10
            });
          }
          SdlEvent::KeyDown {keycode, ..} => println!("{:?}", keycode.unwrap()),
          _ => {}
        }
      }

      let mut builder = AutoCommandBufferBuilder::primary(
        self.renderer.device.clone(), self.renderer.gfx_queue.family(), CommandBufferUsage::OneTimeSubmit
      ).unwrap();

      let render_pass = vulkano::single_pass_renderpass!(self.renderer.device.clone(),
        attachments: {
          color: {
            load: Clear,
            store: Store,
            format: Format::R8G8B8A8_UNORM,
            samples: 1,
          }
        },
        pass: {
          color: [color],
          depth_stencil: {}
        }
      ).unwrap();

      let image = StorageImage::new(
        self.renderer.device.clone(),
        ImageDimensions::Dim2d { width: 800, height: 600, array_layers: 1 },
        Format::R8G8B8A8_UNORM,
        Some(self.renderer.gfx_queue.family())
      ).unwrap();
      let view = ImageView::new_default(image.clone()).unwrap();

      let framebuffer = Framebuffer::new(
        render_pass.clone(),
        FramebufferCreateInfo {
          attachments: vec![view],
          ..Default::default()
        },
      ).unwrap();

      builder
          .begin_render_pass(
            RenderPassBeginInfo {
              clear_values: vec![Some(ClearValue::Float([0.0, 0.0, 1.0, 1.0]))],
              ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
            },
            vulkano::command_buffer::SubpassContents::Inline,
          )
          .unwrap()
          .end_render_pass()
          .unwrap();

      mod vs {
        vulkano_shaders::shader!{
        ty: "vertex",
        src: "
#version 450

layout(location = 0) in vec2 position;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
}"
    }
      }

      mod fs {
        vulkano_shaders::shader!{
        ty: "fragment",
        src: "
#version 450

layout(location = 0) out vec4 f_color;

void main() {
    f_color = vec4(1.0, 0.0, 0.0, 1.0);
}"
    }
      }

      let vs = vs::load(self.renderer.device.clone()).expect("failed to create vs shader module");
      let fs = fs::load(self.renderer.device.clone()).expect("failed to create fs shader module");

      let viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [800.0, 600.0],
        depth_range: 0.0..1.0,
      };

      // let pipeline = GraphicsPipeline::start()
      //     .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())

      self.run_task();

      self.event_queue.run_all();
      self.event_queue.prune();

      Engine::end(start);
    }
  }

  fn run_task(&mut self) {
    match (self.task)(self) {
      Ok(_) => {}
      Err(msg) => println!("{}", msg)
    }
  }

  fn end(start: SystemTime) {
    let max_frame_time = start + FRAME_DURATION;

    match max_frame_time.duration_since(SystemTime::now()) {
      Ok(duration) => thread::sleep(duration),
      Err(_err) => ()
    }
  }
}