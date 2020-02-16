// TODO/rel: explain what image refers to in this case.

use super::device::Device;

use std::sync::Arc;

use vulkano::sync::GpuFuture;
use vulkano::format::{AcceptsPixels, Format, FormatDesc};

pub use vulkano::image::{Dimensions, ImmutableImage, ImageCreationError};
pub use vulkano::sampler::{Filter, Sampler, SamplerCreationError, SamplerAddressMode, MipmapMode};

/// Create an [ImmutableImage] from a data iterator.
/// 
/// Builds an intermediate memory-mapped buffer, writes data to it, builds a copy (upload) command buffer and executes it.
/// 
/// # Panic.
/// 
/// - Panics if fails to submit the copy command buffer.
pub fn create_immutable_image_from_iter<P, I, F>(device: &Device, data_iterator: I, dimensions: Dimensions, format: F)
-> Result<Arc<ImmutableImage<F>>, ImageCreationError>
where
	P : Send + Sync + Clone + 'static,
	F : FormatDesc + AcceptsPixels<P> + Send + Sync + 'static,
	I : ExactSizeIterator<Item = P>,
	Format: AcceptsPixels<P>,
{
	let (image, future) = ImmutableImage::from_iter(data_iterator, dimensions, format, device.transfer_queue.clone())?;

	// TODO: handle synchronization between separate queues in a performant way
	future.flush().unwrap();

	// let time: Box<dyn GpuFuture> = match self.before_frame.take() {
	// 	Some(time) => Box::new(time.join(future)),
	// 	None => Box::new(future),
	// };
	// self.before_frame = Some(time);

	Ok(image)
}
