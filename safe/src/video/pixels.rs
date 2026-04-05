use crate::abi::generated_types::{
    SDL_Color, SDL_Palette, SDL_PixelFormat, SDL_bool, Uint32, Uint8,
};
use crate::core::error::invalid_param_error;
use crate::video::surface::{clear_real_error, real_sdl, sync_error_from_real};

#[no_mangle]
pub unsafe extern "C" fn SDL_AllocFormat(pixel_format: Uint32) -> *mut SDL_PixelFormat {
    clear_real_error();
    let format = (real_sdl().alloc_format)(pixel_format);
    if format.is_null() {
        let _ = sync_error_from_real("Couldn't allocate pixel format");
    }
    format
}

#[no_mangle]
pub unsafe extern "C" fn SDL_FreeFormat(format: *mut SDL_PixelFormat) {
    (real_sdl().free_format)(format);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_AllocPalette(ncolors: libc::c_int) -> *mut SDL_Palette {
    clear_real_error();
    let palette = (real_sdl().alloc_palette)(ncolors);
    if palette.is_null() {
        let _ = sync_error_from_real("Couldn't allocate palette");
    }
    palette
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetPixelFormatPalette(
    format: *mut SDL_PixelFormat,
    palette: *mut SDL_Palette,
) -> libc::c_int {
    clear_real_error();
    let result = (real_sdl().set_pixel_format_palette)(format, palette);
    if result < 0 {
        let _ = sync_error_from_real("Couldn't set pixel format palette");
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetPaletteColors(
    palette: *mut SDL_Palette,
    colors: *const SDL_Color,
    firstcolor: libc::c_int,
    ncolors: libc::c_int,
) -> libc::c_int {
    clear_real_error();
    let result = (real_sdl().set_palette_colors)(palette, colors, firstcolor, ncolors);
    if result < 0 {
        let _ = sync_error_from_real("Couldn't set palette colors");
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn SDL_FreePalette(palette: *mut SDL_Palette) {
    (real_sdl().free_palette)(palette);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetPixelFormatName(format: Uint32) -> *const libc::c_char {
    clear_real_error();
    (real_sdl().get_pixel_format_name)(format)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_PixelFormatEnumToMasks(
    format: Uint32,
    bpp: *mut libc::c_int,
    Rmask: *mut Uint32,
    Gmask: *mut Uint32,
    Bmask: *mut Uint32,
    Amask: *mut Uint32,
) -> SDL_bool {
    clear_real_error();
    let ok = (real_sdl().pixel_format_enum_to_masks)(format, bpp, Rmask, Gmask, Bmask, Amask);
    if ok == 0 {
        let _ = sync_error_from_real("Couldn't decode pixel format masks");
    }
    ok
}

#[no_mangle]
pub unsafe extern "C" fn SDL_MasksToPixelFormatEnum(
    bpp: libc::c_int,
    Rmask: Uint32,
    Gmask: Uint32,
    Bmask: Uint32,
    Amask: Uint32,
) -> Uint32 {
    (real_sdl().masks_to_pixel_format_enum)(bpp, Rmask, Gmask, Bmask, Amask)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_MapRGB(
    format: *const SDL_PixelFormat,
    r: Uint8,
    g: Uint8,
    b: Uint8,
) -> Uint32 {
    (real_sdl().map_rgb)(format, r, g, b)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_MapRGBA(
    format: *const SDL_PixelFormat,
    r: Uint8,
    g: Uint8,
    b: Uint8,
    a: Uint8,
) -> Uint32 {
    (real_sdl().map_rgba)(format, r, g, b, a)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetRGB(
    pixel: Uint32,
    format: *const SDL_PixelFormat,
    r: *mut Uint8,
    g: *mut Uint8,
    b: *mut Uint8,
) {
    (real_sdl().get_rgb)(pixel, format, r, g, b);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetRGBA(
    pixel: Uint32,
    format: *const SDL_PixelFormat,
    r: *mut Uint8,
    g: *mut Uint8,
    b: *mut Uint8,
    a: *mut Uint8,
) {
    (real_sdl().get_rgba)(pixel, format, r, g, b, a);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CalculateGammaRamp(gamma: f32, ramp: *mut u16) {
    if gamma < 0.0 {
        let _ = invalid_param_error("gamma");
        return;
    }
    if ramp.is_null() {
        let _ = invalid_param_error("ramp");
        return;
    }
    if gamma == 0.0 {
        std::ptr::write_bytes(ramp, 0, 256);
        return;
    }
    if gamma == 1.0 {
        for index in 0..256 {
            *ramp.add(index) = ((index as u16) << 8) | index as u16;
        }
        return;
    }

    let gamma = 1.0f64 / gamma as f64;
    for index in 0..256 {
        let mut value = (((index as f64) / 256.0).powf(gamma) * 65535.0 + 0.5) as i32;
        if value > 65535 {
            value = 65535;
        }
        *ramp.add(index) = value as u16;
    }
}
