// vk/surface.rs
//
// VkSurfaceKHR from the host's NativeSurfaceHandle.
//
// Ownership: the Host owns the native window. We borrow its handles to
// create a Vulkan surface. The surface is destroyed before the instance.

use ash::{Entry, Instance, khr, vk};

use rok_abi::surface::{NativeSurfaceHandle, SurfaceType};

use crate::error::{RendererResult, check};

// ---------------------------------------------------------------------------
// VulkanSurface
// ---------------------------------------------------------------------------

pub(crate) struct VulkanSurface {
    surface: vk::SurfaceKHR,
    loader: khr::surface::Instance,
}

impl VulkanSurface {
    /// Create a platform surface from the host's native handles.
    pub(crate) fn new(
        entry: &Entry,
        instance: &Instance,
        handle: &NativeSurfaceHandle,
    ) -> RendererResult<Self> {
        let loader = khr::surface::Instance::new(entry, instance);

        let surface = match handle.type_ {
            SurfaceType::Win32 => create_win32_surface(entry, instance, handle)?,
            SurfaceType::Wayland => create_wayland_surface(entry, instance, handle)?,
            SurfaceType::Android => create_android_surface(entry, instance, handle)?,
        };

        Ok(Self { surface, loader })
    }

    #[inline]
    pub(crate) fn handle(&self) -> vk::SurfaceKHR {
        self.surface
    }

    #[inline]
    pub(crate) fn loader(&self) -> &khr::surface::Instance {
        &self.loader
    }
}

impl Drop for VulkanSurface {
    fn drop(&mut self) {
        unsafe {
            self.loader.destroy_surface(self.surface, None);
        }
    }
}

// ---------------------------------------------------------------------------
// Platform surface creation
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn create_win32_surface(
    entry: &Entry,
    instance: &Instance,
    handle: &NativeSurfaceHandle,
) -> RendererResult<vk::SurfaceKHR> {
    let win32 = unsafe { handle.data.win32 };
    let create_info = vk::Win32SurfaceCreateInfoKHR::default()
        .hwnd(win32.hwnd as vk::HWND)
        .hinstance(win32.hinstance as vk::HINSTANCE);

    let loader = khr::win32_surface::Instance::new(entry, instance);
    unsafe {
        Ok(check!(
            loader.create_win32_surface(&create_info, None),
            "create Win32 surface"
        )?)
    }
}

#[cfg(not(target_os = "windows"))]
fn create_win32_surface(
    _entry: &Entry,
    _instance: &Instance,
    _handle: &NativeSurfaceHandle,
) -> RendererResult<vk::SurfaceKHR> {
    Err(crate::error::RendererError::Config(
        "Win32 surface requested on non-Windows platform",
    ))
}

#[cfg(target_os = "linux")]
fn create_wayland_surface(
    entry: &Entry,
    instance: &Instance,
    handle: &NativeSurfaceHandle,
) -> RendererResult<vk::SurfaceKHR> {
    let wayland = unsafe { handle.data.wayland };
    let create_info = vk::WaylandSurfaceCreateInfoKHR::default()
        .display(wayland.display as *mut _)
        .surface(wayland.surface as *mut _);

    let loader = khr::wayland_surface::Instance::new(entry, instance);
    unsafe {
        Ok(check!(
            loader.create_wayland_surface(&create_info, None),
            "create Wayland surface"
        )?)
    }
}

#[cfg(not(target_os = "linux"))]
fn create_wayland_surface(
    _entry: &Entry,
    _instance: &Instance,
    _handle: &NativeSurfaceHandle,
) -> RendererResult<vk::SurfaceKHR> {
    Err(crate::error::RendererError::Config(
        "Wayland surface requested on non-Linux platform",
    ))
}

#[cfg(target_os = "android")]
fn create_android_surface(
    entry: &Entry,
    instance: &Instance,
    handle: &NativeSurfaceHandle,
) -> RendererResult<vk::SurfaceKHR> {
    let android = unsafe { handle.data.android };
    let create_info = vk::AndroidSurfaceCreateInfoKHR::default().window(android.window as *mut _);

    let loader = khr::android_surface::Instance::new(entry, instance);
    unsafe {
        Ok(check!(
            loader.create_android_surface(&create_info, None),
            "create Android surface"
        )?)
    }
}

#[cfg(not(target_os = "android"))]
fn create_android_surface(
    _entry: &Entry,
    _instance: &Instance,
    _handle: &NativeSurfaceHandle,
) -> RendererResult<vk::SurfaceKHR> {
    Err(crate::error::RendererError::Config(
        "Android surface requested on non-Android platform",
    ))
}
