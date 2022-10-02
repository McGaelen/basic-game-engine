use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use sdl2::{EventPump, Sdl, VideoSubsystem};
use sdl2::video::{Window, WindowContext};
use vulkano::swapchain::Surface;


pub struct GameWindow {
  pub sdl_context: Sdl,
  pub event_pump: EventPump,
  pub video_subsystem: VideoSubsystem,
  pub window: Window,
  pub surface: Surface<Rc<WindowContext>>,
}

impl Debug for GameWindow {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_str("")
  }
}
