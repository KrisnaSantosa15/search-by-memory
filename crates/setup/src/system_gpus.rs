//! The graphics cards of this machine, from DXGI: the adapter list DirectML uses.

use crate::Gpu;

pub fn system_gpus() -> Vec<Gpu> {
    platform::gpus()
}

#[cfg(windows)]
mod platform {
    use std::ffi::c_void;

    use crate::{Gpu, GpuVendor};

    #[repr(C)]
    struct Guid(u32, u16, u16, [u8; 8]);

    /// `IID_IDXGIFactory1`.
    const IID_FACTORY1: Guid = Guid(
        0x770aae78,
        0xf26f,
        0x4dba,
        [0xa8, 0x29, 0x25, 0x3c, 0x83, 0xd1, 0xb3, 0x87],
    );

    /// `DXGI_ADAPTER_DESC1`.
    #[repr(C)]
    struct AdapterDesc {
        description: [u16; 128],
        vendor_id: u32,
        device_id: u32,
        sub_sys_id: u32,
        revision: u32,
        dedicated_video_memory: usize,
        dedicated_system_memory: usize,
        shared_system_memory: usize,
        adapter_luid: (u32, i32),
        flags: u32,
    }

    const DXGI_ADAPTER_FLAG_SOFTWARE: u32 = 2;

    // Positions in the COM vtables (IUnknown, IDXGIObject, IDXGIFactory, ...).
    const RELEASE: usize = 2;
    const FACTORY1_ENUM_ADAPTERS1: usize = 12;
    const ADAPTER1_GET_DESC1: usize = 10;

    #[link(name = "dxgi")]
    unsafe extern "system" {
        fn CreateDXGIFactory1(riid: *const Guid, factory: *mut *mut c_void) -> i32;
    }

    type Release = unsafe extern "system" fn(*mut c_void) -> u32;
    type EnumAdapters1 = unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> i32;
    type GetDesc1 = unsafe extern "system" fn(*mut c_void, *mut AdapterDesc) -> i32;

    /// The function at `index` of the vtable of the COM object `object`.
    unsafe fn method<F: Copy>(object: *mut c_void, index: usize) -> F {
        unsafe {
            let vtable = *(object as *const *const usize);
            std::mem::transmute_copy(&*vtable.add(index))
        }
    }

    pub fn gpus() -> Vec<Gpu> {
        let mut gpus = Vec::new();
        unsafe {
            let mut factory = std::ptr::null_mut();
            if CreateDXGIFactory1(&IID_FACTORY1, &mut factory) < 0 {
                return gpus;
            }
            let enum_adapters: EnumAdapters1 = method(factory, FACTORY1_ENUM_ADAPTERS1);
            let mut index = 0;
            let mut adapter = std::ptr::null_mut();
            // Fails with DXGI_ERROR_NOT_FOUND past the last adapter.
            while enum_adapters(factory, index, &mut adapter) >= 0 {
                let get_desc: GetDesc1 = method(adapter, ADAPTER1_GET_DESC1);
                let mut desc: AdapterDesc = std::mem::zeroed();
                if get_desc(adapter, &mut desc) >= 0 && desc.flags & DXGI_ADAPTER_FLAG_SOFTWARE == 0
                {
                    gpus.push(gpu(&desc));
                }
                method::<Release>(adapter, RELEASE)(adapter);
                index += 1;
            }
            method::<Release>(factory, RELEASE)(factory);
        }
        gpus
    }

    fn gpu(desc: &AdapterDesc) -> Gpu {
        let length = desc.description.iter().position(|&c| c == 0).unwrap_or(128);
        Gpu {
            name: String::from_utf16_lossy(&desc.description[..length]),
            vendor: match desc.vendor_id {
                0x10de => GpuVendor::Nvidia,
                0x1002 => GpuVendor::Amd,
                0x8086 => GpuVendor::Intel,
                _ => GpuVendor::Other,
            },
            dedicated_memory: desc.dedicated_video_memory as u64,
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use crate::Gpu;

    pub fn gpus() -> Vec<Gpu> {
        Vec::new()
    }
}
