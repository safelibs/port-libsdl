#![allow(clippy::all)]

#[path = "common/testutils.rs"]
mod testutils;

use std::ptr;

use safe_sdl::abi::generated_types::{
    SDL_BlendMode_SDL_BLENDMODE_BLEND, SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBA32, SDL_Rect, Uint32,
};
use safe_sdl::video::blit::{SDL_UpperBlit, SDL_UpperBlitScaled};
use safe_sdl::video::pixels::{SDL_GetRGBA, SDL_MapRGBA};
use safe_sdl::video::surface::{
    SDL_CreateRGBSurface, SDL_CreateRGBSurfaceWithFormat, SDL_FillRect, SDL_FreeSurface,
    SDL_GetSurfaceBlendMode, SDL_LockSurface, SDL_UnlockSurface,
};

unsafe fn read_rgba(
    surface: *mut safe_sdl::abi::generated_types::SDL_Surface,
    x: usize,
    y: usize,
) -> (u8, u8, u8, u8) {
    assert_eq!(
        SDL_LockSurface(surface),
        0,
        "{}",
        testutils::current_error()
    );
    let row = (*surface)
        .pixels
        .cast::<u8>()
        .add(y * (*surface).pitch as usize);
    let pixel = row.add(x * (*(*surface).format).BytesPerPixel as usize);
    let raw = match (*(*surface).format).BytesPerPixel {
        1 => pixel.read() as Uint32,
        2 => pixel.cast::<u16>().read_unaligned() as Uint32,
        3 => Uint32::from_le_bytes([pixel.read(), pixel.add(1).read(), pixel.add(2).read(), 0]),
        4 => pixel.cast::<Uint32>().read_unaligned(),
        other => panic!("unsupported bytes per pixel {other}"),
    };
    SDL_UnlockSurface(surface);

    let mut r = 0;
    let mut g = 0;
    let mut b = 0;
    let mut a = 0;
    SDL_GetRGBA(raw, (*surface).format, &mut r, &mut g, &mut b, &mut a);
    (r, g, b, a)
}

#[test]
fn alpha_surface_blit_exposes_blit_map_and_preserves_result_alpha() {
    let _serial = testutils::serial_lock();

    unsafe {
        let base =
            SDL_CreateRGBSurfaceWithFormat(0, 4, 4, 32, SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBA32);
        let overlay =
            SDL_CreateRGBSurfaceWithFormat(0, 4, 4, 32, SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBA32);
        assert!(!base.is_null(), "{}", testutils::current_error());
        assert!(!overlay.is_null(), "{}", testutils::current_error());
        assert!(!(*base).map.is_null());
        assert!(!(*overlay).map.is_null());

        let mut blend = 0;
        assert_eq!(SDL_GetSurfaceBlendMode(overlay, &mut blend), 0);
        assert_eq!(blend, SDL_BlendMode_SDL_BLENDMODE_BLEND);

        let red_half = SDL_MapRGBA((*overlay).format, 255, 0, 0, 128);
        assert_eq!(SDL_FillRect(overlay, ptr::null(), red_half), 0);

        let mut dst = SDL_Rect {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        };
        assert_eq!(
            SDL_UpperBlit(overlay, ptr::null(), base, &mut dst),
            0,
            "{}",
            testutils::current_error()
        );
        assert_eq!(read_rgba(base, 1, 1).3, 128);

        SDL_FreeSurface(overlay);
        SDL_FreeSurface(base);
    }
}

#[test]
fn default_rgb_surface_scaled_blit_keeps_mask_derived_destination_format() {
    let _serial = testutils::serial_lock();

    unsafe {
        let src = SDL_CreateRGBSurface(0, 4, 4, 32, 0, 0, 0, 0);
        assert!(!src.is_null(), "{}", testutils::current_error());
        assert!(!(*src).map.is_null());

        let src_format = *(*src).format;
        let dst = SDL_CreateRGBSurface(
            0,
            8,
            8,
            src_format.BitsPerPixel as i32,
            src_format.Rmask,
            src_format.Gmask,
            src_format.Bmask,
            src_format.Amask,
        );
        assert!(!dst.is_null(), "{}", testutils::current_error());
        assert!(!(*dst).map.is_null());
        assert_eq!((*(*src).format).format, (*(*dst).format).format);

        let red = SDL_MapRGBA((*src).format, 255, 0, 0, 255);
        assert_eq!(SDL_FillRect(src, ptr::null(), red), 0);

        let mut scaled = SDL_Rect {
            x: 0,
            y: 0,
            w: 8,
            h: 8,
        };
        assert_eq!(
            SDL_UpperBlitScaled(src, ptr::null(), dst, &mut scaled),
            0,
            "{}",
            testutils::current_error()
        );
        assert_eq!(read_rgba(dst, 7, 7), (255, 0, 0, 255));

        SDL_FreeSurface(dst);
        SDL_FreeSurface(src);
    }
}
