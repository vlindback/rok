mod ioring_api;

use crate::io_capabilities::IoCapabilities;
use crate::{Completion, IoError, IoToken};
use ioring_api::*;
use std::mem::MaybeUninit;

use std::os::raw::c_void;

use windows_sys::Win32::Foundation::{CloseHandle, FALSE, HANDLE, WAIT_OBJECT_0};
use windows_sys::Win32::System::Threading::{
    CreateEventW, INFINITE, SetEvent, WaitForMultipleObjects,
};

mod io_buffer;
pub use io_buffer::IoBuffer;

pub struct IoRingWindows {
    handle: HIORING,
    sq_size: u32,
    in_flight: u32,
    completion_event: HANDLE,
    shutdown_event: HANDLE,
}

impl IoRingWindows {
    pub fn new(caps: IoCapabilities) -> Result<Self, &'static str> {
        // TODO: 256 is good enough for now but should be configurable.
        let sq_size = 256_u32.min(caps.max_sq_entries);
        let cq_size = (sq_size * 2).min(caps.max_cq_entries);

        let flags = IORING_CREATE_FLAGS {
            Required: IORING_CREATE_REQUIRED_FLAGS_NONE,
            Advisory: IORING_CREATE_ADVISORY_FLAGS_NONE,
        };

        unsafe {
            // Create events before the ring so cleanup is clean on failure
            let completion_event = CreateEventW(
                std::ptr::null(),
                FALSE, // auto-reset, kernel resets it after we wake
                FALSE, // initially unsignaled
                std::ptr::null(),
            );
            if completion_event.is_null() {
                return Err("CreateEventW failed for completion event");
            }

            let shutdown_event = CreateEventW(std::ptr::null(), FALSE, FALSE, std::ptr::null());
            if shutdown_event.is_null() {
                CloseHandle(completion_event);
                return Err("CreateEventW failed for shutdown event");
            }

            let mut handle = MaybeUninit::<HIORING>::uninit();
            let hr = CreateIoRing(
                IORING_VERSION::Version3,
                flags,
                sq_size,
                cq_size,
                handle.as_mut_ptr(),
            );
            if hr < 0 {
                CloseHandle(completion_event);
                CloseHandle(shutdown_event);
                return Err("CreateIoRing failed");
            }

            let ring = handle.assume_init();

            let hr = SetIoRingCompletionEvent(ring, completion_event);
            if hr < 0 {
                CloseIoRing(ring);
                CloseHandle(completion_event);
                CloseHandle(shutdown_event);
                return Err("SetIoRingCompletionEvent failed");
            }

            Ok(Self {
                handle: ring,
                sq_size,
                in_flight: 0,
                completion_event,
                shutdown_event,
            })
        }
    }

    /// Block until the kernel signals completions or shutdown is requested.
    /// Returns true if completions are ready, false if shutting down.
    pub fn wait_for_completions(&self) -> bool {
        let events = [self.completion_event, self.shutdown_event];
        unsafe {
            let result = WaitForMultipleObjects(2, events.as_ptr(), FALSE, INFINITE);
            match result {
                WAIT_OBJECT_0 => true,
                x if x == WAIT_OBJECT_0 + 1 => false,
                _ => panic!("WaitForMultipleObjects failed, OS error"),
            }
        }
    }

    /// Signal the I/O thread to shut down.
    /// Call this from the host before joining the thread.
    pub fn signal_shutdown(&self) {
        unsafe { SetEvent(self.shutdown_event) };
    }

    /// Registers file handles with the ring for indexed access.
    ///
    /// Intended to be called once at startup with all pak/data files.
    /// In editor contexts may be called when new files are opened, but
    /// should never be called per-frame or on the hot path.
    ///
    /// Blocks until the kernel confirms registration is complete.
    pub fn register_file_handles(
        &mut self,
        handles: &[crate::RawHandle],
    ) -> Result<(), &'static str> {
        unsafe {
            let hr = BuildIoRingRegisterFileHandles(
                self.handle,
                handles.len() as u32,
                handles.as_ptr() as *const HANDLE,
                0,
            );
            if hr < 0 {
                return Err("BuildIoRingRegisterFileHandles failed");
            }
        }

        self.in_flight += 1;

        self.submit()?;
        if !self.wait_for_completions() {
            return Err("shutdown signaled during handle registration");
        }

        while let Some(completion) = self.pop_completion() {
            debug_assert!(
                completion.token == IoToken(0),
                "unexpected completion during handle registration"
            );
            break;
        }

        Ok(())
    }

    /// Queue a read using a pre-registered file handle index.
    /// # Safety
    /// The caller must guarantee `buffer` is not dropped until the corresponding
    /// `IoToken` is returned by `pop_completion`. The underlying memory must
    /// remain allocated for the duration of the kernel operation.
    /// Moving the `IoBuffer` struct itself is fine, the kernel holds a raw
    /// pointer to the allocation, not to the wrapper.
    pub unsafe fn submit_read_registered(
        &mut self,
        handle_index: u32,
        buffer: &mut IoBuffer,
        offset: u64,
        size: u32,
        token: IoToken,
    ) -> Result<(), &'static str> {
        if self.in_flight >= self.sq_size {
            return Err("submission queue full");
        }

        debug_assert!(
            size as usize <= buffer.size(),
            "read size exceeds buffer size"
        );

        let file_ref = IORING_HANDLE_REF::from_index(handle_index);
        let buffer_ref = IORING_BUFFER_REF::from_pointer(buffer.as_mut_ptr() as *mut c_void);

        unsafe {
            let hr = BuildIoRingReadFile(
                self.handle,
                file_ref,
                buffer_ref,
                size,
                offset,
                token.0 as usize,
                IOSQE_FLAGS_NONE,
            );

            if hr < 0 {
                return Err("BuildIoRingReadFile failed");
            }
        }

        self.in_flight += 1;
        Ok(())
    }

    /// Submit all queued SQEs to the kernel and wait for at least
    /// `wait_for` completions. Pass 0 to submit without waiting.
    pub fn submit(&mut self) -> Result<(), &'static str> {
        unsafe {
            let hr = SubmitIoRing(self.handle, 0, 0, std::ptr::null_mut());
            if hr < 0 {
                return Err("SubmitIoRing failed");
            }
        }
        Ok(())
    }

    /// Queue a read using a raw file handle.
    /// Slower than `submit_read_registered`, the kernel resolves the handle on every op.
    /// Use for one-off files not worth registering, such as individual shader files.
    ///
    /// # Safety
    /// The caller must guarantee `buffer` is not dropped until the corresponding
    /// `IoToken` is returned by `pop_completion`.
    pub unsafe fn submit_read(
        &mut self,
        handle: crate::RawHandle,
        buffer: &mut IoBuffer,
        offset: u64,
        size: u32,
        token: IoToken,
    ) -> Result<(), &'static str> {
        if self.in_flight >= self.sq_size {
            return Err("submission queue full");
        }

        debug_assert!(
            size as usize <= buffer.size(),
            "read size exceeds buffer size"
        );

        let file_ref = IORING_HANDLE_REF::from_handle(handle.0 as HANDLE);
        let buffer_ref = IORING_BUFFER_REF::from_pointer(buffer.as_mut_ptr() as *mut c_void);

        unsafe {
            let hr = BuildIoRingReadFile(
                self.handle,
                file_ref,
                buffer_ref,
                size,
                offset,
                token.0 as usize,
                IOSQE_FLAGS_NONE,
            );

            if hr < 0 {
                return Err("BuildIoRingReadFile failed");
            }
        }

        self.in_flight += 1;
        Ok(())
    }

    /// Pop one completion from the CQ. Returns None if the queue is empty.
    pub fn pop_completion(&mut self) -> Option<Completion> {
        let mut cqe = MaybeUninit::<IORING_CQE>::uninit();

        unsafe {
            let hr = PopIoRingCompletion(self.handle, cqe.as_mut_ptr());

            // S_FALSE (1) means the queue is empty — not an error
            if hr != 0 {
                return None;
            }

            let cqe = cqe.assume_init();
            self.in_flight = self.in_flight.saturating_sub(1);

            let result = if cqe.ResultCode >= 0 {
                Ok(cqe.Information as usize)
            } else {
                Err(IoError {
                    code: cqe.ResultCode,
                })
            };

            Some(Completion {
                token: IoToken(cqe.UserData as u64),
                result,
            })
        }
    }
}

impl Drop for IoRingWindows {
    fn drop(&mut self) {
        unsafe {
            CloseIoRing(self.handle);
            CloseHandle(self.completion_event);
            CloseHandle(self.shutdown_event);
        }
    }
}

pub fn get_io_capabilities() -> Result<IoCapabilities, &'static str> {
    let caps: IORING_CAPABILITIES = unsafe {
        let mut caps_uninit = MaybeUninit::<IORING_CAPABILITIES>::uninit();
        let hr = QueryIoRingCapabilities(caps_uninit.as_mut_ptr());

        if hr < 0 {
            return Err(
                "QueryIoRingCapabilities failed, Windows IoRing may not be available on this build",
            );
        }

        caps_uninit.assume_init()
    };

    if caps.MaxVersion < IORING_VERSION::Version3 {
        return Err("IoRing Version3 or later required - please update Windows");
    }

    Ok(IoCapabilities {
        max_sq_entries: caps.MaxSubmissionQueueSize,
        max_cq_entries: caps.MaxCompletionQueueSize,
        emulated: (caps.FeatureFlags & IORING_FEATURE_UM_EMULATION) != 0,
    })
}
