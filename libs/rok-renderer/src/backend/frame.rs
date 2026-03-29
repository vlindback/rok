// backend/frame.rs
//
// Per-frame resources for N-buffered rendering.
//
// Synchronization uses a single timeline semaphore shared across all
// frame slots. Each slot records the timeline value it last signaled.
// Before reusing a slot the CPU waits for that value via vkWaitSemaphores.
//
// Binary semaphores are still required for acquire/present (Vulkan spec),
// but the CPU-GPU fence is replaced entirely by the timeline semaphore,
// giving us a single monotonic counter instead of N discrete fences.
//
// We match frames_in_flight to swapchain image count so binary semaphores
// are never reused while the presentation engine still holds them.

use ash::vk;

use rok_log::log_trace;

use crate::error::{RendererResult, check};

// ---------------------------------------------------------------------------
// FrameData
// ---------------------------------------------------------------------------

/// Resources owned by a single frame-in-flight slot.
pub(crate) struct FrameData {
    pub command_pool: vk::CommandPool,
    pub command_buffer: vk::CommandBuffer,

    /// Binary semaphore: signaled by vkAcquireNextImageKHR when an image is ready.
    pub image_available: vk::Semaphore,

    /// Binary semaphore: signaled by queue submit, waited on by present.
    pub render_finished: vk::Semaphore,
}

// ---------------------------------------------------------------------------
// FrameSync
// ---------------------------------------------------------------------------

/// Manages N frame-in-flight slots with a shared timeline semaphore.
pub(crate) struct FrameSync {
    pub(crate) frames: Vec<FrameData>,
    pub(crate) current_frame: usize,

    /// Single timeline semaphore shared across all frame slots.
    /// Replaces per-frame fences with a monotonic counter.
    pub(crate) render_timeline: vk::Semaphore,

    /// Monotonically increasing counter. Incremented on each submit.
    pub(crate) timeline_value: u64,

    /// The timeline value each frame slot last signaled.
    /// Before reusing slot K, the CPU waits for timeline >= frame_values[K].
    pub(crate) frame_timeline_values: Vec<u64>,
}

impl FrameSync {
    /// Allocate `count` frame-in-flight slots with a shared timeline semaphore.
    pub(crate) fn new(
        device: &ash::Device,
        graphics_family: u32,
        count: u32,
    ) -> RendererResult<Self> {
        let mut frames = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let frame = create_frame_data(device, graphics_family)?;
            frames.push(frame);
        }

        // Create the timeline semaphore starting at 0.
        let mut timeline_type_info = vk::SemaphoreTypeCreateInfo::default()
            .semaphore_type(vk::SemaphoreType::TIMELINE)
            .initial_value(0);

        let semaphore_info = vk::SemaphoreCreateInfo::default().push_next(&mut timeline_type_info);

        let render_timeline = unsafe {
            check!(
                device.create_semaphore(&semaphore_info, None),
                "create timeline semaphore"
            )?
        };

        let frame_timeline_values = vec![0u64; count as usize];

        log_trace!(
            "Created {} frame-in-flight slots with timeline semaphore",
            count,
        );

        Ok(Self {
            frames,
            current_frame: 0,
            render_timeline,
            timeline_value: 0,
            frame_timeline_values,
        })
    }

    /// Get the current frame's data.
    #[inline]
    pub(crate) fn current(&self) -> &FrameData {
        &self.frames[self.current_frame]
    }

    /// The timeline value the current frame slot last signaled.
    /// Wait for this before reusing the slot.
    #[inline]
    pub(crate) fn current_wait_value(&self) -> u64 {
        self.frame_timeline_values[self.current_frame]
    }

    /// Increment the timeline counter and record it for the current slot.
    /// Returns the new value to use in the submit's signal info.
    #[inline]
    pub(crate) fn next_timeline_value(&mut self) -> u64 {
        self.timeline_value += 1;
        self.frame_timeline_values[self.current_frame] = self.timeline_value;
        self.timeline_value
    }

    /// Advance to the next frame slot (wraps around).
    #[inline]
    pub(crate) fn advance(&mut self) {
        self.current_frame = (self.current_frame + 1) % self.frames.len();
    }

    /// Wait for the current frame slot's previous work to complete.
    ///
    /// Uses vkWaitSemaphores on the timeline semaphore instead of a fence.
    /// Returns immediately if the slot has never been used (value == 0).
    pub(crate) fn wait_for_current_frame(&self, device: &ash::Device) {
        let wait_value = self.current_wait_value();
        if wait_value == 0 {
            return; // first use, nothing to wait for
        }

        let semaphores = [self.render_timeline];
        let values = [wait_value];

        let wait_info = vk::SemaphoreWaitInfo::default()
            .semaphores(&semaphores)
            .values(&values);

        unsafe {
            // Ignore timeout errors — u64::MAX effectively means infinite.
            let _ = device.wait_semaphores(&wait_info, u64::MAX);
        }
    }

    /// Destroy all frame resources.
    ///
    /// # Safety
    /// The caller must ensure no GPU work is using any of these resources.
    pub(crate) unsafe fn destroy(&mut self, device: &ash::Device) {
        device.destroy_semaphore(self.render_timeline, None);

        for frame in &self.frames {
            device.destroy_semaphore(frame.render_finished, None);
            device.destroy_semaphore(frame.image_available, None);
            device.destroy_command_pool(frame.command_pool, None);
        }
        self.frames.clear();
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn create_frame_data(device: &ash::Device, graphics_family: u32) -> RendererResult<FrameData> {
    // --- Command pool ---
    let pool_info = vk::CommandPoolCreateInfo::default()
        .queue_family_index(graphics_family)
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

    let command_pool = unsafe {
        check!(
            device.create_command_pool(&pool_info, None),
            "create command pool"
        )?
    };

    // --- Command buffer ---
    let alloc_info = vk::CommandBufferAllocateInfo::default()
        .command_pool(command_pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(1);

    let command_buffers = unsafe {
        check!(
            device.allocate_command_buffers(&alloc_info),
            "allocate command buffer"
        )?
    };

    // --- Binary semaphores (required by acquire/present) ---
    let semaphore_info = vk::SemaphoreCreateInfo::default();

    let image_available = unsafe {
        check!(
            device.create_semaphore(&semaphore_info, None),
            "create image_available semaphore"
        )?
    };

    let render_finished = unsafe {
        check!(
            device.create_semaphore(&semaphore_info, None),
            "create render_finished semaphore"
        )?
    };

    Ok(FrameData {
        command_pool,
        command_buffer: command_buffers[0],
        image_available,
        render_finished,
    })
}
