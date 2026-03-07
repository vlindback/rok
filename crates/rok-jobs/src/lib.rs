// lib.rs

mod stop;

mod job;
mod job_fence;
mod job_pool;
mod job_priority;
mod job_record;
mod job_scheduler;
mod job_system;
mod job_worker;
mod job_worker_tls;
mod join_handle;
mod tsc_timer;

// Public API surface — everything else is an implementation detail.
pub use job_fence::JobFence;
pub use job_priority::JobPriority;
pub use job_system::JobSchedulerConfig;
pub use job_system::JobSystem;
pub use join_handle::JoinHandle;
