// job_worker_tls

use crate::job_worker::JobWorkerLocal;

thread_local! {
    static JOB_WORKER_TLS: std::cell::Cell<*const JobWorkerLocal> =  std::cell::Cell::new(std::ptr::null())
}

pub(crate) fn get_job_worker_local_tls() -> Option<&'static JobWorkerLocal> {
    let ptr = JOB_WORKER_TLS.with(|ctx| ctx.get());
    if ptr.is_null() {
        None
    } else {
        // SAFETY: We've confirmed it's not null. The caller who initialized
        // the TLS must ensure this pointer remains valid for the thread's life.
        unsafe { Some(&*ptr) }
    }
}
pub(crate) struct JobWorkerTLSGuard;

impl JobWorkerTLSGuard {
    pub(crate) unsafe fn init(ptr: *const JobWorkerLocal) -> Self {
        JOB_WORKER_TLS.with(|ctx| ctx.set(ptr));
        JobWorkerTLSGuard
    }
}

impl Drop for JobWorkerTLSGuard {
    fn drop(&mut self) {
        JOB_WORKER_TLS.with(|ctx| ctx.set(std::ptr::null()));
    }
}
