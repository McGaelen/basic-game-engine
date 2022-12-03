mod game_engine;
mod vulkano_tutorial;

#[cfg(target_arch="wasm32")]
use wasm_bindgen::prelude::*;

use game_engine::Engine;
use crate::game_engine::MainLoopFn;

#[cfg_attr(target_arch="wasm32", wasm_bindgen(start))]
pub fn main() {
  Engine::run(|engine| {
    Ok(())
  });
}
