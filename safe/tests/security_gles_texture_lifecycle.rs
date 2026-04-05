#![cfg(feature = "host-video-tests")]
#![allow(non_upper_case_globals)]
#![allow(clippy::all)]

#[path = "common/testutils.rs"]
mod testutils;

use safe_sdl::abi::generated_types::{
    SDL_GLattr_SDL_GL_CONTEXT_MAJOR_VERSION, SDL_GLattr_SDL_GL_CONTEXT_MINOR_VERSION,
    SDL_GLattr_SDL_GL_CONTEXT_PROFILE_MASK, SDL_GLprofile_SDL_GL_CONTEXT_PROFILE_ES,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_EXTERNAL_OES, SDL_PixelFormatEnum_SDL_PIXELFORMAT_IYUV,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV12, SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBA32,
    SDL_RendererInfo, SDL_TextureAccess_SDL_TEXTUREACCESS_STATIC,
    SDL_TextureAccess_SDL_TEXTUREACCESS_STREAMING, SDL_WindowFlags_SDL_WINDOW_HIDDEN,
    SDL_WindowFlags_SDL_WINDOW_OPENGL, SDL_INIT_VIDEO,
};
use safe_sdl::render::core::{
    SDL_CreateRenderer, SDL_CreateTexture, SDL_DestroyRenderer, SDL_DestroyTexture,
    SDL_GetRendererInfo,
};
use safe_sdl::render::gl::{SDL_GL_ResetAttributes, SDL_GL_SetAttribute};
use safe_sdl::render::gles::{
    reset_texture_lifecycle_counters, set_texture_creation_failure_step_for_test,
    texture_creation_step_count_for_test, texture_lifecycle_counters,
};
use safe_sdl::video::window::{SDL_CreateWindow, SDL_DestroyWindow};
use std::ffi::CStr;
use std::mem::MaybeUninit;

struct GlesHarness {
    window: *mut safe_sdl::abi::generated_types::SDL_Window,
    renderer: *mut safe_sdl::abi::generated_types::SDL_Renderer,
    renderer_name: String,
}

impl Drop for GlesHarness {
    fn drop(&mut self) {
        unsafe {
            if !self.renderer.is_null() {
                SDL_DestroyRenderer(self.renderer);
            }
            if !self.window.is_null() {
                SDL_DestroyWindow(self.window);
            }
        }
    }
}

unsafe fn create_gles_harness() -> Option<GlesHarness> {
    let candidates = [("opengles2", 2), ("opengles", 1)];

    for (driver, major_version) in candidates {
        let _driver_hint = testutils::ScopedEnvVar::set("SDL_RENDER_DRIVER", driver);
        SDL_GL_ResetAttributes();
        assert_eq!(
            SDL_GL_SetAttribute(
                SDL_GLattr_SDL_GL_CONTEXT_PROFILE_MASK,
                SDL_GLprofile_SDL_GL_CONTEXT_PROFILE_ES as i32,
            ),
            0,
            "{}",
            testutils::current_error()
        );
        assert_eq!(
            SDL_GL_SetAttribute(SDL_GLattr_SDL_GL_CONTEXT_MAJOR_VERSION, major_version),
            0,
            "{}",
            testutils::current_error()
        );
        assert_eq!(
            SDL_GL_SetAttribute(SDL_GLattr_SDL_GL_CONTEXT_MINOR_VERSION, 0),
            0,
            "{}",
            testutils::current_error()
        );

        let window = SDL_CreateWindow(
            testutils::cstring("security-gles").as_ptr(),
            32,
            32,
            96,
            96,
            SDL_WindowFlags_SDL_WINDOW_OPENGL | SDL_WindowFlags_SDL_WINDOW_HIDDEN,
        );
        if window.is_null() {
            continue;
        }

        let renderer = SDL_CreateRenderer(window, -1, 0);
        if renderer.is_null() {
            SDL_DestroyWindow(window);
            continue;
        }

        let mut info = MaybeUninit::<SDL_RendererInfo>::zeroed();
        if SDL_GetRendererInfo(renderer, info.as_mut_ptr()) != 0 {
            SDL_DestroyRenderer(renderer);
            SDL_DestroyWindow(window);
            continue;
        }

        let info = info.assume_init();
        let renderer_name = if info.name.is_null() {
            String::new()
        } else {
            CStr::from_ptr(info.name).to_string_lossy().into_owned()
        };
        let lowered = renderer_name.to_ascii_lowercase();
        if lowered.contains("opengles") || lowered == "gles" || lowered == "gles2" {
            return Some(GlesHarness {
                window,
                renderer,
                renderer_name,
            });
        }

        SDL_DestroyRenderer(renderer);
        SDL_DestroyWindow(window);
    }

    None
}

#[test]
fn successful_streaming_texture_creation_releases_every_resource_on_destroy() {
    let _serial = testutils::serial_lock();
    let Some(_display) = testutils::acquire_x11_display() else {
        return;
    };
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_VIDEO);
    let Some(harness) = (unsafe { create_gles_harness() }) else {
        return;
    };

    unsafe {
        reset_texture_lifecycle_counters();
        set_texture_creation_failure_step_for_test(None);

        let texture = if harness.renderer_name.to_ascii_lowercase().contains("2") {
            SDL_CreateTexture(
                harness.renderer,
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_IYUV,
                SDL_TextureAccess_SDL_TEXTUREACCESS_STREAMING as i32,
                32,
                24,
            )
        } else {
            SDL_CreateTexture(
                harness.renderer,
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBA32,
                SDL_TextureAccess_SDL_TEXTUREACCESS_STREAMING as i32,
                32,
                24,
            )
        };
        assert!(!texture.is_null(), "{}", testutils::current_error());

        let counters = texture_lifecycle_counters();
        assert!(counters.created_textures >= 1, "{counters:?}");
        assert_eq!(counters.destroyed_textures, 0, "{counters:?}");
        assert_eq!(counters.allocated_pixel_buffers, 1, "{counters:?}");
        assert_eq!(counters.freed_pixel_buffers, 0, "{counters:?}");

        SDL_DestroyTexture(texture);

        let counters = texture_lifecycle_counters();
        assert_eq!(counters.created_textures, counters.destroyed_textures);
        assert_eq!(
            counters.allocated_pixel_buffers,
            counters.freed_pixel_buffers
        );
    }
}

#[test]
fn injected_failures_do_not_leak_partially_created_gles_resources() {
    let _serial = testutils::serial_lock();
    let Some(_display) = testutils::acquire_x11_display() else {
        return;
    };
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_VIDEO);
    let Some(harness) = (unsafe { create_gles_harness() }) else {
        return;
    };

    let scenarios = [
        (
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBA32,
            SDL_TextureAccess_SDL_TEXTUREACCESS_STREAMING,
        ),
        (
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_IYUV,
            SDL_TextureAccess_SDL_TEXTUREACCESS_STREAMING,
        ),
        (
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV12,
            SDL_TextureAccess_SDL_TEXTUREACCESS_STATIC,
        ),
        (
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_EXTERNAL_OES,
            SDL_TextureAccess_SDL_TEXTUREACCESS_STATIC,
        ),
    ];

    unsafe {
        for (format, access) in scenarios {
            let max_step = texture_creation_step_count_for_test(format, access as i32);
            assert!(max_step > 0);

            for fail_after in 1..=max_step {
                reset_texture_lifecycle_counters();
                set_texture_creation_failure_step_for_test(Some(fail_after));

                let texture = SDL_CreateTexture(harness.renderer, format, access as i32, 32, 24);
                assert!(
                    texture.is_null(),
                    "expected injected failure at step {fail_after} for format {format:#x}"
                );
                assert!(
                    !testutils::current_error().is_empty(),
                    "missing error for injected failure at step {fail_after}"
                );

                let counters = texture_lifecycle_counters();
                assert_eq!(
                    counters.created_textures, counters.destroyed_textures,
                    "texture leak detected for format {format:#x} at step {fail_after}: {counters:?}"
                );
                assert_eq!(
                    counters.allocated_pixel_buffers, counters.freed_pixel_buffers,
                    "pixel-buffer leak detected for format {format:#x} at step {fail_after}: {counters:?}"
                );
            }
        }

        set_texture_creation_failure_step_for_test(None);
    }
}
