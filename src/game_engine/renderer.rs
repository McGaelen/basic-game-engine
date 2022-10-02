use std::sync::Arc;
use vulkano::device::{Device, Queue};


#[derive(Debug)]
pub struct Renderer {
  pub device: Arc<Device>,
  pub gfx_queue: Arc<Queue>,
  pub transfer_queue: Arc<Queue>,
}

impl Renderer {
  /*
    Probably some Vulkan helper functions will go here
   */
}

// for family in physical_device.queue_families() {
//   println!("Found a queue family with {:?} queue(s)", family.queues_count());
//   println!("Graphics: {:?}", family.supports_graphics());
//   println!("Compute: {:?}", family.supports_compute());
//   println!("Transfers: {:?}", family.explicitly_supports_transfers());
//   println!("-----------------------------")
// }
