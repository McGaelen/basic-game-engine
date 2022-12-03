pub mod vulkano_tutorial {
  use image::{ImageBuffer, Rgba};
  use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
  use vulkano::command_buffer::{AutoCommandBufferBuilder, ClearColorImageInfo, CommandBufferUsage, CopyImageToBufferInfo, RenderPassBeginInfo, SubpassContents};
  use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
  use vulkano::device::{Device, DeviceCreateInfo, QueueCreateInfo};
  use vulkano::device::physical::PhysicalDevice;
  use vulkano::image::{ImageDimensions, StorageImage};
  use vulkano::instance::{Instance, InstanceCreateInfo};
  use vulkano::pipeline::{ComputePipeline, Pipeline, PipelineBindPoint};
  use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo};
  use vulkano::sync;
  use vulkano::sync::GpuFuture;
  use vulkano::format::Format;
  use vulkano::image::view::ImageView;
  use bytemuck::Zeroable;
  use bytemuck::Pod;

  pub fn run() {
    // =================================== Initialization =================================================
    // Create Vulkan instance
    let instance = Instance::new(InstanceCreateInfo::default()).expect("failed to create instance");

    // Get all physical devices in the system
    let physical = PhysicalDevice::enumerate(&instance).next().expect("no device available");

    // Find the first queue on the physical device that supports graphics
    let queue_family = physical.queue_families()
        .find(|&q| q.supports_graphics())
        .expect("couldn't find a graphical queue family");

    // Initialize the device by telling Vulkan what queue families we want to use on the device.
    let (device, mut queues) = Device::new(
      physical,
      DeviceCreateInfo {
        queue_create_infos: vec![QueueCreateInfo::family(queue_family)],
        ..Default::default()
      },
    ).expect("failed to create device");

    // Only use 1 queue for now
    let queue = queues.next().unwrap();

    // =================================== Compute =================================================
    // We are going to multiply 65536 values by 12.
    // Create the array of values
    let data_iter = 0..65536;
    // Put the array in a buffer
    let data_buffer = CpuAccessibleBuffer::from_iter(
      device.clone(), BufferUsage::all(), false, data_iter
    ).expect("failed to create buffer");

    // Write the shader that will do the multiplying
    mod cs {
      vulkano_shaders::shader! {
        ty: "compute",
        src: "
          #version 450

          layout(local_size_x = 64, local_size_y = 1, local_size_z = 1) in;

          layout(set = 0, binding = 0) buffer Data {
            uint data[];
          } buf;

          void main() {
            uint idx = gl_GlobalInvocationID.x;
            buf.data[idx] *= 12;
          }"
      }
    }

    // Load the shader into the Vulkan implementation
    let shader = cs::load(device.clone()).expect("failed to create shader module");

    // Create a compute pipeline with our shader
    let compute_pipeline = ComputePipeline::new(
      device.clone(), shader.entry_point("main").unwrap(), &(), None, |_| {}
    ).expect("failed to create compute pipeline");

    // Get the compute pipeline's first layout
    let layout = compute_pipeline.layout().set_layouts().get(0).unwrap();

    // Create a descriptor set which contains our data (buffer)
    let set = PersistentDescriptorSet::new(
      layout.clone(), [WriteDescriptorSet::buffer(0, data_buffer.clone())]
    ).unwrap();

    // Build a command that does the following:
    // - binds the descriptor set containing our buffer to our compute pipeline,
    // - dispatches the command split up into 1024 groups.
    let mut builder = AutoCommandBufferBuilder::primary(
      device.clone(), queue.family(), CommandBufferUsage::OneTimeSubmit,
    ).unwrap();
    builder
        .bind_pipeline_compute(compute_pipeline.clone())
        .bind_descriptor_sets(
          PipelineBindPoint::Compute,
          compute_pipeline.layout().clone(),
          0,
          set
        )
        .dispatch([1024, 1, 1])
        .unwrap();
    let command_buffer = builder.build().unwrap();

    // Execute the command on the queue
    let future = sync::now(device.clone())
        .then_execute(queue.clone(), command_buffer)
        .unwrap()
        .then_signal_fence_and_flush()
        .unwrap();

    // Wait for the command to complete
    future.wait(None).unwrap();

    // Check the result.
    let result = data_buffer.read().unwrap();
    for (n, val) in result.iter().enumerate() {
      assert_eq!(*val, n as u32 * 12);
    }

    // =================================== Images =================================================
    // We are going to create an Image, which is similar to a buffer in that it's a type of memory that you share with the GPU.
    // Unlike Buffers, the memory layout is implementation-specific (we can't read or modify it. No such thing as CpuAccessibleImage.)
    let image = StorageImage::new(
      device.clone(), 
      ImageDimensions::Dim2d {
        width: 1024,
        height: 1024,
        array_layers: 1,
      }, 
      Format::R8G8B8A8_UNORM, 
      Some(queue.family()),
    ).unwrap();

    // To read the Image, we have to create a Buffer and ask the GPU to copy the Image into it.
    let buf = CpuAccessibleBuffer::from_iter(
      device.clone(), BufferUsage::all(), false, (0..1024 * 1024 * 4).map(|_| 0u8 ),
    ).expect("failed to create buffer");

    // Since we can't modify the Image directly, we have to ask the GPU to do it.
    // We'll create a command that asks the GPU to:
    // - fill the image with a single color
    // - copy the image to the buffer so we can observe the results 
    let mut builder = AutoCommandBufferBuilder::primary(
      device.clone(), queue.family(), CommandBufferUsage::OneTimeSubmit
    ).unwrap();
    builder
        .clear_color_image(ClearColorImageInfo {
          clear_value: vulkano::format::ClearColorValue::Float([0.0, 0.0, 1.0, 1.0]),
          ..ClearColorImageInfo::image(image.clone())
        })
        .unwrap()
        .copy_image_to_buffer(CopyImageToBufferInfo::image_buffer(image.clone(), buf.clone()))
        .unwrap();
    let command_buffer = builder.build().unwrap();

    // Run the command
    let future = sync::now(device.clone())
        .then_execute(queue.clone(), command_buffer)
        .unwrap()
        .then_signal_fence_and_flush()
        .unwrap();
    future.wait(None).unwrap();

    // Read from the Buffer, which should have the image copied into it
    let buffer_content = buf.read().unwrap();
    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(1024, 1024, &buffer_content[..]).unwrap();

    image.save("image.png").unwrap();

    // Now let's create a more complex image, by using a shader to compute each pixel.
    // This shader will compute a Mandelbrot set and render it to an image.
    mod cs2 {
      vulkano_shaders::shader! {
        ty: "compute",
        src: "
          #version 450

          layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

          layout(set = 0, binding = 0, rgba8) uniform writeonly image2D img;

          void main() {
            vec2 norm_coordinates = (gl_GlobalInvocationID.xy + vec2(0.5)) / vec2(imageSize(img));
            vec2 c = (norm_coordinates - vec2(0.5)) * 2.0 - vec2(1.0, 0.0);

            vec2 z = vec2(0.0, 0.0);
            float i;
            for (i = 0.0; i < 1.0; i += 0.005) {
              z = vec2(
                  z.x * z.x - z.y * z.y + c.x,
                  z.y * z.x + z.x * z.y + c.y
              );

              if (length(z) > 4.0) {
                  break;
              }
            }

            vec4 to_write = vec4(vec3(i), 1.0);
            imageStore(img, ivec2(gl_GlobalInvocationID.xy), to_write);
          }"
      }
    }

    // Create new Image to be populated
    let image = StorageImage::new(
      device.clone(),
      ImageDimensions::Dim2d { width: 1024, height: 1024, array_layers: 1 },
      Format::R8G8B8A8_UNORM,
      Some(queue.family())
    ).unwrap();

    // Create a view for the image, so GPU knows how to load/use the image.
    let view = ImageView::new_default(image.clone()).unwrap();

    // Create the descriptor set for our Image to be passed into the shader. 
    let layout = compute_pipeline.layout().set_layouts().get(0).unwrap();
    let set = PersistentDescriptorSet::new(
      layout.clone(),
      [WriteDescriptorSet::image_view(0, view.clone())],
    ).unwrap();

    // Create a buffer to copy the contents of the Image into.
    let buf = CpuAccessibleBuffer::from_iter(
        device.clone(),
        BufferUsage::all(),
        false,
        (0..1024 * 1024 * 4).map(|_| 0u8),
    ).expect("failed to create buffer");

    let mut builder = AutoCommandBufferBuilder::primary(
        device.clone(),
        queue.family(),
        CommandBufferUsage::OneTimeSubmit,
    )
    .unwrap();
    builder
        .bind_pipeline_compute(compute_pipeline.clone())
        .bind_descriptor_sets(
            PipelineBindPoint::Compute,
            compute_pipeline.layout().clone(),
            0,
            set,
        )
        .dispatch([1024 / 8, 1024 / 8, 1])
        .unwrap()
        .copy_image_to_buffer(CopyImageToBufferInfo::image_buffer(
            image.clone(),
            buf.clone(),
        ))    
        .unwrap();

    let command_buffer = builder.build().unwrap();

    let future = sync::now(device.clone())
        .then_execute(queue.clone(), command_buffer)
        .unwrap()
        .then_signal_fence_and_flush()
        .unwrap();

    future.wait(None).unwrap();

    let buffer_content = buf.read().unwrap();
    let image_buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(1024, 1024, &buffer_content[..]).unwrap();
    image_buffer.save("image2.png").unwrap();


    // =================================== Graphics =================================================
    #[repr(C)]
    #[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
    struct Vertex {
      position: [f32; 2]
    }
    vulkano::impl_vertex!(Vertex, position);

    // Create 3 Vertices
    let v1 = Vertex { position: [-0.5, -0.5] };
    let v2 = Vertex { position: [0.0, 0.5] };
    let v3 = Vertex { position: [0.5, -0.25] };

    // Create a buffer containing the vertices to pass them to the GPU
    let vertex_buffer = CpuAccessibleBuffer::from_iter(
      device.clone(), BufferUsage::vertex_buffer(), false, vec![v1, v2, v3].into_iter()
    ).unwrap();

    // Create the shader that will run on each vertex.
    // This reads in each vertex's position field, then sets the global var gl_Position to the x-y coords of the vertex,
    // which sets the vertex's position.
    mod vertex_shader {
      vulkano_shaders::shader! {
        ty: "vertex",
        src: "
          #version 450

          layout(location = 0) in vec2 position;

          void main() {
            gl_Position = vec4(position, 0.0, 1.0);
          }
        "
      }
    }

    // Create the fragment shader that will run on each pixel that's inside the bounds of the triangle created by our vertices.
    // This sets the color value for the pixel to red.
    mod fragment_shader {
      vulkano_shaders::shader! {
        ty: "fragment",
        src: "
          #version 450

          layout(location = 0) out vec4 f_color;

          void main() {
            f_color = vec4(1.0, 0.0, 0.0, 1.0);
          }
        "
      }
    }

    // Create a render pass object, which describes the render pass we want to perform and declares attachments
    let render_pass = vulkano::single_pass_renderpass!(device.clone(),
      attachments: {
        color: {
          load: Clear,
          store: Store,
          format: vulkano::format::Format::R8G8B8A8_UNORM,
          samples: 1,
        }
      },
      pass: {
        color: [color],
        depth_stencil: {}
      }
    ).unwrap();
    
    let view = ImageView::new_default(image.clone()).unwrap();
    let framebuffer = Framebuffer::new(
      render_pass.clone(),
      FramebufferCreateInfo {
        attachments: vec![view],
        ..Default::default()
      },
    ).unwrap();

    let mut builder = AutoCommandBufferBuilder::primary(
      device.clone(),
      queue.family(),
      CommandBufferUsage::OneTimeSubmit,
    ).unwrap();

    // builder
    //     .begin_render_pass(
    //       RenderPassBeginInfo {
    //         framebuffer: framebuffer.clone(),
    //
    //       },
    //       SubpassContents::Inline,
    //       vec![[0.0, 0.0, 1.0, 1.0].into()],
    //     )
    //     .unwrap()
    //     .end_render_pass()
    //     .unwrap();
  }
}