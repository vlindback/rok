// rok-io/src/io_backend_windows_api.rs

// Since windows-sys did not have bindings of IoRing we do it ourselves:

// This file has been partially generated.

#![allow(non_camel_case_types, non_snake_case, dead_code)]

use core::ffi::c_void;
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::core::{BOOL, HRESULT};

// --- ntioring_x.h ---

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum IORING_VERSION {
    Invalid = 0,
    Version1 = 1,
    Version2 = 2,
    Version3 = 3,
}

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum IORING_OP_CODE {
    Nop = 0,
    Read = 1,
    RegisterFiles = 2,
    RegisterBuffers = 3,
    Cancel = 4,
    Write = 5,
    Flush = 6,
    ReadScatter = 7,
    WriteGather = 8,
}

pub type IORING_FEATURE_FLAGS = u32;
pub const IORING_FEATURE_FLAGS_NONE: IORING_FEATURE_FLAGS = 0;
pub const IORING_FEATURE_UM_EMULATION: IORING_FEATURE_FLAGS = 0x0000_0001;
pub const IORING_FEATURE_SET_COMPLETION_EVENT: IORING_FEATURE_FLAGS = 0x0000_0002;

/// Pre-registered buffer identified by ring-local index and byte offset.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct IORING_REGISTERED_BUFFER {
    pub BufferIndex: u32,
    pub Offset: u32,
}

/// Buffer descriptor for `BuildIoRingRegisterBuffers`.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct IORING_BUFFER_INFO {
    pub Address: *mut c_void,
    pub Length: u32,
}

// --- ioringapi.h ---

/// Opaque IoRing instance handle.
/// **Must** be released with `CloseIoRing` — NOT `CloseHandle`.
#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct HIORING(pub HANDLE);

// --- Flags ---
// Modelled as u32 + constants (matching windows-sys style) because the C
// headers use DEFINE_ENUM_FLAG_OPERATORS, making them bitfields in practice.

pub type IORING_SQE_FLAGS = u32;
pub const IOSQE_FLAGS_NONE: IORING_SQE_FLAGS = 0;
pub const IOSQE_FLAGS_DRAIN_PRECEDING_OPS: IORING_SQE_FLAGS = 0x0000_0001;

pub type IORING_CREATE_REQUIRED_FLAGS = u32;
pub const IORING_CREATE_REQUIRED_FLAGS_NONE: IORING_CREATE_REQUIRED_FLAGS = 0;

pub type IORING_CREATE_ADVISORY_FLAGS = u32;
pub const IORING_CREATE_ADVISORY_FLAGS_NONE: IORING_CREATE_ADVISORY_FLAGS = 0;
/// Skips builder-side parameter validation. Errors are still caught in the
/// kernel and surface through CQE result codes. Safe to set in release builds.
pub const IORING_CREATE_SKIP_BUILDER_PARAM_CHECKS: IORING_CREATE_ADVISORY_FLAGS = 0x0000_0001;

// --- Structs ---

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct IORING_CREATE_FLAGS {
    pub Required: IORING_CREATE_REQUIRED_FLAGS,
    pub Advisory: IORING_CREATE_ADVISORY_FLAGS,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct IORING_INFO {
    pub IoRingVersion: IORING_VERSION,
    pub Flags: IORING_CREATE_FLAGS,
    pub SubmissionQueueSize: u32,
    pub CompletionQueueSize: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct IORING_CAPABILITIES {
    pub MaxVersion: IORING_VERSION,
    pub MaxSubmissionQueueSize: u32,
    pub MaxCompletionQueueSize: u32,
    pub FeatureFlags: IORING_FEATURE_FLAGS,
}

// --- Reference types ---

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum IORING_REF_KIND {
    Raw = 0,
    Registered = 1,
}

// Union arm of `IORING_HANDLE_REF`.
/// On x64, `HANDLE` is 8 bytes, so the union is 8 bytes; the enclosing struct
/// gets 4 bytes of padding after `Kind` to align it. Total: 16 bytes.
#[repr(C)]
#[derive(Copy, Clone)]
pub union IORING_HANDLE_REF_U {
    pub Handle: HANDLE, // Kind == Raw
    pub Index: u32,     // Kind == Registered
}

/// Reference to a file handle: either a live HANDLE or an index into the
/// ring's pre-registered handle table.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct IORING_HANDLE_REF {
    pub Kind: IORING_REF_KIND,
    pub Handle: IORING_HANDLE_REF_U,
}

impl IORING_HANDLE_REF {
    /// Mirrors `IoRingHandleRefFromHandle`.
    #[inline]
    pub fn from_handle(h: HANDLE) -> Self {
        Self {
            Kind: IORING_REF_KIND::Raw,
            Handle: IORING_HANDLE_REF_U { Handle: h },
        }
    }

    /// Mirrors `IoRingHandleRefFromIndex`.
    #[inline]
    pub fn from_index(index: u32) -> Self {
        Self {
            Kind: IORING_REF_KIND::Registered,
            Handle: IORING_HANDLE_REF_U { Index: index },
        }
    }
}

/// Union arm of `IORING_BUFFER_REF`.
#[repr(C)]
#[derive(Copy, Clone)]
pub union IORING_BUFFER_REF_U {
    pub Address: *mut c_void,                     // Kind == Raw
    pub IndexAndOffset: IORING_REGISTERED_BUFFER, // Kind == Registered
}

/// Reference to a data buffer: either a raw pointer or a pre-registered
/// buffer identified by index + offset.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct IORING_BUFFER_REF {
    pub Kind: IORING_REF_KIND,
    pub Buffer: IORING_BUFFER_REF_U,
}

impl IORING_BUFFER_REF {
    /// Mirrors `IoRingBufferRefFromPointer`.
    #[inline]
    pub fn from_pointer(p: *mut c_void) -> Self {
        Self {
            Kind: IORING_REF_KIND::Raw,
            Buffer: IORING_BUFFER_REF_U { Address: p },
        }
    }

    /// Mirrors `IoRingBufferRefFromIndexAndOffset`.
    #[inline]
    pub fn from_index_and_offset(index: u32, offset: u32) -> Self {
        Self {
            Kind: IORING_REF_KIND::Registered,
            Buffer: IORING_BUFFER_REF_U {
                IndexAndOffset: IORING_REGISTERED_BUFFER {
                    BufferIndex: index,
                    Offset: offset,
                },
            },
        }
    }
}

/// A completed I/O Ring entry, popped from the completion queue.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct IORING_CQE {
    /// Caller-supplied tag echoed from the matching SQE.
    pub UserData: usize, // UINT_PTR
    /// HRESULT status of the completed operation.
    pub ResultCode: HRESULT,
    /// Operation-defined result value (e.g. bytes transferred for reads/writes).
    pub Information: usize, // ULONG_PTR
}

// --- FILE_WRITE_FLAGS / FILE_FLUSH_MODE (from winbase.h) ---

pub type FILE_WRITE_FLAGS = u32;
pub const FILE_WRITE_FLAGS_NONE: FILE_WRITE_FLAGS = 0;
pub const FILE_WRITE_FLAGS_WRITE_THROUGH: FILE_WRITE_FLAGS = 0x0000_0001;

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FILE_FLUSH_MODE {
    Default = 0,
    Data = 1,
    MinMetadata = 2,
    NoSync = 3,
}

/// Scatter/gather I/O segment element.
/// Always 8 bytes. Access `Buffer` as a 64-bit pointer-width value; the
/// `Alignment` arm exists only to force 8-byte size on 32-bit targets.
#[repr(C)]
#[derive(Copy, Clone)]
pub union FILE_SEGMENT_ELEMENT {
    pub Buffer: u64, // PVOID64 — always 64-bit regardless of target
    pub Alignment: u64,
}

// --- External linkage ---

#[link(name = "KernelBase")]
unsafe extern "system" {
    pub fn QueryIoRingCapabilities(capabilities: *mut IORING_CAPABILITIES) -> HRESULT;

    pub fn IsIoRingOpSupported(ioRing: HIORING, op: IORING_OP_CODE) -> BOOL;

    pub fn CreateIoRing(
        ioringVersion: IORING_VERSION,
        flags: IORING_CREATE_FLAGS,
        submissionQueueSize: u32,
        completionQueueSize: u32,
        h: *mut HIORING,
    ) -> HRESULT;

    pub fn GetIoRingInfo(ioRing: HIORING, info: *mut IORING_INFO) -> HRESULT;

    pub fn SubmitIoRing(
        ioRing: HIORING,
        waitOperations: u32,
        milliseconds: u32,
        submittedEntries: *mut u32, // optional; pass null if not needed
    ) -> HRESULT;

    pub fn CloseIoRing(ioRing: HIORING) -> HRESULT;

    pub fn PopIoRingCompletion(ioRing: HIORING, cqe: *mut IORING_CQE) -> HRESULT;

    pub fn SetIoRingCompletionEvent(ioRing: HIORING, hEvent: HANDLE) -> HRESULT;

    // --- SQE builders (api-ms-win-core-ioring-l1-1-0) ---

    pub fn BuildIoRingCancelRequest(
        ioRing: HIORING,
        file: IORING_HANDLE_REF,
        opToCancel: usize,
        userData: usize,
    ) -> HRESULT;

    pub fn BuildIoRingReadFile(
        ioRing: HIORING,
        fileRef: IORING_HANDLE_REF,
        dataRef: IORING_BUFFER_REF,
        numberOfBytesToRead: u32,
        fileOffset: u64,
        userData: usize,
        sqeFlags: IORING_SQE_FLAGS,
    ) -> HRESULT;

    pub fn BuildIoRingRegisterFileHandles(
        ioRing: HIORING,
        count: u32,
        handles: *const HANDLE,
        userData: usize,
    ) -> HRESULT;

    pub fn BuildIoRingRegisterBuffers(
        ioRing: HIORING,
        count: u32,
        buffers: *const IORING_BUFFER_INFO,
        userData: usize,
    ) -> HRESULT;

    // --- SQE builders (api-ms-win-core-ioring-l1-1-1) ---

    pub fn BuildIoRingWriteFile(
        ioRing: HIORING,
        fileRef: IORING_HANDLE_REF,
        bufferRef: IORING_BUFFER_REF,
        numberOfBytesToWrite: u32,
        fileOffset: u64,
        writeFlags: FILE_WRITE_FLAGS,
        userData: usize,
        sqeFlags: IORING_SQE_FLAGS,
    ) -> HRESULT;

    pub fn BuildIoRingFlushFile(
        ioRing: HIORING,
        fileRef: IORING_HANDLE_REF,
        flushMode: FILE_FLUSH_MODE,
        userData: usize,
        sqeFlags: IORING_SQE_FLAGS,
    ) -> HRESULT;

    // --- SQE builders (api-ms-win-core-ioring-l1-1-2) ---

    pub fn BuildIoRingReadFileScatter(
        ioRing: HIORING,
        fileRef: IORING_HANDLE_REF,
        segmentCount: u32,
        segmentArray: *const FILE_SEGMENT_ELEMENT,
        numberOfBytesToRead: u32,
        fileOffset: u64,
        userData: usize,
        sqeFlags: IORING_SQE_FLAGS,
    ) -> HRESULT;

    pub fn BuildIoRingWriteFileGather(
        ioRing: HIORING,
        fileRef: IORING_HANDLE_REF,
        segmentCount: u32,
        segmentArray: *const FILE_SEGMENT_ELEMENT,
        numberOfBytesToWrite: u32,
        fileOffset: u64,
        writeFlags: FILE_WRITE_FLAGS,
        userData: usize,
        sqeFlags: IORING_SQE_FLAGS,
    ) -> HRESULT;
}
