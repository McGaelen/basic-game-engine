use std::thread;
use std::time::{Duration, SystemTime};

use super::event::Event;
use super::eventqueue::EventQueue;
use super::renderer::Renderer;
use super::game_window::GameWindow;

use std::{ffi::CString};
use sdl2::video::{VkInstance};
use sdl2::keyboard::Keycode;
use sdl2::event::Event as SdlEvent;
use vulkano::{
  instance::{Instance, InstanceCreateInfo, InstanceExtensions},
  device::{physical::{PhysicalDevice}, Device, QueueCreateInfo, DeviceCreateInfo}, Version, VulkanObject, Handle, swapchain::{Surface, SurfaceApi}
};

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