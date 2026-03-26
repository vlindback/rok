// surface.rs
//
// Platform window/display handles passed from Host to Engine at init.
//
// Ownership: The Host owns the native window. The Engine receives a *borrowed*
// NativeSurfaceHandle and uses it to create a VkSurfaceKHR — nothing more.
// The handle pointer is only guaranteed valid during Engine::init. If the Engine
// needs to recreate the surface later (e.g. after device loss) the Host must
// call Engine::on_surface_changed with a fresh handle.

use core::ffi::c_void;

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SurfaceType {
    Win32 = 0,
    Wayland = 1,
    Android = 2,
}

/// Win32: HWND + HINSTANCE needed by vkCreateWin32SurfaceKHR.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Win32Surface {
    pub hwnd: *mut c_void,      // HWND
    pub hinstance: *mut c_void, // HINSTANCE
}

/// Wayland: wl_display + wl_surface needed by vkCreateWaylandSurfaceKHR.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct WaylandSurface {
    pub display: *mut c_void, // wl_display*
    pub surface: *mut c_void, // wl_surface*
}

/// Android: ANativeWindow* needed by vkCreateAndroidSurfaceKHR.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct AndroidSurface {
    pub window: *mut c_void, // ANativeWindow*
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union SurfaceData {
    pub win32: Win32Surface,
    pub wayland: WaylandSurface,
    pub android: AndroidSurface,
}

/// A borrowed snapshot of the platform window handles.
///
/// The Engine must not store this pointer beyond the call it was passed in.
/// If it needs the handles again later (surface recreation), the Host will
/// provide a fresh one via `EngineVTable::on_surface_changed`.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct NativeSurfaceHandle {
    pub type_: SurfaceType,
    pub data: SurfaceData,
    /// Current drawable dimensions in physical pixels.
    pub width: u32,
    pub height: u32,
}
