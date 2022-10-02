use crate::game_engine::event::Event;

pub trait EventQueue {
  fn remove(&mut self, name: String);
  fn run_all(&mut self);
  fn prune(&mut self);
}

impl EventQueue for Vec<Event> {
  fn remove(&mut self, name: String) {
    match self.iter().position(|event| event.name == name) {
      None => {},
      Some(i) => {self.swap_remove(i);}
    }
  }

  fn run_all(&mut self) {
    self.iter_mut().for_each(|event| {
      (event.task)();
      event.dec();
    });
  }

  fn prune(&mut self) {
    for n in 0..self.len() {
      match self.get(n) {
        None => (),
        Some(event) => if event.frames == 0 {
          self.remove(n);
        }
      }
    }
  }
}
