use crate::abi::generated_types::{
    SDL_Color, SDL_Palette, SDL_PixelFormat, SDL_bool, Uint32, Uint8,
};
use crate::core::error::invalid_param_error;
use crate::video::surface::{clear_real_error, real_sdl, sync_error_from_real};

const LOOKUP_0: [u8; 256] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
    50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73,
    74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97,
    98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116,
    117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135,
    136, 137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154,
    155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173,
    174, 175, 176, 177, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 190, 191, 192,
    193, 194, 195, 196, 197, 198, 199, 200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211,
    212, 213, 214, 215, 216, 217, 218, 219, 220, 221, 222, 223, 224, 225, 226, 227, 228, 229, 230,
    231, 232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249,
    250, 251, 252, 253, 254, 255,
];
const LOOKUP_1: [u8; 128] = [
    0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24, 26, 28, 30, 32, 34, 36, 38, 40, 42, 44, 46, 48,
    50, 52, 54, 56, 58, 60, 62, 64, 66, 68, 70, 72, 74, 76, 78, 80, 82, 84, 86, 88, 90, 92, 94, 96,
    98, 100, 102, 104, 106, 108, 110, 112, 114, 116, 118, 120, 122, 124, 126, 128, 130, 132, 134,
    136, 138, 140, 142, 144, 146, 148, 150, 152, 154, 156, 158, 160, 162, 164, 166, 168, 170, 172,
    174, 176, 178, 180, 182, 184, 186, 188, 190, 192, 194, 196, 198, 200, 202, 204, 206, 208, 210,
    212, 214, 216, 218, 220, 222, 224, 226, 228, 230, 232, 234, 236, 238, 240, 242, 244, 246, 248,
    250, 252, 255,
];
const LOOKUP_2: [u8; 64] = [
    0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44, 48, 52, 56, 60, 64, 68, 72, 76, 80, 85, 89, 93,
    97, 101, 105, 109, 113, 117, 121, 125, 129, 133, 137, 141, 145, 149, 153, 157, 161, 165, 170,
    174, 178, 182, 186, 190, 194, 198, 202, 206, 210, 214, 218, 222, 226, 230, 234, 238, 242, 246,
    250, 255,
];
const LOOKUP_3: [u8; 32] = [
    0, 8, 16, 24, 32, 41, 49, 57, 65, 74, 82, 90, 98, 106, 115, 123, 131, 139, 148, 156, 164, 172,
    180, 189, 197, 205, 213, 222, 230, 238, 246, 255,
];
const LOOKUP_4: [u8; 16] = [
    0, 17, 34, 51, 68, 85, 102, 119, 136, 153, 170, 187, 204, 221, 238, 255,
];
const LOOKUP_5: [u8; 8] = [0, 36, 72, 109, 145, 182, 218, 255];
const LOOKUP_6: [u8; 4] = [0, 85, 170, 255];
const LOOKUP_7: [u8; 2] = [0, 255];
const LOOKUP_8: [u8; 1] = [255];

fn expand_component(loss: u8, value: u32) -> u8 {
    match loss {
        0 => LOOKUP_0[value as usize],
        1 => LOOKUP_1[value as usize],
        2 => LOOKUP_2[value as usize],
        3 => LOOKUP_3[value as usize],
        4 => LOOKUP_4[value as usize],
        5 => LOOKUP_5[value as usize],
        6 => LOOKUP_6[value as usize],
        7 => LOOKUP_7[value as usize],
        8 => LOOKUP_8[0],
        _ => 0,
    }
}

fn scale_component_to_mask(value: Uint8, mask: Uint32, shift: Uint8, loss: Uint8) -> Uint32 {
    if mask == 0 {
        return 0;
    }

    let narrowed = if loss <= 8 {
        (value as Uint32) >> loss
    } else {
        let bits = mask.count_ones();
        let max = (1u32 << bits) - 1;
        ((value as Uint32 * max) + 127) / 255
    };
    (narrowed << shift) & mask
}

fn scale_component_from_mask(
    pixel: Uint32,
    mask: Uint32,
    shift: Uint8,
    loss: Uint8,
    default: Uint8,
) -> Uint8 {
    if mask == 0 {
        return default;
    }

    let value = (pixel & mask) >> shift;
    if loss <= 8 {
        expand_component(loss, value)
    } else {
        let bits = mask.count_ones();
        let max = (1u32 << bits) - 1;
        (((value * 255) + (max / 2)) / max) as Uint8
    }
}

unsafe fn find_palette_color(
    palette: *mut SDL_Palette,
    r: Uint8,
    g: Uint8,
    b: Uint8,
    a: Uint8,
) -> Uint32 {
    if palette.is_null() || (*palette).colors.is_null() || (*palette).ncolors <= 0 {
        return 0;
    }

    let mut best_index = 0usize;
    let mut best_distance = u32::MAX;
    for index in 0..((*palette).ncolors as usize) {
        let color = *(*palette).colors.add(index);
        let dr = color.r.abs_diff(r) as u32;
        let dg = color.g.abs_diff(g) as u32;
        let db = color.b.abs_diff(b) as u32;
        let da = color.a.abs_diff(a) as u32;
        let distance = dr * dr + dg * dg + db * db + da * da;
        if distance < best_distance {
            best_distance = distance;
            best_index = index;
            if distance == 0 {
                break;
            }
        }
    }
    best_index as Uint32
}

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
    if format.is_null() {
        let _ = invalid_param_error("format");
        return 0;
    }
    if (*format).palette.is_null() {
        scale_component_to_mask(r, (*format).Rmask, (*format).Rshift, (*format).Rloss)
            | scale_component_to_mask(g, (*format).Gmask, (*format).Gshift, (*format).Gloss)
            | scale_component_to_mask(b, (*format).Bmask, (*format).Bshift, (*format).Bloss)
            | (*format).Amask
    } else {
        find_palette_color((*format).palette, r, g, b, 255)
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_MapRGBA(
    format: *const SDL_PixelFormat,
    r: Uint8,
    g: Uint8,
    b: Uint8,
    a: Uint8,
) -> Uint32 {
    if format.is_null() {
        let _ = invalid_param_error("format");
        return 0;
    }
    if (*format).palette.is_null() {
        scale_component_to_mask(r, (*format).Rmask, (*format).Rshift, (*format).Rloss)
            | scale_component_to_mask(g, (*format).Gmask, (*format).Gshift, (*format).Gloss)
            | scale_component_to_mask(b, (*format).Bmask, (*format).Bshift, (*format).Bloss)
            | scale_component_to_mask(a, (*format).Amask, (*format).Ashift, (*format).Aloss)
    } else {
        find_palette_color((*format).palette, r, g, b, a)
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetRGB(
    pixel: Uint32,
    format: *const SDL_PixelFormat,
    r: *mut Uint8,
    g: *mut Uint8,
    b: *mut Uint8,
) {
    if format.is_null() || r.is_null() || g.is_null() || b.is_null() {
        return;
    }
    if (*format).palette.is_null() {
        *r =
            scale_component_from_mask(pixel, (*format).Rmask, (*format).Rshift, (*format).Rloss, 0);
        *g =
            scale_component_from_mask(pixel, (*format).Gmask, (*format).Gshift, (*format).Gloss, 0);
        *b =
            scale_component_from_mask(pixel, (*format).Bmask, (*format).Bshift, (*format).Bloss, 0);
    } else if pixel < (*(*format).palette).ncolors as Uint32 {
        let color = *(*(*format).palette).colors.add(pixel as usize);
        *r = color.r;
        *g = color.g;
        *b = color.b;
    } else {
        *r = 0;
        *g = 0;
        *b = 0;
    }
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
    if format.is_null() || r.is_null() || g.is_null() || b.is_null() || a.is_null() {
        return;
    }
    if (*format).palette.is_null() {
        *r =
            scale_component_from_mask(pixel, (*format).Rmask, (*format).Rshift, (*format).Rloss, 0);
        *g =
            scale_component_from_mask(pixel, (*format).Gmask, (*format).Gshift, (*format).Gloss, 0);
        *b =
            scale_component_from_mask(pixel, (*format).Bmask, (*format).Bshift, (*format).Bloss, 0);
        *a = scale_component_from_mask(
            pixel,
            (*format).Amask,
            (*format).Ashift,
            (*format).Aloss,
            255,
        );
    } else if pixel < (*(*format).palette).ncolors as Uint32 {
        let color = *(*(*format).palette).colors.add(pixel as usize);
        *r = color.r;
        *g = color.g;
        *b = color.b;
        *a = color.a;
    } else {
        *r = 0;
        *g = 0;
        *b = 0;
        *a = 0;
    }
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
