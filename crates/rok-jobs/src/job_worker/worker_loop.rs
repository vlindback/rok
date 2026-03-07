// job_loop.rs

use crate::{
    job_priority::JobPriority, job_worker::JobWorkerLocal, job_worker_tls::JobWorkerTLSGuard,
};

// Worker loop
pub(crate) fn job_worker_loop(local: &mut JobWorkerLocal) {
    let _tls_guard = unsafe { JobWorkerTLSGuard::init(local) };

    while !local.stop_token.is_stop_requested() {
        // 1. FAST PATH: Local LIFO Deque
        if let Some(task) = local.pop_local() {
            // TODO: This function must handle panics properly + system poison
            local.scheduler.run_job(task);
            continue;
        }

        let mut has_work = false;

        for prio in JobPriority::ALL {
            if local.drain_shared_inbox(prio) {
                has_work = true;
            }
        }

        // Found work in inbox. Skipping to next iteration to process contents.
        if has_work {
            continue;
        }

        // 3. TODO: steal from others. if we steal execute then continue.

        // --- OUT OF WORK: BEGIN BACKOFF ---

        // SPIN: CPU stays hot, branch predictor stays ready.
        let mut found_work = false;
        for _ in 0..32 {
            std::hint::spin_loop();
            if local.has_work_anywhere() {
                found_work = true;
                break;
            }
        }
        if found_work {
            continue;
        }

        // 5. YIELD: Tell the OS to let someone else run.
        for _ in 0..16 {
            std::thread::yield_now();
            if local.has_work_anywhere() {
                found_work = true;
                break;
            }
        }
        if found_work {
            continue;
        }

        // 6. PARK: We are completely starved. Go to sleep.
        if !local.has_work_anywhere() {
            local.parker.park();
        }
    }
}
