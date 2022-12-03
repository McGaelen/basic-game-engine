#[derive(Debug)]
pub struct GameEvent {
  pub name: String,
  pub frames: u32,
  pub task: fn(),
}

impl GameEvent {
  pub fn dec(&mut self) {
    self.frames -= 1
  }
}