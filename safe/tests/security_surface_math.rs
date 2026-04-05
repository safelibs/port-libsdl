#![cfg(feature = "host-video-tests")]
#![allow(clippy::all)]

#[path = "common/testutils.rs"]
mod testutils;

use safe_sdl::abi::generated_types::SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888;
use safe_sdl::core::error::SDL_ClearError;
use safe_sdl::security::checked_math::{
    calculate_surface_allocation, validate_copy_layout, validate_preallocated_surface, MathError,
};
use safe_sdl::video::blit::{SDL_ConvertPixels, SDL_UpperBlit};
use safe_sdl::video::surface::{
    SDL_ConvertSurfaceFormat, SDL_CreateRGBSurfaceFrom, SDL_CreateRGBSurfaceWithFormat,
    SDL_DuplicateSurface, SDL_FillRect, SDL_FreeSurface,
};

#[test]
fn checked_math_rejects_wrapping_dimensions_pitches_and_copy_lengths() {
    assert!(matches!(
        calculate_surface_allocation(i32::MAX, i32::MAX, 64, 8),
        Err(MathError::Overflow(_))
    ));
    assert!(matches!(
        validate_preallocated_surface(i32::MAX, 2, i32::MAX, 64, 8),
        Err(MathError::InvalidParam("pitch"))
    ));
    assert!(matches!(
        validate_preallocated_surface(2048, 2048, 1, 32, 4),
        Err(MathError::InvalidParam("pitch"))
    ));
    assert!(matches!(
        validate_copy_layout(i32::MAX, i32::MAX, 64, 8, i32::MAX),
        Err(MathError::InvalidParam("pitch"))
    ));
}

#[test]
fn constructors_and_copy_paths_reject_values_that_used_to_wrap() {
    let _serial = testutils::serial_lock();

    unsafe {
        let surface = SDL_CreateRGBSurfaceWithFormat(
            0,
            i32::MAX,
            i32::MAX,
            32,
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888,
        );
        assert!(surface.is_null());
        assert!(!testutils::current_error().is_empty());

        let mut pixels = [0u8; 16];
        let null_pixels = SDL_CreateRGBSurfaceFrom(
            std::ptr::null_mut(),
            4,
            4,
            32,
            16,
            0x000000ff,
            0x0000ff00,
            0x00ff0000,
            0xff000000,
        );
        assert!(null_pixels.is_null());
        assert!(!testutils::current_error().is_empty());

        let from = SDL_CreateRGBSurfaceFrom(
            pixels.as_mut_ptr().cast(),
            1024,
            1024,
            32,
            1,
            0x000000ff,
            0x0000ff00,
            0x00ff0000,
            0xff000000,
        );
        assert!(from.is_null());
        assert!(!testutils::current_error().is_empty());

        let mut dst = [0u8; 16];
        assert_eq!(
            SDL_ConvertPixels(
                i32::MAX,
                i32::MAX,
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888,
                pixels.as_ptr().cast(),
                i32::MAX,
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888,
                dst.as_mut_ptr().cast(),
                i32::MAX,
            ),
            -1
        );
        assert!(!testutils::current_error().is_empty());

        let src = SDL_CreateRGBSurfaceWithFormat(
            0,
            4,
            4,
            32,
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888,
        );
        let dst_surface = SDL_CreateRGBSurfaceWithFormat(
            0,
            4,
            4,
            32,
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888,
        );
        assert!(!src.is_null());
        assert!(!dst_surface.is_null());
        let original_pitch = (*src).pitch;
        (*src).pitch = 1;
        let mut dstrect = safe_sdl::abi::generated_types::SDL_Rect {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        };
        assert_eq!(
            SDL_UpperBlit(src, std::ptr::null(), dst_surface, &mut dstrect),
            -1
        );
        assert!(!testutils::current_error().is_empty());
        (*src).pitch = original_pitch;
        SDL_FreeSurface(dst_surface);
        SDL_FreeSurface(src);
    }
}

#[test]
fn hostile_surface_with_null_pixels_is_rejected_before_host_calls() {
    let _serial = testutils::serial_lock();

    unsafe {
        let surface = SDL_CreateRGBSurfaceWithFormat(
            0,
            4,
            4,
            32,
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888,
        );
        assert!(!surface.is_null(), "{}", testutils::current_error());

        let original_pixels = (*surface).pixels;
        (*surface).pixels = std::ptr::null_mut();

        SDL_ClearError();
        let duplicate = SDL_DuplicateSurface(surface);
        assert!(duplicate.is_null());
        assert_eq!(testutils::current_error(), "Parameter 'surface' is invalid");

        SDL_ClearError();
        let converted =
            SDL_ConvertSurfaceFormat(surface, SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888, 0);
        assert!(converted.is_null());
        assert_eq!(testutils::current_error(), "Parameter 'surface' is invalid");

        SDL_ClearError();
        assert_eq!(SDL_FillRect(surface, std::ptr::null(), 0), -1);
        assert_eq!(testutils::current_error(), "Parameter 'surface' is invalid");

        (*surface).pixels = original_pixels;
        let original_pitch = (*surface).pitch;
        (*surface).pitch = i32::MAX;

        SDL_ClearError();
        let duplicate = SDL_DuplicateSurface(surface);
        assert!(duplicate.is_null());
        assert_eq!(testutils::current_error(), "Parameter 'surface' is invalid");

        (*surface).pitch = original_pitch;
        SDL_FreeSurface(surface);
    }
}
