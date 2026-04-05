use std::sync::OnceLock;

use crate::abi::generated_types::{SDL_GLContext, SDL_GLattr, SDL_MetalView, SDL_Window, SDL_bool};

struct GlApi {
    gl_load_library: unsafe extern "C" fn(*const libc::c_char) -> libc::c_int,
    gl_get_proc_address: unsafe extern "C" fn(*const libc::c_char) -> *mut libc::c_void,
    gl_unload_library: unsafe extern "C" fn(),
    gl_extension_supported: unsafe extern "C" fn(*const libc::c_char) -> SDL_bool,
    gl_reset_attributes: unsafe extern "C" fn(),
    gl_set_attribute: unsafe extern "C" fn(SDL_GLattr, libc::c_int) -> libc::c_int,
    gl_get_attribute: unsafe extern "C" fn(SDL_GLattr, *mut libc::c_int) -> libc::c_int,
    gl_create_context: unsafe extern "C" fn(*mut SDL_Window) -> SDL_GLContext,
    gl_make_current: unsafe extern "C" fn(*mut SDL_Window, SDL_GLContext) -> libc::c_int,
    gl_get_current_window: unsafe extern "C" fn() -> *mut SDL_Window,
    gl_get_current_context: unsafe extern "C" fn() -> SDL_GLContext,
    gl_get_drawable_size: unsafe extern "C" fn(*mut SDL_Window, *mut libc::c_int, *mut libc::c_int),
    gl_set_swap_interval: unsafe extern "C" fn(libc::c_int) -> libc::c_int,
    gl_get_swap_interval: unsafe extern "C" fn() -> libc::c_int,
    gl_swap_window: unsafe extern "C" fn(*mut SDL_Window),
    gl_delete_context: unsafe extern "C" fn(SDL_GLContext),
    metal_create_view: unsafe extern "C" fn(*mut SDL_Window) -> SDL_MetalView,
    metal_destroy_view: unsafe extern "C" fn(SDL_MetalView),
    metal_get_layer: unsafe extern "C" fn(SDL_MetalView) -> *mut libc::c_void,
    metal_get_drawable_size:
        unsafe extern "C" fn(*mut SDL_Window, *mut libc::c_int, *mut libc::c_int),
}

fn api() -> &'static GlApi {
    static API: OnceLock<GlApi> = OnceLock::new();
    API.get_or_init(|| GlApi {
        gl_load_library: crate::video::load_symbol(b"SDL_GL_LoadLibrary\0"),
        gl_get_proc_address: crate::video::load_symbol(b"SDL_GL_GetProcAddress\0"),
        gl_unload_library: crate::video::load_symbol(b"SDL_GL_UnloadLibrary\0"),
        gl_extension_supported: crate::video::load_symbol(b"SDL_GL_ExtensionSupported\0"),
        gl_reset_attributes: crate::video::load_symbol(b"SDL_GL_ResetAttributes\0"),
        gl_set_attribute: crate::video::load_symbol(b"SDL_GL_SetAttribute\0"),
        gl_get_attribute: crate::video::load_symbol(b"SDL_GL_GetAttribute\0"),
        gl_create_context: crate::video::load_symbol(b"SDL_GL_CreateContext\0"),
        gl_make_current: crate::video::load_symbol(b"SDL_GL_MakeCurrent\0"),
        gl_get_current_window: crate::video::load_symbol(b"SDL_GL_GetCurrentWindow\0"),
        gl_get_current_context: crate::video::load_symbol(b"SDL_GL_GetCurrentContext\0"),
        gl_get_drawable_size: crate::video::load_symbol(b"SDL_GL_GetDrawableSize\0"),
        gl_set_swap_interval: crate::video::load_symbol(b"SDL_GL_SetSwapInterval\0"),
        gl_get_swap_interval: crate::video::load_symbol(b"SDL_GL_GetSwapInterval\0"),
        gl_swap_window: crate::video::load_symbol(b"SDL_GL_SwapWindow\0"),
        gl_delete_context: crate::video::load_symbol(b"SDL_GL_DeleteContext\0"),
        metal_create_view: crate::video::load_symbol(b"SDL_Metal_CreateView\0"),
        metal_destroy_view: crate::video::load_symbol(b"SDL_Metal_DestroyView\0"),
        metal_get_layer: crate::video::load_symbol(b"SDL_Metal_GetLayer\0"),
        metal_get_drawable_size: crate::video::load_symbol(b"SDL_Metal_GetDrawableSize\0"),
    })
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_LoadLibrary(path: *const libc::c_char) -> libc::c_int {
    crate::video::clear_real_error();
    (api().gl_load_library)(path)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_GetProcAddress(
    proc_: *const libc::c_char,
) -> *mut libc::c_void {
    crate::video::clear_real_error();
    (api().gl_get_proc_address)(proc_)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_UnloadLibrary() {
    crate::video::clear_real_error();
    (api().gl_unload_library)();
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_ExtensionSupported(extension: *const libc::c_char) -> SDL_bool {
    crate::video::clear_real_error();
    (api().gl_extension_supported)(extension)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_ResetAttributes() {
    crate::video::clear_real_error();
    (api().gl_reset_attributes)();
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_SetAttribute(
    attr: SDL_GLattr,
    value: libc::c_int,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().gl_set_attribute)(attr, value)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_GetAttribute(
    attr: SDL_GLattr,
    value: *mut libc::c_int,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().gl_get_attribute)(attr, value)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_CreateContext(window: *mut SDL_Window) -> SDL_GLContext {
    crate::video::clear_real_error();
    (api().gl_create_context)(window)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_MakeCurrent(
    window: *mut SDL_Window,
    context: SDL_GLContext,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().gl_make_current)(window, context)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_GetCurrentWindow() -> *mut SDL_Window {
    crate::video::clear_real_error();
    (api().gl_get_current_window)()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_GetCurrentContext() -> SDL_GLContext {
    crate::video::clear_real_error();
    (api().gl_get_current_context)()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_GetDrawableSize(
    window: *mut SDL_Window,
    w: *mut libc::c_int,
    h: *mut libc::c_int,
) {
    crate::video::clear_real_error();
    (api().gl_get_drawable_size)(window, w, h);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_SetSwapInterval(interval: libc::c_int) -> libc::c_int {
    crate::video::clear_real_error();
    (api().gl_set_swap_interval)(interval)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_GetSwapInterval() -> libc::c_int {
    crate::video::clear_real_error();
    (api().gl_get_swap_interval)()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_SwapWindow(window: *mut SDL_Window) {
    crate::video::clear_real_error();
    (api().gl_swap_window)(window);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_DeleteContext(context: SDL_GLContext) {
    crate::video::clear_real_error();
    (api().gl_delete_context)(context);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_Metal_CreateView(window: *mut SDL_Window) -> SDL_MetalView {
    crate::video::clear_real_error();
    (api().metal_create_view)(window)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_Metal_DestroyView(view: SDL_MetalView) {
    crate::video::clear_real_error();
    (api().metal_destroy_view)(view);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_Metal_GetLayer(view: SDL_MetalView) -> *mut libc::c_void {
    crate::video::clear_real_error();
    (api().metal_get_layer)(view)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_Metal_GetDrawableSize(
    window: *mut SDL_Window,
    w: *mut libc::c_int,
    h: *mut libc::c_int,
) {
    crate::video::clear_real_error();
    (api().metal_get_drawable_size)(window, w, h);
}
