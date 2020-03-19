extern crate gaclen;

mod shaders;
mod geometry;

use gaclen::graphics;

use winit::event_loop::{ControlFlow, EventLoop};
use winit::event::{Event, WindowEvent};
use winit::window::WindowBuilder;

#[derive(Default, Debug, Clone)]
struct Vertex {
	position: [f32; 3],
	color: [f32; 4],
}
vulkano::impl_vertex!(Vertex, position, color);

fn main() {
	let mut frame_count: u64 = 0;
	let start_time = std::time::Instant::now();

	let event_loop = EventLoop::new();
	let window = std::sync::Arc::new(
		WindowBuilder::new()
			.with_title("Shadowing example")
			.with_inner_size(winit::dpi::PhysicalSize::new(1280, 720))
			.with_min_inner_size(winit::dpi::PhysicalSize::new(1280, 720))
			.build(&event_loop).unwrap()
	);
	
	let context = graphics::context::Context::new().unwrap();
	let device = graphics::device::Device::new(&context).unwrap();
	println!("Initialized device: {:?}", device);
	let mut swapchain = graphics::swapchain::Swapchain::new(&context, &device, window.clone(), graphics::swapchain::PresentMode::Immediate, graphics::PixelFormat::D16Unorm).expect("Failed to create swapchain!");

	let albedo_pass = {
		let vs = shaders::albedo::vertex::Shader::load(&device).unwrap();
		let fs = shaders::albedo::fragment::Shader::load(&device).unwrap();

		graphics::pass::GraphicalPass::start()
			.single_buffer_input::<Vertex>()
			.vertex_shader(vs.main_entry_point(), ())
			.fragment_shader(fs.main_entry_point(), ())
			.add_image_attachment_swapchain_cleared(&swapchain)
			.add_depth_attachment_swapchain_discard(&swapchain, graphics::pass::LoadOp::Clear).unwrap()
			.build(&device).unwrap()
	};

	let quad = geometry::generate_quad(&device);
	let cube = geometry::generate_quad(&device);

	let mut recreate_swapchain = false;

	// Wrap the device in a stack-allocated container to allow for temporary ownership.
	let mut device = Some(device);

	event_loop.run(move |event, _, control_flow| {
		match event {
			Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
				*control_flow = ControlFlow::Exit;
				let run_duration = start_time.elapsed().as_secs_f64();
				let fps: f64 = frame_count as f64 / run_duration;
				println!("Produced {} frames over {:.2} seconds ({:.2} avg fps)", frame_count, run_duration, fps);
			},
			Event::WindowEvent { event: WindowEvent::Resized(_), .. } => recreate_swapchain = true,
			Event::RedrawEventsCleared => {
				if recreate_swapchain {
					let dimensions = window.inner_size();
		
					// Sometimes the swapchain fails to create :(
					match swapchain.resize(dimensions.into()) {
						Ok(()) => (),
						Err(graphics::ResizeError::Swapchain(_)) => {
							println!("Failed to resize window, skipping frame!");
							return;
						},
						Err(err) => panic!(err),
					};
					recreate_swapchain = false;
				}
		
				let clear_color = [0.0, 0.0, 0.0, 1.0];
		
				let frame = graphics::frame::Frame::begin(device.take().unwrap(), &swapchain).unwrap();
		
				let framebuffer = std::sync::Arc::new(albedo_pass.start_framebuffer()
					.add(swapchain.get_color_image_for(&frame)).unwrap()
					.add(swapchain.get_depth_image_for(&frame)).unwrap()
					.build().unwrap()
				);
		
				let after_frame = frame.begin_pass(&albedo_pass, framebuffer, vec![clear_color.into(), 1f32.into()])
					.draw(vec![quad.clone()], (), ())
					.draw(vec![cube.clone()], (), ())
					.finish_pass()
				.finish();
				
				device = match after_frame {
					Ok(device) => Some(device),
					Err((device, err)) => {
						if err == graphics::frame::FrameFinishError::Flush(vulkano::sync::FlushError::OutOfDate) { recreate_swapchain = true; };
						Some(device)
					},
				};
		
				frame_count += 1;
			},
			_ => ()
		};
	});
}
