// io_windows.rs

mod ioring_api;

use crate::io_capabilities::IoCapabilities;

use ioring_api::{
    IORING_CAPABILITIES, IORING_FEATURE_UM_EMULATION, IORING_VERSION, QueryIoRingCapabilities,
};

use std::mem::MaybeUninit;

pub struct IoRingWindows {}

impl IoRingWindows {
    pub fn new(caps: IoCapabilities) -> Self {
        Self {}
    }
}

pub(crate) fn get_io_capabilities() -> Result<IoCapabilities, &'static str> {
    let caps: IORING_CAPABILITIES = unsafe {
        let mut caps_uninit = MaybeUninit::<IORING_CAPABILITIES>::uninit();
        let hr = QueryIoRingCapabilities(caps_uninit.as_mut_ptr());

        if hr < 0 {
            return Err(
                "QueryIoRingCapabilities failed - Windows IoRing may not be available on this build",
            );
        }

        caps_uninit.assume_init()
    };

    if caps.MaxVersion < IORING_VERSION::Version3 {
        return Err("IoRing Version3 or later required - please update Windows");
    }

    let iocap = IoCapabilities {
        max_sq_entries: caps.MaxSubmissionQueueSize,
        max_cq_entries: caps.MaxCompletionQueueSize,
        emulated: (caps.FeatureFlags & IORING_FEATURE_UM_EMULATION) != 0,
    };

    Ok(iocap)
}
