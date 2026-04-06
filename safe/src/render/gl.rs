use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

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

struct LocalGlContext {
    window: usize,
}

#[derive(Default)]
struct LocalGlState {
    attributes: HashMap<SDL_GLattr, libc::c_int>,
    contexts: HashMap<usize, Box<LocalGlContext>>,
    current_window: usize,
    current_context: usize,
    swap_interval: libc::c_int,
}

fn host_api() -> &'static GlApi {
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

fn host_api_if_available() -> Option<&'static GlApi> {
    crate::video::real_sdl_is_available().then(host_api)
}

fn local_gl_state() -> &'static Mutex<LocalGlState> {
    static STATE: OnceLock<Mutex<LocalGlState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(LocalGlState::default()))
}

fn lock_local_gl_state() -> std::sync::MutexGuard<'static, LocalGlState> {
    match local_gl_state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn reset_local_gl_state(state: &mut LocalGlState) {
    *state = LocalGlState::default();
}

unsafe fn create_local_context(window: *mut SDL_Window) -> SDL_GLContext {
    if !crate::video::window::is_stub_window(window) {
        crate::core::error::set_error_message("OpenGL is not available for this window");
        return std::ptr::null_mut();
    }

    let context = Box::new(LocalGlContext {
        window: window as usize,
    });
    let raw = (&*context) as *const LocalGlContext as SDL_GLContext;
    let mut state = lock_local_gl_state();
    state.contexts.insert(raw as usize, context);
    state.current_window = window as usize;
    state.current_context = raw as usize;
    raw
}

fn local_context_window(context: SDL_GLContext) -> Option<usize> {
    if context.is_null() {
        return None;
    }
    let state = lock_local_gl_state();
    state
        .contexts
        .get(&(context as usize))
        .map(|entry| entry.window)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_LoadLibrary(path: *const libc::c_char) -> libc::c_int {
    crate::video::clear_real_error();
    if let Some(api) = host_api_if_available() {
        return (api.gl_load_library)(path);
    }
    let _ = path;
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_GetProcAddress(proc_: *const libc::c_char) -> *mut libc::c_void {
    crate::video::clear_real_error();
    if let Some(api) = host_api_if_available() {
        return (api.gl_get_proc_address)(proc_);
    }
    let _ = proc_;
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_UnloadLibrary() {
    crate::video::clear_real_error();
    if let Some(api) = host_api_if_available() {
        (api.gl_unload_library)();
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_ExtensionSupported(extension: *const libc::c_char) -> SDL_bool {
    crate::video::clear_real_error();
    if let Some(api) = host_api_if_available() {
        return (api.gl_extension_supported)(extension);
    }
    let _ = extension;
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_ResetAttributes() {
    crate::video::clear_real_error();
    if let Some(api) = host_api_if_available() {
        (api.gl_reset_attributes)();
        return;
    }
    let mut state = lock_local_gl_state();
    reset_local_gl_state(&mut state);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_SetAttribute(attr: SDL_GLattr, value: libc::c_int) -> libc::c_int {
    crate::video::clear_real_error();
    if let Some(api) = host_api_if_available() {
        return (api.gl_set_attribute)(attr, value);
    }
    let mut state = lock_local_gl_state();
    state.attributes.insert(attr, value);
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_GetAttribute(
    attr: SDL_GLattr,
    value: *mut libc::c_int,
) -> libc::c_int {
    crate::video::clear_real_error();
    if let Some(api) = host_api_if_available() {
        return (api.gl_get_attribute)(attr, value);
    }
    if value.is_null() {
        return crate::core::error::invalid_param_error("value");
    }
    let state = lock_local_gl_state();
    *value = state.attributes.get(&attr).copied().unwrap_or(0);
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_CreateContext(window: *mut SDL_Window) -> SDL_GLContext {
    crate::video::clear_real_error();

    if crate::video::window::is_stub_window(window) {
        return create_local_context(window);
    }

    if let Some(api) = host_api_if_available() {
        return (api.gl_create_context)(window);
    }

    crate::core::error::set_error_message("OpenGL is not available for this window");
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_MakeCurrent(
    window: *mut SDL_Window,
    context: SDL_GLContext,
) -> libc::c_int {
    crate::video::clear_real_error();

    if crate::video::window::is_stub_window(window) || local_context_window(context).is_some() {
        let mut state = lock_local_gl_state();
        if context.is_null() {
            state.current_window = 0;
            state.current_context = 0;
            return 0;
        }
        let Some(context_window) = state
            .contexts
            .get(&(context as usize))
            .map(|entry| entry.window)
        else {
            return crate::core::error::set_error_message("Invalid OpenGL context");
        };
        if !window.is_null() && context_window != window as usize {
            return crate::core::error::set_error_message(
                "OpenGL context does not belong to this window",
            );
        }
        state.current_window = context_window;
        state.current_context = context as usize;
        return 0;
    }

    if let Some(api) = host_api_if_available() {
        return (api.gl_make_current)(window, context);
    }

    crate::core::error::set_error_message("OpenGL is not available for this window")
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_GetCurrentWindow() -> *mut SDL_Window {
    crate::video::clear_real_error();
    if let Some(api) = host_api_if_available() {
        return (api.gl_get_current_window)();
    }
    let state = lock_local_gl_state();
    state.current_window as *mut SDL_Window
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_GetCurrentContext() -> SDL_GLContext {
    crate::video::clear_real_error();
    if let Some(api) = host_api_if_available() {
        return (api.gl_get_current_context)();
    }
    let state = lock_local_gl_state();
    state.current_context as SDL_GLContext
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_GetDrawableSize(
    window: *mut SDL_Window,
    w: *mut libc::c_int,
    h: *mut libc::c_int,
) {
    crate::video::clear_real_error();

    if crate::video::window::is_stub_window(window) {
        crate::video::window::SDL_GetWindowSizeInPixels(window, w, h);
        return;
    }

    if let Some(api) = host_api_if_available() {
        (api.gl_get_drawable_size)(window, w, h);
    } else {
        let _ = crate::core::error::set_error_message("OpenGL is not available for this window");
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_SetSwapInterval(interval: libc::c_int) -> libc::c_int {
    crate::video::clear_real_error();
    if let Some(api) = host_api_if_available() {
        return (api.gl_set_swap_interval)(interval);
    }
    let mut state = lock_local_gl_state();
    state.swap_interval = interval;
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_GetSwapInterval() -> libc::c_int {
    crate::video::clear_real_error();
    if let Some(api) = host_api_if_available() {
        return (api.gl_get_swap_interval)();
    }
    let state = lock_local_gl_state();
    state.swap_interval
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_SwapWindow(window: *mut SDL_Window) {
    crate::video::clear_real_error();

    if crate::video::window::is_stub_window(window) {
        return;
    }

    if let Some(api) = host_api_if_available() {
        (api.gl_swap_window)(window);
    } else {
        let _ = crate::core::error::set_error_message("OpenGL is not available for this window");
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GL_DeleteContext(context: SDL_GLContext) {
    crate::video::clear_real_error();
    if context.is_null() {
        return;
    }

    {
        let mut state = lock_local_gl_state();
        if state.contexts.remove(&(context as usize)).is_some() {
            if state.current_context == context as usize {
                state.current_context = 0;
                state.current_window = 0;
            }
            return;
        }
    }

    if let Some(api) = host_api_if_available() {
        (api.gl_delete_context)(context);
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_Metal_CreateView(window: *mut SDL_Window) -> SDL_MetalView {
    crate::video::clear_real_error();

    if crate::video::window::is_stub_window(window) {
        crate::core::error::set_error_message("Metal is not available for this window");
        return std::ptr::null_mut();
    }

    if let Some(api) = host_api_if_available() {
        return (api.metal_create_view)(window);
    }

    crate::core::error::set_error_message("Metal is not available for this window");
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_Metal_DestroyView(view: SDL_MetalView) {
    crate::video::clear_real_error();
    if let Some(api) = host_api_if_available() {
        (api.metal_destroy_view)(view);
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_Metal_GetLayer(view: SDL_MetalView) -> *mut libc::c_void {
    crate::video::clear_real_error();
    if let Some(api) = host_api_if_available() {
        return (api.metal_get_layer)(view);
    }
    let _ = view;
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_Metal_GetDrawableSize(
    window: *mut SDL_Window,
    w: *mut libc::c_int,
    h: *mut libc::c_int,
) {
    crate::video::clear_real_error();

    if crate::video::window::is_stub_window(window) {
        crate::video::window::SDL_GetWindowSizeInPixels(window, w, h);
        return;
    }

    if let Some(api) = host_api_if_available() {
        (api.metal_get_drawable_size)(window, w, h);
    }
}
