#![allow(non_upper_case_globals)]
#![cfg(feature = "host-video-tests")]
#![allow(clippy::all)]

#[path = "common/testutils.rs"]
mod testutils;
#[path = "common/testyuv_cvt.rs"]
mod testyuv_cvt;

use std::mem::MaybeUninit;
use std::ptr;

use safe_sdl::abi::generated_types::{
    SDL_BlendMode_SDL_BLENDMODE_NONE, SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_IYUV, SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV12,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV21, SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB24,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_UYVY, SDL_PixelFormatEnum_SDL_PIXELFORMAT_YUY2,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_YV12, SDL_PixelFormatEnum_SDL_PIXELFORMAT_YVYU, SDL_Rect,
    SDL_RendererFlags_SDL_RENDERER_SOFTWARE, SDL_RendererInfo, SDL_ScaleMode_SDL_ScaleModeLinear,
    SDL_Surface, SDL_TextureAccess_SDL_TEXTUREACCESS_STREAMING, SDL_Vertex, SDL_bool_SDL_TRUE,
    Uint32, Uint8, SDL_YUV_CONVERSION_MODE_SDL_YUV_CONVERSION_BT601,
};
use safe_sdl::render::core::{
    SDL_CreateTexture, SDL_CreateTextureFromSurface, SDL_DestroyRenderer, SDL_DestroyTexture,
    SDL_GetNumRenderDrivers, SDL_GetRenderDriverInfo, SDL_GetRendererInfo,
    SDL_GetRendererOutputSize, SDL_GetTextureBlendMode, SDL_GetTextureColorMod,
    SDL_GetTextureScaleMode, SDL_GetTextureUserData, SDL_LockTexture, SDL_QueryTexture,
    SDL_RenderClear, SDL_RenderCopy, SDL_RenderFillRect, SDL_RenderGeometry, SDL_RenderGetClipRect,
    SDL_RenderGetIntegerScale, SDL_RenderGetLogicalSize, SDL_RenderGetScale,
    SDL_RenderIsClipEnabled, SDL_RenderSetClipRect, SDL_RenderSetIntegerScale,
    SDL_RenderSetLogicalSize, SDL_RenderSetScale, SDL_SetRenderDrawColor, SDL_SetTextureBlendMode,
    SDL_SetTextureColorMod, SDL_SetTextureScaleMode, SDL_SetTextureUserData, SDL_UnlockTexture,
};
use safe_sdl::render::software::SDL_CreateSoftwareRenderer;
use safe_sdl::video::blit::{
    SDL_ConvertPixels, SDL_GetYUVConversionMode, SDL_SetYUVConversionMode,
};
use safe_sdl::video::pixels::{SDL_GetRGBA, SDL_MapRGBA};
use safe_sdl::video::surface::{
    SDL_CreateRGBSurfaceWithFormat, SDL_FillRect, SDL_FreeSurface, SDL_LockSurface,
    SDL_UnlockSurface,
};

struct SoftwareRendererHarness {
    surface: *mut SDL_Surface,
    renderer: *mut safe_sdl::abi::generated_types::SDL_Renderer,
}

impl SoftwareRendererHarness {
    unsafe fn new(width: i32, height: i32) -> Self {
        let surface = SDL_CreateRGBSurfaceWithFormat(
            0,
            width,
            height,
            32,
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888,
        );
        assert!(!surface.is_null(), "{}", testutils::current_error());
        let renderer = SDL_CreateSoftwareRenderer(surface);
        assert!(!renderer.is_null(), "{}", testutils::current_error());
        Self { surface, renderer }
    }

    unsafe fn read_pixel(&self, x: usize, y: usize) -> (u8, u8, u8, u8) {
        assert_eq!(
            SDL_LockSurface(self.surface),
            0,
            "{}",
            testutils::current_error()
        );
        let row = (*self.surface)
            .pixels
            .cast::<u8>()
            .add(y * (*self.surface).pitch as usize);
        let pixel = row.add(x * 4).cast::<Uint32>().read_unaligned();
        SDL_UnlockSurface(self.surface);

        let (mut r, mut g, mut b, mut a) = (0, 0, 0, 0);
        SDL_GetRGBA(
            pixel,
            (*self.surface).format,
            &mut r,
            &mut g,
            &mut b,
            &mut a,
        );
        (r, g, b, a)
    }
}

impl Drop for SoftwareRendererHarness {
    fn drop(&mut self) {
        unsafe {
            if !self.renderer.is_null() {
                SDL_DestroyRenderer(self.renderer);
            }
            if !self.surface.is_null() {
                SDL_FreeSurface(self.surface);
            }
        }
    }
}

fn yuv_buffer_len(format: Uint32, width: usize, height: usize) -> usize {
    match format {
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_YV12
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_IYUV
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV12
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV21 => {
            width * height + 2 * width.div_ceil(2) * height.div_ceil(2)
        }
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_YUY2
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_UYVY
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_YVYU => {
            testyuv_cvt::calculate_yuv_pitch(format, width as i32) as usize * height
        }
        _ => 0,
    }
}

#[test]
fn render_driver_inventory_and_software_renderer_info_are_available() {
    let _serial = testutils::serial_lock();

    unsafe {
        let harness = SoftwareRendererHarness::new(48, 32);
        let driver_count = SDL_GetNumRenderDrivers();
        assert!(driver_count >= 1, "expected at least one renderer driver");

        for index in 0..driver_count {
            let mut info = MaybeUninit::<SDL_RendererInfo>::zeroed();
            assert_eq!(
                SDL_GetRenderDriverInfo(index, info.as_mut_ptr()),
                0,
                "{}",
                testutils::current_error()
            );
            assert!(!info.assume_init().name.is_null());
        }

        let mut info = MaybeUninit::<SDL_RendererInfo>::zeroed();
        assert_eq!(
            SDL_GetRendererInfo(harness.renderer, info.as_mut_ptr()),
            0,
            "{}",
            testutils::current_error()
        );
        let info = info.assume_init();
        assert!(!info.name.is_null());
        assert_ne!(info.flags & SDL_RendererFlags_SDL_RENDERER_SOFTWARE, 0);

        let (mut w, mut h) = (0, 0);
        assert_eq!(
            SDL_GetRendererOutputSize(harness.renderer, &mut w, &mut h),
            0,
            "{}",
            testutils::current_error()
        );
        assert_eq!((w, h), (48, 32));
    }
}

#[test]
fn primitives_clip_and_scale_roundtrip_pixels() {
    let _serial = testutils::serial_lock();

    unsafe {
        let harness = SoftwareRendererHarness::new(32, 32);
        assert_eq!(
            SDL_SetRenderDrawColor(harness.renderer, 255, 255, 255, 255),
            0
        );
        assert_eq!(SDL_RenderClear(harness.renderer), 0);

        let clip = SDL_Rect {
            x: 0,
            y: 0,
            w: 8,
            h: 8,
        };
        assert_eq!(SDL_RenderSetClipRect(harness.renderer, &clip), 0);
        assert_ne!(SDL_RenderIsClipEnabled(harness.renderer), 0);
        let mut observed_clip = MaybeUninit::<SDL_Rect>::zeroed();
        SDL_RenderGetClipRect(harness.renderer, observed_clip.as_mut_ptr());
        let observed_clip = observed_clip.assume_init();
        assert_eq!(
            (
                observed_clip.x,
                observed_clip.y,
                observed_clip.w,
                observed_clip.h
            ),
            (0, 0, 8, 8)
        );

        assert_eq!(SDL_SetRenderDrawColor(harness.renderer, 0, 255, 0, 255), 0);
        assert_eq!(SDL_RenderFillRect(harness.renderer, ptr::null()), 0);
        assert_eq!(harness.read_pixel(4, 4), (0, 255, 0, 255));
        assert_eq!(harness.read_pixel(12, 12), (255, 255, 255, 255));

        assert_eq!(SDL_RenderSetClipRect(harness.renderer, ptr::null()), 0);
        assert_eq!(SDL_RenderSetLogicalSize(harness.renderer, 16, 16), 0);
        let (mut logical_w, mut logical_h) = (0, 0);
        SDL_RenderGetLogicalSize(harness.renderer, &mut logical_w, &mut logical_h);
        assert_eq!((logical_w, logical_h), (16, 16));
        assert_eq!(
            SDL_RenderSetIntegerScale(harness.renderer, SDL_bool_SDL_TRUE),
            0
        );
        assert_ne!(SDL_RenderGetIntegerScale(harness.renderer), 0);

        assert_eq!(SDL_RenderSetScale(harness.renderer, 2.0, 2.0), 0);
        let (mut scale_x, mut scale_y) = (0.0f32, 0.0f32);
        SDL_RenderGetScale(harness.renderer, &mut scale_x, &mut scale_y);
        assert_eq!((scale_x, scale_y), (2.0, 2.0));

        assert_eq!(SDL_SetRenderDrawColor(harness.renderer, 0, 0, 255, 255), 0);
        let point_rect = SDL_Rect {
            x: 2,
            y: 3,
            w: 1,
            h: 1,
        };
        assert_eq!(SDL_RenderFillRect(harness.renderer, &point_rect), 0);
        assert_eq!(harness.read_pixel(4, 6), (0, 0, 255, 255));
    }
}

#[test]
fn texture_query_userdata_lock_and_copy_roundtrip() {
    let _serial = testutils::serial_lock();

    unsafe {
        let harness = SoftwareRendererHarness::new(16, 16);
        let texture = SDL_CreateTexture(
            harness.renderer,
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888,
            SDL_TextureAccess_SDL_TEXTUREACCESS_STREAMING as i32,
            4,
            4,
        );
        assert!(!texture.is_null(), "{}", testutils::current_error());

        let mut format = 0;
        let (mut access, mut w, mut h) = (0, 0, 0);
        assert_eq!(
            SDL_QueryTexture(texture, &mut format, &mut access, &mut w, &mut h),
            0,
            "{}",
            testutils::current_error()
        );
        assert_eq!(format, SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888);
        assert_eq!(access, SDL_TextureAccess_SDL_TEXTUREACCESS_STREAMING as i32);
        assert_eq!((w, h), (4, 4));

        assert_eq!(
            SDL_SetTextureScaleMode(texture, SDL_ScaleMode_SDL_ScaleModeLinear),
            0
        );
        let mut scale_mode = 0;
        assert_eq!(SDL_GetTextureScaleMode(texture, &mut scale_mode), 0);
        assert_eq!(scale_mode, SDL_ScaleMode_SDL_ScaleModeLinear);

        let sentinel = 0x44usize as *mut libc::c_void;
        assert_eq!(SDL_SetTextureUserData(texture, sentinel), 0);
        assert_eq!(SDL_GetTextureUserData(texture), sentinel);

        assert_eq!(
            SDL_SetTextureBlendMode(texture, SDL_BlendMode_SDL_BLENDMODE_NONE),
            0
        );
        let mut blend_mode = 0;
        assert_eq!(SDL_GetTextureBlendMode(texture, &mut blend_mode), 0);
        assert_eq!(blend_mode, SDL_BlendMode_SDL_BLENDMODE_NONE);

        assert_eq!(SDL_SetTextureColorMod(texture, 255, 255, 255), 0);
        let (mut r, mut g, mut b): (Uint8, Uint8, Uint8) = (0, 0, 0);
        assert_eq!(SDL_GetTextureColorMod(texture, &mut r, &mut g, &mut b), 0);
        assert_eq!((r, g, b), (255, 255, 255));

        let mut pixels = ptr::null_mut();
        let mut pitch = 0;
        assert_eq!(
            SDL_LockTexture(texture, ptr::null(), &mut pixels, &mut pitch),
            0,
            "{}",
            testutils::current_error()
        );
        ptr::write_bytes(pixels, 0xff, (pitch * 4) as usize);
        SDL_UnlockTexture(texture);

        assert_eq!(SDL_SetRenderDrawColor(harness.renderer, 0, 0, 0, 255), 0);
        assert_eq!(SDL_RenderClear(harness.renderer), 0);
        let dest = SDL_Rect {
            x: 4,
            y: 4,
            w: 4,
            h: 4,
        };
        assert_eq!(
            SDL_RenderCopy(harness.renderer, texture, ptr::null(), &dest),
            0,
            "{}",
            testutils::current_error()
        );
        assert_eq!(harness.read_pixel(5, 5), (255, 255, 255, 255));

        let source = SDL_CreateRGBSurfaceWithFormat(
            0,
            2,
            2,
            32,
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888,
        );
        assert!(!source.is_null(), "{}", testutils::current_error());
        let red = SDL_MapRGBA((*source).format, 255, 0, 0, 255);
        assert_eq!(SDL_FillRect(source, ptr::null(), red), 0);
        let from_surface = SDL_CreateTextureFromSurface(harness.renderer, source);
        assert!(!from_surface.is_null(), "{}", testutils::current_error());
        let source_dest = SDL_Rect {
            x: 10,
            y: 10,
            w: 2,
            h: 2,
        };
        assert_eq!(
            SDL_RenderCopy(harness.renderer, from_surface, ptr::null(), &source_dest),
            0
        );
        assert_eq!(harness.read_pixel(10, 10), (255, 0, 0, 255));

        SDL_DestroyTexture(from_surface);
        SDL_FreeSurface(source);
        SDL_DestroyTexture(texture);
    }
}

#[test]
fn render_geometry_and_yuv_helper_match_sdl() {
    let _serial = testutils::serial_lock();

    unsafe {
        let harness = SoftwareRendererHarness::new(16, 16);
        assert_eq!(SDL_SetRenderDrawColor(harness.renderer, 0, 0, 0, 255), 0);
        assert_eq!(SDL_RenderClear(harness.renderer), 0);

        let vertices = [
            SDL_Vertex {
                position: safe_sdl::abi::generated_types::SDL_FPoint { x: 1.0, y: 1.0 },
                color: safe_sdl::abi::generated_types::SDL_Color {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 255,
                },
                tex_coord: safe_sdl::abi::generated_types::SDL_FPoint { x: 0.0, y: 0.0 },
            },
            SDL_Vertex {
                position: safe_sdl::abi::generated_types::SDL_FPoint { x: 14.0, y: 1.0 },
                color: safe_sdl::abi::generated_types::SDL_Color {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 255,
                },
                tex_coord: safe_sdl::abi::generated_types::SDL_FPoint { x: 0.0, y: 0.0 },
            },
            SDL_Vertex {
                position: safe_sdl::abi::generated_types::SDL_FPoint { x: 8.0, y: 14.0 },
                color: safe_sdl::abi::generated_types::SDL_Color {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 255,
                },
                tex_coord: safe_sdl::abi::generated_types::SDL_FPoint { x: 0.0, y: 0.0 },
            },
        ];
        assert_eq!(
            SDL_RenderGeometry(
                harness.renderer,
                ptr::null_mut(),
                vertices.as_ptr(),
                vertices.len() as i32,
                ptr::null(),
                0,
            ),
            0,
            "{}",
            testutils::current_error()
        );
        let (r, _g, _b, a) = harness.read_pixel(8, 4);
        assert!(
            r > 0 && a > 0,
            "geometry path did not render into the surface"
        );

        let rgb = [
            255u8, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255, 255, 255, 0, 0, 255, 255, 255, 0,
            255, 32, 64, 96, 240, 80, 40, 10, 20, 30, 200, 210, 220, 90, 40, 10, 25, 75, 125, 200,
            180, 160, 12, 200, 32, 220, 110, 44,
        ];
        let formats = [
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_YV12,
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_IYUV,
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV12,
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV21,
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_YUY2,
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_UYVY,
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_YVYU,
        ];

        SDL_SetYUVConversionMode(SDL_YUV_CONVERSION_MODE_SDL_YUV_CONVERSION_BT601);
        assert_eq!(
            SDL_GetYUVConversionMode(),
            SDL_YUV_CONVERSION_MODE_SDL_YUV_CONVERSION_BT601
        );

        for format in formats {
            let pitch = testyuv_cvt::calculate_yuv_pitch(format, 4) as usize;
            let mut helper = vec![0u8; yuv_buffer_len(format, 4, 4)];
            let mut sdl = vec![0u8; helper.len()];

            assert!(testyuv_cvt::convert_rgb_to_yuv(
                format,
                &rgb,
                4 * 3,
                &mut helper,
                4,
                4,
                SDL_YUV_CONVERSION_MODE_SDL_YUV_CONVERSION_BT601,
                false,
                100,
            ));
            assert_eq!(
                SDL_ConvertPixels(
                    4,
                    4,
                    SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB24,
                    rgb.as_ptr().cast(),
                    4 * 3,
                    format,
                    sdl.as_mut_ptr().cast(),
                    pitch as i32,
                ),
                0,
                "{}",
                testutils::current_error()
            );
            for (index, (lhs, rhs)) in helper.iter().zip(&sdl).enumerate() {
                let delta = (*lhs as i16 - *rhs as i16).abs();
                assert!(
                    delta <= 1,
                    "helper diverged for format {format:#x} at byte {index}: {lhs} vs {rhs}"
                );
            }
        }
    }
}
