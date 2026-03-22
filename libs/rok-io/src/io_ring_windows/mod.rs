mod ioring_api;

use crate::io_capabilities::IoCapabilities;
use crate::{Completion, IoError, IoToken};
use ioring_api::*;
use std::mem::MaybeUninit;

use std::os::raw::c_void;
use std::os::windows::io::AsRawHandle;

mod io_buffer;
pub use io_buffer::IoBuffer;

pub struct IoRingWindows {
    handle: HIORING,
    sq_size: u32,
    cq_size: u32,
    in_flight: u32,
}

impl IoRingWindows {
    pub fn new(caps: IoCapabilities) -> Result<Self, &'static str> {
        // TODO: 256 is a good starting point but should be configurable.

        // clamp requested sizes to what the system supports
        let sq_size = 256_u32.min(caps.max_sq_entries);
        let cq_size = (sq_size * 2).min(caps.max_cq_entries);

        let flags = IORING_CREATE_FLAGS {
            Required: IORING_CREATE_REQUIRED_FLAGS_NONE,
            Advisory: IORING_CREATE_ADVISORY_FLAGS_NONE,
        };

        let mut handle = MaybeUninit::<HIORING>::uninit();

        unsafe {
            let hr = CreateIoRing(
                IORING_VERSION::Version3,
                flags,
                sq_size,
                cq_size,
                handle.as_mut_ptr(),
            );

            if hr < 0 {
                return Err("CreateIoRing failed");
            }

            Ok(Self {
                handle: handle.assume_init(),
                sq_size,
                cq_size,
                in_flight: 0,
            })
        }
    }

    /// Queue a read SQE. Does not submit to the kernel yet.
    /// `token` is your correlation id, echoed back in the completion.
    /// Returns Err if the SQ is full.
    pub fn submit_read(
        &mut self,
        file: &std::fs::File,
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

        let file_ref = IORING_HANDLE_REF::from_handle(file.as_raw_handle());
        let buffer_ref = IORING_BUFFER_REF::from_pointer(buffer.as_mut_ptr() as *mut c_void);

        unsafe {
            let hr = BuildIoRingReadFile(
                self.handle,
                file_ref,
                buffer_ref,
                size,
                offset,
                token.0 as usize, // user_data echoed in CQE
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
    pub fn submit(&mut self, wait_for: u32) -> Result<(), &'static str> {
        unsafe {
            let hr = SubmitIoRing(
                self.handle,
                wait_for,
                u32::MAX, // no timeout — block until completions arrive
                std::ptr::null_mut(),
            );

            if hr < 0 {
                return Err("SubmitIoRing failed");
            }
        }

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
        unsafe { CloseIoRing(self.handle) };
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
