// engine-abi/src/task.rs
#[repr(C)]
pub struct ITaskSystem {
    // FFI-safe function pointer to kick a job
    pub enqueue: extern "C" fn(priority: u8, task_fn: extern "C" fn(*mut u8), data: *mut u8),
    pub wait_for_all: extern "C" fn(),
}
