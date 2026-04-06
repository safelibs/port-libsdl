use std::collections::HashMap;
use std::ffi::CStr;
use std::ptr;
use std::sync::{Mutex, OnceLock};

use crate::abi::generated_types::{
    SDL_BlendMode, SDL_BlendMode_SDL_BLENDMODE_ADD, SDL_BlendMode_SDL_BLENDMODE_BLEND,
    SDL_BlendMode_SDL_BLENDMODE_MOD, SDL_BlendMode_SDL_BLENDMODE_NONE, SDL_Color, SDL_Palette,
    SDL_PixelFormat, SDL_RWops, SDL_Rect, SDL_Surface, SDL_bool, Uint32, Uint8, SDL_PREALLOC,
    SDL_RLEACCEL,
};
use crate::core::error::{invalid_param_error, out_of_memory_error, set_error_message};
use crate::security::checked_math::{self, MathError};

#[derive(Debug, Clone, Copy)]
pub(crate) struct FormatDescriptor {
    pub bits_per_pixel: u8,
    pub bytes_per_pixel: u8,
}

enum SurfaceStorage {
    Owned(Vec<u8>),
    Borrowed,
}

enum SurfaceShell {
    Host,
    Local(Box<SDL_Surface>),
}

unsafe impl Send for SurfaceShell {}

struct SurfaceRecord {
    buffer_len: usize,
    expected_pixels: usize,
    storage: SurfaceStorage,
    shell: SurfaceShell,
    owns_format: bool,
    color_key_enabled: bool,
    color_key: Uint32,
    color_mod: (Uint8, Uint8, Uint8),
    alpha_mod: Uint8,
    blend_mode: SDL_BlendMode,
    rle_enabled: bool,
}

unsafe impl Send for SurfaceRecord {}

#[derive(Clone, Copy)]
pub(crate) struct SurfaceState {
    pub(crate) color_key_enabled: bool,
    pub(crate) color_key: Uint32,
    pub(crate) color_mod: (Uint8, Uint8, Uint8),
    pub(crate) alpha_mod: Uint8,
    pub(crate) blend_mode: SDL_BlendMode,
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
    crate::video::open_real_sdl_with_flags(libc::RTLD_LOCAL | libc::RTLD_NOW)
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

fn surface_registry() -> &'static Mutex<HashMap<usize, SurfaceRecord>> {
    static REGISTRY: OnceLock<Mutex<HashMap<usize, SurfaceRecord>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn register_surface_record(surface: *mut SDL_Surface, record: SurfaceRecord) {
    surface_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(surface as usize, record);
}

fn take_surface_record(surface: *mut SDL_Surface) -> Option<SurfaceRecord> {
    surface_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(&(surface as usize))
}

fn surface_record_metadata(surface: *mut SDL_Surface) -> Option<(usize, usize)> {
    surface_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(&(surface as usize))
        .map(|record| (record.buffer_len, record.expected_pixels))
}

fn with_surface_record_mut<T>(
    surface: *mut SDL_Surface,
    f: impl FnOnce(&mut SurfaceRecord) -> T,
) -> Option<T> {
    surface_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get_mut(&(surface as usize))
        .map(f)
}

fn with_surface_record<T>(
    surface: *mut SDL_Surface,
    f: impl FnOnce(&SurfaceRecord) -> T,
) -> Option<T> {
    surface_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(&(surface as usize))
        .map(f)
}

fn default_blend_mode(format: *const SDL_PixelFormat) -> SDL_BlendMode {
    if format.is_null() {
        SDL_BlendMode_SDL_BLENDMODE_NONE
    } else if unsafe { (*format).Amask } != 0 {
        SDL_BlendMode_SDL_BLENDMODE_BLEND
    } else {
        SDL_BlendMode_SDL_BLENDMODE_NONE
    }
}

pub(crate) fn surface_state(surface: *mut SDL_Surface) -> SurfaceState {
    with_surface_record(surface, |record| SurfaceState {
        color_key_enabled: record.color_key_enabled,
        color_key: record.color_key,
        color_mod: record.color_mod,
        alpha_mod: record.alpha_mod,
        blend_mode: record.blend_mode,
    })
    .unwrap_or(SurfaceState {
        color_key_enabled: false,
        color_key: 0,
        color_mod: (255, 255, 255),
        alpha_mod: 255,
        blend_mode: SDL_BlendMode_SDL_BLENDMODE_NONE,
    })
}

pub(crate) fn is_registered_surface(surface: *mut SDL_Surface) -> bool {
    surface_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .contains_key(&(surface as usize))
}

pub(crate) fn clear_real_error() {
    if crate::video::real_sdl_is_available() {
        unsafe {
            (real_sdl().clear_error)();
        }
    }
}

pub(crate) fn sync_error_from_real(default: &str) -> libc::c_int {
    if !crate::video::real_sdl_is_available() {
        return set_error_message(default);
    }
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
    const PIXELFLAG_STANDARD: u32 = 1;
    const PIXELTYPE_INDEX1: u8 = 1;
    const PIXELTYPE_INDEX4: u8 = 2;
    const PIXELTYPE_INDEX8: u8 = 3;
    const PIXELTYPE_PACKED8: u8 = 4;
    const PIXELTYPE_PACKED16: u8 = 5;
    const PIXELTYPE_PACKED32: u8 = 6;
    const PIXELTYPE_ARRAYU8: u8 = 7;
    const PIXELTYPE_INDEX2: u8 = 12;
    const PACKEDLAYOUT_332: u8 = 1;
    const PACKEDLAYOUT_4444: u8 = 2;
    const PACKEDLAYOUT_1555: u8 = 3;
    const PACKEDLAYOUT_5551: u8 = 4;
    const PACKEDLAYOUT_565: u8 = 5;
    const PACKEDLAYOUT_8888: u8 = 6;
    const PACKEDLAYOUT_2101010: u8 = 7;

    if format == 0 || ((format >> 28) & 0x0f) != PIXELFLAG_STANDARD {
        return None;
    }

    let pixel_type = ((format >> 24) & 0x0f) as u8;
    let layout = ((format >> 16) & 0x0f) as u8;
    let bits = ((format >> 8) & 0xff) as u8;
    let bytes = (format & 0xff) as u8;

    let valid = match pixel_type {
        PIXELTYPE_INDEX1 => bits == 1 && bytes == 0 && layout == 0,
        PIXELTYPE_INDEX2 => bits == 2 && bytes == 0 && layout == 0,
        PIXELTYPE_INDEX4 => bits == 4 && bytes == 0 && layout == 0,
        PIXELTYPE_INDEX8 => bits == 8 && bytes == 1 && layout == 0,
        PIXELTYPE_PACKED8 => bits == 8 && bytes == 1 && layout == PACKEDLAYOUT_332,
        PIXELTYPE_PACKED16 => match layout {
            PACKEDLAYOUT_4444 => bytes == 2 && matches!(bits, 12 | 16),
            PACKEDLAYOUT_1555 => bytes == 2 && matches!(bits, 15 | 16),
            PACKEDLAYOUT_5551 | PACKEDLAYOUT_565 => bytes == 2 && bits == 16,
            _ => false,
        },
        PIXELTYPE_PACKED32 => match layout {
            PACKEDLAYOUT_8888 => bytes == 4 && matches!(bits, 24 | 32),
            PACKEDLAYOUT_2101010 => bytes == 4 && bits == 32,
            _ => false,
        },
        PIXELTYPE_ARRAYU8 => layout == 0 && bytes == 3 && bits == 24,
        _ => false,
    };

    valid.then_some(FormatDescriptor {
        bits_per_pixel: bits,
        bytes_per_pixel: bytes,
    })
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

    if let Some((buffer_len, expected_pixels)) = surface_record_metadata(surface) {
        if layout_size > 0 && (*surface).pixels.is_null() {
            return Err(MathError::InvalidParam("surface"));
        }
        if (*surface).pixels as usize != expected_pixels {
            return Err(MathError::InvalidParam("surface"));
        }
        if layout_size > buffer_len {
            return Err(MathError::InvalidParam("surface"));
        }
    } else if layout_size > 0 && (*surface).pixels.is_null() {
        return Err(MathError::InvalidParam("surface"));
    }

    Ok(descriptor)
}

pub(crate) fn intersect_rects(a: &SDL_Rect, b: &SDL_Rect) -> Option<SDL_Rect> {
    if a.w <= 0 || a.h <= 0 || b.w <= 0 || b.h <= 0 {
        return None;
    }

    let x1 = (a.x as i64).max(b.x as i64);
    let y1 = (a.y as i64).max(b.y as i64);
    let x2 = (a.x as i64 + a.w as i64).min(b.x as i64 + b.w as i64);
    let y2 = (a.y as i64 + a.h as i64).min(b.y as i64 + b.h as i64);

    if x2 <= x1 || y2 <= y1 {
        None
    } else {
        Some(SDL_Rect {
            x: x1 as libc::c_int,
            y: y1 as libc::c_int,
            w: (x2 - x1) as libc::c_int,
            h: (y2 - y1) as libc::c_int,
        })
    }
}

pub(crate) unsafe fn full_surface_rect(surface: *mut SDL_Surface) -> SDL_Rect {
    SDL_Rect {
        x: 0,
        y: 0,
        w: (*surface).w.max(0),
        h: (*surface).h.max(0),
    }
}

unsafe fn rect_inside_surface(surface: *mut SDL_Surface, rect: &SDL_Rect) -> Result<(), MathError> {
    if rect.x < 0 || rect.y < 0 || rect.w < 0 || rect.h < 0 {
        return Err(MathError::InvalidParam("rect"));
    }

    let bounds = full_surface_rect(surface);
    match intersect_rects(&bounds, rect) {
        Some(clipped)
            if clipped.x == rect.x
                && clipped.y == rect.y
                && clipped.w == rect.w
                && clipped.h == rect.h =>
        {
            Ok(())
        }
        _ => Err(MathError::InvalidParam("rect")),
    }
}

pub(crate) unsafe fn raw_pixel(ptr: *const u8, bytes_per_pixel: u8) -> Uint32 {
    match bytes_per_pixel {
        1 => ptr.read() as Uint32,
        2 => ptr.cast::<u16>().read_unaligned() as Uint32,
        3 => {
            let bytes = [ptr.read(), ptr.add(1).read(), ptr.add(2).read(), 0];
            Uint32::from_le_bytes(bytes)
        }
        4 => ptr.cast::<Uint32>().read_unaligned(),
        _ => 0,
    }
}

pub(crate) unsafe fn write_raw_pixel(ptr: *mut u8, bytes_per_pixel: u8, pixel: Uint32) {
    match bytes_per_pixel {
        1 => ptr.write(pixel as u8),
        2 => ptr.cast::<u16>().write_unaligned(pixel as u16),
        3 => {
            let bytes = pixel.to_le_bytes();
            ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, 3);
        }
        4 => ptr.cast::<Uint32>().write_unaligned(pixel),
        _ => {}
    }
}

pub(crate) unsafe fn read_rgba_pixel(
    format: *const SDL_PixelFormat,
    bytes_per_pixel: u8,
    ptr: *const u8,
) -> (u8, u8, u8, u8) {
    let pixel = raw_pixel(ptr, bytes_per_pixel);
    let (mut r, mut g, mut b, mut a) = (0, 0, 0, 0);
    crate::video::pixels::SDL_GetRGBA(pixel, format, &mut r, &mut g, &mut b, &mut a);
    (r, g, b, a)
}

pub(crate) unsafe fn write_rgba_pixel(
    format: *const SDL_PixelFormat,
    bytes_per_pixel: u8,
    ptr: *mut u8,
    rgba: (u8, u8, u8, u8),
) {
    let pixel = crate::video::pixels::SDL_MapRGBA(format, rgba.0, rgba.1, rgba.2, rgba.3);
    write_raw_pixel(ptr, bytes_per_pixel, pixel);
}

pub(crate) unsafe fn pixel_pointer(
    surface: *mut SDL_Surface,
    bytes_per_pixel: u8,
    x: libc::c_int,
    y: libc::c_int,
    message: &'static str,
) -> Result<*mut u8, MathError> {
    let row = checked_math::nonnegative_to_usize("y", y)?;
    let column = checked_math::checked_mul_usize(
        checked_math::nonnegative_to_usize("x", x)?,
        bytes_per_pixel as usize,
        message,
    )?;
    let pitch = usize::try_from((*surface).pitch).map_err(|_| MathError::InvalidParam("pitch"))?;
    let offset = checked_math::calculate_buffer_offset(row, pitch, column, message)?;
    Ok((*surface).pixels.cast::<u8>().add(offset))
}

pub(crate) fn apply_color_mod(value: Uint8, modulation: Uint8) -> Uint8 {
    (((value as u32) * (modulation as u32)) / 255) as Uint8
}

pub(crate) fn blend_pixel(
    src: (Uint8, Uint8, Uint8, Uint8),
    dst: (Uint8, Uint8, Uint8, Uint8),
    mode: SDL_BlendMode,
) -> (Uint8, Uint8, Uint8, Uint8) {
    match mode {
        SDL_BlendMode_SDL_BLENDMODE_BLEND => {
            let src_alpha = src.3 as u32;
            let inv_alpha = 255u32.saturating_sub(src_alpha);
            let blend_component = |src: Uint8, dst: Uint8| -> Uint8 {
                (((src as u32 * src_alpha) + (dst as u32 * inv_alpha)) / 255) as Uint8
            };
            (
                blend_component(src.0, dst.0),
                blend_component(src.1, dst.1),
                blend_component(src.2, dst.2),
                (src_alpha + dst.3 as u32 - ((src_alpha * dst.3 as u32) / 255)).min(255) as Uint8,
            )
        }
        SDL_BlendMode_SDL_BLENDMODE_ADD => (
            ((src.0 as u32 * src.3 as u32) / 255)
                .saturating_add(dst.0 as u32)
                .min(255) as Uint8,
            ((src.1 as u32 * src.3 as u32) / 255)
                .saturating_add(dst.1 as u32)
                .min(255) as Uint8,
            ((src.2 as u32 * src.3 as u32) / 255)
                .saturating_add(dst.2 as u32)
                .min(255) as Uint8,
            dst.3,
        ),
        SDL_BlendMode_SDL_BLENDMODE_MOD => (
            (((src.0 as u32) * (dst.0 as u32)) / 255) as Uint8,
            (((src.1 as u32) * (dst.1 as u32)) / 255) as Uint8,
            (((src.2 as u32) * (dst.2 as u32)) / 255) as Uint8,
            dst.3,
        ),
        _ => src,
    }
}

pub(crate) unsafe fn blit_surface_pixels(
    src: *mut SDL_Surface,
    src_rect: &SDL_Rect,
    dst: *mut SDL_Surface,
    dst_rect: &SDL_Rect,
) -> Result<(), MathError> {
    let src_descriptor = validate_surface_storage(src)?;
    let dst_descriptor = validate_surface_storage(dst)?;
    rect_inside_surface(src, src_rect)?;
    rect_inside_surface(dst, dst_rect)?;

    if src_rect.w != dst_rect.w || src_rect.h != dst_rect.h {
        return Err(MathError::InvalidParam("dstrect"));
    }
    if src_rect.w <= 0 || src_rect.h <= 0 {
        return Ok(());
    }

    let src_format = (*src).format;
    let dst_format = (*dst).format;
    let same_format = !src_format.is_null()
        && !dst_format.is_null()
        && (*src_format).format == (*dst_format).format;
    let state = surface_state(src);
    let simple_copy = !state.color_key_enabled
        && state.color_mod == (255, 255, 255)
        && state.alpha_mod == 255
        && state.blend_mode == SDL_BlendMode_SDL_BLENDMODE_NONE;

    if simple_copy
        && same_format
        && src_descriptor.bytes_per_pixel == dst_descriptor.bytes_per_pixel
    {
        let row_bytes = checked_math::checked_mul_usize(
            checked_math::nonnegative_to_usize("width", src_rect.w)?,
            src_descriptor.bytes_per_pixel as usize,
            "blit copy length overflow",
        )?;
        for row in 0..src_rect.h {
            let src_row = pixel_pointer(
                src,
                src_descriptor.bytes_per_pixel,
                src_rect.x,
                src_rect.y + row,
                "blit copy length overflow",
            )?;
            let dst_row = pixel_pointer(
                dst,
                dst_descriptor.bytes_per_pixel,
                dst_rect.x,
                dst_rect.y + row,
                "blit copy length overflow",
            )?;
            ptr::copy(src_row, dst_row, row_bytes);
        }
        return Ok(());
    }

    for row in 0..src_rect.h {
        let src_row = pixel_pointer(
            src,
            src_descriptor.bytes_per_pixel,
            src_rect.x,
            src_rect.y + row,
            "blit copy length overflow",
        )?;
        let dst_row = pixel_pointer(
            dst,
            dst_descriptor.bytes_per_pixel,
            dst_rect.x,
            dst_rect.y + row,
            "blit copy length overflow",
        )?;

        for column in 0..src_rect.w {
            let src_pixel = src_row.add(column as usize * src_descriptor.bytes_per_pixel as usize);
            let dst_pixel = dst_row.add(column as usize * dst_descriptor.bytes_per_pixel as usize);
            let raw_src = raw_pixel(src_pixel, src_descriptor.bytes_per_pixel);
            if state.color_key_enabled && raw_src == state.color_key {
                continue;
            }

            let mut rgba =
                read_rgba_pixel((*src).format, src_descriptor.bytes_per_pixel, src_pixel);
            rgba.0 = apply_color_mod(rgba.0, state.color_mod.0);
            rgba.1 = apply_color_mod(rgba.1, state.color_mod.1);
            rgba.2 = apply_color_mod(rgba.2, state.color_mod.2);
            rgba.3 = apply_color_mod(rgba.3, state.alpha_mod);

            if state.blend_mode != SDL_BlendMode_SDL_BLENDMODE_NONE {
                let dst_rgba =
                    read_rgba_pixel((*dst).format, dst_descriptor.bytes_per_pixel, dst_pixel);
                rgba = blend_pixel(rgba, dst_rgba, state.blend_mode);
            }
            write_rgba_pixel(
                (*dst).format,
                dst_descriptor.bytes_per_pixel,
                dst_pixel,
                rgba,
            );
        }
    }

    Ok(())
}

pub(crate) unsafe fn scale_surface_pixels_nearest(
    src: *mut SDL_Surface,
    src_rect: &SDL_Rect,
    dst: *mut SDL_Surface,
    dst_rect: &SDL_Rect,
) -> Result<(), MathError> {
    let src_descriptor = validate_surface_storage(src)?;
    let dst_descriptor = validate_surface_storage(dst)?;
    rect_inside_surface(src, src_rect)?;
    rect_inside_surface(dst, dst_rect)?;

    if src_rect.w <= 0 || src_rect.h <= 0 || dst_rect.w <= 0 || dst_rect.h <= 0 {
        return Ok(());
    }

    for dy in 0..dst_rect.h {
        let sy = src_rect.y + ((dy as i64 * src_rect.h as i64) / dst_rect.h as i64) as libc::c_int;
        let dst_row = pixel_pointer(
            dst,
            dst_descriptor.bytes_per_pixel,
            dst_rect.x,
            dst_rect.y + dy,
            "blit copy length overflow",
        )?;
        for dx in 0..dst_rect.w {
            let sx =
                src_rect.x + ((dx as i64 * src_rect.w as i64) / dst_rect.w as i64) as libc::c_int;
            let src_pixel = pixel_pointer(
                src,
                src_descriptor.bytes_per_pixel,
                sx,
                sy,
                "blit copy length overflow",
            )?;
            let dst_pixel = dst_row.add(dx as usize * dst_descriptor.bytes_per_pixel as usize);
            let rgba = read_rgba_pixel((*src).format, src_descriptor.bytes_per_pixel, src_pixel);
            write_rgba_pixel(
                (*dst).format,
                dst_descriptor.bytes_per_pixel,
                dst_pixel,
                rgba,
            );
        }
    }

    Ok(())
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

unsafe fn create_surface_shell(
    _depth: libc::c_int,
    format: Uint32,
    default_error: &str,
) -> Option<(*mut SDL_Surface, SurfaceShell, bool)> {
    let format_ptr = crate::video::pixels::SDL_AllocFormat(format);
    if format_ptr.is_null() {
        let _ = set_error_message(default_error);
        return None;
    }

    let mut shell = Box::new(SDL_Surface {
        flags: 0,
        format: format_ptr,
        w: 0,
        h: 0,
        pitch: 0,
        pixels: ptr::null_mut(),
        userdata: ptr::null_mut(),
        locked: 0,
        list_blitmap: ptr::null_mut(),
        clip_rect: SDL_Rect {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        },
        map: ptr::null_mut(),
        refcount: 1,
    });
    let surface = shell.as_mut() as *mut SDL_Surface;
    let format = (*surface).format;
    if !format.is_null()
        && (*format).BitsPerPixel > 0
        && (*format).BitsPerPixel <= 8
        && (*format).Rmask == 0
        && (*format).Gmask == 0
        && (*format).Bmask == 0
        && (*format).Amask == 0
    {
        let colors = 1i32.checked_shl((*format).BitsPerPixel as u32).unwrap_or(0);
        let palette = crate::video::pixels::SDL_AllocPalette(colors);
        if palette.is_null() {
            crate::video::pixels::SDL_FreeFormat(format);
            return None;
        }
        if colors == 2 && !(*palette).colors.is_null() {
            *(*palette).colors.add(0) = SDL_Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            };
            *(*palette).colors.add(1) = SDL_Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            };
        }
        if crate::video::pixels::SDL_SetPixelFormatPalette(format, palette) < 0 {
            crate::video::pixels::SDL_FreePalette(palette);
            crate::video::pixels::SDL_FreeFormat(format);
            return None;
        }
        crate::video::pixels::SDL_FreePalette(palette);
    }
    Some((surface, SurfaceShell::Local(shell), true))
}

unsafe fn destroy_surface_shell(surface: *mut SDL_Surface, shell: SurfaceShell, owns_format: bool) {
    let format = if surface.is_null() {
        ptr::null_mut()
    } else {
        (*surface).format
    };
    if owns_format && !format.is_null() {
        crate::video::pixels::SDL_FreeFormat(format);
    }
    match shell {
        SurfaceShell::Host => {
            if !surface.is_null() {
                (real_sdl().free_surface)(surface);
            }
        }
        SurfaceShell::Local(_) => {}
    }
}

fn allocate_pixel_buffer(size: usize) -> Result<Vec<u8>, ()> {
    let mut buffer = Vec::new();
    buffer.try_reserve_exact(size).map_err(|_| ())?;
    buffer.resize(size, 0);
    Ok(buffer)
}

unsafe fn finalize_registered_surface(
    surface: *mut SDL_Surface,
    width: libc::c_int,
    height: libc::c_int,
    pitch: libc::c_int,
    pixels: *mut libc::c_void,
    mut record: SurfaceRecord,
) -> *mut SDL_Surface {
    (*surface).flags |= SDL_PREALLOC;
    (*surface).w = width;
    (*surface).h = height;
    (*surface).pitch = pitch;
    (*surface).pixels = pixels;
    (*surface).clip_rect = SDL_Rect {
        x: 0,
        y: 0,
        w: width.max(0),
        h: height.max(0),
    };
    (*surface).locked = 0;
    (*surface).refcount = 1;
    record.blend_mode = default_blend_mode((*surface).format);
    register_surface_record(surface, record);
    surface
}

pub(crate) unsafe fn create_owned_surface_with_format(
    _flags: Uint32,
    width: libc::c_int,
    height: libc::c_int,
    depth: libc::c_int,
    format: Uint32,
) -> *mut SDL_Surface {
    if let Err(error) = preflight_surface_allocation(format, width, height) {
        return apply_math_error_ptr(error);
    }

    let Some((surface, shell, owns_format)) =
        create_surface_shell(depth, format, "Couldn't create RGB surface")
    else {
        return ptr::null_mut();
    };

    let descriptor = match descriptor_from_format_ptr((*surface).format) {
        Ok(descriptor) => descriptor,
        Err(error) => {
            destroy_surface_shell(surface, shell, owns_format);
            return apply_math_error_ptr(error);
        }
    };
    let (pitch, size) = match checked_math::calculate_surface_allocation(
        width,
        height,
        descriptor.bits_per_pixel,
        descriptor.bytes_per_pixel,
    ) {
        Ok(result) => result,
        Err(error) => {
            destroy_surface_shell(surface, shell, owns_format);
            return apply_math_error_ptr(error);
        }
    };

    let mut buffer = match allocate_pixel_buffer(size) {
        Ok(buffer) => buffer,
        Err(()) => {
            destroy_surface_shell(surface, shell, owns_format);
            let _ = out_of_memory_error();
            return ptr::null_mut();
        }
    };
    let pixels = if size == 0 {
        ptr::null_mut()
    } else {
        buffer.as_mut_ptr().cast()
    };

    finalize_registered_surface(
        surface,
        width,
        height,
        pitch as libc::c_int,
        pixels,
        SurfaceRecord {
            buffer_len: size,
            expected_pixels: pixels as usize,
            storage: SurfaceStorage::Owned(buffer),
            shell,
            owns_format,
            color_key_enabled: false,
            color_key: 0,
            color_mod: (255, 255, 255),
            alpha_mod: 255,
            blend_mode: SDL_BlendMode_SDL_BLENDMODE_NONE,
            rle_enabled: false,
        },
    )
}

pub(crate) unsafe fn create_preallocated_surface_with_format(
    _flags: Uint32,
    pixels: *mut libc::c_void,
    width: libc::c_int,
    height: libc::c_int,
    depth: libc::c_int,
    pitch: libc::c_int,
    format: Uint32,
) -> *mut SDL_Surface {
    if let Err(error) = preflight_preallocated_surface(format, width, height, pitch) {
        return apply_math_error_ptr(error);
    }

    if pixels.is_null() && width > 0 && height > 0 && pitch > 0 {
        let _ = invalid_param_error("pixels");
        return ptr::null_mut();
    }

    let Some((surface, shell, owns_format)) =
        create_surface_shell(depth, format, "Couldn't create RGB surface")
    else {
        return ptr::null_mut();
    };

    let descriptor = match descriptor_from_format_ptr((*surface).format) {
        Ok(descriptor) => descriptor,
        Err(error) => {
            destroy_surface_shell(surface, shell, owns_format);
            return apply_math_error_ptr(error);
        }
    };
    let size = match checked_math::validate_preallocated_surface(
        width,
        height,
        pitch,
        descriptor.bits_per_pixel,
        descriptor.bytes_per_pixel,
    ) {
        Ok(size) => size,
        Err(error) => {
            destroy_surface_shell(surface, shell, owns_format);
            return apply_math_error_ptr(error);
        }
    };

    finalize_registered_surface(
        surface,
        width,
        height,
        pitch,
        pixels,
        SurfaceRecord {
            buffer_len: size,
            expected_pixels: pixels as usize,
            storage: SurfaceStorage::Borrowed,
            shell,
            owns_format,
            color_key_enabled: false,
            color_key: 0,
            color_mod: (255, 255, 255),
            alpha_mod: 255,
            blend_mode: SDL_BlendMode_SDL_BLENDMODE_NONE,
            rle_enabled: false,
        },
    )
}

unsafe fn copy_palette_if_present(dst: *mut SDL_Surface, format: *const SDL_PixelFormat) {
    if format.is_null() || (*format).palette.is_null() || (*(*dst).format).palette.is_null() {
        return;
    }

    let palette = (*format).palette;
    let dst_palette = (*(*dst).format).palette;
    let count = (*palette).ncolors.min((*dst_palette).ncolors);
    if count > 0 {
        let _ =
            crate::video::pixels::SDL_SetPaletteColors(dst_palette, (*palette).colors, 0, count);
    }
}

unsafe fn copy_surface_state(src: *mut SDL_Surface, dst: *mut SDL_Surface) {
    let clip = (*src).clip_rect;
    let _ = SDL_SetClipRect(dst, &clip);
    let state = surface_state(src);
    let _ = SDL_SetSurfaceColorMod(dst, state.color_mod.0, state.color_mod.1, state.color_mod.2);
    let _ = SDL_SetSurfaceAlphaMod(dst, state.alpha_mod);
    let _ = SDL_SetSurfaceBlendMode(dst, state.blend_mode);
    let _ = SDL_SetColorKey(
        dst,
        if state.color_key_enabled { 1 } else { 0 },
        state.color_key,
    );
    let rle = with_surface_record(src, |record| record.rle_enabled).unwrap_or(false);
    let _ = SDL_SetSurfaceRLE(dst, if rle { 1 } else { 0 });
}

unsafe fn convert_surface_pixels_local(
    src: *mut SDL_Surface,
    dst: *mut SDL_Surface,
) -> Result<(), MathError> {
    let src_descriptor = validate_surface_storage(src)?;
    let dst_descriptor = validate_surface_storage(dst)?;
    if (*src).w != (*dst).w || (*src).h != (*dst).h {
        return Err(MathError::InvalidParam("dstrect"));
    }
    if (*src).w <= 0 || (*src).h <= 0 {
        return Ok(());
    }

    let src_format = (*src).format;
    let dst_format = (*dst).format;
    let same_format = !src_format.is_null()
        && !dst_format.is_null()
        && (*src_format).format == (*dst_format).format;

    if same_format && src_descriptor.bytes_per_pixel == dst_descriptor.bytes_per_pixel {
        let row_bytes = checked_math::validate_copy_layout(
            (*src).w,
            (*src).h,
            src_descriptor.bits_per_pixel,
            src_descriptor.bytes_per_pixel,
            (*src).pitch,
        )?
        .0;
        for row in 0..(*src).h {
            let src_row = pixel_pointer(
                src,
                src_descriptor.bytes_per_pixel,
                0,
                row,
                "surface convert copy overflow",
            )?;
            let dst_row = pixel_pointer(
                dst,
                dst_descriptor.bytes_per_pixel,
                0,
                row,
                "surface convert copy overflow",
            )?;
            ptr::copy(src_row, dst_row, row_bytes);
        }
        return Ok(());
    }

    for row in 0..(*src).h {
        for column in 0..(*src).w {
            let src_pixel = pixel_pointer(
                src,
                src_descriptor.bytes_per_pixel,
                column,
                row,
                "surface convert copy overflow",
            )?;
            let dst_pixel = pixel_pointer(
                dst,
                dst_descriptor.bytes_per_pixel,
                column,
                row,
                "surface convert copy overflow",
            )?;
            let rgba = read_rgba_pixel((*src).format, src_descriptor.bytes_per_pixel, src_pixel);
            write_rgba_pixel(
                (*dst).format,
                dst_descriptor.bytes_per_pixel,
                dst_pixel,
                rgba,
            );
        }
    }

    Ok(())
}

unsafe fn convert_registered_surface_local(
    src: *mut SDL_Surface,
    fmt: *const SDL_PixelFormat,
    flags: Uint32,
) -> *mut SDL_Surface {
    let converted = create_owned_surface_with_format(
        flags,
        (*src).w,
        (*src).h,
        (*fmt).BitsPerPixel as libc::c_int,
        (*fmt).format,
    );
    if converted.is_null() {
        return ptr::null_mut();
    }

    copy_palette_if_present(converted, fmt);
    if convert_surface_pixels_local(src, converted).is_err() {
        SDL_FreeSurface(converted);
        let _ = set_error_message("Couldn't convert surface");
        return ptr::null_mut();
    }
    copy_surface_state(src, converted);
    converted
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

    let format =
        crate::video::pixels::SDL_MasksToPixelFormatEnum(depth, Rmask, Gmask, Bmask, Amask);
    if format == 0 {
        let _ = set_error_message("Unknown pixel format");
        return ptr::null_mut();
    }
    create_owned_surface_with_format(flags, width, height, depth, format)
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
    create_owned_surface_with_format(flags, width, height, depth, format)
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

    let format =
        crate::video::pixels::SDL_MasksToPixelFormatEnum(depth, Rmask, Gmask, Bmask, Amask);
    if format == 0 {
        let _ = set_error_message("Unknown pixel format");
        return ptr::null_mut();
    }

    create_preallocated_surface_with_format(0, pixels, width, height, depth, pitch, format)
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

    create_preallocated_surface_with_format(0, pixels, width, height, depth, pitch, format)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_FreeSurface(surface: *mut SDL_Surface) {
    if surface.is_null() {
        return;
    }

    if let Some(record) = take_surface_record(surface) {
        let SurfaceRecord {
            shell, owns_format, ..
        } = record;
        destroy_surface_shell(surface, shell, owns_format);
        return;
    }
    if crate::video::real_sdl_is_available() {
        (real_sdl().free_surface)(surface);
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetSurfacePalette(
    surface: *mut SDL_Surface,
    palette: *mut SDL_Palette,
) -> libc::c_int {
    if surface.is_null() {
        return invalid_param_error("surface");
    }
    if is_registered_surface(surface) {
        if (*surface).format.is_null() {
            return invalid_param_error("surface");
        }
        return crate::video::pixels::SDL_SetPixelFormatPalette((*surface).format, palette);
    }
    clear_real_error();
    let result = (real_sdl().set_surface_palette)(surface, palette);
    if result < 0 {
        let _ = sync_error_from_real("Couldn't set surface palette");
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn SDL_LockSurface(surface: *mut SDL_Surface) -> libc::c_int {
    if surface.is_null() {
        return invalid_param_error("surface");
    }
    if is_registered_surface(surface) {
        if let Err(error) = validate_surface_storage(surface) {
            return apply_math_error(error);
        }
        (*surface).locked += 1;
        return 0;
    }
    clear_real_error();
    let result = (real_sdl().lock_surface)(surface);
    if result < 0 {
        let _ = sync_error_from_real("Couldn't lock surface");
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn SDL_UnlockSurface(surface: *mut SDL_Surface) {
    if surface.is_null() {
        return;
    }
    if is_registered_surface(surface) {
        if (*surface).locked > 0 {
            (*surface).locked -= 1;
        }
        return;
    }
    (real_sdl().unlock_surface)(surface);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetSurfaceRLE(
    surface: *mut SDL_Surface,
    flag: libc::c_int,
) -> libc::c_int {
    if surface.is_null() {
        return invalid_param_error("surface");
    }
    if is_registered_surface(surface) {
        let enabled = flag != 0;
        let _ = with_surface_record_mut(surface, |record| record.rle_enabled = enabled);
        if enabled {
            (*surface).flags |= SDL_RLEACCEL;
        } else {
            (*surface).flags &= !SDL_RLEACCEL;
        }
        return 0;
    }
    clear_real_error();
    let result = (real_sdl().set_surface_rle)(surface, flag);
    if result < 0 {
        let _ = sync_error_from_real("Couldn't set surface RLE");
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HasSurfaceRLE(surface: *mut SDL_Surface) -> SDL_bool {
    if surface.is_null() {
        return 0;
    }
    if is_registered_surface(surface) {
        return with_surface_record(surface, |record| record.rle_enabled as SDL_bool).unwrap_or(0);
    }
    (real_sdl().has_surface_rle)(surface)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetColorKey(
    surface: *mut SDL_Surface,
    flag: libc::c_int,
    key: Uint32,
) -> libc::c_int {
    if surface.is_null() {
        return invalid_param_error("surface");
    }
    if is_registered_surface(surface) {
        let enabled = flag != 0;
        let _ = with_surface_record_mut(surface, |record| {
            record.color_key_enabled = enabled;
            record.color_key = key;
            record.rle_enabled = (flag & SDL_RLEACCEL as libc::c_int) != 0;
        });
        if (flag & SDL_RLEACCEL as libc::c_int) != 0 {
            (*surface).flags |= SDL_RLEACCEL;
        } else {
            (*surface).flags &= !SDL_RLEACCEL;
        }
        return 0;
    }
    clear_real_error();
    let result = (real_sdl().set_color_key)(surface, flag, key);
    if result < 0 {
        let _ = sync_error_from_real("Couldn't set color key");
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HasColorKey(surface: *mut SDL_Surface) -> SDL_bool {
    if surface.is_null() {
        return 0;
    }
    if is_registered_surface(surface) {
        return surface_state(surface).color_key_enabled as SDL_bool;
    }
    (real_sdl().has_color_key)(surface)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetColorKey(
    surface: *mut SDL_Surface,
    key: *mut Uint32,
) -> libc::c_int {
    if surface.is_null() {
        return invalid_param_error("surface");
    }
    if key.is_null() {
        return invalid_param_error("key");
    }
    if is_registered_surface(surface) {
        let state = surface_state(surface);
        if !state.color_key_enabled {
            return set_error_message("Surface doesn't have a colorkey");
        }
        *key = state.color_key;
        return 0;
    }
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
    if surface.is_null() {
        return invalid_param_error("surface");
    }
    if is_registered_surface(surface) {
        let _ = with_surface_record_mut(surface, |record| record.color_mod = (r, g, b));
        return 0;
    }
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
    if surface.is_null() {
        return invalid_param_error("surface");
    }
    if r.is_null() || g.is_null() || b.is_null() {
        return invalid_param_error("r");
    }
    if is_registered_surface(surface) {
        let modulate = surface_state(surface).color_mod;
        *r = modulate.0;
        *g = modulate.1;
        *b = modulate.2;
        return 0;
    }
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
    if surface.is_null() {
        return invalid_param_error("surface");
    }
    if is_registered_surface(surface) {
        let _ = with_surface_record_mut(surface, |record| record.alpha_mod = alpha);
        return 0;
    }
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
    if surface.is_null() {
        return invalid_param_error("surface");
    }
    if alpha.is_null() {
        return invalid_param_error("alpha");
    }
    if is_registered_surface(surface) {
        *alpha = surface_state(surface).alpha_mod;
        return 0;
    }
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
    if surface.is_null() {
        return invalid_param_error("surface");
    }
    if is_registered_surface(surface) {
        let _ = with_surface_record_mut(surface, |record| record.blend_mode = blendMode);
        return 0;
    }
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
    if surface.is_null() {
        return invalid_param_error("surface");
    }
    if blendMode.is_null() {
        return invalid_param_error("blendMode");
    }
    if is_registered_surface(surface) {
        *blendMode = surface_state(surface).blend_mode;
        return 0;
    }
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
    if surface.is_null() {
        return 0;
    }
    if is_registered_surface(surface) {
        let bounds = full_surface_rect(surface);
        let clipped = if rect.is_null() {
            bounds
        } else {
            intersect_rects(&bounds, &*rect).unwrap_or(SDL_Rect {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
            })
        };
        (*surface).clip_rect = clipped;
        return (clipped.w > 0 && clipped.h > 0) as SDL_bool;
    }
    (real_sdl().set_clip_rect)(surface, rect)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetClipRect(surface: *mut SDL_Surface, rect: *mut SDL_Rect) {
    if surface.is_null() || rect.is_null() {
        return;
    }
    if is_registered_surface(surface) {
        *rect = (*surface).clip_rect;
        return;
    }
    (real_sdl().get_clip_rect)(surface, rect);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_DuplicateSurface(surface: *mut SDL_Surface) -> *mut SDL_Surface {
    if let Err(error) = validate_surface_storage(surface) {
        return apply_math_error_ptr(error);
    }
    SDL_ConvertSurface(surface, (*surface).format, (*surface).flags)
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
    if fmt.is_null() {
        let _ = invalid_param_error("format");
        return ptr::null_mut();
    }

    if !is_registered_surface(src) {
        clear_real_error();
        let converted = (real_sdl().convert_surface)(src, fmt, flags);
        if converted.is_null() {
            let _ = sync_error_from_real("Couldn't convert surface");
        }
        return converted;
    }

    convert_registered_surface_local(src, fmt, flags)
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

    let fmt = crate::video::pixels::SDL_AllocFormat(pixel_format);
    if fmt.is_null() {
        let _ = set_error_message("Couldn't convert surface");
        return ptr::null_mut();
    }

    let converted = SDL_ConvertSurface(src, fmt, flags);
    crate::video::pixels::SDL_FreeFormat(fmt);
    converted
}

#[no_mangle]
pub unsafe extern "C" fn SDL_FillRect(
    dst: *mut SDL_Surface,
    rect: *const SDL_Rect,
    color: Uint32,
) -> libc::c_int {
    if dst.is_null() {
        return invalid_param_error("dst");
    }
    if !is_registered_surface(dst) {
        if let Err(error) = validate_surface_storage(dst) {
            return apply_math_error(error);
        }
        clear_real_error();
        let result = (real_sdl().fill_rect)(dst, rect, color);
        if result < 0 {
            let _ = sync_error_from_real("Couldn't fill surface");
        }
        return result;
    }

    let fill_rect = if rect.is_null() {
        (*dst).clip_rect
    } else {
        *rect
    };
    SDL_FillRects(dst, &fill_rect, 1, color)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_FillRects(
    dst: *mut SDL_Surface,
    rects: *const SDL_Rect,
    count: libc::c_int,
    color: Uint32,
) -> libc::c_int {
    if dst.is_null() {
        return invalid_param_error("dst");
    }
    if !is_registered_surface(dst) {
        if let Err(error) = validate_surface_storage(dst) {
            return apply_math_error(error);
        }
        clear_real_error();
        let result = (real_sdl().fill_rects)(dst, rects, count, color);
        if result < 0 {
            let _ = sync_error_from_real("Couldn't fill surface");
        }
        return result;
    }

    let descriptor = match validate_surface_storage(dst) {
        Ok(descriptor) => descriptor,
        Err(error) => return apply_math_error(error),
    };

    if count < 0 {
        return invalid_param_error("count");
    }
    if count == 0 {
        return 0;
    }
    if rects.is_null() {
        return invalid_param_error("rects");
    }
    if descriptor.bits_per_pixel < 8 || descriptor.bytes_per_pixel == 0 {
        return set_error_message("SDL_FillRects(): Unsupported surface format");
    }

    let bounds = full_surface_rect(dst);
    let clip = intersect_rects(&bounds, &(*dst).clip_rect).unwrap_or(SDL_Rect {
        x: 0,
        y: 0,
        w: 0,
        h: 0,
    });
    let packed = color;

    for index in 0..count as isize {
        let rect = *rects.offset(index);
        if let Some(clipped) = intersect_rects(&rect, &clip) {
            for row in 0..clipped.h {
                let row_ptr = match pixel_pointer(
                    dst,
                    descriptor.bytes_per_pixel,
                    clipped.x,
                    clipped.y + row,
                    "blit copy length overflow",
                ) {
                    Ok(ptr) => ptr,
                    Err(error) => return apply_math_error(error),
                };
                for column in 0..clipped.w {
                    write_raw_pixel(
                        row_ptr.add(column as usize * descriptor.bytes_per_pixel as usize),
                        descriptor.bytes_per_pixel,
                        packed,
                    );
                }
            }
        }
    }

    0
}
