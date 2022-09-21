mod engine;

use crate::engine::engine::Engine;
use crate::engine::event::Event;

fn main() {
  Engine::run(|engine| {
    println!("hi there, {:?}", engine.event_queue);



    // engine.event_queue.push(Event {
    //   task: || {
    //     println!("doing the thingy")
    //   },
    //   name: "event".to_string(),
    //   frames: 10
    // });
    Ok(())
  });
}
