// rok-renderer/src/lib.rs
//
// Public API for the rok renderer.
//
// The Engine creates a Renderer at startup and owns it for the engine's
// lifetime. The Renderer manages all Vulkan state: instance, device,
// swapchain, command buffers, pipelines, and resource pools.
//
// The Target never touches this crate directly — it submits draw calls
// through EngineApi handles, and the Engine translates them into renderer
// operations.

use std::num::NonZeroU32;

use ash::vk;
use rok_math::vec2::Vec2;

use crate::backend;
use crate::backend::descriptor::TextureDescriptor;
use crate::backend::light::LightUbo;
use crate::backend::texture::Texture;
use crate::error::{RendererError, RendererResult};
use backend::device::VulkanDevice;
use backend::frame::FrameSync;
use backend::image::DepthImage;
use backend::instance::VulkanInstance;
use backend::physical_device;
use backend::surface::VulkanSurface;
use backend::swapchain::VulkanSwapchain;
use rok_abi::surface::{NativeSurfaceHandle, SurfaceType};
use rok_log::{log_info, log_warn};

use backend::pipeline::{GraphicsPipeline, PipelineDesc};

use backend::buffer::{self, Buffer};
use backend::vertex::Vertex;
use rok_math::mat4x4::Mat4x4;
use rok_math::vec3::Vec3;

pub use crate::command::RenderCommand;

const QUAD_VERT_SPV: &[u8] = include_bytes!("../shaders/quad.vert.spv");
const QUAD_FRAG_SPV: &[u8] = include_bytes!("../shaders/quad.frag.spv");

const CAT_PNG: &[u8] = include_bytes!("../textures/grouchy_cat.png");

const DEPTH_FORMAT: vk::Format = vk::Format::D32_SFLOAT; // universal for depth attachment; a
// production path would query support

// ---------------------------------------------------------------------------
// RendererConfig
// ---------------------------------------------------------------------------

/// Configuration for renderer creation. The Engine decides these values
/// and they are fixed for the renderer's lifetime.
pub struct RendererConfig {
    /// Application name shown in debug tools (RenderDoc, validation layers).
    pub app_name: String,

    /// Number of frames that can be in flight simultaneously.
    /// Typically 2 (double buffering) or 3 (triple buffering).
    /// The Engine decides this at startup; it cannot change at runtime.
    ///
    /// When presenting, this is clamped to the swapchain image count to
    /// avoid binary semaphore reuse hazards.
    pub frames_in_flight: NonZeroU32,

    /// The native surface to present to (if presentation is needed).
    /// None for headless / compute-only mode.
    pub surface: Option<NativeSurfaceHandle>,

    /// If true, use FIFO (vsync). If false, use IMMEDIATE (no vsync).
    pub vsync: bool,
}

// ---------------------------------------------------------------------------
// Renderer
// ---------------------------------------------------------------------------

/// The renderer. Owns all Vulkan state.
///
/// Drop order matters — Rust drops fields in declaration order.
/// Frame sync and swapchain must be destroyed before device;
/// device before surface; surface before instance.
pub struct Renderer {
    // Device-bound resources
    // Must be declared before the device.
    frame_sync: Option<FrameSync>,
    swapchain: Option<VulkanSwapchain>,
    pipeline: Option<GraphicsPipeline>,
    vertex_buffer: Option<Buffer>,
    index_buffer: Option<Buffer>,
    depth: Option<DepthImage>,
    texture: Option<Texture>,
    descriptor: Option<TextureDescriptor>,
    light_buffer: Option<Buffer>,

    // Device
    device: VulkanDevice,
    physical_device: vk::PhysicalDevice,
    surface: Option<VulkanSurface>,
    instance: VulkanInstance,
    // Cached state
    vsync: bool,
    needs_resize: bool,
    current_extent: vk::Extent2D,
    mem_props: vk::PhysicalDeviceMemoryProperties,
}

impl Renderer {
    /// Create and initialise the renderer.
    pub fn new(config: &RendererConfig) -> RendererResult<Self> {
        // --- Instance ---
        let required_extensions = surface_extensions(config.surface.as_ref())?;
        let ext_refs: Vec<&std::ffi::CStr> = required_extensions.iter().copied().collect();

        let instance = VulkanInstance::new(&config.app_name, &ext_refs)?;
        log_info!("rok-renderer: Vulkan instance created");

        // --- Surface (optional) ---
        let surface = match &config.surface {
            Some(handle) => Some(VulkanSurface::new(
                instance.entry(),
                instance.handle(),
                handle,
            )?),
            None => None,
        };

        if surface.is_some() {
            log_info!("rok-renderer: surface created");
        }

        // --- Physical device selection ---
        let selection = physical_device::select_physical_device(
            instance.handle(),
            surface.as_ref().map(|s| s.loader()),
            surface.as_ref().map(|s| s.handle()),
        )?;

        let phys_device = selection.device;

        // --- Logical device ---
        let device = VulkanDevice::new(instance.handle(), &selection)?;

        // --- Swapchain ---
        let initial_extent = config
            .surface
            .as_ref()
            .map(|s| vk::Extent2D {
                width: s.width,
                height: s.height,
            })
            .unwrap_or(vk::Extent2D {
                width: 0,
                height: 0,
            });

        let swapchain = if let Some(ref surf) = surface {
            Some(VulkanSwapchain::new(
                &device,
                surf.loader(),
                phys_device,
                surf.handle(),
                &selection.queue_families,
                initial_extent,
                None,
                config.vsync,
            )?)
        } else {
            None
        };

        // --- Frame sync ---
        // Match frames_in_flight to swapchain image count when presenting.
        // This guarantees binary semaphores (required by acquire/present) are
        // never reused while the presentation engine still holds them.
        let frame_count = match &swapchain {
            Some(sc) => sc.images.len() as u32,
            None => config.frames_in_flight.get(),
        };

        let frame_sync = if swapchain.is_some() {
            Some(FrameSync::new(
                device.handle(),
                selection.queue_families.graphics,
                frame_count,
            )?)
        } else {
            None
        };

        let mem_props = unsafe {
            instance
                .handle()
                .get_physical_device_memory_properties(phys_device)
        };

        let depth = match &swapchain {
            Some(sc) => Some(DepthImage::new(
                device.handle(),
                &mem_props,
                sc.config.extent,
                DEPTH_FORMAT,
            )?),
            None => None,
        };

        let texture = match &swapchain {
            Some(_) => {
                let decoder = png::Decoder::new(std::io::Cursor::new(CAT_PNG));
                let mut reader = decoder
                    .read_info()
                    .map_err(|_| RendererError::Config("png read_info failed"))?;
                let mut buf = vec![0u8; reader.output_buffer_size()];
                let frame = reader
                    .next_frame(&mut buf)
                    .map_err(|_| RendererError::Config("png decode failed"))?;
                // Ensure RGBA8. If your cat exported as RGB, the crate can be told
                // to expand — simplest is to export/verify RGBA. buf[..frame.buffer_size()]
                // holds width*height*4 bytes when RGBA8.
                let pixels = &buf[..frame.buffer_size()];
                Some(Texture::from_rgba8(
                    device.handle(),
                    &mem_props,
                    device.queues.graphics,
                    device.queue_families.graphics,
                    frame.width,
                    frame.height,
                    pixels,
                )?)
            }
            None => None,
        };

        // TODO: remove later
        let light = LightUbo {
            direction: {
                // normalize (-0.5, -1.0, -0.3): down, slightly left and forward
                let (x, y, z) = (-0.5f32, -1.0, -0.3);
                let len = (x * x + y * y + z * z).sqrt();
                [x / len, y / len, z / len]
            },
            _pad0: 0.0,
            color: [1.0, 1.0, 1.0],
            _pad1: 0.0,
        };

        let light_buffer = match &swapchain {
            Some(_) => Some(buffer::upload_via_staging(
                device.handle(),
                &mem_props,
                device.queues.graphics,
                device.queue_families.graphics,
                std::slice::from_ref(&light),
                vk::BufferUsageFlags::UNIFORM_BUFFER,
            )?),
            None => None,
        };

        let descriptor = match (&texture, &light_buffer) {
            (Some(tex), Some(lb)) => Some(TextureDescriptor::new(
                device.handle(),
                tex.view,
                tex.sampler,
                lb.buffer,
            )?),
            _ => None,
        };

        // Unit cube centered at origin; color from corner position.
        let h = 0.5;
        let vertices = [
            // +Z front, n = (0,0,1)
            Vertex {
                position: Vec3::new(-h, -h, h),
                uv: Vec2::new(0.0, 1.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            Vertex {
                position: Vec3::new(h, -h, h),
                uv: Vec2::new(1.0, 1.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            Vertex {
                position: Vec3::new(h, h, h),
                uv: Vec2::new(1.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            Vertex {
                position: Vec3::new(-h, h, h),
                uv: Vec2::new(0.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            // -Z back, n = (0,0,-1)
            Vertex {
                position: Vec3::new(h, -h, -h),
                uv: Vec2::new(0.0, 1.0),
                normal: Vec3::new(0.0, 0.0, -1.0),
            },
            Vertex {
                position: Vec3::new(-h, -h, -h),
                uv: Vec2::new(1.0, 1.0),
                normal: Vec3::new(0.0, 0.0, -1.0),
            },
            Vertex {
                position: Vec3::new(-h, h, -h),
                uv: Vec2::new(1.0, 0.0),
                normal: Vec3::new(0.0, 0.0, -1.0),
            },
            Vertex {
                position: Vec3::new(h, h, -h),
                uv: Vec2::new(0.0, 0.0),
                normal: Vec3::new(0.0, 0.0, -1.0),
            },
            // +X right, n = (1,0,0)
            Vertex {
                position: Vec3::new(h, -h, h),
                uv: Vec2::new(0.0, 1.0),
                normal: Vec3::new(1.0, 0.0, 0.0),
            },
            Vertex {
                position: Vec3::new(h, -h, -h),
                uv: Vec2::new(1.0, 1.0),
                normal: Vec3::new(1.0, 0.0, 0.0),
            },
            Vertex {
                position: Vec3::new(h, h, -h),
                uv: Vec2::new(1.0, 0.0),
                normal: Vec3::new(1.0, 0.0, 0.0),
            },
            Vertex {
                position: Vec3::new(h, h, h),
                uv: Vec2::new(0.0, 0.0),
                normal: Vec3::new(1.0, 0.0, 0.0),
            },
            // -X left, n = (-1,0,0)
            Vertex {
                position: Vec3::new(-h, -h, -h),
                uv: Vec2::new(0.0, 1.0),
                normal: Vec3::new(-1.0, 0.0, 0.0),
            },
            Vertex {
                position: Vec3::new(-h, -h, h),
                uv: Vec2::new(1.0, 1.0),
                normal: Vec3::new(-1.0, 0.0, 0.0),
            },
            Vertex {
                position: Vec3::new(-h, h, h),
                uv: Vec2::new(1.0, 0.0),
                normal: Vec3::new(-1.0, 0.0, 0.0),
            },
            Vertex {
                position: Vec3::new(-h, h, -h),
                uv: Vec2::new(0.0, 0.0),
                normal: Vec3::new(-1.0, 0.0, 0.0),
            },
            // +Y top, n = (0,1,0)
            Vertex {
                position: Vec3::new(-h, h, h),
                uv: Vec2::new(0.0, 1.0),
                normal: Vec3::new(0.0, 1.0, 0.0),
            },
            Vertex {
                position: Vec3::new(h, h, h),
                uv: Vec2::new(1.0, 1.0),
                normal: Vec3::new(0.0, 1.0, 0.0),
            },
            Vertex {
                position: Vec3::new(h, h, -h),
                uv: Vec2::new(1.0, 0.0),
                normal: Vec3::new(0.0, 1.0, 0.0),
            },
            Vertex {
                position: Vec3::new(-h, h, -h),
                uv: Vec2::new(0.0, 0.0),
                normal: Vec3::new(0.0, 1.0, 0.0),
            },
            // -Y bottom, n = (0,-1,0)
            Vertex {
                position: Vec3::new(-h, -h, -h),
                uv: Vec2::new(0.0, 1.0),
                normal: Vec3::new(0.0, -1.0, 0.0),
            },
            Vertex {
                position: Vec3::new(h, -h, -h),
                uv: Vec2::new(1.0, 1.0),
                normal: Vec3::new(0.0, -1.0, 0.0),
            },
            Vertex {
                position: Vec3::new(h, -h, h),
                uv: Vec2::new(1.0, 0.0),
                normal: Vec3::new(0.0, -1.0, 0.0),
            },
            Vertex {
                position: Vec3::new(-h, -h, h),
                uv: Vec2::new(0.0, 0.0),
                normal: Vec3::new(0.0, -1.0, 0.0),
            },
        ];

        let indices: [u16; 36] = [
            0, 1, 2, 2, 3, 0, // +Z
            4, 5, 6, 6, 7, 4, // -Z
            8, 9, 10, 10, 11, 8, // +X
            12, 13, 14, 14, 15, 12, // -X
            16, 17, 18, 18, 19, 16, // +Y
            20, 21, 22, 22, 23, 20, // -Y
        ];

        let (vertex_buffer, index_buffer) = match &swapchain {
            Some(_) => {
                let vb = buffer::upload_via_staging(
                    device.handle(),
                    &mem_props,
                    device.queues.graphics,
                    device.queue_families.graphics,
                    &vertices,
                    vk::BufferUsageFlags::VERTEX_BUFFER,
                )?;
                let ib = buffer::upload_via_staging(
                    device.handle(),
                    &mem_props,
                    device.queues.graphics,
                    device.queue_families.graphics,
                    &indices,
                    vk::BufferUsageFlags::INDEX_BUFFER,
                )?;
                (Some(vb), Some(ib))
            }
            None => (None, None),
        };

        let push_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(std::mem::size_of::<[f32; 32]>() as u32);

        let set_layouts = descriptor.as_ref().map(|d| d.layout);

        let pipeline = match &swapchain {
            Some(sc) => {
                let binding = Vertex::binding();
                let attributes = Vertex::attributes();
                let desc = PipelineDesc {
                    vertex_spv: QUAD_VERT_SPV,
                    fragment_spv: QUAD_FRAG_SPV,
                    color_format: sc.config.format.format,
                    topology: vk::PrimitiveTopology::TRIANGLE_LIST,
                    polygon_mode: vk::PolygonMode::FILL,
                    cull_mode: vk::CullModeFlags::NONE, // permissive to start
                    front_face: vk::FrontFace::COUNTER_CLOCKWISE,
                    vertex_bindings: std::slice::from_ref(&binding),
                    vertex_attributes: &attributes,
                    push_constant_ranges: std::slice::from_ref(&push_range),
                    depth_format: Some(DEPTH_FORMAT),
                    set_layouts: set_layouts
                        .as_ref()
                        .map(std::slice::from_ref)
                        .unwrap_or(&[]),
                };
                Some(GraphicsPipeline::create(device.handle(), &desc)?)
            }
            None => None,
        };

        log_info!("rok-renderer: initialization complete");

        Ok(Self {
            frame_sync,
            swapchain,
            pipeline,
            vertex_buffer,
            index_buffer,
            texture,
            descriptor,
            light_buffer,
            depth,
            device,
            physical_device: phys_device,
            surface,
            instance,
            vsync: config.vsync,
            needs_resize: false,
            current_extent: initial_extent,
            mem_props,
        })
    }

    /// Notify the renderer that the surface has changed size.
    /// The actual swapchain recreation is deferred to the next render() call.
    pub fn on_resize(&mut self, width: u32, height: u32) {
        self.current_extent = vk::Extent2D { width, height };
        self.needs_resize = true;
    }

    /// Change vsync mode. Takes effect on the next frame (triggers swapchain recreation).
    pub fn set_vsync(&mut self, vsync: bool) {
        if vsync != self.vsync {
            self.vsync = vsync;
            self.needs_resize = true;
        }
    }

    /// Render a frame. Currently just clears to a color.
    ///
    /// Returns `true` if a frame was presented, `false` if skipped
    /// (e.g. window is minimized or no swapchain).
    pub fn render(&mut self, view: Mat4x4, commands: &[RenderCommand]) -> bool {
        // No swapchain = headless / compute-only.
        if self.swapchain.is_none() || self.frame_sync.is_none() || self.surface.is_none() {
            return false;
        }

        // Skip rendering if minimized (zero extent).
        if self.current_extent.width == 0 || self.current_extent.height == 0 {
            return false;
        }

        // Handle pending resize.
        if self.needs_resize {
            self.needs_resize = false;
            self.device.wait_idle();
            if let Err(e) = self.recreate_swapchain() {
                log_warn!("Swapchain recreation failed: {}", e);
                return false;
            }
        }

        // Safety: we checked all Options are Some above.
        unsafe { self.render_frame(view, commands) }
    }

    /// Core render loop. Assumes swapchain, frame_sync, and surface are Some.
    ///
    /// # Safety
    /// Caller must guarantee swapchain, frame_sync, surface, depth are all Some.
    unsafe fn render_frame(&mut self, view: Mat4x4, commands: &[RenderCommand]) -> bool {
        let swapchain = unsafe { self.swapchain.as_mut().unwrap_unchecked() };
        let frame_sync = unsafe { self.frame_sync.as_mut().unwrap_unchecked() };
        let pipeline = self.pipeline.as_ref();
        let device = self.device.handle();
        let depth = unsafe { self.depth.as_ref().unwrap_unchecked() };
        let vertex_buffer = self.vertex_buffer.as_ref();
        let index_buffer = self.index_buffer.as_ref();

        // Wait for this frame slot's previous work via timeline semaphore
        frame_sync.wait_for_current_frame(device);

        let frame = frame_sync.current();
        let image_available = frame.image_available;
        let render_finished = frame.render_finished;

        // Acquire next swapchain image
        let image_index = unsafe {
            match self.device.swapchain_loader.acquire_next_image(
                swapchain.swapchain,
                u64::MAX,
                image_available,
                vk::Fence::null(),
            ) {
                Ok((index, false)) => index,
                Ok((_, true)) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    self.needs_resize = true;
                    return false;
                }
                Err(e) => {
                    log_warn!("acquire_next_image failed: {:?}", e);
                    return false;
                }
            }
        };

        // Record command buffer
        let cmd = frame.command_buffer;

        unsafe {
            let _ = device.reset_command_buffer(cmd, vk::CommandBufferResetFlags::empty());

            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

            let _ = device.begin_command_buffer(cmd, &begin_info);

            let depth_barrier = vk::ImageMemoryBarrier2::default()
                .src_stage_mask(vk::PipelineStageFlags2::TOP_OF_PIPE)
                .src_access_mask(vk::AccessFlags2::NONE)
                .dst_stage_mask(
                    vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS
                        | vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS,
                )
                .dst_access_mask(vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE)
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL)
                .image(depth.image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::DEPTH,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });

            // Transition image: UNDEFINED → COLOR_ATTACHMENT_OPTIMAL
            let image_barrier_to_render = vk::ImageMemoryBarrier2::default()
                .src_stage_mask(vk::PipelineStageFlags2::TOP_OF_PIPE)
                .src_access_mask(vk::AccessFlags2::NONE)
                .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                .dst_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .image(swapchain.images[image_index as usize])
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });

            let barriers_to_render = [image_barrier_to_render, depth_barrier];

            let dep_info = vk::DependencyInfo::default().image_memory_barriers(&barriers_to_render);

            device.cmd_pipeline_barrier2(cmd, &dep_info);

            // Dynamic rendering (Vulkan 1.3)
            let clear_value = vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.02, 0.02, 0.03, 1.0], // near-black
                },
            };

            let depth_clear = vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 0.0,
                    stencil: 0,
                },
            };

            let color_attachment = vk::RenderingAttachmentInfo::default()
                .image_view(swapchain.image_views[image_index as usize])
                .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .clear_value(clear_value);

            let depth_attachment = vk::RenderingAttachmentInfo::default()
                .image_view(depth.view)
                .image_layout(vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::DONT_CARE) // depth isn't read after the frame
                .clear_value(depth_clear);

            let rendering_info = vk::RenderingInfo::default()
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: swapchain.config.extent,
                })
                .layer_count(1)
                .color_attachments(std::slice::from_ref(&color_attachment))
                .depth_attachment(&depth_attachment);

            device.cmd_begin_rendering(cmd, &rendering_info);

            if let (Some(gp), Some(vb), Some(ib)) = (pipeline, vertex_buffer, index_buffer) {
                let extent = swapchain.config.extent;
                let aspect = extent.width as f32 / extent.height as f32;

                let proj = Mat4x4::perspective(60f32.to_radians(), aspect, 0.1, 100.0);
                let viewport = vk::Viewport {
                    x: 0.0,
                    y: extent.height as f32,
                    width: extent.width as f32,
                    height: -(extent.height as f32),
                    min_depth: 0.0,
                    max_depth: 1.0,
                };
                let scissor = vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent,
                };

                device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, gp.pipeline);

                if let Some(desc) = self.descriptor.as_ref() {
                    device.cmd_bind_descriptor_sets(
                        cmd,
                        vk::PipelineBindPoint::GRAPHICS,
                        gp.layout,
                        0, // first set
                        std::slice::from_ref(&desc.set),
                        &[], // no dynamic offsets
                    );
                }

                device.cmd_set_viewport(cmd, 0, std::slice::from_ref(&viewport));
                device.cmd_set_scissor(cmd, 0, std::slice::from_ref(&scissor));
                device.cmd_bind_vertex_buffers(cmd, 0, std::slice::from_ref(&vb.buffer), &[0]);
                device.cmd_bind_index_buffer(cmd, ib.buffer, 0, vk::IndexType::UINT16);
                device.cmd_draw_indexed(cmd, 36, 1, 0, 0, 0);

                // Drain the list.
                for &command in commands {
                    match command {
                        RenderCommand::DrawMesh { model } => {
                            let mvp = proj * view * model;
                            let mut push = [0f32; 32];
                            push[..16].copy_from_slice(&mvp.to_cols_array());
                            push[16..].copy_from_slice(&model.to_cols_array());
                            let push_bytes = std::slice::from_raw_parts(
                                push.as_ptr() as *const u8,
                                std::mem::size_of::<[f32; 32]>(),
                            );
                            device.cmd_push_constants(
                                cmd,
                                gp.layout,
                                vk::ShaderStageFlags::VERTEX,
                                0,
                                push_bytes,
                            );
                            device.cmd_draw_indexed(cmd, 36, 1, 0, 0, 0);
                        }
                    }
                }
            }

            device.cmd_end_rendering(cmd);

            // Transition image: COLOR_ATTACHMENT_OPTIMAL > PRESENT_SRC_KHR
            let image_barrier_to_present = vk::ImageMemoryBarrier2::default()
                .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                .src_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
                .dst_stage_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE)
                .dst_access_mask(vk::AccessFlags2::NONE)
                .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .image(swapchain.images[image_index as usize])
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });

            let dep_info = vk::DependencyInfo::default()
                .image_memory_barriers(std::slice::from_ref(&image_barrier_to_present));

            device.cmd_pipeline_barrier2(cmd, &dep_info);

            let _ = device.end_command_buffer(cmd);
        }

        // Submit via vkQueueSubmit2 (synchronization2)
        let signal_timeline_value = frame_sync.next_timeline_value();

        let wait_infos = [vk::SemaphoreSubmitInfo::default()
            .semaphore(image_available)
            .stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
            .value(0)]; // value 0 = binary semaphore

        let signal_infos = [
            // Binary semaphore for present
            vk::SemaphoreSubmitInfo::default()
                .semaphore(render_finished)
                .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                .value(0), // binary
            // Timeline semaphore for CPU-GPU sync
            vk::SemaphoreSubmitInfo::default()
                .semaphore(frame_sync.render_timeline)
                .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                .value(signal_timeline_value),
        ];

        let cmd_infos = [vk::CommandBufferSubmitInfo::default().command_buffer(cmd)];

        let submit_info = vk::SubmitInfo2::default()
            .wait_semaphore_infos(&wait_infos)
            .signal_semaphore_infos(&signal_infos)
            .command_buffer_infos(&cmd_infos);

        unsafe {
            let result = device.queue_submit2(
                self.device.queues.graphics,
                &[submit_info],
                vk::Fence::null(), // no fence — timeline semaphore handles sync
            );
            if let Err(e) = result {
                log_warn!("queue_submit2 failed: {:?}", e);
                return false;
            }
        }

        // --- Present ---
        let wait_semaphores = [render_finished];
        let swapchains = [swapchain.swapchain];
        let image_indices = [image_index];

        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&wait_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        unsafe {
            match self
                .device
                .swapchain_loader
                .queue_present(self.device.queues.present, &present_info)
            {
                Ok(false) => {}
                Ok(true) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    self.needs_resize = true;
                }
                Err(e) => {
                    log_warn!("queue_present failed: {:?}", e);
                }
            }
        }

        frame_sync.advance();
        true
    }

    /// Recreate the swapchain after a resize or present mode change.
    fn recreate_swapchain(&mut self) -> RendererResult<()> {
        let surface = self
            .surface
            .as_ref()
            .ok_or(RendererError::Config("no surface for swapchain recreation"))?;

        let old_swapchain = self.swapchain.as_ref().map(|s| s.swapchain);

        // Destroy old image views.
        if let Some(ref mut old) = self.swapchain {
            unsafe {
                for &view in &old.image_views {
                    self.device.handle().destroy_image_view(view, None);
                }
                old.image_views.clear();
            }
        }

        let new_swapchain = VulkanSwapchain::new(
            &self.device,
            surface.loader(),
            self.physical_device,
            surface.handle(),
            &self.device.queue_families,
            self.current_extent,
            old_swapchain,
            self.vsync,
        )?;

        // Destroy old swapchain handle after new one is created.
        if let Some(old_handle) = old_swapchain {
            unsafe {
                self.device
                    .swapchain_loader
                    .destroy_swapchain(old_handle, None);
            }
        }

        let new_image_count = new_swapchain.images.len() as u32;
        self.swapchain = Some(new_swapchain);

        // Rebuild depth to new extent.
        unsafe {
            if let Some(ref mut depth) = self.depth {
                depth.destroy(self.device.handle());
            }
        }
        self.depth = Some(DepthImage::new(
            self.device.handle(),
            &self.mem_props,
            self.current_extent,
            DEPTH_FORMAT,
        )?);

        // Recreate frame sync if image count changed (e.g. present mode switch).
        if let Some(ref frame_sync) = self.frame_sync {
            if frame_sync.frames.len() as u32 != new_image_count {
                unsafe {
                    self.frame_sync
                        .as_mut()
                        .unwrap()
                        .destroy(self.device.handle());
                }
                self.frame_sync = Some(FrameSync::new(
                    self.device.handle(),
                    self.device.queue_families.graphics,
                    new_image_count,
                )?);
            }
        }

        log_info!(
            "Swapchain recreated: {}x{}",
            self.current_extent.width,
            self.current_extent.height,
        );

        Ok(())
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.device.wait_idle();

        // TODO: this shouldnt be manual like this.

        unsafe {
            if let Some(ref mut vb) = self.vertex_buffer {
                vb.destroy(self.device.handle());
            }
            if let Some(ref mut ib) = self.index_buffer {
                ib.destroy(self.device.handle());
            }
            if let Some(ref mut lb) = self.light_buffer {
                lb.destroy(self.device.handle());
            }

            if let Some(ref mut descriptor) = self.descriptor {
                descriptor.destroy(self.device.handle());
            }
            if let Some(ref mut texture) = self.texture {
                texture.destroy(self.device.handle());
            }

            if let Some(ref mut depth) = self.depth {
                depth.destroy(self.device.handle());
            }

            if let Some(ref mut pipeline) = self.pipeline {
                pipeline.destroy(self.device.handle());
            }

            if let Some(ref mut frame_sync) = self.frame_sync {
                frame_sync.destroy(self.device.handle());
            }
            if let Some(ref mut swapchain) = self.swapchain {
                swapchain.destroy(&self.device);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Platform surface extensions
// ---------------------------------------------------------------------------

fn surface_extensions(
    surface: Option<&NativeSurfaceHandle>,
) -> RendererResult<Vec<&'static std::ffi::CStr>> {
    let Some(surface) = surface else {
        return Ok(Vec::new());
    };

    match surface.type_ {
        SurfaceType::Win32 => Ok(vec![ash::khr::win32_surface::NAME]),
        SurfaceType::Wayland => Ok(vec![ash::khr::wayland_surface::NAME]),
        SurfaceType::Android => Ok(vec![ash::khr::android_surface::NAME]),
    }
}
