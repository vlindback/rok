// buffer.rs
//

use ash::vk;

use crate::error::{RendererError, RendererResult, check};

/// A GPU buffer and its backing allocation. Freed via `destroy`.
pub(crate) struct Buffer {
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub size: vk::DeviceSize,
}

impl Buffer {
    pub(crate) unsafe fn destroy(&mut self, device: &ash::Device) {
        device.destroy_buffer(self.buffer, None);
        device.free_memory(self.memory, None);
    }
}

/// Pick a memory type satisfying `type_filter` (from the buffer's memory
/// requirements) that contains all `required` property flags.
pub(crate) fn find_memory_type(
    mem_props: &vk::PhysicalDeviceMemoryProperties,
    type_filter: u32,
    required: vk::MemoryPropertyFlags,
) -> RendererResult<u32> {
    for i in 0..mem_props.memory_type_count {
        let suitable = type_filter & (1 << i) != 0;
        let has_props = mem_props.memory_types[i as usize]
            .property_flags
            .contains(required);
        if suitable && has_props {
            return Ok(i);
        }
    }
    Err(RendererError::Config("no suitable memory type for buffer"))
}

fn create_buffer(
    device: &ash::Device,
    mem_props: &vk::PhysicalDeviceMemoryProperties,
    size: vk::DeviceSize,
    usage: vk::BufferUsageFlags,
    memory_flags: vk::MemoryPropertyFlags,
) -> RendererResult<Buffer> {
    let info = vk::BufferCreateInfo::default()
        .size(size)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
    let buffer = check!(
        unsafe { device.create_buffer(&info, None) },
        "create buffer"
    )?;

    let reqs = unsafe { device.get_buffer_memory_requirements(buffer) };
    let mem_type = find_memory_type(mem_props, reqs.memory_type_bits, memory_flags)?;

    let alloc = vk::MemoryAllocateInfo::default()
        .allocation_size(reqs.size)
        .memory_type_index(mem_type);
    let memory = check!(
        unsafe { device.allocate_memory(&alloc, None) },
        "allocate buffer memory"
    )?;
    check!(
        unsafe { device.bind_buffer_memory(buffer, memory, 0) },
        "bind buffer memory"
    )?;

    Ok(Buffer {
        buffer,
        memory,
        size,
    })
}

/// Host-visible, coherent buffer filled with `data` (TRANSFER_SRC). The
/// staging half, exposed so image uploads can copy it to an image.
pub(crate) fn create_host_buffer<T: Copy>(
    device: &ash::Device,
    mem_props: &vk::PhysicalDeviceMemoryProperties,
    data: &[T],
) -> RendererResult<Buffer> {
    let size = std::mem::size_of_val(data) as vk::DeviceSize;
    let mut staging = create_buffer(
        device,
        mem_props,
        size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;
    unsafe {
        let ptr = check!(
            device.map_memory(staging.memory, 0, size, vk::MemoryMapFlags::empty()),
            "map staging"
        )? as *mut T;
        ptr.copy_from_nonoverlapping(data.as_ptr(), data.len());
        device.unmap_memory(staging.memory);
    }
    Ok(staging)
}

/// Record into a one-shot command buffer, submit to `queue`, block until done.
/// Waits on a per-submit fence rather than the whole queue.
pub(crate) fn immediate_submit<F>(
    device: &ash::Device,
    queue: vk::Queue,
    queue_family: u32,
    record: F,
) -> RendererResult<()>
where
    F: FnOnce(&ash::Device, vk::CommandBuffer),
{
    let pool_info = vk::CommandPoolCreateInfo::default()
        .queue_family_index(queue_family)
        .flags(vk::CommandPoolCreateFlags::TRANSIENT);
    let pool = check!(
        unsafe { device.create_command_pool(&pool_info, None) },
        "create transient pool"
    )?;

    let alloc = vk::CommandBufferAllocateInfo::default()
        .command_pool(pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(1);
    let cmd = check!(
        unsafe { device.allocate_command_buffers(&alloc) },
        "allocate transient cmd"
    )?[0];

    unsafe {
        let begin = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        check!(
            device.begin_command_buffer(cmd, &begin),
            "begin transient cmd"
        )?;
        record(device, cmd);
        check!(device.end_command_buffer(cmd), "end transient cmd")?;

        let submit = vk::SubmitInfo::default().command_buffers(std::slice::from_ref(&cmd));
        let fence = check!(
            device.create_fence(&vk::FenceCreateInfo::default(), None),
            "create transient fence"
        )?;
        check!(
            device.queue_submit(queue, std::slice::from_ref(&submit), fence),
            "submit transient cmd"
        )?;
        check!(
            device.wait_for_fences(std::slice::from_ref(&fence), true, u64::MAX),
            "wait transient fence"
        )?;

        device.destroy_fence(fence, None);
        device.destroy_command_pool(pool, None);
    }
    Ok(())
}

/// Create a DEVICE_LOCAL buffer (`usage | TRANSFER_DST`) and fill it with
/// `data` by staging through a HOST_VISIBLE|COHERENT buffer.
pub(crate) fn upload_via_staging<T: Copy>(
    device: &ash::Device,
    mem_props: &vk::PhysicalDeviceMemoryProperties,
    queue: vk::Queue,
    queue_family: u32,
    data: &[T],
    usage: vk::BufferUsageFlags,
) -> RendererResult<Buffer> {
    let size = std::mem::size_of_val(data) as vk::DeviceSize;

    let mut staging = create_buffer(
        device,
        mem_props,
        size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;

    unsafe {
        let ptr = check!(
            device.map_memory(staging.memory, 0, size, vk::MemoryMapFlags::empty()),
            "map staging"
        )? as *mut T;
        ptr.copy_from_nonoverlapping(data.as_ptr(), data.len());
        device.unmap_memory(staging.memory);
    }

    let dst = create_buffer(
        device,
        mem_props,
        size,
        usage | vk::BufferUsageFlags::TRANSFER_DST,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    immediate_submit(device, queue, queue_family, |device, cmd| unsafe {
        let region = vk::BufferCopy::default().size(size);
        device.cmd_copy_buffer(
            cmd,
            staging.buffer,
            dst.buffer,
            std::slice::from_ref(&region),
        );
    })?;

    unsafe { staging.destroy(device) };
    Ok(dst)
}
