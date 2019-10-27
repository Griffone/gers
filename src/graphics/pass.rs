use crate::window::Window;
use super::device::Device;
use super::ResizeError;

use vulkano::command_buffer::DynamicState;
use vulkano::framebuffer::{FramebufferAbstract, RenderPassAbstract, RenderPassCreationError, Subpass};
use vulkano::pipeline::{GraphicsPipeline, GraphicsPipelineAbstract, GraphicsPipelineCreationError};
use vulkano::pipeline::shader::{GraphicsEntryPointAbstract};

use std::sync::Arc;

// A GraphicalPass produces visible images as its result.
pub trait GraphicalPass {
	type Pipeline: ?Sized + GraphicsPipelineAbstract + Send + Sync + 'static;
	type Framebuffer: ?Sized + FramebufferAbstract + Send + Sync + 'static;

	// Get dynamic state of the GraphicalPass
	fn dynamic_state(&self) -> &DynamicState;
	// Get the underlying pipeline of the GraphicalPass
	fn pipeline(&self) -> Arc<Self::Pipeline>;
	// TODO: consider switching to a slice instead
	// Get the resulting framebuffers of the GraphicalPass
	fn framebuffers(&self) -> Vec<Arc<Self::Framebuffer>>;
}

pub struct AlbedoPass {
	render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
	graphics_pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
	framebuffers: Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,

	dynamic_state: DynamicState,
}

impl GraphicalPass for AlbedoPass {
	type Pipeline = dyn GraphicsPipelineAbstract + Send + Sync + 'static;
	type Framebuffer = dyn FramebufferAbstract + Send + Sync + 'static;

	#[inline(always)]
	fn dynamic_state(&self) -> &DynamicState { &self.dynamic_state }
	#[inline(always)]
	fn pipeline(&self) -> Arc<Self::Pipeline> { self.graphics_pipeline.clone() }
	#[inline(always)]
	fn framebuffers(&self) -> Vec<Arc<Self::Framebuffer>> { self.framebuffers.clone() }
}

#[derive(Debug)]
pub enum PassCreationError {
	RenderPass(RenderPassCreationError), // Error during creation of the underlying vulkan render-pass
	GraphicsPipeline(GraphicsPipelineCreationError), // Error during creation of the underlying vulkan graphics-pipeline
	DynamicState(ResizeError), // Error during initial resizing
}

impl AlbedoPass {
	pub fn new<VS, FS, T>(
		device: &Device,
		window: &Arc<Window>,
		vertex_shader: VS,
		vssc: VS::SpecializationConstants,
		fragment_shader: FS,
		fssc: FS::SpecializationConstants
	) -> Result<AlbedoPass, PassCreationError>
	where
		VS : GraphicsEntryPointAbstract,
		FS : GraphicsEntryPointAbstract,
		VS::PipelineLayout : Send + Sync + Clone + 'static,
		FS::PipelineLayout : Send + Sync + Clone + 'static,
		T : Send + Sync + 'static,
		vulkano::pipeline::vertex::SingleBufferDefinition<T> : vulkano::pipeline::vertex::VertexDefinition<VS::InputDefinition>
	{
		let render_pass = Arc::new(vulkano::single_pass_renderpass!(
			device.device.clone(),
			attachments: {
				color: {
					load: Clear,
					store: Store,
					format: device.swapchain.format(),
					samples: 1,
				}
			},
			pass: {
				color: [color],
				depth_stencil: {}
			})?);

		let graphics_pipeline = Arc::new(GraphicsPipeline::start()
			.vertex_input_single_buffer::<T>()
			.vertex_shader(vertex_shader, vssc)
			.triangle_list()
			.cull_mode_back()
			.viewports_dynamic_scissors_irrelevant(1)
			.fragment_shader(fragment_shader, fssc)
			.render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
			.build(device.device.clone())?);
		
		let mut pass = AlbedoPass {
			graphics_pipeline,
			render_pass,
			framebuffers: Vec::new(),
			dynamic_state: DynamicState::default(),
		};
		pass.resize_for_window(device, window)?;
		Ok(pass)
	}

	pub fn resize_for_window(&mut self, device: &Device, window: &Arc<Window>) -> Result<(), ResizeError> {
		let dimensions: (f32, f32) = match window.get_inner_size() {
			Some(size) => {
				let size: (u32, u32) = size.into();
				(size.0 as f32, size.1 as f32)
			},
			None => return Err(ResizeError::UnsizedWindow)
		};
		
		let viewport = vulkano::pipeline::viewport::Viewport {
			origin: [0.0, dimensions.1],
			dimensions: [dimensions.0, -dimensions.1],
			depth_range: 0.0 .. 1.0,
		};

		self.dynamic_state.viewports = Some(vec!(viewport));

		self.framebuffers = device.swapchain_images.iter().map(|image| {
			Arc::new(
				vulkano::framebuffer::Framebuffer::start(self.render_pass.clone())
					.add(image.clone()).unwrap()
					.build().unwrap()
			) as Arc<dyn FramebufferAbstract + Send + Sync>
		}).collect::<Vec<_>>();
		Ok(())
	}
}

impl From<RenderPassCreationError> for PassCreationError {
	fn from(err: RenderPassCreationError) -> PassCreationError { PassCreationError::RenderPass(err) }
}
impl From<GraphicsPipelineCreationError> for PassCreationError {
	fn from(err: GraphicsPipelineCreationError) -> PassCreationError { PassCreationError::GraphicsPipeline(err) }
}
impl From<ResizeError> for PassCreationError {
	fn from(err: ResizeError) -> PassCreationError { PassCreationError::DynamicState(err) }
}