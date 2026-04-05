use std::collections::HashMap;
use std::ffi::CStr;
use std::ptr;
use std::sync::{Mutex, OnceLock};

use crate::abi::generated_types::{
    SDL_BlendMode, SDL_Color, SDL_Palette, SDL_PixelFormat, SDL_RWops, SDL_Rect, SDL_Surface,
    SDL_bool, Uint32, Uint8,
};
use crate::core::error::{invalid_param_error, set_error_message};
use crate::security::checked_math::{self, MathError};

#[derive(Debug, Clone, Copy)]
pub(crate) struct FormatDescriptor {
    pub bits_per_pixel: u8,
    pub bytes_per_pixel: u8,
}

macro_rules! real_sdl_api {
    ($(fn $field:ident = $symbol:literal : $ty:ty;)+) => {
        pub(crate) struct RealSdl {
            _handle: *mut libc::c_void,
            $(pub(crate) $field: $ty,)+
        }

        unsafe impl Send for RealSdl {}
        unsafe impl Sync for RealSdl {}

        fn load_real_sdl() -> RealSdl {
            let handle = open_real_sdl();
            RealSdl {
                _handle: handle,
                $($field: load_symbol::<$ty>(handle, concat!($symbol, "\0").as_ptr().cast()),)+
            }
        }
    };
}

real_sdl_api! {
    fn clear_error = "SDL_ClearError": unsafe extern "C" fn();
    fn get_error = "SDL_GetError": unsafe extern "C" fn() -> *const libc::c_char;
    fn alloc_format = "SDL_AllocFormat": unsafe extern "C" fn(Uint32) -> *mut SDL_PixelFormat;
    fn free_format = "SDL_FreeFormat": unsafe extern "C" fn(*mut SDL_PixelFormat);
    fn alloc_palette = "SDL_AllocPalette": unsafe extern "C" fn(libc::c_int) -> *mut SDL_Palette;
    fn set_pixel_format_palette = "SDL_SetPixelFormatPalette": unsafe extern "C" fn(*mut SDL_PixelFormat, *mut SDL_Palette) -> libc::c_int;
    fn set_palette_colors = "SDL_SetPaletteColors": unsafe extern "C" fn(*mut SDL_Palette, *const SDL_Color, libc::c_int, libc::c_int) -> libc::c_int;
    fn free_palette = "SDL_FreePalette": unsafe extern "C" fn(*mut SDL_Palette);
    fn get_pixel_format_name = "SDL_GetPixelFormatName": unsafe extern "C" fn(Uint32) -> *const libc::c_char;
    fn pixel_format_enum_to_masks = "SDL_PixelFormatEnumToMasks": unsafe extern "C" fn(Uint32, *mut libc::c_int, *mut Uint32, *mut Uint32, *mut Uint32, *mut Uint32) -> SDL_bool;
    fn masks_to_pixel_format_enum = "SDL_MasksToPixelFormatEnum": unsafe extern "C" fn(libc::c_int, Uint32, Uint32, Uint32, Uint32) -> Uint32;
    fn map_rgb = "SDL_MapRGB": unsafe extern "C" fn(*const SDL_PixelFormat, Uint8, Uint8, Uint8) -> Uint32;
    fn map_rgba = "SDL_MapRGBA": unsafe extern "C" fn(*const SDL_PixelFormat, Uint8, Uint8, Uint8, Uint8) -> Uint32;
    fn get_rgb = "SDL_GetRGB": unsafe extern "C" fn(Uint32, *const SDL_PixelFormat, *mut Uint8, *mut Uint8, *mut Uint8);
    fn get_rgba = "SDL_GetRGBA": unsafe extern "C" fn(Uint32, *const SDL_PixelFormat, *mut Uint8, *mut Uint8, *mut Uint8, *mut Uint8);
    fn calculate_gamma_ramp = "SDL_CalculateGammaRamp": unsafe extern "C" fn(f32, *mut u16);
    fn has_intersection = "SDL_HasIntersection": unsafe extern "C" fn(*const SDL_Rect, *const SDL_Rect) -> SDL_bool;
    fn intersect_rect = "SDL_IntersectRect": unsafe extern "C" fn(*const SDL_Rect, *const SDL_Rect, *mut SDL_Rect) -> SDL_bool;
    fn union_rect = "SDL_UnionRect": unsafe extern "C" fn(*const SDL_Rect, *const SDL_Rect, *mut SDL_Rect);
    fn enclose_points = "SDL_EnclosePoints": unsafe extern "C" fn(*const crate::abi::generated_types::SDL_Point, libc::c_int, *const SDL_Rect, *mut SDL_Rect) -> SDL_bool;
    fn intersect_rect_and_line = "SDL_IntersectRectAndLine": unsafe extern "C" fn(*const SDL_Rect, *mut libc::c_int, *mut libc::c_int, *mut libc::c_int, *mut libc::c_int) -> SDL_bool;
    fn has_intersection_f = "SDL_HasIntersectionF": unsafe extern "C" fn(*const crate::abi::generated_types::SDL_FRect, *const crate::abi::generated_types::SDL_FRect) -> SDL_bool;
    fn intersect_f_rect = "SDL_IntersectFRect": unsafe extern "C" fn(*const crate::abi::generated_types::SDL_FRect, *const crate::abi::generated_types::SDL_FRect, *mut crate::abi::generated_types::SDL_FRect) -> SDL_bool;
    fn union_f_rect = "SDL_UnionFRect": unsafe extern "C" fn(*const crate::abi::generated_types::SDL_FRect, *const crate::abi::generated_types::SDL_FRect, *mut crate::abi::generated_types::SDL_FRect);
    fn enclose_f_points = "SDL_EncloseFPoints": unsafe extern "C" fn(*const crate::abi::generated_types::SDL_FPoint, libc::c_int, *const crate::abi::generated_types::SDL_FRect, *mut crate::abi::generated_types::SDL_FRect) -> SDL_bool;
    fn intersect_f_rect_and_line = "SDL_IntersectFRectAndLine": unsafe extern "C" fn(*const crate::abi::generated_types::SDL_FRect, *mut f32, *mut f32, *mut f32, *mut f32) -> SDL_bool;
    fn create_rgb_surface = "SDL_CreateRGBSurface": unsafe extern "C" fn(Uint32, libc::c_int, libc::c_int, libc::c_int, Uint32, Uint32, Uint32, Uint32) -> *mut SDL_Surface;
    fn create_rgb_surface_with_format = "SDL_CreateRGBSurfaceWithFormat": unsafe extern "C" fn(Uint32, libc::c_int, libc::c_int, libc::c_int, Uint32) -> *mut SDL_Surface;
    fn create_rgb_surface_from = "SDL_CreateRGBSurfaceFrom": unsafe extern "C" fn(*mut libc::c_void, libc::c_int, libc::c_int, libc::c_int, libc::c_int, Uint32, Uint32, Uint32, Uint32) -> *mut SDL_Surface;
    fn create_rgb_surface_with_format_from = "SDL_CreateRGBSurfaceWithFormatFrom": unsafe extern "C" fn(*mut libc::c_void, libc::c_int, libc::c_int, libc::c_int, libc::c_int, Uint32) -> *mut SDL_Surface;
    fn free_surface = "SDL_FreeSurface": unsafe extern "C" fn(*mut SDL_Surface);
    fn set_surface_palette = "SDL_SetSurfacePalette": unsafe extern "C" fn(*mut SDL_Surface, *mut SDL_Palette) -> libc::c_int;
    fn lock_surface = "SDL_LockSurface": unsafe extern "C" fn(*mut SDL_Surface) -> libc::c_int;
    fn unlock_surface = "SDL_UnlockSurface": unsafe extern "C" fn(*mut SDL_Surface);
    fn load_bmp_rw = "SDL_LoadBMP_RW": unsafe extern "C" fn(*mut SDL_RWops, libc::c_int) -> *mut SDL_Surface;
    fn save_bmp_rw = "SDL_SaveBMP_RW": unsafe extern "C" fn(*mut SDL_Surface, *mut SDL_RWops, libc::c_int) -> libc::c_int;
    fn set_surface_rle = "SDL_SetSurfaceRLE": unsafe extern "C" fn(*mut SDL_Surface, libc::c_int) -> libc::c_int;
    fn has_surface_rle = "SDL_HasSurfaceRLE": unsafe extern "C" fn(*mut SDL_Surface) -> SDL_bool;
    fn set_color_key = "SDL_SetColorKey": unsafe extern "C" fn(*mut SDL_Surface, libc::c_int, Uint32) -> libc::c_int;
    fn has_color_key = "SDL_HasColorKey": unsafe extern "C" fn(*mut SDL_Surface) -> SDL_bool;
    fn get_color_key = "SDL_GetColorKey": unsafe extern "C" fn(*mut SDL_Surface, *mut Uint32) -> libc::c_int;
    fn set_surface_color_mod = "SDL_SetSurfaceColorMod": unsafe extern "C" fn(*mut SDL_Surface, Uint8, Uint8, Uint8) -> libc::c_int;
    fn get_surface_color_mod = "SDL_GetSurfaceColorMod": unsafe extern "C" fn(*mut SDL_Surface, *mut Uint8, *mut Uint8, *mut Uint8) -> libc::c_int;
    fn set_surface_alpha_mod = "SDL_SetSurfaceAlphaMod": unsafe extern "C" fn(*mut SDL_Surface, Uint8) -> libc::c_int;
    fn get_surface_alpha_mod = "SDL_GetSurfaceAlphaMod": unsafe extern "C" fn(*mut SDL_Surface, *mut Uint8) -> libc::c_int;
    fn set_surface_blend_mode = "SDL_SetSurfaceBlendMode": unsafe extern "C" fn(*mut SDL_Surface, SDL_BlendMode) -> libc::c_int;
    fn get_surface_blend_mode = "SDL_GetSurfaceBlendMode": unsafe extern "C" fn(*mut SDL_Surface, *mut SDL_BlendMode) -> libc::c_int;
    fn set_clip_rect = "SDL_SetClipRect": unsafe extern "C" fn(*mut SDL_Surface, *const SDL_Rect) -> SDL_bool;
    fn get_clip_rect = "SDL_GetClipRect": unsafe extern "C" fn(*mut SDL_Surface, *mut SDL_Rect);
    fn duplicate_surface = "SDL_DuplicateSurface": unsafe extern "C" fn(*mut SDL_Surface) -> *mut SDL_Surface;
    fn convert_surface = "SDL_ConvertSurface": unsafe extern "C" fn(*mut SDL_Surface, *const SDL_PixelFormat, Uint32) -> *mut SDL_Surface;
    fn convert_surface_format = "SDL_ConvertSurfaceFormat": unsafe extern "C" fn(*mut SDL_Surface, Uint32, Uint32) -> *mut SDL_Surface;
    fn convert_pixels = "SDL_ConvertPixels": unsafe extern "C" fn(libc::c_int, libc::c_int, Uint32, *const libc::c_void, libc::c_int, Uint32, *mut libc::c_void, libc::c_int) -> libc::c_int;
    fn premultiply_alpha = "SDL_PremultiplyAlpha": unsafe extern "C" fn(libc::c_int, libc::c_int, Uint32, *const libc::c_void, libc::c_int, Uint32, *mut libc::c_void, libc::c_int) -> libc::c_int;
    fn fill_rect = "SDL_FillRect": unsafe extern "C" fn(*mut SDL_Surface, *const SDL_Rect, Uint32) -> libc::c_int;
    fn fill_rects = "SDL_FillRects": unsafe extern "C" fn(*mut SDL_Surface, *const SDL_Rect, libc::c_int, Uint32) -> libc::c_int;
    fn upper_blit = "SDL_UpperBlit": unsafe extern "C" fn(*mut SDL_Surface, *const SDL_Rect, *mut SDL_Surface, *mut SDL_Rect) -> libc::c_int;
    fn lower_blit = "SDL_LowerBlit": unsafe extern "C" fn(*mut SDL_Surface, *mut SDL_Rect, *mut SDL_Surface, *mut SDL_Rect) -> libc::c_int;
    fn soft_stretch = "SDL_SoftStretch": unsafe extern "C" fn(*mut SDL_Surface, *const SDL_Rect, *mut SDL_Surface, *const SDL_Rect) -> libc::c_int;
    fn soft_stretch_linear = "SDL_SoftStretchLinear": unsafe extern "C" fn(*mut SDL_Surface, *const SDL_Rect, *mut SDL_Surface, *const SDL_Rect) -> libc::c_int;
    fn upper_blit_scaled = "SDL_UpperBlitScaled": unsafe extern "C" fn(*mut SDL_Surface, *const SDL_Rect, *mut SDL_Surface, *mut SDL_Rect) -> libc::c_int;
    fn lower_blit_scaled = "SDL_LowerBlitScaled": unsafe extern "C" fn(*mut SDL_Surface, *mut SDL_Rect, *mut SDL_Surface, *mut SDL_Rect) -> libc::c_int;
}

fn open_real_sdl() -> *mut libc::c_void {
    for candidate in crate::video::real_sdl_dlopen_candidates() {
        let handle = unsafe { libc::dlopen(candidate.as_ptr(), libc::RTLD_LOCAL | libc::RTLD_NOW) };
        if !handle.is_null() {
            return handle;
        }
    }

    panic!("unable to load the host SDL2 runtime");
}

fn load_symbol<T>(handle: *mut libc::c_void, name: *const libc::c_char) -> T {
    let symbol = unsafe { libc::dlsym(handle, name) };
    assert!(!symbol.is_null(), "missing host SDL2 symbol");
    unsafe { std::mem::transmute_copy(&symbol) }
}

pub(crate) fn real_sdl() -> &'static RealSdl {
    static REAL: OnceLock<RealSdl> = OnceLock::new();
    REAL.get_or_init(load_real_sdl)
}

pub(crate) fn clear_real_error() {
    unsafe {
        (real_sdl().clear_error)();
    }
}

pub(crate) fn sync_error_from_real(default: &str) -> libc::c_int {
    let message = unsafe {
        let ptr = (real_sdl().get_error)();
        if ptr.is_null() {
            None
        } else {
            Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
        }
    };

    match message {
        Some(message) if !message.is_empty() => set_error_message(&message),
        _ => set_error_message(default),
    }
}

pub(crate) fn apply_math_error(error: MathError) -> libc::c_int {
    match error {
        MathError::NegativeParam(param) | MathError::InvalidParam(param) => {
            invalid_param_error(param)
        }
        MathError::Overflow(message) => set_error_message(message),
    }
}

pub(crate) fn apply_math_error_ptr<T>(error: MathError) -> *mut T {
    let _ = apply_math_error(error);
    ptr::null_mut()
}

pub(crate) unsafe fn descriptor_from_format_ptr(
    format: *const SDL_PixelFormat,
) -> Result<FormatDescriptor, MathError> {
    if format.is_null() {
        return Err(MathError::InvalidParam("format"));
    }

    Ok(FormatDescriptor {
        bits_per_pixel: (*format).BitsPerPixel,
        bytes_per_pixel: (*format).BytesPerPixel,
    })
}

pub(crate) fn format_descriptor(format: Uint32) -> Option<FormatDescriptor> {
    static CACHE: OnceLock<Mutex<HashMap<Uint32, Option<FormatDescriptor>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(cached) = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(&format)
        .copied()
    {
        return cached;
    }

    clear_real_error();
    let raw = unsafe { (real_sdl().alloc_format)(format) };
    let descriptor = if raw.is_null() {
        None
    } else {
        let descriptor = unsafe {
            FormatDescriptor {
                bits_per_pixel: (*raw).BitsPerPixel,
                bytes_per_pixel: (*raw).BytesPerPixel,
            }
        };
        unsafe {
            (real_sdl().free_format)(raw);
        }
        Some(descriptor)
    };

    cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(format, descriptor);
    descriptor
}

pub(crate) unsafe fn validate_surface_storage(
    surface: *mut SDL_Surface,
) -> Result<FormatDescriptor, MathError> {
    if surface.is_null() {
        return Err(MathError::InvalidParam("surface"));
    }

    let descriptor = descriptor_from_format_ptr((*surface).format)?;
    let layout_size = checked_math::validate_surface_layout(
        (*surface).w,
        (*surface).h,
        (*surface).pitch,
        descriptor.bits_per_pixel,
        descriptor.bytes_per_pixel,
    )?;
    if layout_size > 0 && (*surface).pixels.is_null() {
        return Err(MathError::InvalidParam("surface"));
    }
    Ok(descriptor)
}

fn preflight_surface_allocation(
    format: Uint32,
    width: libc::c_int,
    height: libc::c_int,
) -> Result<(), MathError> {
    if let Some(descriptor) = format_descriptor(format) {
        let _ = checked_math::calculate_surface_allocation(
            width,
            height,
            descriptor.bits_per_pixel,
            descriptor.bytes_per_pixel,
        )?;
    }
    Ok(())
}

fn preflight_preallocated_surface(
    format: Uint32,
    width: libc::c_int,
    height: libc::c_int,
    pitch: libc::c_int,
) -> Result<(), MathError> {
    if let Some(descriptor) = format_descriptor(format) {
        let _ = checked_math::validate_preallocated_surface(
            width,
            height,
            pitch,
            descriptor.bits_per_pixel,
            descriptor.bytes_per_pixel,
        )?;
    }
    Ok(())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CreateRGBSurface(
    flags: Uint32,
    width: libc::c_int,
    height: libc::c_int,
    depth: libc::c_int,
    Rmask: Uint32,
    Gmask: Uint32,
    Bmask: Uint32,
    Amask: Uint32,
) -> *mut SDL_Surface {
    if width < 0 {
        let _ = invalid_param_error("width");
        return ptr::null_mut();
    }
    if height < 0 {
        let _ = invalid_param_error("height");
        return ptr::null_mut();
    }

    let format = (real_sdl().masks_to_pixel_format_enum)(depth, Rmask, Gmask, Bmask, Amask);
    if let Err(error) = preflight_surface_allocation(format, width, height) {
        return apply_math_error_ptr(error);
    }

    clear_real_error();
    let surface =
        (real_sdl().create_rgb_surface)(flags, width, height, depth, Rmask, Gmask, Bmask, Amask);
    if surface.is_null() {
        let _ = sync_error_from_real("Couldn't create RGB surface");
    }
    surface
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CreateRGBSurfaceWithFormat(
    flags: Uint32,
    width: libc::c_int,
    height: libc::c_int,
    depth: libc::c_int,
    format: Uint32,
) -> *mut SDL_Surface {
    if width < 0 {
        let _ = invalid_param_error("width");
        return ptr::null_mut();
    }
    if height < 0 {
        let _ = invalid_param_error("height");
        return ptr::null_mut();
    }
    if let Err(error) = preflight_surface_allocation(format, width, height) {
        return apply_math_error_ptr(error);
    }

    clear_real_error();
    let surface = (real_sdl().create_rgb_surface_with_format)(flags, width, height, depth, format);
    if surface.is_null() {
        let _ = sync_error_from_real("Couldn't create RGB surface");
    }
    surface
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CreateRGBSurfaceFrom(
    pixels: *mut libc::c_void,
    width: libc::c_int,
    height: libc::c_int,
    depth: libc::c_int,
    pitch: libc::c_int,
    Rmask: Uint32,
    Gmask: Uint32,
    Bmask: Uint32,
    Amask: Uint32,
) -> *mut SDL_Surface {
    if width < 0 {
        let _ = invalid_param_error("width");
        return ptr::null_mut();
    }
    if height < 0 {
        let _ = invalid_param_error("height");
        return ptr::null_mut();
    }
    if pitch < 0 {
        let _ = invalid_param_error("pitch");
        return ptr::null_mut();
    }

    let format = (real_sdl().masks_to_pixel_format_enum)(depth, Rmask, Gmask, Bmask, Amask);
    if let Err(error) = preflight_preallocated_surface(format, width, height, pitch) {
        return apply_math_error_ptr(error);
    }

    clear_real_error();
    let surface = (real_sdl().create_rgb_surface_from)(
        pixels, width, height, depth, pitch, Rmask, Gmask, Bmask, Amask,
    );
    if surface.is_null() {
        let _ = sync_error_from_real("Couldn't create RGB surface");
    }
    surface
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CreateRGBSurfaceWithFormatFrom(
    pixels: *mut libc::c_void,
    width: libc::c_int,
    height: libc::c_int,
    depth: libc::c_int,
    pitch: libc::c_int,
    format: Uint32,
) -> *mut SDL_Surface {
    if width < 0 {
        let _ = invalid_param_error("width");
        return ptr::null_mut();
    }
    if height < 0 {
        let _ = invalid_param_error("height");
        return ptr::null_mut();
    }
    if pitch < 0 {
        let _ = invalid_param_error("pitch");
        return ptr::null_mut();
    }
    if let Err(error) = preflight_preallocated_surface(format, width, height, pitch) {
        return apply_math_error_ptr(error);
    }

    clear_real_error();
    let surface = (real_sdl().create_rgb_surface_with_format_from)(
        pixels, width, height, depth, pitch, format,
    );
    if surface.is_null() {
        let _ = sync_error_from_real("Couldn't create RGB surface");
    }
    surface
}

#[no_mangle]
pub unsafe extern "C" fn SDL_FreeSurface(surface: *mut SDL_Surface) {
    (real_sdl().free_surface)(surface);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetSurfacePalette(
    surface: *mut SDL_Surface,
    palette: *mut SDL_Palette,
) -> libc::c_int {
    clear_real_error();
    let result = (real_sdl().set_surface_palette)(surface, palette);
    if result < 0 {
        let _ = sync_error_from_real("Couldn't set surface palette");
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn SDL_LockSurface(surface: *mut SDL_Surface) -> libc::c_int {
    clear_real_error();
    let result = (real_sdl().lock_surface)(surface);
    if result < 0 {
        let _ = sync_error_from_real("Couldn't lock surface");
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn SDL_UnlockSurface(surface: *mut SDL_Surface) {
    (real_sdl().unlock_surface)(surface);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetSurfaceRLE(
    surface: *mut SDL_Surface,
    flag: libc::c_int,
) -> libc::c_int {
    clear_real_error();
    let result = (real_sdl().set_surface_rle)(surface, flag);
    if result < 0 {
        let _ = sync_error_from_real("Couldn't set surface RLE");
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HasSurfaceRLE(surface: *mut SDL_Surface) -> SDL_bool {
    (real_sdl().has_surface_rle)(surface)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetColorKey(
    surface: *mut SDL_Surface,
    flag: libc::c_int,
    key: Uint32,
) -> libc::c_int {
    clear_real_error();
    let result = (real_sdl().set_color_key)(surface, flag, key);
    if result < 0 {
        let _ = sync_error_from_real("Couldn't set color key");
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HasColorKey(surface: *mut SDL_Surface) -> SDL_bool {
    (real_sdl().has_color_key)(surface)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetColorKey(
    surface: *mut SDL_Surface,
    key: *mut Uint32,
) -> libc::c_int {
    clear_real_error();
    let result = (real_sdl().get_color_key)(surface, key);
    if result < 0 {
        let _ = sync_error_from_real("Couldn't get color key");
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetSurfaceColorMod(
    surface: *mut SDL_Surface,
    r: Uint8,
    g: Uint8,
    b: Uint8,
) -> libc::c_int {
    clear_real_error();
    let result = (real_sdl().set_surface_color_mod)(surface, r, g, b);
    if result < 0 {
        let _ = sync_error_from_real("Couldn't set surface color modulation");
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetSurfaceColorMod(
    surface: *mut SDL_Surface,
    r: *mut Uint8,
    g: *mut Uint8,
    b: *mut Uint8,
) -> libc::c_int {
    clear_real_error();
    let result = (real_sdl().get_surface_color_mod)(surface, r, g, b);
    if result < 0 {
        let _ = sync_error_from_real("Couldn't get surface color modulation");
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetSurfaceAlphaMod(
    surface: *mut SDL_Surface,
    alpha: Uint8,
) -> libc::c_int {
    clear_real_error();
    let result = (real_sdl().set_surface_alpha_mod)(surface, alpha);
    if result < 0 {
        let _ = sync_error_from_real("Couldn't set surface alpha modulation");
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetSurfaceAlphaMod(
    surface: *mut SDL_Surface,
    alpha: *mut Uint8,
) -> libc::c_int {
    clear_real_error();
    let result = (real_sdl().get_surface_alpha_mod)(surface, alpha);
    if result < 0 {
        let _ = sync_error_from_real("Couldn't get surface alpha modulation");
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetSurfaceBlendMode(
    surface: *mut SDL_Surface,
    blendMode: SDL_BlendMode,
) -> libc::c_int {
    clear_real_error();
    let result = (real_sdl().set_surface_blend_mode)(surface, blendMode);
    if result < 0 {
        let _ = sync_error_from_real("Couldn't set surface blend mode");
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetSurfaceBlendMode(
    surface: *mut SDL_Surface,
    blendMode: *mut SDL_BlendMode,
) -> libc::c_int {
    clear_real_error();
    let result = (real_sdl().get_surface_blend_mode)(surface, blendMode);
    if result < 0 {
        let _ = sync_error_from_real("Couldn't get surface blend mode");
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetClipRect(
    surface: *mut SDL_Surface,
    rect: *const SDL_Rect,
) -> SDL_bool {
    (real_sdl().set_clip_rect)(surface, rect)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetClipRect(surface: *mut SDL_Surface, rect: *mut SDL_Rect) {
    (real_sdl().get_clip_rect)(surface, rect);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_DuplicateSurface(surface: *mut SDL_Surface) -> *mut SDL_Surface {
    if let Err(error) = validate_surface_storage(surface) {
        return apply_math_error_ptr(error);
    }
    clear_real_error();
    let duplicate = (real_sdl().duplicate_surface)(surface);
    if duplicate.is_null() {
        let _ = sync_error_from_real("Couldn't duplicate surface");
    }
    duplicate
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ConvertSurface(
    src: *mut SDL_Surface,
    fmt: *const SDL_PixelFormat,
    flags: Uint32,
) -> *mut SDL_Surface {
    if let Err(error) = validate_surface_storage(src) {
        return apply_math_error_ptr(error);
    }
    clear_real_error();
    let converted = (real_sdl().convert_surface)(src, fmt, flags);
    if converted.is_null() {
        let _ = sync_error_from_real("Couldn't convert surface");
    }
    converted
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ConvertSurfaceFormat(
    src: *mut SDL_Surface,
    pixel_format: Uint32,
    flags: Uint32,
) -> *mut SDL_Surface {
    if let Err(error) = validate_surface_storage(src) {
        return apply_math_error_ptr(error);
    }
    clear_real_error();
    let converted = (real_sdl().convert_surface_format)(src, pixel_format, flags);
    if converted.is_null() {
        let _ = sync_error_from_real("Couldn't convert surface");
    }
    converted
}

#[no_mangle]
pub unsafe extern "C" fn SDL_FillRect(
    dst: *mut SDL_Surface,
    rect: *const SDL_Rect,
    color: Uint32,
) -> libc::c_int {
    if let Err(error) = validate_surface_storage(dst) {
        return apply_math_error(error);
    }
    clear_real_error();
    let result = (real_sdl().fill_rect)(dst, rect, color);
    if result < 0 {
        let _ = sync_error_from_real("Couldn't fill surface");
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn SDL_FillRects(
    dst: *mut SDL_Surface,
    rects: *const SDL_Rect,
    count: libc::c_int,
    color: Uint32,
) -> libc::c_int {
    if let Err(error) = validate_surface_storage(dst) {
        return apply_math_error(error);
    }
    clear_real_error();
    let result = (real_sdl().fill_rects)(dst, rects, count, color);
    if result < 0 {
        let _ = sync_error_from_real("Couldn't fill surface");
    }
    result
}
