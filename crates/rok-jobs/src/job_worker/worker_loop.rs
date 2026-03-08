// job_loop.rs

use crate::{
    job, job_priority::JobPriority, job_worker::JobWorkerLocal, job_worker_tls::JobWorkerTLSGuard,
};

// Worker loop
pub(crate) fn job_worker_loop(local: &mut JobWorkerLocal) {
    let _tls_guard = unsafe { JobWorkerTLSGuard::init(local) };

    while !local.stop_token.is_stop_requested() {
        // FAST PATH: Local LIFO Deque
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

        // STEAL from others. if we steal execute then continue.
        match local.scheduler.try_steal(local.id, &mut local.loot) {
            // SAFETY: `try_steal` returns the number of initialized elements in `loot`.
            // It guarantees that if `n_stolen > 0`, index 0 is valid and initialized.

            // No work found, proceed to the backoff logic.
            0 => {}
            // Found >= 1 jobs.
            // Reserve and run the highest priority job, put rest in local FIFO deque.
            n_stolen => {
                // The first job is the highest priority one because try_steal sorts it.
                let reserved = unsafe { local.loot.get_unchecked(0).assume_init() }.0;

                for i in 1..n_stolen {
                    let (index, prio) = unsafe { local.loot.get_unchecked(i).assume_init() };
                    local.push_work(index, prio);
                }

                local.scheduler.run_job(reserved);
                continue;
            }
        }

        // --- OUT OF WORK: BEGIN BACKOFF ---

        // SPIN: CPU stays hot, branch predictor stays ready.
        for _ in 0..32 {
            std::hint::spin_loop();
            if local.has_work_anywhere() {
                has_work = true;
                break;
            }
        }
        if has_work {
            continue;
        }

        // YIELD: Tell the OS to let someone else run.
        for _ in 0..16 {
            std::thread::yield_now();
            if local.has_work_anywhere() {
                has_work = true;
                break;
            }
        }
        if has_work {
            continue;
        }

        // PARK: We are completely starved. Go to sleep.
        if !local.has_work_anywhere() {
            local.parker.park();
        }
    }
}
