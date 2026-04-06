use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::abi::generated_types::{
    SDL_Color, SDL_Palette, SDL_PixelFormat, SDL_PixelFormatEnum_SDL_PIXELFORMAT_ABGR1555,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_ABGR4444, SDL_PixelFormatEnum_SDL_PIXELFORMAT_ABGR8888,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB1555, SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB2101010,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB4444, SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGR24, SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGR444,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGR555, SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGR565,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGR888, SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGRA4444,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGRA5551, SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGRA8888,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGRX8888, SDL_PixelFormatEnum_SDL_PIXELFORMAT_INDEX1LSB,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_INDEX1MSB, SDL_PixelFormatEnum_SDL_PIXELFORMAT_INDEX2LSB,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_INDEX2MSB, SDL_PixelFormatEnum_SDL_PIXELFORMAT_INDEX4LSB,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_INDEX4MSB, SDL_PixelFormatEnum_SDL_PIXELFORMAT_INDEX8,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_IYUV, SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV12,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV21, SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB24,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB332, SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB444,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB555, SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB565,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB888, SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBA32,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBA4444, SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBA5551,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBA8888, SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBX8888,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_UYVY, SDL_PixelFormatEnum_SDL_PIXELFORMAT_YUY2,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_YV12, SDL_PixelFormatEnum_SDL_PIXELFORMAT_YVYU, SDL_bool,
    Uint32, Uint8,
};
use crate::core::error::{invalid_param_error, set_error_message};
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

struct LocalPaletteOwner {
    palette: Box<SDL_Palette>,
    colors: Vec<SDL_Color>,
}

struct LocalFormatOwner {
    format: Box<SDL_PixelFormat>,
}

unsafe impl Send for LocalPaletteOwner {}
unsafe impl Send for LocalFormatOwner {}

#[derive(Clone, Copy)]
struct PixelFormatSpec {
    bits_per_pixel: Uint8,
    bytes_per_pixel: Uint8,
    rmask: Uint32,
    gmask: Uint32,
    bmask: Uint32,
    amask: Uint32,
    palette_colors: libc::c_int,
    name: &'static [u8],
}

fn local_palette_registry() -> &'static Mutex<HashMap<usize, LocalPaletteOwner>> {
    static REGISTRY: OnceLock<Mutex<HashMap<usize, LocalPaletteOwner>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn local_format_registry() -> &'static Mutex<HashMap<usize, LocalFormatOwner>> {
    static REGISTRY: OnceLock<Mutex<HashMap<usize, LocalFormatOwner>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn is_local_palette(palette: *mut SDL_Palette) -> bool {
    if palette.is_null() {
        return false;
    }
    local_palette_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .contains_key(&(palette as usize))
}

fn is_local_format(format: *mut SDL_PixelFormat) -> bool {
    if format.is_null() {
        return false;
    }
    local_format_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .contains_key(&(format as usize))
}

fn pixel_format_spec(format: Uint32) -> Option<PixelFormatSpec> {
    Some(match format {
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_INDEX1LSB => PixelFormatSpec {
            bits_per_pixel: 1,
            bytes_per_pixel: 1,
            rmask: 0,
            gmask: 0,
            bmask: 0,
            amask: 0,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_INDEX1LSB\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_INDEX1MSB => PixelFormatSpec {
            bits_per_pixel: 1,
            bytes_per_pixel: 1,
            rmask: 0,
            gmask: 0,
            bmask: 0,
            amask: 0,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_INDEX1MSB\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_INDEX2LSB => PixelFormatSpec {
            bits_per_pixel: 2,
            bytes_per_pixel: 1,
            rmask: 0,
            gmask: 0,
            bmask: 0,
            amask: 0,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_INDEX2LSB\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_INDEX2MSB => PixelFormatSpec {
            bits_per_pixel: 2,
            bytes_per_pixel: 1,
            rmask: 0,
            gmask: 0,
            bmask: 0,
            amask: 0,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_INDEX2MSB\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_INDEX4LSB => PixelFormatSpec {
            bits_per_pixel: 4,
            bytes_per_pixel: 1,
            rmask: 0,
            gmask: 0,
            bmask: 0,
            amask: 0,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_INDEX4LSB\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_INDEX4MSB => PixelFormatSpec {
            bits_per_pixel: 4,
            bytes_per_pixel: 1,
            rmask: 0,
            gmask: 0,
            bmask: 0,
            amask: 0,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_INDEX4MSB\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_INDEX8 => PixelFormatSpec {
            bits_per_pixel: 8,
            bytes_per_pixel: 1,
            rmask: 0,
            gmask: 0,
            bmask: 0,
            amask: 0,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_INDEX8\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB332 => PixelFormatSpec {
            bits_per_pixel: 8,
            bytes_per_pixel: 1,
            rmask: 0xE0,
            gmask: 0x1C,
            bmask: 0x03,
            amask: 0,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_RGB332\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB444 => PixelFormatSpec {
            bits_per_pixel: 12,
            bytes_per_pixel: 2,
            rmask: 0x0F00,
            gmask: 0x00F0,
            bmask: 0x000F,
            amask: 0,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_RGB444\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGR444 => PixelFormatSpec {
            bits_per_pixel: 12,
            bytes_per_pixel: 2,
            rmask: 0x000F,
            gmask: 0x00F0,
            bmask: 0x0F00,
            amask: 0,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_BGR444\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB555 => PixelFormatSpec {
            bits_per_pixel: 15,
            bytes_per_pixel: 2,
            rmask: 0x7C00,
            gmask: 0x03E0,
            bmask: 0x001F,
            amask: 0,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_RGB555\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGR555 => PixelFormatSpec {
            bits_per_pixel: 15,
            bytes_per_pixel: 2,
            rmask: 0x001F,
            gmask: 0x03E0,
            bmask: 0x7C00,
            amask: 0,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_BGR555\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB4444 => PixelFormatSpec {
            bits_per_pixel: 16,
            bytes_per_pixel: 2,
            rmask: 0x0F00,
            gmask: 0x00F0,
            bmask: 0x000F,
            amask: 0xF000,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_ARGB4444\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBA4444 => PixelFormatSpec {
            bits_per_pixel: 16,
            bytes_per_pixel: 2,
            rmask: 0xF000,
            gmask: 0x0F00,
            bmask: 0x00F0,
            amask: 0x000F,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_RGBA4444\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_ABGR4444 => PixelFormatSpec {
            bits_per_pixel: 16,
            bytes_per_pixel: 2,
            rmask: 0x000F,
            gmask: 0x00F0,
            bmask: 0x0F00,
            amask: 0xF000,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_ABGR4444\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGRA4444 => PixelFormatSpec {
            bits_per_pixel: 16,
            bytes_per_pixel: 2,
            rmask: 0x00F0,
            gmask: 0x0F00,
            bmask: 0xF000,
            amask: 0x000F,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_BGRA4444\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB1555 => PixelFormatSpec {
            bits_per_pixel: 16,
            bytes_per_pixel: 2,
            rmask: 0x7C00,
            gmask: 0x03E0,
            bmask: 0x001F,
            amask: 0x8000,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_ARGB1555\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBA5551 => PixelFormatSpec {
            bits_per_pixel: 16,
            bytes_per_pixel: 2,
            rmask: 0xF800,
            gmask: 0x07C0,
            bmask: 0x003E,
            amask: 0x0001,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_RGBA5551\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_ABGR1555 => PixelFormatSpec {
            bits_per_pixel: 16,
            bytes_per_pixel: 2,
            rmask: 0x001F,
            gmask: 0x03E0,
            bmask: 0x7C00,
            amask: 0x8000,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_ABGR1555\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGRA5551 => PixelFormatSpec {
            bits_per_pixel: 16,
            bytes_per_pixel: 2,
            rmask: 0x003E,
            gmask: 0x07C0,
            bmask: 0xF800,
            amask: 0x0001,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_BGRA5551\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB565 => PixelFormatSpec {
            bits_per_pixel: 16,
            bytes_per_pixel: 2,
            rmask: 0xF800,
            gmask: 0x07E0,
            bmask: 0x001F,
            amask: 0,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_RGB565\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGR565 => PixelFormatSpec {
            bits_per_pixel: 16,
            bytes_per_pixel: 2,
            rmask: 0x001F,
            gmask: 0x07E0,
            bmask: 0xF800,
            amask: 0,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_BGR565\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB24 => PixelFormatSpec {
            bits_per_pixel: 24,
            bytes_per_pixel: 3,
            rmask: 0x000000FF,
            gmask: 0x0000FF00,
            bmask: 0x00FF0000,
            amask: 0,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_RGB24\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGR24 => PixelFormatSpec {
            bits_per_pixel: 24,
            bytes_per_pixel: 3,
            rmask: 0x00FF0000,
            gmask: 0x0000FF00,
            bmask: 0x000000FF,
            amask: 0,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_BGR24\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB888 => PixelFormatSpec {
            bits_per_pixel: 24,
            bytes_per_pixel: 4,
            rmask: 0x00FF0000,
            gmask: 0x0000FF00,
            bmask: 0x000000FF,
            amask: 0,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_RGB888\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBX8888 => PixelFormatSpec {
            bits_per_pixel: 32,
            bytes_per_pixel: 4,
            rmask: 0xFF000000,
            gmask: 0x00FF0000,
            bmask: 0x0000FF00,
            amask: 0,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_RGBX8888\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGR888 => PixelFormatSpec {
            bits_per_pixel: 24,
            bytes_per_pixel: 4,
            rmask: 0x000000FF,
            gmask: 0x0000FF00,
            bmask: 0x00FF0000,
            amask: 0,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_BGR888\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGRX8888 => PixelFormatSpec {
            bits_per_pixel: 32,
            bytes_per_pixel: 4,
            rmask: 0x0000FF00,
            gmask: 0x00FF0000,
            bmask: 0xFF000000,
            amask: 0,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_BGRX8888\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888 => PixelFormatSpec {
            bits_per_pixel: 32,
            bytes_per_pixel: 4,
            rmask: 0x00FF0000,
            gmask: 0x0000FF00,
            bmask: 0x000000FF,
            amask: 0xFF000000,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_ARGB8888\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBA8888 => PixelFormatSpec {
            bits_per_pixel: 32,
            bytes_per_pixel: 4,
            rmask: 0xFF000000,
            gmask: 0x00FF0000,
            bmask: 0x0000FF00,
            amask: 0x000000FF,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_RGBA8888\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_ABGR8888 => PixelFormatSpec {
            bits_per_pixel: 32,
            bytes_per_pixel: 4,
            rmask: 0x000000FF,
            gmask: 0x0000FF00,
            bmask: 0x00FF0000,
            amask: 0xFF000000,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_ABGR8888\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGRA8888 => PixelFormatSpec {
            bits_per_pixel: 32,
            bytes_per_pixel: 4,
            rmask: 0x0000FF00,
            gmask: 0x00FF0000,
            bmask: 0xFF000000,
            amask: 0x000000FF,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_BGRA8888\0",
        },
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB2101010 => PixelFormatSpec {
            bits_per_pixel: 32,
            bytes_per_pixel: 4,
            rmask: 0x3FF00000,
            gmask: 0x000FFC00,
            bmask: 0x000003FF,
            amask: 0xC0000000,
            palette_colors: 0,
            name: b"SDL_PIXELFORMAT_ARGB2101010\0",
        },
        _ => return None,
    })
}

fn mask_shift(mask: Uint32) -> Uint8 {
    if mask == 0 {
        0
    } else {
        mask.trailing_zeros() as Uint8
    }
}

fn mask_loss(mask: Uint32) -> Uint8 {
    if mask == 0 {
        8
    } else {
        8u8.saturating_sub(mask.count_ones() as Uint8)
    }
}

fn default_palette_colors(ncolors: usize) -> Vec<SDL_Color> {
    vec![
        SDL_Color {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        };
        ncolors
    ]
}

fn local_alloc_palette_impl(ncolors: libc::c_int) -> *mut SDL_Palette {
    if ncolors < 1 {
        let _ = invalid_param_error("ncolors");
        return std::ptr::null_mut();
    }
    let mut colors = default_palette_colors(ncolors as usize);
    let mut palette = Box::new(SDL_Palette {
        ncolors,
        colors: if colors.is_empty() {
            std::ptr::null_mut()
        } else {
            colors.as_mut_ptr()
        },
        version: 1,
        refcount: 1,
    });
    let ptr = palette.as_mut() as *mut SDL_Palette;
    local_palette_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(ptr as usize, LocalPaletteOwner { palette, colors });
    ptr
}

fn local_free_palette_impl(palette: *mut SDL_Palette) {
    if palette.is_null() {
        return;
    }
    let mut palettes = local_palette_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(owner) = palettes.get_mut(&(palette as usize)) else {
        return;
    };
    owner.palette.refcount -= 1;
    if owner.palette.refcount <= 0 {
        palettes.remove(&(palette as usize));
    }
}

fn local_retain_palette_impl(palette: *mut SDL_Palette) {
    if palette.is_null() {
        return;
    }
    if let Some(owner) = local_palette_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get_mut(&(palette as usize))
    {
        owner.palette.refcount += 1;
    }
}

fn local_alloc_format_impl(pixel_format: Uint32) -> *mut SDL_PixelFormat {
    let (bits_per_pixel, bytes_per_pixel, rmask, gmask, bmask, amask) = if pixel_format == 0 {
        (0, 0, 0, 0, 0, 0)
    } else if let Some(spec) = pixel_format_spec(pixel_format) {
        (
            spec.bits_per_pixel,
            spec.bytes_per_pixel,
            spec.rmask,
            spec.gmask,
            spec.bmask,
            spec.amask,
        )
    } else {
        let _ = set_error_message("Unknown pixel format");
        return std::ptr::null_mut();
    };

    let mut format = Box::new(SDL_PixelFormat {
        format: pixel_format,
        palette: std::ptr::null_mut(),
        BitsPerPixel: bits_per_pixel,
        BytesPerPixel: bytes_per_pixel,
        padding: [0; 2],
        Rmask: rmask,
        Gmask: gmask,
        Bmask: bmask,
        Amask: amask,
        Rloss: mask_loss(rmask),
        Gloss: mask_loss(gmask),
        Bloss: mask_loss(bmask),
        Aloss: mask_loss(amask),
        Rshift: mask_shift(rmask),
        Gshift: mask_shift(gmask),
        Bshift: mask_shift(bmask),
        Ashift: mask_shift(amask),
        refcount: 1,
        next: std::ptr::null_mut(),
    });
    let ptr = format.as_mut() as *mut SDL_PixelFormat;
    local_format_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(ptr as usize, LocalFormatOwner { format });
    ptr
}

fn local_free_format_impl(format: *mut SDL_PixelFormat) {
    if format.is_null() {
        return;
    }
    if let Some(owner) = local_format_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(&(format as usize))
    {
        if !owner.format.palette.is_null() && is_local_palette(owner.format.palette) {
            local_free_palette_impl(owner.format.palette);
        }
    }
}

fn local_set_pixel_format_palette_impl(
    format: *mut SDL_PixelFormat,
    palette: *mut SDL_Palette,
) -> libc::c_int {
    if format.is_null() {
        return invalid_param_error("format");
    }
    let max_colors = unsafe {
        if (*format).BitsPerPixel >= 31 {
            libc::c_int::MAX
        } else {
            1i32 << (*format).BitsPerPixel as u32
        }
    };
    if !palette.is_null() && unsafe { (*palette).ncolors > max_colors } {
        return set_error_message(
            "SDL_SetPixelFormatPalette() passed a palette that doesn't match the format",
        );
    }
    if unsafe { (*format).palette == palette } {
        return 0;
    }
    if unsafe { !(*format).palette.is_null() && is_local_palette((*format).palette) } {
        unsafe {
            local_free_palette_impl((*format).palette);
        }
    }
    if !palette.is_null() && is_local_palette(palette) {
        local_retain_palette_impl(palette);
    }
    unsafe {
        (*format).palette = palette;
    }
    0
}

fn local_set_palette_colors_impl(
    palette: *mut SDL_Palette,
    colors: *const SDL_Color,
    firstcolor: libc::c_int,
    ncolors: libc::c_int,
) -> libc::c_int {
    if palette.is_null() {
        return invalid_param_error("palette");
    }
    if colors.is_null() {
        return invalid_param_error("colors");
    }
    if firstcolor < 0 {
        return invalid_param_error("firstcolor");
    }
    if ncolors < 0 {
        return invalid_param_error("ncolors");
    }

    let mut palettes = local_palette_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(owner) = palettes.get_mut(&(palette as usize)) else {
        return set_error_message("Unknown palette");
    };
    let end = firstcolor.saturating_add(ncolors);
    if end > owner.palette.ncolors {
        return invalid_param_error("ncolors");
    }
    for index in 0..ncolors as usize {
        owner.colors[firstcolor as usize + index] = unsafe { *colors.add(index) };
    }
    owner.palette.colors = if owner.colors.is_empty() {
        std::ptr::null_mut()
    } else {
        owner.colors.as_mut_ptr()
    };
    owner.palette.version = owner.palette.version.wrapping_add(1);
    0
}

fn local_pixel_format_enum_to_masks_impl(
    format: Uint32,
    bpp: *mut libc::c_int,
    rmask: *mut Uint32,
    gmask: *mut Uint32,
    bmask: *mut Uint32,
    amask: *mut Uint32,
) -> SDL_bool {
    let Some(spec) = pixel_format_spec(format) else {
        let _ = set_error_message("Unknown pixel format");
        return 0;
    };

    unsafe {
        if !bpp.is_null() {
            *bpp = spec.bits_per_pixel as libc::c_int;
        }
        if !rmask.is_null() {
            *rmask = spec.rmask;
        }
        if !gmask.is_null() {
            *gmask = spec.gmask;
        }
        if !bmask.is_null() {
            *bmask = spec.bmask;
        }
        if !amask.is_null() {
            *amask = spec.amask;
        }
    }
    1
}

fn local_masks_to_pixel_format_enum_impl(
    bpp: libc::c_int,
    rmask: Uint32,
    gmask: Uint32,
    bmask: Uint32,
    amask: Uint32,
) -> Uint32 {
    match bpp {
        1 => SDL_PixelFormatEnum_SDL_PIXELFORMAT_INDEX1MSB,
        2 => SDL_PixelFormatEnum_SDL_PIXELFORMAT_INDEX2MSB,
        4 => SDL_PixelFormatEnum_SDL_PIXELFORMAT_INDEX4MSB,
        8 => {
            if rmask == 0 {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_INDEX8
            } else if rmask == 0xE0 && gmask == 0x1C && bmask == 0x03 && amask == 0 {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB332
            } else {
                0
            }
        }
        12 => {
            if rmask == 0 || (rmask == 0x0F00 && gmask == 0x00F0 && bmask == 0x000F && amask == 0) {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB444
            } else if rmask == 0x000F && gmask == 0x00F0 && bmask == 0x0F00 && amask == 0 {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGR444
            } else {
                0
            }
        }
        15 => {
            if rmask == 0 || (rmask == 0x7C00 && gmask == 0x03E0 && bmask == 0x001F && amask == 0) {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB555
            } else if rmask == 0x001F && gmask == 0x03E0 && bmask == 0x7C00 && amask == 0 {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGR555
            } else {
                0
            }
        }
        16 => {
            if rmask == 0 {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB565
            } else if rmask == 0x7C00 && gmask == 0x03E0 && bmask == 0x001F && amask == 0 {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB555
            } else if rmask == 0x001F && gmask == 0x03E0 && bmask == 0x7C00 && amask == 0 {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGR555
            } else if rmask == 0x0F00 && gmask == 0x00F0 && bmask == 0x000F && amask == 0xF000 {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB4444
            } else if rmask == 0xF000 && gmask == 0x0F00 && bmask == 0x00F0 && amask == 0x000F {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBA4444
            } else if rmask == 0x000F && gmask == 0x00F0 && bmask == 0x0F00 && amask == 0xF000 {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_ABGR4444
            } else if rmask == 0x00F0 && gmask == 0x0F00 && bmask == 0xF000 && amask == 0x000F {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGRA4444
            } else if rmask == 0x7C00 && gmask == 0x03E0 && bmask == 0x001F && amask == 0x8000 {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB1555
            } else if rmask == 0xF800 && gmask == 0x07C0 && bmask == 0x003E && amask == 0x0001 {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBA5551
            } else if rmask == 0x001F && gmask == 0x03E0 && bmask == 0x7C00 && amask == 0x8000 {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_ABGR1555
            } else if rmask == 0x003E && gmask == 0x07C0 && bmask == 0xF800 && amask == 0x0001 {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGRA5551
            } else if rmask == 0xF800 && gmask == 0x07E0 && bmask == 0x001F && amask == 0 {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB565
            } else if rmask == 0x001F && gmask == 0x07E0 && bmask == 0xF800 && amask == 0 {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGR565
            } else if rmask == 0x003F && gmask == 0x07C0 && bmask == 0xF800 && amask == 0 {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB565
            } else {
                0
            }
        }
        24 => match rmask {
            0 | 0x000000FF => SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB24,
            0x00FF0000 => SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGR24,
            _ => 0,
        },
        32 => {
            if rmask == 0
                || (rmask == 0x00FF0000 && gmask == 0x0000FF00 && bmask == 0x000000FF && amask == 0)
            {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB888
            } else if rmask == 0xFF000000
                && gmask == 0x00FF0000
                && bmask == 0x0000FF00
                && amask == 0
            {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBX8888
            } else if rmask == 0x000000FF
                && gmask == 0x0000FF00
                && bmask == 0x00FF0000
                && amask == 0
            {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGR888
            } else if rmask == 0x0000FF00
                && gmask == 0x00FF0000
                && bmask == 0xFF000000
                && amask == 0
            {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGRX8888
            } else if rmask == 0x00FF0000
                && gmask == 0x0000FF00
                && bmask == 0x000000FF
                && amask == 0xFF000000
            {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888
            } else if rmask == 0xFF000000
                && gmask == 0x00FF0000
                && bmask == 0x0000FF00
                && amask == 0x000000FF
            {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBA8888
            } else if rmask == 0x000000FF
                && gmask == 0x0000FF00
                && bmask == 0x00FF0000
                && amask == 0xFF000000
            {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_ABGR8888
            } else if rmask == 0x0000FF00
                && gmask == 0x00FF0000
                && bmask == 0xFF000000
                && amask == 0x000000FF
            {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGRA8888
            } else if rmask == 0x3FF00000
                && gmask == 0x000FFC00
                && bmask == 0x000003FF
                && amask == 0xC0000000
            {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB2101010
            } else {
                0
            }
        }
        _ => 0,
    }
}

fn pixel_format_name_bytes(format: Uint32) -> &'static [u8] {
    if let Some(spec) = pixel_format_spec(format) {
        return spec.name;
    }
    match format {
        0 => b"SDL_PIXELFORMAT_UNKNOWN\0",
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_YV12 => b"SDL_PIXELFORMAT_YV12\0",
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_IYUV => b"SDL_PIXELFORMAT_IYUV\0",
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_YUY2 => b"SDL_PIXELFORMAT_YUY2\0",
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_UYVY => b"SDL_PIXELFORMAT_UYVY\0",
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_YVYU => b"SDL_PIXELFORMAT_YVYU\0",
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV12 => b"SDL_PIXELFORMAT_NV12\0",
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV21 => b"SDL_PIXELFORMAT_NV21\0",
        _ => b"SDL_PIXELFORMAT_UNKNOWN\0",
    }
}

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
    if pixel_format_spec(pixel_format).is_some() || !crate::video::real_sdl_is_available() {
        return local_alloc_format_impl(pixel_format);
    }
    clear_real_error();
    let format = (real_sdl().alloc_format)(pixel_format);
    if format.is_null() {
        let _ = sync_error_from_real("Couldn't allocate pixel format");
    }
    format
}

#[no_mangle]
pub unsafe extern "C" fn SDL_FreeFormat(format: *mut SDL_PixelFormat) {
    if is_local_format(format) || !crate::video::real_sdl_is_available() {
        local_free_format_impl(format);
        return;
    }
    (real_sdl().free_format)(format);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_AllocPalette(ncolors: libc::c_int) -> *mut SDL_Palette {
    if !crate::video::real_sdl_is_available() {
        return local_alloc_palette_impl(ncolors);
    }
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
    if is_local_format(format)
        || is_local_palette(palette)
        || !crate::video::real_sdl_is_available()
    {
        return local_set_pixel_format_palette_impl(format, palette);
    }
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
    if is_local_palette(palette) || !crate::video::real_sdl_is_available() {
        return local_set_palette_colors_impl(palette, colors, firstcolor, ncolors);
    }
    clear_real_error();
    let result = (real_sdl().set_palette_colors)(palette, colors, firstcolor, ncolors);
    if result < 0 {
        let _ = sync_error_from_real("Couldn't set palette colors");
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn SDL_FreePalette(palette: *mut SDL_Palette) {
    if is_local_palette(palette) || !crate::video::real_sdl_is_available() {
        local_free_palette_impl(palette);
        return;
    }
    (real_sdl().free_palette)(palette);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetPixelFormatName(format: Uint32) -> *const libc::c_char {
    let local_name = pixel_format_name_bytes(format);
    if local_name != b"SDL_PIXELFORMAT_UNKNOWN\0"
        || format == 0
        || !crate::video::real_sdl_is_available()
    {
        return local_name.as_ptr().cast();
    }
    if crate::video::real_sdl_is_available() {
        clear_real_error();
        return (real_sdl().get_pixel_format_name)(format);
    }
    b"SDL_PIXELFORMAT_UNKNOWN\0".as_ptr().cast()
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
    if pixel_format_spec(format).is_some() || !crate::video::real_sdl_is_available() {
        return local_pixel_format_enum_to_masks_impl(format, bpp, Rmask, Gmask, Bmask, Amask);
    }
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
    let local = local_masks_to_pixel_format_enum_impl(bpp, Rmask, Gmask, Bmask, Amask);
    if local != 0 || !crate::video::real_sdl_is_available() {
        return local;
    }
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
