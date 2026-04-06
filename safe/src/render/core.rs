use std::collections::HashMap;
use std::ptr;
use std::sync::{Mutex, OnceLock};

use crate::abi::generated_types::{
    SDL_BlendMode, SDL_Color, SDL_FPoint, SDL_FRect, SDL_Point, SDL_Rect, SDL_Renderer,
    SDL_RendererFlip, SDL_RendererInfo, SDL_ScaleMode, SDL_ScaleMode_SDL_ScaleModeLinear,
    SDL_Surface, SDL_Texture, SDL_Vertex, SDL_Window, SDL_bool, Uint32, Uint8,
};

macro_rules! real_sdl_api {
    ($(fn $field:ident = $symbol:literal : $ty:ty;)+) => {
        struct RenderApi {
            $( $field: $ty, )+
        }

        fn api() -> &'static RenderApi {
            static API: OnceLock<RenderApi> = OnceLock::new();
            API.get_or_init(|| RenderApi {
                $( $field: crate::video::load_symbol(concat!($symbol, "\0").as_bytes()), )+
            })
        }
    };
}

real_sdl_api! {
    fn get_num_render_drivers = "SDL_GetNumRenderDrivers": unsafe extern "C" fn() -> libc::c_int;
    fn get_render_driver_info = "SDL_GetRenderDriverInfo": unsafe extern "C" fn(libc::c_int, *mut SDL_RendererInfo) -> libc::c_int;
    fn create_renderer = "SDL_CreateRenderer": unsafe extern "C" fn(*mut SDL_Window, libc::c_int, Uint32) -> *mut SDL_Renderer;
    fn get_renderer = "SDL_GetRenderer": unsafe extern "C" fn(*mut SDL_Window) -> *mut SDL_Renderer;
    fn render_get_window = "SDL_RenderGetWindow": unsafe extern "C" fn(*mut SDL_Renderer) -> *mut SDL_Window;
    fn get_renderer_info = "SDL_GetRendererInfo": unsafe extern "C" fn(*mut SDL_Renderer, *mut SDL_RendererInfo) -> libc::c_int;
    fn get_renderer_output_size = "SDL_GetRendererOutputSize": unsafe extern "C" fn(*mut SDL_Renderer, *mut libc::c_int, *mut libc::c_int) -> libc::c_int;
    fn create_texture = "SDL_CreateTexture": unsafe extern "C" fn(*mut SDL_Renderer, Uint32, libc::c_int, libc::c_int, libc::c_int) -> *mut SDL_Texture;
    fn create_texture_from_surface = "SDL_CreateTextureFromSurface": unsafe extern "C" fn(*mut SDL_Renderer, *mut SDL_Surface) -> *mut SDL_Texture;
    fn query_texture = "SDL_QueryTexture": unsafe extern "C" fn(*mut SDL_Texture, *mut Uint32, *mut libc::c_int, *mut libc::c_int, *mut libc::c_int) -> libc::c_int;
    fn set_texture_color_mod = "SDL_SetTextureColorMod": unsafe extern "C" fn(*mut SDL_Texture, Uint8, Uint8, Uint8) -> libc::c_int;
    fn get_texture_color_mod = "SDL_GetTextureColorMod": unsafe extern "C" fn(*mut SDL_Texture, *mut Uint8, *mut Uint8, *mut Uint8) -> libc::c_int;
    fn set_texture_alpha_mod = "SDL_SetTextureAlphaMod": unsafe extern "C" fn(*mut SDL_Texture, Uint8) -> libc::c_int;
    fn get_texture_alpha_mod = "SDL_GetTextureAlphaMod": unsafe extern "C" fn(*mut SDL_Texture, *mut Uint8) -> libc::c_int;
    fn set_texture_blend_mode = "SDL_SetTextureBlendMode": unsafe extern "C" fn(*mut SDL_Texture, SDL_BlendMode) -> libc::c_int;
    fn get_texture_blend_mode = "SDL_GetTextureBlendMode": unsafe extern "C" fn(*mut SDL_Texture, *mut SDL_BlendMode) -> libc::c_int;
    fn set_texture_scale_mode = "SDL_SetTextureScaleMode": unsafe extern "C" fn(*mut SDL_Texture, SDL_ScaleMode) -> libc::c_int;
    fn get_texture_scale_mode = "SDL_GetTextureScaleMode": unsafe extern "C" fn(*mut SDL_Texture, *mut SDL_ScaleMode) -> libc::c_int;
    fn set_texture_user_data = "SDL_SetTextureUserData": unsafe extern "C" fn(*mut SDL_Texture, *mut libc::c_void) -> libc::c_int;
    fn get_texture_user_data = "SDL_GetTextureUserData": unsafe extern "C" fn(*mut SDL_Texture) -> *mut libc::c_void;
    fn update_texture = "SDL_UpdateTexture": unsafe extern "C" fn(*mut SDL_Texture, *const SDL_Rect, *const libc::c_void, libc::c_int) -> libc::c_int;
    fn update_yuv_texture = "SDL_UpdateYUVTexture": unsafe extern "C" fn(*mut SDL_Texture, *const SDL_Rect, *const Uint8, libc::c_int, *const Uint8, libc::c_int, *const Uint8, libc::c_int) -> libc::c_int;
    fn update_nv_texture = "SDL_UpdateNVTexture": unsafe extern "C" fn(*mut SDL_Texture, *const SDL_Rect, *const Uint8, libc::c_int, *const Uint8, libc::c_int) -> libc::c_int;
    fn lock_texture = "SDL_LockTexture": unsafe extern "C" fn(*mut SDL_Texture, *const SDL_Rect, *mut *mut libc::c_void, *mut libc::c_int) -> libc::c_int;
    fn lock_texture_to_surface = "SDL_LockTextureToSurface": unsafe extern "C" fn(*mut SDL_Texture, *const SDL_Rect, *mut *mut SDL_Surface) -> libc::c_int;
    fn unlock_texture = "SDL_UnlockTexture": unsafe extern "C" fn(*mut SDL_Texture);
    fn render_target_supported = "SDL_RenderTargetSupported": unsafe extern "C" fn(*mut SDL_Renderer) -> SDL_bool;
    fn set_render_target = "SDL_SetRenderTarget": unsafe extern "C" fn(*mut SDL_Renderer, *mut SDL_Texture) -> libc::c_int;
    fn get_render_target = "SDL_GetRenderTarget": unsafe extern "C" fn(*mut SDL_Renderer) -> *mut SDL_Texture;
    fn render_set_logical_size = "SDL_RenderSetLogicalSize": unsafe extern "C" fn(*mut SDL_Renderer, libc::c_int, libc::c_int) -> libc::c_int;
    fn render_get_logical_size = "SDL_RenderGetLogicalSize": unsafe extern "C" fn(*mut SDL_Renderer, *mut libc::c_int, *mut libc::c_int);
    fn render_set_integer_scale = "SDL_RenderSetIntegerScale": unsafe extern "C" fn(*mut SDL_Renderer, SDL_bool) -> libc::c_int;
    fn render_get_integer_scale = "SDL_RenderGetIntegerScale": unsafe extern "C" fn(*mut SDL_Renderer) -> SDL_bool;
    fn render_set_viewport = "SDL_RenderSetViewport": unsafe extern "C" fn(*mut SDL_Renderer, *const SDL_Rect) -> libc::c_int;
    fn render_get_viewport = "SDL_RenderGetViewport": unsafe extern "C" fn(*mut SDL_Renderer, *mut SDL_Rect);
    fn render_set_clip_rect = "SDL_RenderSetClipRect": unsafe extern "C" fn(*mut SDL_Renderer, *const SDL_Rect) -> libc::c_int;
    fn render_get_clip_rect = "SDL_RenderGetClipRect": unsafe extern "C" fn(*mut SDL_Renderer, *mut SDL_Rect);
    fn render_is_clip_enabled = "SDL_RenderIsClipEnabled": unsafe extern "C" fn(*mut SDL_Renderer) -> SDL_bool;
    fn render_set_scale = "SDL_RenderSetScale": unsafe extern "C" fn(*mut SDL_Renderer, f32, f32) -> libc::c_int;
    fn render_get_scale = "SDL_RenderGetScale": unsafe extern "C" fn(*mut SDL_Renderer, *mut f32, *mut f32);
    fn render_window_to_logical = "SDL_RenderWindowToLogical": unsafe extern "C" fn(*mut SDL_Renderer, libc::c_int, libc::c_int, *mut f32, *mut f32) -> libc::c_int;
    fn render_logical_to_window = "SDL_RenderLogicalToWindow": unsafe extern "C" fn(*mut SDL_Renderer, f32, f32, *mut libc::c_int, *mut libc::c_int) -> libc::c_int;
    fn set_render_draw_color = "SDL_SetRenderDrawColor": unsafe extern "C" fn(*mut SDL_Renderer, Uint8, Uint8, Uint8, Uint8) -> libc::c_int;
    fn get_render_draw_color = "SDL_GetRenderDrawColor": unsafe extern "C" fn(*mut SDL_Renderer, *mut Uint8, *mut Uint8, *mut Uint8, *mut Uint8) -> libc::c_int;
    fn set_render_draw_blend_mode = "SDL_SetRenderDrawBlendMode": unsafe extern "C" fn(*mut SDL_Renderer, SDL_BlendMode) -> libc::c_int;
    fn get_render_draw_blend_mode = "SDL_GetRenderDrawBlendMode": unsafe extern "C" fn(*mut SDL_Renderer, *mut SDL_BlendMode) -> libc::c_int;
    fn render_clear = "SDL_RenderClear": unsafe extern "C" fn(*mut SDL_Renderer) -> libc::c_int;
    fn render_draw_point = "SDL_RenderDrawPoint": unsafe extern "C" fn(*mut SDL_Renderer, libc::c_int, libc::c_int) -> libc::c_int;
    fn render_draw_points = "SDL_RenderDrawPoints": unsafe extern "C" fn(*mut SDL_Renderer, *const crate::abi::generated_types::SDL_Point, libc::c_int) -> libc::c_int;
    fn render_draw_line = "SDL_RenderDrawLine": unsafe extern "C" fn(*mut SDL_Renderer, libc::c_int, libc::c_int, libc::c_int, libc::c_int) -> libc::c_int;
    fn render_draw_lines = "SDL_RenderDrawLines": unsafe extern "C" fn(*mut SDL_Renderer, *const crate::abi::generated_types::SDL_Point, libc::c_int) -> libc::c_int;
    fn render_draw_rect = "SDL_RenderDrawRect": unsafe extern "C" fn(*mut SDL_Renderer, *const SDL_Rect) -> libc::c_int;
    fn render_draw_rects = "SDL_RenderDrawRects": unsafe extern "C" fn(*mut SDL_Renderer, *const SDL_Rect, libc::c_int) -> libc::c_int;
    fn render_fill_rect = "SDL_RenderFillRect": unsafe extern "C" fn(*mut SDL_Renderer, *const SDL_Rect) -> libc::c_int;
    fn render_fill_rects = "SDL_RenderFillRects": unsafe extern "C" fn(*mut SDL_Renderer, *const SDL_Rect, libc::c_int) -> libc::c_int;
    fn render_copy = "SDL_RenderCopy": unsafe extern "C" fn(*mut SDL_Renderer, *mut SDL_Texture, *const SDL_Rect, *const SDL_Rect) -> libc::c_int;
    fn render_copy_ex = "SDL_RenderCopyEx": unsafe extern "C" fn(*mut SDL_Renderer, *mut SDL_Texture, *const SDL_Rect, *const SDL_Rect, f64, *const SDL_Point, SDL_RendererFlip) -> libc::c_int;
    fn render_draw_point_f = "SDL_RenderDrawPointF": unsafe extern "C" fn(*mut SDL_Renderer, f32, f32) -> libc::c_int;
    fn render_draw_points_f = "SDL_RenderDrawPointsF": unsafe extern "C" fn(*mut SDL_Renderer, *const SDL_FPoint, libc::c_int) -> libc::c_int;
    fn render_draw_line_f = "SDL_RenderDrawLineF": unsafe extern "C" fn(*mut SDL_Renderer, f32, f32, f32, f32) -> libc::c_int;
    fn render_draw_lines_f = "SDL_RenderDrawLinesF": unsafe extern "C" fn(*mut SDL_Renderer, *const SDL_FPoint, libc::c_int) -> libc::c_int;
    fn render_draw_rect_f = "SDL_RenderDrawRectF": unsafe extern "C" fn(*mut SDL_Renderer, *const SDL_FRect) -> libc::c_int;
    fn render_draw_rects_f = "SDL_RenderDrawRectsF": unsafe extern "C" fn(*mut SDL_Renderer, *const SDL_FRect, libc::c_int) -> libc::c_int;
    fn render_fill_rect_f = "SDL_RenderFillRectF": unsafe extern "C" fn(*mut SDL_Renderer, *const SDL_FRect) -> libc::c_int;
    fn render_fill_rects_f = "SDL_RenderFillRectsF": unsafe extern "C" fn(*mut SDL_Renderer, *const SDL_FRect, libc::c_int) -> libc::c_int;
    fn render_copy_f = "SDL_RenderCopyF": unsafe extern "C" fn(*mut SDL_Renderer, *mut SDL_Texture, *const SDL_Rect, *const SDL_FRect) -> libc::c_int;
    fn render_copy_ex_f = "SDL_RenderCopyExF": unsafe extern "C" fn(*mut SDL_Renderer, *mut SDL_Texture, *const SDL_Rect, *const SDL_FRect, f64, *const SDL_FPoint, SDL_RendererFlip) -> libc::c_int;
    fn render_geometry = "SDL_RenderGeometry": unsafe extern "C" fn(*mut SDL_Renderer, *mut SDL_Texture, *const SDL_Vertex, libc::c_int, *const libc::c_int, libc::c_int) -> libc::c_int;
    fn render_geometry_raw = "SDL_RenderGeometryRaw": unsafe extern "C" fn(*mut SDL_Renderer, *mut SDL_Texture, *const f32, libc::c_int, *const SDL_Color, libc::c_int, *const f32, libc::c_int, libc::c_int, *const libc::c_void, libc::c_int, libc::c_int) -> libc::c_int;
    fn render_read_pixels = "SDL_RenderReadPixels": unsafe extern "C" fn(*mut SDL_Renderer, *const SDL_Rect, Uint32, *mut libc::c_void, libc::c_int) -> libc::c_int;
    fn render_present = "SDL_RenderPresent": unsafe extern "C" fn(*mut SDL_Renderer);
    fn destroy_texture = "SDL_DestroyTexture": unsafe extern "C" fn(*mut SDL_Texture);
    fn destroy_renderer = "SDL_DestroyRenderer": unsafe extern "C" fn(*mut SDL_Renderer);
    fn render_flush = "SDL_RenderFlush": unsafe extern "C" fn(*mut SDL_Renderer) -> libc::c_int;
    fn gl_bind_texture = "SDL_GL_BindTexture": unsafe extern "C" fn(*mut SDL_Texture, *mut f32, *mut f32) -> libc::c_int;
    fn gl_unbind_texture = "SDL_GL_UnbindTexture": unsafe extern "C" fn(*mut SDL_Texture) -> libc::c_int;
    fn render_get_metal_layer = "SDL_RenderGetMetalLayer": unsafe extern "C" fn(*mut SDL_Renderer) -> *mut libc::c_void;
    fn render_get_metal_command_encoder = "SDL_RenderGetMetalCommandEncoder": unsafe extern "C" fn(*mut SDL_Renderer) -> *mut libc::c_void;
    fn render_set_vsync = "SDL_RenderSetVSync": unsafe extern "C" fn(*mut SDL_Renderer, libc::c_int) -> libc::c_int;
}

struct ManagedTexture {
    raw: *mut SDL_Texture,
    renderer: *mut SDL_Renderer,
    _shadow: crate::render::gles::ShadowTexture,
}

#[derive(Default)]
struct ManagedTextureRegistry {
    by_raw: HashMap<usize, usize>,
}

#[derive(Default)]
struct ManagedRendererRegistry {
    by_window: HashMap<usize, usize>,
}

const MANAGED_TEXTURE_TAG: usize = 1;

fn managed_texture_registry() -> &'static Mutex<ManagedTextureRegistry> {
    static REGISTRY: OnceLock<Mutex<ManagedTextureRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(ManagedTextureRegistry::default()))
}

fn managed_renderer_registry() -> &'static Mutex<ManagedRendererRegistry> {
    static REGISTRY: OnceLock<Mutex<ManagedRendererRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(ManagedRendererRegistry::default()))
}

fn managed_texture_ptr(address: usize) -> *mut ManagedTexture {
    (address & !MANAGED_TEXTURE_TAG) as *mut ManagedTexture
}

fn managed_texture_handle(managed: *mut ManagedTexture) -> *mut SDL_Texture {
    ((managed as usize) | MANAGED_TEXTURE_TAG) as *mut SDL_Texture
}

unsafe fn unwrap_texture(texture: *mut SDL_Texture) -> *mut SDL_Texture {
    if texture.is_null() {
        return ptr::null_mut();
    }
    if crate::render::local::is_local_texture(texture) {
        return ptr::null_mut();
    }
    if (texture as usize & MANAGED_TEXTURE_TAG) == 0 {
        texture
    } else {
        (*managed_texture_ptr(texture as usize)).raw
    }
}

unsafe fn wrap_texture(raw: *mut SDL_Texture) -> *mut SDL_Texture {
    if raw.is_null() {
        return ptr::null_mut();
    }

    let registry = managed_texture_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    registry
        .by_raw
        .get(&(raw as usize))
        .map(|managed| *managed as *mut SDL_Texture)
        .unwrap_or(raw)
}

unsafe fn register_texture(
    raw: *mut SDL_Texture,
    renderer: *mut SDL_Renderer,
    shadow: crate::render::gles::ShadowTexture,
) -> *mut SDL_Texture {
    let managed = Box::new(ManagedTexture {
        raw,
        renderer,
        _shadow: shadow,
    });
    let managed = Box::into_raw(managed);
    let handle = managed_texture_handle(managed);
    let mut registry = managed_texture_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    registry.by_raw.insert(raw as usize, handle as usize);
    handle
}

unsafe fn take_texture(texture: *mut SDL_Texture) -> Option<Box<ManagedTexture>> {
    if texture.is_null() || (texture as usize & MANAGED_TEXTURE_TAG) == 0 {
        return None;
    }

    let mut registry = managed_texture_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    registry
        .by_raw
        .remove(&((*managed_texture_ptr(texture as usize)).raw as usize));
    drop(registry);
    Some(Box::from_raw(managed_texture_ptr(texture as usize)))
}

unsafe fn clear_renderer_textures(renderer: *mut SDL_Renderer) {
    let handles: Vec<usize> = {
        let registry = managed_texture_registry()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        registry
            .by_raw
            .iter()
            .filter_map(|(_, handle)| {
                ((*managed_texture_ptr(*handle)).renderer == renderer).then_some(*handle)
            })
            .collect()
    };

    for handle in handles {
        let _ = take_texture(handle as *mut SDL_Texture);
    }
}

unsafe fn register_window_renderer(
    window: *mut SDL_Window,
    renderer: *mut SDL_Renderer,
) -> *mut SDL_Renderer {
    let mut registry = managed_renderer_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    registry
        .by_window
        .insert(window as usize, renderer as usize);
    renderer
}

fn renderer_for_window(window: *mut SDL_Window) -> *mut SDL_Renderer {
    if window.is_null() {
        return ptr::null_mut();
    }
    let registry = managed_renderer_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    registry
        .by_window
        .get(&(window as usize))
        .copied()
        .unwrap_or(0) as *mut SDL_Renderer
}

unsafe fn take_window_renderer(renderer: *mut SDL_Renderer) -> Option<*mut SDL_Renderer> {
    if renderer.is_null() {
        return None;
    }

    let mut registry = managed_renderer_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let window = registry
        .by_window
        .iter()
        .find_map(|(window, handle)| (*handle == renderer as usize).then_some(*window))?;
    registry.by_window.remove(&window);
    Some(renderer)
}

unsafe fn take_window_renderer_for_window(window: *mut SDL_Window) -> Option<*mut SDL_Renderer> {
    if window.is_null() {
        return None;
    }

    let mut registry = managed_renderer_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    registry
        .by_window
        .remove(&(window as usize))
        .map(|renderer| renderer as *mut SDL_Renderer)
}

pub(crate) fn window_has_renderer(window: *mut SDL_Window) -> bool {
    !renderer_for_window(window).is_null()
}

pub(crate) unsafe fn destroy_window_renderer(window: *mut SDL_Window) {
    if let Some(renderer) = take_window_renderer_for_window(window) {
        SDL_DestroyRenderer(renderer);
    }
}

pub(crate) unsafe fn reset_managed_window_renderers() {
    let renderers: Vec<usize> = {
        let registry = managed_renderer_registry()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        registry.by_window.values().copied().collect()
    };

    for renderer in renderers {
        if let Some(renderer) = take_window_renderer(renderer as *mut SDL_Renderer) {
            SDL_DestroyRenderer(renderer);
        }
    }
}

unsafe fn renderer_window(renderer: *mut SDL_Renderer) -> Option<*mut SDL_Window> {
    if renderer.is_null() {
        return None;
    }
    crate::render::local::renderer_window(renderer)
}

unsafe fn use_local_driver_inventory() -> bool {
    let driver = crate::video::display::SDL_GetCurrentVideoDriver();
    if !driver.is_null() {
        let name = std::ffi::CStr::from_ptr(driver).to_bytes();
        if matches!(name, b"dummy" | b"offscreen") {
            return true;
        }
    }
    !crate::video::real_sdl_is_available()
}

macro_rules! forward_ret {
    ($(fn $name:ident($($arg:ident: $ty:ty),* $(,)?) -> $ret:ty = $field:ident;)+) => {
        $(
            #[no_mangle]
            pub unsafe extern "C" fn $name($($arg: $ty),*) -> $ret {
                crate::video::clear_real_error();
                (api().$field)($($arg),*)
            }
        )+
    };
}

macro_rules! forward_void {
    ($(fn $name:ident($($arg:ident: $ty:ty),* $(,)?) = $field:ident;)+) => {
        $(
            #[no_mangle]
            pub unsafe extern "C" fn $name($($arg: $ty),*) {
                crate::video::clear_real_error();
                (api().$field)($($arg),*);
            }
        )+
    };
}

macro_rules! forward_texture_first_ret {
    ($(fn $name:ident($texture:ident: *mut SDL_Texture $(, $arg:ident: $ty:ty)*) -> $ret:ty = $field:ident;)+) => {
        $(
            #[no_mangle]
            pub unsafe extern "C" fn $name($texture: *mut SDL_Texture, $($arg: $ty),*) -> $ret {
                crate::video::clear_real_error();
                (api().$field)(unwrap_texture($texture) $(, $arg)*)
            }
        )+
    };
}

macro_rules! forward_texture_first_void {
    ($(fn $name:ident($texture:ident: *mut SDL_Texture $(, $arg:ident: $ty:ty)*) = $field:ident;)+) => {
        $(
            #[no_mangle]
            pub unsafe extern "C" fn $name($texture: *mut SDL_Texture, $($arg: $ty),*) {
                crate::video::clear_real_error();
                (api().$field)(unwrap_texture($texture) $(, $arg)*);
            }
        )+
    };
}

macro_rules! forward_texture_second_ret {
    ($(fn $name:ident($renderer:ident: $rty:ty, $texture:ident: *mut SDL_Texture $(, $arg:ident: $ty:ty)*) -> $ret:ty = $field:ident;)+) => {
        $(
            #[no_mangle]
            pub unsafe extern "C" fn $name($renderer: $rty, $texture: *mut SDL_Texture, $($arg: $ty),*) -> $ret {
                crate::video::clear_real_error();
                (api().$field)($renderer, unwrap_texture($texture) $(, $arg)*)
            }
        )+
    };
}

macro_rules! forward_renderer_first_ret_local {
    ($(fn $name:ident($renderer:ident: *mut SDL_Renderer $(, $arg:ident: $ty:ty)*) -> $ret:ty = $field:ident => $local:path;)+) => {
        $(
            #[no_mangle]
            pub unsafe extern "C" fn $name($renderer: *mut SDL_Renderer, $($arg: $ty),*) -> $ret {
                crate::video::clear_real_error();
                if crate::render::local::is_local_renderer($renderer) {
                    return $local($renderer $(, $arg)*);
                }
                (api().$field)($renderer $(, $arg)*)
            }
        )+
    };
}

macro_rules! forward_renderer_first_void_local {
    ($(fn $name:ident($renderer:ident: *mut SDL_Renderer $(, $arg:ident: $ty:ty)*) = $field:ident => $local:path;)+) => {
        $(
            #[no_mangle]
            pub unsafe extern "C" fn $name($renderer: *mut SDL_Renderer, $($arg: $ty),*) {
                crate::video::clear_real_error();
                if crate::render::local::is_local_renderer($renderer) {
                    $local($renderer $(, $arg)*);
                } else {
                    (api().$field)($renderer $(, $arg)*);
                }
            }
        )+
    };
}

macro_rules! forward_texture_first_ret_local {
    ($(fn $name:ident($texture:ident: *mut SDL_Texture $(, $arg:ident: $ty:ty)*) -> $ret:ty = $field:ident => $local:path;)+) => {
        $(
            #[no_mangle]
            pub unsafe extern "C" fn $name($texture: *mut SDL_Texture, $($arg: $ty),*) -> $ret {
                crate::video::clear_real_error();
                if crate::render::local::is_local_texture($texture) {
                    return $local($texture $(, $arg)*);
                }
                (api().$field)(unwrap_texture($texture) $(, $arg)*)
            }
        )+
    };
}

macro_rules! forward_texture_first_void_local {
    ($(fn $name:ident($texture:ident: *mut SDL_Texture $(, $arg:ident: $ty:ty)*) = $field:ident => $local:path;)+) => {
        $(
            #[no_mangle]
            pub unsafe extern "C" fn $name($texture: *mut SDL_Texture, $($arg: $ty),*) {
                crate::video::clear_real_error();
                if crate::render::local::is_local_texture($texture) {
                    $local($texture $(, $arg)*);
                } else {
                    (api().$field)(unwrap_texture($texture) $(, $arg)*);
                }
            }
        )+
    };
}

macro_rules! forward_texture_second_ret_local {
    ($(fn $name:ident($renderer:ident: *mut SDL_Renderer, $texture:ident: *mut SDL_Texture $(, $arg:ident: $ty:ty)*) -> $ret:ty = $field:ident => $local:path;)+) => {
        $(
            #[no_mangle]
            pub unsafe extern "C" fn $name($renderer: *mut SDL_Renderer, $texture: *mut SDL_Texture, $($arg: $ty),*) -> $ret {
                crate::video::clear_real_error();
                if crate::render::local::is_local_renderer($renderer)
                    || crate::render::local::is_local_texture($texture)
                {
                    return $local($renderer, $texture $(, $arg)*);
                }
                (api().$field)($renderer, unwrap_texture($texture) $(, $arg)*)
            }
        )+
    };
}

forward_renderer_first_ret_local! {
    fn SDL_GetRendererInfo(renderer: *mut SDL_Renderer, info: *mut SDL_RendererInfo) -> libc::c_int = get_renderer_info => crate::render::local::get_renderer_info;
    fn SDL_GetRendererOutputSize(renderer: *mut SDL_Renderer, w: *mut libc::c_int, h: *mut libc::c_int) -> libc::c_int = get_renderer_output_size => crate::render::local::get_renderer_output_size;
    fn SDL_RenderTargetSupported(renderer: *mut SDL_Renderer) -> SDL_bool = render_target_supported => crate::render::local::render_target_supported;
    fn SDL_RenderSetLogicalSize(renderer: *mut SDL_Renderer, w: libc::c_int, h: libc::c_int) -> libc::c_int = render_set_logical_size => crate::render::local::render_set_logical_size;
    fn SDL_RenderSetIntegerScale(renderer: *mut SDL_Renderer, enable: SDL_bool) -> libc::c_int = render_set_integer_scale => crate::render::local::render_set_integer_scale;
    fn SDL_RenderGetIntegerScale(renderer: *mut SDL_Renderer) -> SDL_bool = render_get_integer_scale => crate::render::local::render_get_integer_scale;
    fn SDL_RenderSetViewport(renderer: *mut SDL_Renderer, rect: *const SDL_Rect) -> libc::c_int = render_set_viewport => crate::render::local::render_set_viewport;
    fn SDL_RenderSetClipRect(renderer: *mut SDL_Renderer, rect: *const SDL_Rect) -> libc::c_int = render_set_clip_rect => crate::render::local::render_set_clip_rect;
    fn SDL_RenderIsClipEnabled(renderer: *mut SDL_Renderer) -> SDL_bool = render_is_clip_enabled => crate::render::local::render_is_clip_enabled;
    fn SDL_RenderSetScale(renderer: *mut SDL_Renderer, scaleX: f32, scaleY: f32) -> libc::c_int = render_set_scale => crate::render::local::render_set_scale;
    fn SDL_RenderWindowToLogical(renderer: *mut SDL_Renderer, windowX: libc::c_int, windowY: libc::c_int, logicalX: *mut f32, logicalY: *mut f32) -> libc::c_int = render_window_to_logical => crate::render::local::render_window_to_logical;
    fn SDL_RenderLogicalToWindow(renderer: *mut SDL_Renderer, logicalX: f32, logicalY: f32, windowX: *mut libc::c_int, windowY: *mut libc::c_int) -> libc::c_int = render_logical_to_window => crate::render::local::render_logical_to_window;
    fn SDL_SetRenderDrawColor(renderer: *mut SDL_Renderer, r: Uint8, g: Uint8, b: Uint8, a: Uint8) -> libc::c_int = set_render_draw_color => crate::render::local::set_render_draw_color;
    fn SDL_GetRenderDrawColor(renderer: *mut SDL_Renderer, r: *mut Uint8, g: *mut Uint8, b: *mut Uint8, a: *mut Uint8) -> libc::c_int = get_render_draw_color => crate::render::local::get_render_draw_color;
    fn SDL_SetRenderDrawBlendMode(renderer: *mut SDL_Renderer, blendMode: SDL_BlendMode) -> libc::c_int = set_render_draw_blend_mode => crate::render::local::set_render_draw_blend_mode;
    fn SDL_GetRenderDrawBlendMode(renderer: *mut SDL_Renderer, blendMode: *mut SDL_BlendMode) -> libc::c_int = get_render_draw_blend_mode => crate::render::local::get_render_draw_blend_mode;
    fn SDL_RenderClear(renderer: *mut SDL_Renderer) -> libc::c_int = render_clear => crate::render::local::render_clear;
    fn SDL_RenderDrawPoint(renderer: *mut SDL_Renderer, x: libc::c_int, y: libc::c_int) -> libc::c_int = render_draw_point => crate::render::local::render_draw_point;
    fn SDL_RenderDrawPoints(renderer: *mut SDL_Renderer, points: *const crate::abi::generated_types::SDL_Point, count: libc::c_int) -> libc::c_int = render_draw_points => crate::render::local::render_draw_points;
    fn SDL_RenderDrawLine(renderer: *mut SDL_Renderer, x1: libc::c_int, y1: libc::c_int, x2: libc::c_int, y2: libc::c_int) -> libc::c_int = render_draw_line => crate::render::local::render_draw_line;
    fn SDL_RenderDrawLines(renderer: *mut SDL_Renderer, points: *const crate::abi::generated_types::SDL_Point, count: libc::c_int) -> libc::c_int = render_draw_lines => crate::render::local::render_draw_lines;
    fn SDL_RenderDrawRect(renderer: *mut SDL_Renderer, rect: *const SDL_Rect) -> libc::c_int = render_draw_rect => crate::render::local::render_draw_rect;
    fn SDL_RenderDrawRects(renderer: *mut SDL_Renderer, rects: *const SDL_Rect, count: libc::c_int) -> libc::c_int = render_draw_rects => crate::render::local::render_draw_rects;
    fn SDL_RenderFillRect(renderer: *mut SDL_Renderer, rect: *const SDL_Rect) -> libc::c_int = render_fill_rect => crate::render::local::render_fill_rect;
    fn SDL_RenderFillRects(renderer: *mut SDL_Renderer, rects: *const SDL_Rect, count: libc::c_int) -> libc::c_int = render_fill_rects => crate::render::local::render_fill_rects;
    fn SDL_RenderDrawPointF(renderer: *mut SDL_Renderer, x: f32, y: f32) -> libc::c_int = render_draw_point_f => crate::render::local::render_draw_point_f;
    fn SDL_RenderDrawPointsF(renderer: *mut SDL_Renderer, points: *const SDL_FPoint, count: libc::c_int) -> libc::c_int = render_draw_points_f => crate::render::local::render_draw_points_f;
    fn SDL_RenderDrawLineF(renderer: *mut SDL_Renderer, x1: f32, y1: f32, x2: f32, y2: f32) -> libc::c_int = render_draw_line_f => crate::render::local::render_draw_line_f;
    fn SDL_RenderDrawLinesF(renderer: *mut SDL_Renderer, points: *const SDL_FPoint, count: libc::c_int) -> libc::c_int = render_draw_lines_f => crate::render::local::render_draw_lines_f;
    fn SDL_RenderDrawRectF(renderer: *mut SDL_Renderer, rect: *const SDL_FRect) -> libc::c_int = render_draw_rect_f => crate::render::local::render_draw_rect_f;
    fn SDL_RenderDrawRectsF(renderer: *mut SDL_Renderer, rects: *const SDL_FRect, count: libc::c_int) -> libc::c_int = render_draw_rects_f => crate::render::local::render_draw_rects_f;
    fn SDL_RenderFillRectF(renderer: *mut SDL_Renderer, rect: *const SDL_FRect) -> libc::c_int = render_fill_rect_f => crate::render::local::render_fill_rect_f;
    fn SDL_RenderFillRectsF(renderer: *mut SDL_Renderer, rects: *const SDL_FRect, count: libc::c_int) -> libc::c_int = render_fill_rects_f => crate::render::local::render_fill_rects_f;
    fn SDL_RenderReadPixels(renderer: *mut SDL_Renderer, rect: *const SDL_Rect, format: Uint32, pixels: *mut libc::c_void, pitch: libc::c_int) -> libc::c_int = render_read_pixels => crate::render::local::render_read_pixels;
    fn SDL_RenderFlush(renderer: *mut SDL_Renderer) -> libc::c_int = render_flush => crate::render::local::render_flush;
    fn SDL_RenderGetMetalLayer(renderer: *mut SDL_Renderer) -> *mut libc::c_void = render_get_metal_layer => crate::render::local::render_get_metal_layer;
    fn SDL_RenderGetMetalCommandEncoder(renderer: *mut SDL_Renderer) -> *mut libc::c_void = render_get_metal_command_encoder => crate::render::local::render_get_metal_command_encoder;
    fn SDL_RenderSetVSync(renderer: *mut SDL_Renderer, vsync: libc::c_int) -> libc::c_int = render_set_vsync => crate::render::local::render_set_vsync;
}

forward_renderer_first_void_local! {
    fn SDL_RenderGetLogicalSize(renderer: *mut SDL_Renderer, w: *mut libc::c_int, h: *mut libc::c_int) = render_get_logical_size => crate::render::local::render_get_logical_size;
    fn SDL_RenderGetViewport(renderer: *mut SDL_Renderer, rect: *mut SDL_Rect) = render_get_viewport => crate::render::local::render_get_viewport;
    fn SDL_RenderGetClipRect(renderer: *mut SDL_Renderer, rect: *mut SDL_Rect) = render_get_clip_rect => crate::render::local::render_get_clip_rect;
    fn SDL_RenderGetScale(renderer: *mut SDL_Renderer, scaleX: *mut f32, scaleY: *mut f32) = render_get_scale => crate::render::local::render_get_scale;
    fn SDL_RenderPresent(renderer: *mut SDL_Renderer) = render_present => crate::render::local::render_present;
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CreateRenderer(
    window: *mut SDL_Window,
    index: libc::c_int,
    flags: Uint32,
) -> *mut SDL_Renderer {
    crate::video::clear_real_error();

    if crate::video::window::is_stub_window(window) {
        if crate::video::window::stub_window_has_surface(window).unwrap_or(false) {
            crate::core::error::set_error_message("Surface already associated with window");
            return ptr::null_mut();
        }
        if window_has_renderer(window) {
            crate::core::error::set_error_message("Renderer already associated with window");
            return ptr::null_mut();
        }

        let (width, height) = match crate::video::window::stub_window_size(window) {
            Ok(size) => size,
            Err(()) => return ptr::null_mut(),
        };
        let renderer = crate::render::local::create_window_renderer(window, width, height);
        if renderer.is_null() {
            return ptr::null_mut();
        }

        let _ = (index, flags);
        return register_window_renderer(window, renderer);
    }

    (api().create_renderer)(window, index, flags)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetRenderer(window: *mut SDL_Window) -> *mut SDL_Renderer {
    crate::video::clear_real_error();

    if crate::video::window::is_stub_window(window) {
        return renderer_for_window(window);
    }

    (api().get_renderer)(window)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_RenderGetWindow(renderer: *mut SDL_Renderer) -> *mut SDL_Window {
    crate::video::clear_real_error();

    if let Some(window) = renderer_window(renderer) {
        return window;
    }

    (api().render_get_window)(renderer)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetNumRenderDrivers() -> libc::c_int {
    crate::video::clear_real_error();
    if use_local_driver_inventory() {
        return crate::render::local::get_num_render_drivers();
    }
    (api().get_num_render_drivers)()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetRenderDriverInfo(
    index: libc::c_int,
    info: *mut SDL_RendererInfo,
) -> libc::c_int {
    crate::video::clear_real_error();
    if use_local_driver_inventory() {
        return crate::render::local::get_render_driver_info(index, info);
    }
    (api().get_render_driver_info)(index, info)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CreateTextureFromSurface(
    renderer: *mut SDL_Renderer,
    surface: *mut SDL_Surface,
) -> *mut SDL_Texture {
    crate::video::clear_real_error();
    if crate::render::local::is_local_renderer(renderer) {
        return crate::render::local::create_texture_from_surface(renderer, surface);
    }
    (api().create_texture_from_surface)(renderer, surface)
}

forward_texture_first_ret_local! {
    fn SDL_QueryTexture(texture: *mut SDL_Texture, format: *mut Uint32, access: *mut libc::c_int, w: *mut libc::c_int, h: *mut libc::c_int) -> libc::c_int = query_texture => crate::render::local::query_texture;
    fn SDL_SetTextureColorMod(texture: *mut SDL_Texture, r: Uint8, g: Uint8, b: Uint8) -> libc::c_int = set_texture_color_mod => crate::render::local::set_texture_color_mod;
    fn SDL_GetTextureColorMod(texture: *mut SDL_Texture, r: *mut Uint8, g: *mut Uint8, b: *mut Uint8) -> libc::c_int = get_texture_color_mod => crate::render::local::get_texture_color_mod;
    fn SDL_SetTextureAlphaMod(texture: *mut SDL_Texture, alpha: Uint8) -> libc::c_int = set_texture_alpha_mod => crate::render::local::set_texture_alpha_mod;
    fn SDL_GetTextureAlphaMod(texture: *mut SDL_Texture, alpha: *mut Uint8) -> libc::c_int = get_texture_alpha_mod => crate::render::local::get_texture_alpha_mod;
    fn SDL_SetTextureBlendMode(texture: *mut SDL_Texture, blendMode: SDL_BlendMode) -> libc::c_int = set_texture_blend_mode => crate::render::local::set_texture_blend_mode;
    fn SDL_GetTextureBlendMode(texture: *mut SDL_Texture, blendMode: *mut SDL_BlendMode) -> libc::c_int = get_texture_blend_mode => crate::render::local::get_texture_blend_mode;
    fn SDL_SetTextureScaleMode(texture: *mut SDL_Texture, scaleMode: SDL_ScaleMode) -> libc::c_int = set_texture_scale_mode => crate::render::local::set_texture_scale_mode;
    fn SDL_GetTextureScaleMode(texture: *mut SDL_Texture, scaleMode: *mut SDL_ScaleMode) -> libc::c_int = get_texture_scale_mode => crate::render::local::get_texture_scale_mode;
    fn SDL_SetTextureUserData(texture: *mut SDL_Texture, userdata: *mut libc::c_void) -> libc::c_int = set_texture_user_data => crate::render::local::set_texture_user_data;
    fn SDL_GetTextureUserData(texture: *mut SDL_Texture) -> *mut libc::c_void = get_texture_user_data => crate::render::local::get_texture_user_data;
    fn SDL_UpdateTexture(texture: *mut SDL_Texture, rect: *const SDL_Rect, pixels: *const libc::c_void, pitch: libc::c_int) -> libc::c_int = update_texture => crate::render::local::update_texture;
    fn SDL_UpdateYUVTexture(texture: *mut SDL_Texture, rect: *const SDL_Rect, Yplane: *const Uint8, Ypitch: libc::c_int, Uplane: *const Uint8, Upitch: libc::c_int, Vplane: *const Uint8, Vpitch: libc::c_int) -> libc::c_int = update_yuv_texture => crate::render::local::update_yuv_texture;
    fn SDL_UpdateNVTexture(texture: *mut SDL_Texture, rect: *const SDL_Rect, Yplane: *const Uint8, Ypitch: libc::c_int, UVplane: *const Uint8, UVpitch: libc::c_int) -> libc::c_int = update_nv_texture => crate::render::local::update_nv_texture;
    fn SDL_LockTexture(texture: *mut SDL_Texture, rect: *const SDL_Rect, pixels: *mut *mut libc::c_void, pitch: *mut libc::c_int) -> libc::c_int = lock_texture => crate::render::local::lock_texture;
    fn SDL_LockTextureToSurface(texture: *mut SDL_Texture, rect: *const SDL_Rect, surface: *mut *mut SDL_Surface) -> libc::c_int = lock_texture_to_surface => crate::render::local::lock_texture_to_surface;
    fn SDL_GL_BindTexture(texture: *mut SDL_Texture, texw: *mut f32, texh: *mut f32) -> libc::c_int = gl_bind_texture => crate::render::local::gl_bind_texture;
    fn SDL_GL_UnbindTexture(texture: *mut SDL_Texture) -> libc::c_int = gl_unbind_texture => crate::render::local::gl_unbind_texture;
}

forward_texture_first_void_local! {
    fn SDL_UnlockTexture(texture: *mut SDL_Texture) = unlock_texture => crate::render::local::unlock_texture;
}

forward_texture_second_ret_local! {
    fn SDL_SetRenderTarget(renderer: *mut SDL_Renderer, texture: *mut SDL_Texture) -> libc::c_int = set_render_target => crate::render::local::set_render_target;
    fn SDL_RenderCopy(renderer: *mut SDL_Renderer, texture: *mut SDL_Texture, srcrect: *const SDL_Rect, dstrect: *const SDL_Rect) -> libc::c_int = render_copy => crate::render::local::render_copy;
    fn SDL_RenderCopyEx(renderer: *mut SDL_Renderer, texture: *mut SDL_Texture, srcrect: *const SDL_Rect, dstrect: *const SDL_Rect, angle: f64, center: *const SDL_Point, flip: SDL_RendererFlip) -> libc::c_int = render_copy_ex => crate::render::local::render_copy_ex;
    fn SDL_RenderCopyF(renderer: *mut SDL_Renderer, texture: *mut SDL_Texture, srcrect: *const SDL_Rect, dstrect: *const SDL_FRect) -> libc::c_int = render_copy_f => crate::render::local::render_copy_f;
    fn SDL_RenderCopyExF(renderer: *mut SDL_Renderer, texture: *mut SDL_Texture, srcrect: *const SDL_Rect, dstrect: *const SDL_FRect, angle: f64, center: *const SDL_FPoint, flip: SDL_RendererFlip) -> libc::c_int = render_copy_ex_f => crate::render::local::render_copy_ex_f;
    fn SDL_RenderGeometry(renderer: *mut SDL_Renderer, texture: *mut SDL_Texture, vertices: *const SDL_Vertex, num_vertices: libc::c_int, indices: *const libc::c_int, num_indices: libc::c_int) -> libc::c_int = render_geometry => crate::render::local::render_geometry;
    fn SDL_RenderGeometryRaw(renderer: *mut SDL_Renderer, texture: *mut SDL_Texture, xy: *const f32, xy_stride: libc::c_int, color: *const SDL_Color, color_stride: libc::c_int, uv: *const f32, uv_stride: libc::c_int, num_vertices: libc::c_int, indices: *const libc::c_void, num_indices: libc::c_int, size_indices: libc::c_int) -> libc::c_int = render_geometry_raw => crate::render::local::render_geometry_raw;
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CreateTexture(
    renderer: *mut SDL_Renderer,
    format: Uint32,
    access: libc::c_int,
    w: libc::c_int,
    h: libc::c_int,
) -> *mut SDL_Texture {
    crate::video::clear_real_error();

    if crate::render::local::is_local_renderer(renderer) {
        return crate::render::local::create_texture(renderer, format, access, w, h);
    }

    let use_gles_path = crate::render::gles::renderer_uses_gles_texture_path(renderer);
    let shadow = if use_gles_path {
        let _ = (api().render_flush)(renderer);
        let window = (api().render_get_window)(renderer);
        match crate::render::gles::prepare_texture_shadow(
            renderer,
            window,
            format,
            access,
            SDL_ScaleMode_SDL_ScaleModeLinear,
            w,
            h,
        ) {
            Ok(shadow) => shadow,
            Err(message) => {
                crate::core::error::set_error_message(&message);
                return ptr::null_mut();
            }
        }
    } else {
        None
    };

    let texture = (api().create_texture)(renderer, format, access, w, h);
    if texture.is_null() {
        return ptr::null_mut();
    }

    if let Some(shadow) = shadow {
        register_texture(texture, renderer, shadow)
    } else {
        texture
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetRenderTarget(renderer: *mut SDL_Renderer) -> *mut SDL_Texture {
    crate::video::clear_real_error();
    if crate::render::local::is_local_renderer(renderer) {
        return crate::render::local::get_render_target(renderer);
    }
    wrap_texture((api().get_render_target)(renderer))
}

#[no_mangle]
pub unsafe extern "C" fn SDL_DestroyTexture(texture: *mut SDL_Texture) {
    crate::video::clear_real_error();
    if crate::render::local::is_local_texture(texture) {
        crate::render::local::destroy_texture(texture);
        return;
    }
    if let Some(managed) = take_texture(texture) {
        (api().destroy_texture)(managed.raw);
        drop(managed);
    } else {
        (api().destroy_texture)(texture);
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_DestroyRenderer(renderer: *mut SDL_Renderer) {
    crate::video::clear_real_error();
    if crate::render::local::is_local_renderer(renderer) {
        let _ = take_window_renderer(renderer);
        crate::render::local::destroy_renderer(renderer);
        return;
    }
    clear_renderer_textures(renderer);

    if let Some(renderer) = take_window_renderer(renderer) {
        (api().destroy_renderer)(renderer);
    } else {
        (api().destroy_renderer)(renderer);
    }
}
