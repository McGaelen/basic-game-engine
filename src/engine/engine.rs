use std::error::Error;
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime};
use crate::engine::event::Event;
use crate::engine::eventqueue::EventQueue;

const FRAME_DURATION: Duration = Duration::from_nanos(33_333_333);

pub type TaskFn = fn(engine: &mut Engine) -> Result<(), String>;

pub struct Engine {
  pub event_queue: Vec<Event>,
  task: TaskFn
}

impl Engine {
  pub fn run(task: TaskFn) {
    let mut engine = Engine {
      event_queue: Vec::new(),
      task
    };

    engine.main_loop();
  }

  fn main_loop(&mut self) {
    loop {
      let start = SystemTime::now();

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