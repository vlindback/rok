// join_handle.rs

use std::marker::PhantomData;

use crate::job_fence::JobFence;

/// A handle to a submitted job or group of jobs.
///
/// By default the handle will **detach** on drop — the job keeps running
/// but you lose the ability to wait on it. In debug builds a warning fires
/// if you drop the handle without explicitly calling [`wait`] or [`detach`],
/// to catch accidental handle discards.
///
/// # Example
/// ```
/// let handle = job_system.submit(|| do_work()).dispatch();
/// handle.wait(); // block until complete, helping execute tasks
///
/// // or explicitly opt in to fire-and-forget:
/// job_system.submit(|| do_work()).dispatch().detach();
/// ```
pub struct JoinHandle<'fence> {
    // Ties the handle's lifetime to the fence it was created with,
    // preventing the fence from being dropped while jobs still reference it.
    _fence_lifetime: PhantomData<&'fence JobFence>,

    // Raw pointer to the fence, if one was provided at dispatch time.
    // None for fire-and-forget submits.
    fence: Option<*const JobFence>,

    #[cfg(debug_assertions)]
    resolved: bool,
}

impl<'fence> JoinHandle<'fence> {
    pub(crate) fn new(fence: Option<*const JobFence>) -> Self {
        Self {
            _fence_lifetime: PhantomData,
            fence,
            #[cfg(debug_assertions)]
            resolved: false,
        }
    }

    /// Block the calling thread until all jobs associated with this handle
    /// are complete.
    ///
    /// If no fence was provided at dispatch time this returns immediately.
    pub fn wait(mut self) {
        #[cfg(debug_assertions)]
        {
            self.resolved = true;
        }
        if let Some(fence) = self.fence {
            // Safety: fence lifetime is tied to 'fence which the borrow
            // checker guarantees is still live at this call site.
            unsafe {
                (*fence).wait();
            }
        }
    }

    /// Block the calling thread by spinning until all associated jobs are complete.
    ///
    /// If no fence was provided at dispatch time this returns immediately.
    pub fn wait_spin(mut self) {
        #[cfg(debug_assertions)]
        {
            self.resolved = true;
        }
        if let Some(fence) = self.fence {
            unsafe {
                (*fence).wait_spin();
            }
        }
    }

    /// Returns true if all associated jobs have finished.
    ///
    /// Always returns true if no fence was provided at dispatch time.
    pub fn is_complete(&self) -> bool {
        match self.fence {
            Some(fence) => unsafe { (*fence).is_complete() },
            None => true,
        }
    }

    /// Explicitly detach the handle. The jobs continue running but you
    /// give up the ability to wait on them.
    pub fn detach(mut self) {
        #[cfg(debug_assertions)]
        {
            self.resolved = true;
        }
        // Nothing to do — Drop will see resolved = true and stay silent.
    }
}

impl Drop for JoinHandle<'_> {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        {
            debug_assert!(
                self.resolved,
                "JoinHandle dropped without calling wait() or detach(). \
                 If fire-and-forget is intentional, call .detach() explicitly."
            );
        }
        // Release build: silently detach.
    }
}
