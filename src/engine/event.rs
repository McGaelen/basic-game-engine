#[derive(Debug)]
pub struct Event {
  pub name: String,
  pub frames: u32,
  pub task: fn(),
}

impl Event {
  pub fn dec(&mut self) {
    self.frames -= 1
  }
}