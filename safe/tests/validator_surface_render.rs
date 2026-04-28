#![allow(clippy::all)]

#[path = "common/testutils.rs"]
mod testutils;

use std::ptr;

use safe_sdl::abi::generated_types::{
    SDL_BlendMode_SDL_BLENDMODE_BLEND, SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGR888,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_IYUV, SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV12,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB888, SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBA32,
    SDL_Rect, SDL_TextureAccess_SDL_TEXTUREACCESS_STREAMING, Uint32,
};
use safe_sdl::render::core::{
    SDL_CreateTexture, SDL_DestroyRenderer, SDL_DestroyTexture, SDL_QueryTexture, SDL_RenderCopy,
    SDL_UpdateNVTexture, SDL_UpdateYUVTexture,
};
use safe_sdl::render::software::SDL_CreateSoftwareRenderer;
use safe_sdl::video::blit::{
    SDL_GetYUVConversionMode, SDL_GetYUVConversionModeForResolution, SDL_SetYUVConversionMode,
    SDL_UpperBlit, SDL_UpperBlitScaled,
};
use safe_sdl::video::pixels::{
    SDL_AllocFormat, SDL_FreeFormat, SDL_GetRGBA, SDL_MapRGBA, SDL_PixelFormatEnumToMasks,
};
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
fn local_renderer_updates_planar_and_nv_yuv_textures_without_host_sdl() {
    let _serial = testutils::serial_lock();

    unsafe {
        let target =
            SDL_CreateRGBSurfaceWithFormat(0, 8, 4, 32, SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBA32);
        assert!(!target.is_null(), "{}", testutils::current_error());
        let renderer = SDL_CreateSoftwareRenderer(target);
        assert!(!renderer.is_null(), "{}", testutils::current_error());

        let iyuv = SDL_CreateTexture(
            renderer,
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_IYUV,
            SDL_TextureAccess_SDL_TEXTUREACCESS_STREAMING as i32,
            4,
            4,
        );
        let nv12 = SDL_CreateTexture(
            renderer,
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV12,
            SDL_TextureAccess_SDL_TEXTUREACCESS_STREAMING as i32,
            4,
            4,
        );
        assert!(!iyuv.is_null(), "{}", testutils::current_error());
        assert!(!nv12.is_null(), "{}", testutils::current_error());

        let (mut format, mut access, mut w, mut h) = (0, 0, 0, 0);
        assert_eq!(
            SDL_QueryTexture(iyuv, &mut format, &mut access, &mut w, &mut h),
            0
        );
        assert_eq!(format, SDL_PixelFormatEnum_SDL_PIXELFORMAT_IYUV);
        assert_eq!(access, SDL_TextureAccess_SDL_TEXTUREACCESS_STREAMING as i32);
        assert_eq!((w, h), (4, 4));

        let y = [235u8; 16];
        let u = [128u8; 4];
        let v = [128u8; 4];
        assert_eq!(
            SDL_UpdateYUVTexture(
                iyuv,
                ptr::null(),
                y.as_ptr(),
                4,
                u.as_ptr(),
                2,
                v.as_ptr(),
                2
            ),
            0,
            "{}",
            testutils::current_error()
        );
        let uv = [128u8; 8];
        assert_eq!(
            SDL_UpdateNVTexture(nv12, ptr::null(), y.as_ptr(), 4, uv.as_ptr(), 4),
            0,
            "{}",
            testutils::current_error()
        );

        assert_eq!(SDL_RenderCopy(renderer, iyuv, ptr::null(), ptr::null()), 0);
        assert!(read_rgba(target, 1, 1).0 > 200);

        let dest = SDL_Rect {
            x: 4,
            y: 0,
            w: 4,
            h: 4,
        };
        assert_eq!(SDL_RenderCopy(renderer, nv12, ptr::null(), &dest), 0);
        assert!(read_rgba(target, 5, 1).0 > 200);

        SDL_DestroyTexture(nv12);
        SDL_DestroyTexture(iyuv);
        SDL_DestroyRenderer(renderer);
        SDL_FreeSurface(target);
    }
}

#[test]
fn yuv_conversion_mode_is_local_state_and_resolves_automatic_by_height() {
    let _serial = testutils::serial_lock();

    unsafe {
        SDL_SetYUVConversionMode(
            safe_sdl::abi::generated_types::SDL_YUV_CONVERSION_MODE_SDL_YUV_CONVERSION_BT709,
        );
        assert_eq!(
            SDL_GetYUVConversionMode(),
            safe_sdl::abi::generated_types::SDL_YUV_CONVERSION_MODE_SDL_YUV_CONVERSION_BT709
        );
        SDL_SetYUVConversionMode(
            safe_sdl::abi::generated_types::SDL_YUV_CONVERSION_MODE_SDL_YUV_CONVERSION_AUTOMATIC,
        );
        assert_eq!(
            SDL_GetYUVConversionModeForResolution(640, 480),
            safe_sdl::abi::generated_types::SDL_YUV_CONVERSION_MODE_SDL_YUV_CONVERSION_BT601
        );
        assert_eq!(
            SDL_GetYUVConversionModeForResolution(1280, 720),
            safe_sdl::abi::generated_types::SDL_YUV_CONVERSION_MODE_SDL_YUV_CONVERSION_BT709
        );
        SDL_SetYUVConversionMode(
            safe_sdl::abi::generated_types::SDL_YUV_CONVERSION_MODE_SDL_YUV_CONVERSION_BT601,
        );
    }
}

#[test]
fn rgb888_bgr888_metadata_matches_sdl2_public_contract() {
    let _serial = testutils::serial_lock();

    unsafe {
        let cases = [
            (
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB888,
                0x00FF0000,
                0x0000FF00,
                0x000000FF,
            ),
            (
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGR888,
                0x000000FF,
                0x0000FF00,
                0x00FF0000,
            ),
        ];
        for (format, expected_r, expected_g, expected_b) in cases {
            let allocated = SDL_AllocFormat(format);
            assert!(!allocated.is_null(), "{}", testutils::current_error());
            assert_eq!((*allocated).BitsPerPixel, 24);
            assert_eq!((*allocated).BytesPerPixel, 4);
            assert_eq!((*allocated).Rmask, expected_r);
            assert_eq!((*allocated).Gmask, expected_g);
            assert_eq!((*allocated).Bmask, expected_b);
            assert_eq!((*allocated).Amask, 0);
            SDL_FreeFormat(allocated);

            let mut bpp = 0;
            let mut rmask = 0;
            let mut gmask = 0;
            let mut bmask = 0;
            let mut amask = 0;
            assert_ne!(
                SDL_PixelFormatEnumToMasks(
                    format, &mut bpp, &mut rmask, &mut gmask, &mut bmask, &mut amask,
                ),
                0,
                "{}",
                testutils::current_error()
            );
            assert_eq!(bpp, 24);
            assert_eq!(
                (rmask, gmask, bmask, amask),
                (expected_r, expected_g, expected_b, 0)
            );
        }
    }
}

#[test]
fn default_rgb_surface_scaled_blit_keeps_32bit_mask_derived_destination_format() {
    let _serial = testutils::serial_lock();

    unsafe {
        let src = SDL_CreateRGBSurface(0, 4, 4, 32, 0, 0, 0, 0);
        assert!(!src.is_null(), "{}", testutils::current_error());
        assert!(!(*src).map.is_null());

        let src_format = *(*src).format;
        assert_eq!(
            src_format.format,
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB888
        );
        assert_eq!(src_format.BitsPerPixel, 32);
        assert_eq!(src_format.BytesPerPixel, 4);
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
        assert_eq!((*(*dst).format).BitsPerPixel, 32);
        assert_eq!((*(*dst).format).BytesPerPixel, 4);

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
