#![allow(dead_code)]

use std::ffi::CStr;

use crate::abi::generated_types::{
    SDL_Renderer, SDL_RendererFlags_SDL_RENDERER_SOFTWARE, SDL_RendererInfo, SDL_Surface,
};

type CreateSoftwareRendererFn = unsafe extern "C" fn(*mut SDL_Surface) -> *mut SDL_Renderer;
type GetRendererInfoFn =
    unsafe extern "C" fn(*mut SDL_Renderer, *mut SDL_RendererInfo) -> libc::c_int;

fn create_software_renderer_fn() -> CreateSoftwareRendererFn {
    static FN: std::sync::OnceLock<CreateSoftwareRendererFn> = std::sync::OnceLock::new();
    *FN.get_or_init(|| crate::video::load_symbol(b"SDL_CreateSoftwareRenderer\0"))
}

fn get_renderer_info_fn() -> GetRendererInfoFn {
    static FN: std::sync::OnceLock<GetRendererInfoFn> = std::sync::OnceLock::new();
    *FN.get_or_init(|| crate::video::load_symbol(b"SDL_GetRendererInfo\0"))
}

pub(crate) unsafe fn renderer_name(renderer: *mut SDL_Renderer) -> Option<String> {
    if renderer.is_null() {
        return None;
    }
    if crate::render::local::is_local_renderer(renderer) {
        return crate::render::local::renderer_name(renderer);
    }

    let mut info = std::mem::MaybeUninit::<SDL_RendererInfo>::zeroed();
    if get_renderer_info_fn()(renderer, info.as_mut_ptr()) != 0 {
        return None;
    }
    let info = info.assume_init();
    if info.name.is_null() {
        None
    } else {
        Some(CStr::from_ptr(info.name).to_string_lossy().into_owned())
    }
}

pub(crate) unsafe fn renderer_is_software(renderer: *mut SDL_Renderer) -> bool {
    if renderer.is_null() {
        return false;
    }
    if crate::render::local::is_local_renderer(renderer) {
        return crate::render::local::renderer_is_software(renderer);
    }

    let mut info = std::mem::MaybeUninit::<SDL_RendererInfo>::zeroed();
    if get_renderer_info_fn()(renderer, info.as_mut_ptr()) != 0 {
        return false;
    }
    (info.assume_init().flags & SDL_RendererFlags_SDL_RENDERER_SOFTWARE) != 0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CreateSoftwareRenderer(
    surface: *mut SDL_Surface,
) -> *mut SDL_Renderer {
    crate::video::clear_real_error();
    if surface.is_null() {
        let _ = crate::core::error::invalid_param_error("surface");
        return std::ptr::null_mut();
    }
    if crate::video::surface::is_registered_surface(surface)
        || !crate::video::real_sdl_is_available()
    {
        return crate::render::local::create_software_renderer(surface);
    }
    create_software_renderer_fn()(surface)
}
