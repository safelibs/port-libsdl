use std::sync::OnceLock;

use crate::abi::generated_types::{SDL_Window, SDL_bool};

struct VulkanApi {
    load_library: unsafe extern "C" fn(*const libc::c_char) -> libc::c_int,
    get_vk_get_instance_proc_addr: unsafe extern "C" fn() -> *mut libc::c_void,
    unload_library: unsafe extern "C" fn(),
    get_instance_extensions:
        unsafe extern "C" fn(*mut SDL_Window, *mut u32, *mut *const libc::c_char) -> SDL_bool,
    create_surface: unsafe extern "C" fn(*mut SDL_Window, usize, *mut u64) -> SDL_bool,
    get_drawable_size: unsafe extern "C" fn(*mut SDL_Window, *mut libc::c_int, *mut libc::c_int),
}

fn api() -> &'static VulkanApi {
    static API: OnceLock<VulkanApi> = OnceLock::new();
    API.get_or_init(|| VulkanApi {
        load_library: crate::video::load_symbol(b"SDL_Vulkan_LoadLibrary\0"),
        get_vk_get_instance_proc_addr: crate::video::load_symbol(
            b"SDL_Vulkan_GetVkGetInstanceProcAddr\0",
        ),
        unload_library: crate::video::load_symbol(b"SDL_Vulkan_UnloadLibrary\0"),
        get_instance_extensions: crate::video::load_symbol(b"SDL_Vulkan_GetInstanceExtensions\0"),
        create_surface: crate::video::load_symbol(b"SDL_Vulkan_CreateSurface\0"),
        get_drawable_size: crate::video::load_symbol(b"SDL_Vulkan_GetDrawableSize\0"),
    })
}

#[no_mangle]
pub unsafe extern "C" fn SDL_Vulkan_LoadLibrary(path: *const libc::c_char) -> libc::c_int {
    crate::video::clear_real_error();
    (api().load_library)(path)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_Vulkan_GetVkGetInstanceProcAddr() -> *mut libc::c_void {
    crate::video::clear_real_error();
    (api().get_vk_get_instance_proc_addr)()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_Vulkan_UnloadLibrary() {
    crate::video::clear_real_error();
    (api().unload_library)();
}

#[no_mangle]
pub unsafe extern "C" fn SDL_Vulkan_GetInstanceExtensions(
    window: *mut SDL_Window,
    pCount: *mut u32,
    pNames: *mut *const libc::c_char,
) -> SDL_bool {
    crate::video::clear_real_error();

    if crate::video::window::is_stub_window(window) {
        let _ = (pCount, pNames);
        let _ = crate::core::error::set_error_message("Vulkan is not available for this window");
        return 0;
    }

    (api().get_instance_extensions)(window, pCount, pNames)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_Vulkan_CreateSurface(
    window: *mut SDL_Window,
    instance: usize,
    surface: *mut u64,
) -> SDL_bool {
    crate::video::clear_real_error();

    if crate::video::window::is_stub_window(window) {
        let _ = (instance, surface);
        let _ = crate::core::error::set_error_message("Vulkan is not available for this window");
        return 0;
    }

    (api().create_surface)(window, instance, surface)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_Vulkan_GetDrawableSize(
    window: *mut SDL_Window,
    w: *mut libc::c_int,
    h: *mut libc::c_int,
) {
    crate::video::clear_real_error();

    if crate::video::window::is_stub_window(window) {
        crate::video::window::SDL_GetWindowSizeInPixels(window, w, h);
        return;
    }

    (api().get_drawable_size)(window, w, h);
}
