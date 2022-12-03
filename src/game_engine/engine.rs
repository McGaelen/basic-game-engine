#[cfg(target_arch="wasm32")]
use wasm_bindgen::prelude::*;

use std::borrow::Borrow;
use std::task::Poll;
use std::thread;
use std::time::{Duration, SystemTime};
use wgpu::SurfaceError;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

use super::task::GameEvent;
use super::taskqueue::taskqueue::GameEventQueue;
// use crate::game_engine::taskqueue::;

const FRAME_DURATION: Duration = Duration::from_nanos(33_333_333);

pub type MainLoopFn = fn(engine: &mut Engine) -> Result<(), String>;

pub struct Engine {
  pub event_queue: Vec<GameEvent>,
  task: MainLoopFn,


}

impl Engine {
  pub fn run(task: MainLoopFn) {
    let mut engine = Engine {
      event_queue: Vec::new(),
      task,
    };

    pollster::block_on(Engine::init());

    // engine.init()
    // engine.main_loop();
  }

  async fn init() {
    cfg_if::cfg_if! {
      if #[cfg(target_arch = "wasm32")] {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(log::Level::Warn).expect("Couldn't initialize logger");
      } else {
        env_logger::init();
      }
    }
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    #[cfg(target_arch = "wasm32")]
    {
      // Winit prevents sizing with CSS, so we have to set
      // the size manually when on web.
      use winit::dpi::PhysicalSize;
      window.set_inner_size(PhysicalSize::new(450, 400));

      use winit::platform::web::WindowExtWebSys;
      web_sys::window()
          .and_then(|win| win.document())
          .and_then(|doc| {
            let dst = doc.get_element_by_id("wasm-example")?;
            let canvas = web_sys::Element::from(window.canvas());
            dst.append_child(&canvas).ok()?;
            Some(())
          })
          .expect("Couldn't append canvas to document body.");
    }

    let mut gfx_state =
        super::graphics::graphics_state::GraphicsState::new(&window).await;

    event_loop.run(move |event, _, control_flow| {
      control_flow.set_poll();

      match event {
        Event::NewEvents(_) => {}

        Event::WindowEvent {
          window_id,
          ref event
        } if window_id == window.id() => match event {

          WindowEvent::CloseRequested |
          WindowEvent::KeyboardInput {
            input: KeyboardInput {
              state: ElementState::Pressed,
              virtual_keycode: Some(VirtualKeyCode::Escape), ..
            }, ..
          } => control_flow.set_exit(),

          WindowEvent::KeyboardInput {
            input: KeyboardInput {
              state: ElementState::Pressed,
              virtual_keycode: Some(VirtualKeyCode::A), ..
            }, ..
          } => println!("Hello there"),

          WindowEvent::Resized(physical_size) =>
            gfx_state.resize(physical_size.width, physical_size.height),

          WindowEvent::ScaleFactorChanged {new_inner_size, ..} =>
            gfx_state.resize(new_inner_size.width, new_inner_size.height),

          _ => {},
        }

        Event::WindowEvent { .. } => {}
        Event::DeviceEvent { .. } => {}
        Event::UserEvent(_) => {}

        Event::Suspended => {}
        Event::Resumed => {}

        Event::MainEventsCleared => {
          match gfx_state.render() {
            Ok(_) => {},
            Err(SurfaceError::Lost) => gfx_state.resize(
              gfx_state.config.width,
              gfx_state.config.height),
            Err(SurfaceError::OutOfMemory) => control_flow.set_exit(),
            Err(e) => println!("{:?}", e),
          }
        }
        // Event::RedrawRequested(_) => {}
        // Event::RedrawEventsCleared => {}
        Event::LoopDestroyed => {}
        _ => {}
      };

    });
  }

  fn main_loop(&mut self) {
    // loop {
      let start = SystemTime::now();

    println!("hello!!!!");

      self.run_task();

      self.event_queue.run_all();
      self.event_queue.prune();

      Engine::end(start);
    // }
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