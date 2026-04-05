#![cfg(feature = "host-video-tests")]
#![allow(clippy::all)]

#[path = "common/testutils.rs"]
mod testutils;
#[path = "common/testyuv_cvt.rs"]
mod testyuv_cvt;

use std::ffi::CString;
use std::path::Path;
use std::ptr;

use safe_sdl::abi::generated_types::{
    SDL_GLattr_SDL_GL_CONTEXT_MAJOR_VERSION, SDL_GLattr_SDL_GL_CONTEXT_MINOR_VERSION,
    SDL_GLattr_SDL_GL_CONTEXT_PROFILE_MASK, SDL_GLprofile_SDL_GL_CONTEXT_PROFILE_ES,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888, SDL_PixelFormatEnum_SDL_PIXELFORMAT_IYUV,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV12, SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB24, SDL_Rect,
    SDL_Renderer, SDL_Surface, SDL_TextureAccess_SDL_TEXTUREACCESS_STREAMING,
    SDL_TextureAccess_SDL_TEXTUREACCESS_TARGET, SDL_Window, SDL_WindowFlags_SDL_WINDOW_HIDDEN,
    SDL_WindowFlags_SDL_WINDOW_OPENGL, SDL_WindowFlags_SDL_WINDOW_VULKAN, Uint32, SDL_INIT_VIDEO,
    SDL_YUV_CONVERSION_MODE_SDL_YUV_CONVERSION_BT601,
};
use safe_sdl::core::rwops::SDL_RWFromFile;
use safe_sdl::render::core::{
    SDL_CreateRenderer, SDL_CreateTexture, SDL_CreateTextureFromSurface, SDL_DestroyRenderer,
    SDL_DestroyTexture, SDL_GetRenderer, SDL_RenderClear, SDL_RenderCopy, SDL_RenderCopyEx,
    SDL_RenderFillRect, SDL_RenderGetWindow, SDL_RenderPresent, SDL_RenderReadPixels,
    SDL_RenderSetScale, SDL_RenderSetViewport, SDL_RenderTargetSupported, SDL_SetRenderDrawColor,
    SDL_SetRenderTarget, SDL_UpdateNVTexture, SDL_UpdateTexture, SDL_UpdateYUVTexture,
};
use safe_sdl::render::gl::{
    SDL_GL_CreateContext, SDL_GL_DeleteContext, SDL_GL_GetDrawableSize, SDL_GL_GetProcAddress,
    SDL_GL_LoadLibrary, SDL_GL_MakeCurrent, SDL_GL_ResetAttributes, SDL_GL_SetAttribute,
    SDL_GL_UnloadLibrary,
};
use safe_sdl::video::bmp::SDL_LoadBMP_RW;
use safe_sdl::video::display::SDL_GetCurrentVideoDriver;
use safe_sdl::video::egl::EglLibrary;
use safe_sdl::video::pixels::SDL_GetRGBA;
use safe_sdl::video::surface::{
    SDL_ConvertSurfaceFormat, SDL_CreateRGBSurfaceWithFormat, SDL_FreeSurface, SDL_LockSurface,
    SDL_UnlockSurface,
};
use safe_sdl::video::vulkan::{
    SDL_Vulkan_GetDrawableSize, SDL_Vulkan_GetInstanceExtensions,
    SDL_Vulkan_GetVkGetInstanceProcAddr, SDL_Vulkan_LoadLibrary, SDL_Vulkan_UnloadLibrary,
};
use safe_sdl::video::window::{SDL_CreateWindow, SDL_DestroyWindow};

struct SurfaceRendererHarness {
    surface: *mut SDL_Surface,
    renderer: *mut SDL_Renderer,
}

impl SurfaceRendererHarness {
    unsafe fn new(width: i32, height: i32) -> Self {
        let surface = SDL_CreateRGBSurfaceWithFormat(
            0,
            width,
            height,
            32,
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888,
        );
        assert!(!surface.is_null(), "{}", testutils::current_error());
        let renderer = safe_sdl::render::software::SDL_CreateSoftwareRenderer(surface);
        assert!(!renderer.is_null(), "{}", testutils::current_error());
        Self { surface, renderer }
    }
}

impl Drop for SurfaceRendererHarness {
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

struct WindowRendererHarness {
    window: *mut SDL_Window,
    renderer: *mut SDL_Renderer,
    width: i32,
    height: i32,
}

impl WindowRendererHarness {
    unsafe fn new(title: &str, width: i32, height: i32, flags: Uint32) -> Self {
        let window = SDL_CreateWindow(
            testutils::cstring(title).as_ptr(),
            32,
            32,
            width,
            height,
            flags | SDL_WindowFlags_SDL_WINDOW_HIDDEN,
        );
        assert!(!window.is_null(), "{}", testutils::current_error());
        let renderer = SDL_CreateRenderer(window, -1, 0);
        assert!(!renderer.is_null(), "{}", testutils::current_error());
        Self {
            window,
            renderer,
            width,
            height,
        }
    }
}

impl Drop for WindowRendererHarness {
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

unsafe fn read_surface_pixel(surface: *mut SDL_Surface, x: usize, y: usize) -> (u8, u8, u8, u8) {
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
    let pixel = row.add(x * 4).cast::<Uint32>().read_unaligned();
    SDL_UnlockSurface(surface);

    let (mut r, mut g, mut b, mut a) = (0, 0, 0, 0);
    SDL_GetRGBA(pixel, (*surface).format, &mut r, &mut g, &mut b, &mut a);
    (r, g, b, a)
}

unsafe fn read_renderer_pixel(
    renderer: *mut SDL_Renderer,
    width: i32,
    height: i32,
    x: usize,
    y: usize,
) -> (u8, u8, u8, u8) {
    let surface = SDL_CreateRGBSurfaceWithFormat(
        0,
        width,
        height,
        32,
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888,
    );
    assert!(!surface.is_null(), "{}", testutils::current_error());
    assert_eq!(
        SDL_RenderReadPixels(
            renderer,
            ptr::null(),
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888,
            (*surface).pixels,
            (*surface).pitch,
        ),
        0,
        "{}",
        testutils::current_error()
    );
    let pixel = read_surface_pixel(surface, x, y);
    SDL_FreeSurface(surface);
    pixel
}

unsafe fn load_bmp(name: &str) -> *mut SDL_Surface {
    let path = testutils::get_resource_filename(None, name);
    let path = testutils::cstring(path.to_str().expect("utf-8 fixture path"));
    let mode = testutils::cstring("rb");
    let rw = SDL_RWFromFile(path.as_ptr(), mode.as_ptr());
    assert!(!rw.is_null(), "{}", testutils::current_error());
    let surface = SDL_LoadBMP_RW(rw, 1);
    assert!(!surface.is_null(), "{}", testutils::current_error());
    surface
}

fn yuv_buffer_len(width: usize, height: usize) -> usize {
    width * height + 2 * width.div_ceil(2) * height.div_ceil(2)
}

#[test]
fn testdraw2_testdrawchessboard_testgeometry_and_testviewport_ports_draw_expected_patterns() {
    let _serial = testutils::serial_lock();

    unsafe {
        let harness = SurfaceRendererHarness::new(32, 32);
        assert_eq!(SDL_SetRenderDrawColor(harness.renderer, 0, 0, 0, 255), 0);
        assert_eq!(SDL_RenderClear(harness.renderer), 0);

        for y in 0..4 {
            for x in 0..4 {
                let color = if (x + y) % 2 == 0 { 255 } else { 32 };
                assert_eq!(
                    SDL_SetRenderDrawColor(harness.renderer, color, color, color, 255),
                    0
                );
                let rect = SDL_Rect {
                    x: x * 8,
                    y: y * 8,
                    w: 8,
                    h: 8,
                };
                assert_eq!(SDL_RenderFillRect(harness.renderer, &rect), 0);
            }
        }
        assert_ne!(
            read_surface_pixel(harness.surface, 4, 4),
            read_surface_pixel(harness.surface, 12, 4)
        );

        let viewport = SDL_Rect {
            x: 16,
            y: 0,
            w: 16,
            h: 16,
        };
        assert_eq!(SDL_RenderSetViewport(harness.renderer, &viewport), 0);
        assert_eq!(SDL_SetRenderDrawColor(harness.renderer, 0, 0, 255, 255), 0);
        assert_eq!(SDL_RenderFillRect(harness.renderer, ptr::null()), 0);
        assert_eq!(read_surface_pixel(harness.surface, 20, 4), (0, 0, 255, 255));
        assert_ne!(read_surface_pixel(harness.surface, 4, 20), (0, 0, 255, 255));
        assert_eq!(SDL_RenderSetViewport(harness.renderer, ptr::null()), 0);

        let vertices = [
            safe_sdl::abi::generated_types::SDL_Vertex {
                position: safe_sdl::abi::generated_types::SDL_FPoint { x: 4.0, y: 28.0 },
                color: safe_sdl::abi::generated_types::SDL_Color {
                    r: 255,
                    g: 64,
                    b: 64,
                    a: 255,
                },
                tex_coord: safe_sdl::abi::generated_types::SDL_FPoint { x: 0.0, y: 0.0 },
            },
            safe_sdl::abi::generated_types::SDL_Vertex {
                position: safe_sdl::abi::generated_types::SDL_FPoint { x: 28.0, y: 28.0 },
                color: safe_sdl::abi::generated_types::SDL_Color {
                    r: 255,
                    g: 64,
                    b: 64,
                    a: 255,
                },
                tex_coord: safe_sdl::abi::generated_types::SDL_FPoint { x: 0.0, y: 0.0 },
            },
            safe_sdl::abi::generated_types::SDL_Vertex {
                position: safe_sdl::abi::generated_types::SDL_FPoint { x: 16.0, y: 12.0 },
                color: safe_sdl::abi::generated_types::SDL_Color {
                    r: 255,
                    g: 64,
                    b: 64,
                    a: 255,
                },
                tex_coord: safe_sdl::abi::generated_types::SDL_FPoint { x: 0.0, y: 0.0 },
            },
        ];
        assert_eq!(
            safe_sdl::render::core::SDL_RenderGeometry(
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
        let (r, g, b, _) = read_surface_pixel(harness.surface, 16, 20);
        assert!(r > 0 && g > 0 && b > 0);
    }
}

#[test]
fn testoffscreen_and_testrendertarget_ports_preserve_offscreen_and_target_workflows() {
    let _serial = testutils::serial_lock();
    let Ok(_driver) = testutils::VideoDriverGuard::init("offscreen") else {
        return;
    };

    unsafe {
        let harness = WindowRendererHarness::new("offscreen-port", 64, 48, 0);
        assert_eq!(
            testutils::string_from_c(SDL_GetCurrentVideoDriver()),
            "offscreen"
        );
        assert_eq!(SDL_GetRenderer(harness.window), harness.renderer);
        assert_eq!(SDL_RenderGetWindow(harness.renderer), harness.window);

        assert_eq!(
            SDL_SetRenderDrawColor(harness.renderer, 0x10, 0x9a, 0xce, 0xff),
            0
        );
        assert_eq!(SDL_RenderClear(harness.renderer), 0);
        SDL_RenderPresent(harness.renderer);
        let offscreen_pixel =
            read_renderer_pixel(harness.renderer, harness.width, harness.height, 1, 1);
        assert_eq!(offscreen_pixel, (0x10, 0x9a, 0xce, 0xff));

        if SDL_RenderTargetSupported(harness.renderer) == 0 {
            return;
        }

        let target = SDL_CreateTexture(
            harness.renderer,
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888,
            SDL_TextureAccess_SDL_TEXTUREACCESS_TARGET as i32,
            32,
            24,
        );
        assert!(!target.is_null(), "{}", testutils::current_error());
        assert_eq!(SDL_SetRenderTarget(harness.renderer, target), 0);
        assert_eq!(SDL_SetRenderDrawColor(harness.renderer, 255, 0, 0, 255), 0);
        assert_eq!(SDL_RenderClear(harness.renderer), 0);
        assert_eq!(SDL_SetRenderTarget(harness.renderer, ptr::null_mut()), 0);
        assert_eq!(
            SDL_RenderCopy(harness.renderer, target, ptr::null(), ptr::null()),
            0,
            "{}",
            testutils::current_error()
        );
        let target_pixel =
            read_renderer_pixel(harness.renderer, harness.width, harness.height, 8, 8);
        assert_eq!(target_pixel, (255, 0, 0, 255));
        SDL_DestroyTexture(target);
    }
}

#[test]
fn testoverlay2_testyuv_teststreaming_testrendercopyex_testscale_testsprite2_and_testspriteminimal_ports_use_authoritative_resources(
) {
    let _serial = testutils::serial_lock();
    let _driver = testutils::ScopedEnvVar::set("SDL_VIDEODRIVER", "dummy");
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_VIDEO);

    unsafe {
        let harness = WindowRendererHarness::new("render-demo-port", 96, 96, 0);
        assert_eq!(SDL_GetRenderer(harness.window), harness.renderer);
        assert_eq!(SDL_RenderGetWindow(harness.renderer), harness.window);
        let sample = load_bmp("sample.bmp");
        let sprite = load_bmp("icon.bmp");
        let yuv_source = load_bmp("testyuv.bmp");
        let yuv_source =
            SDL_ConvertSurfaceFormat(yuv_source, SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB24, 0);
        assert!(!yuv_source.is_null(), "{}", testutils::current_error());

        let sample_texture = SDL_CreateTextureFromSurface(harness.renderer, sample);
        let sprite_texture = SDL_CreateTextureFromSurface(harness.renderer, sprite);
        assert!(!sample_texture.is_null(), "{}", testutils::current_error());
        assert!(!sprite_texture.is_null(), "{}", testutils::current_error());

        assert_eq!(SDL_SetRenderDrawColor(harness.renderer, 0, 0, 0, 255), 0);
        assert_eq!(SDL_RenderClear(harness.renderer), 0);
        assert_eq!(
            SDL_RenderCopy(harness.renderer, sample_texture, ptr::null(), ptr::null()),
            0
        );
        assert_eq!(SDL_RenderSetScale(harness.renderer, 0.5, 0.5), 0);
        let rotated_dest = SDL_Rect {
            x: 96,
            y: 0,
            w: 48,
            h: 48,
        };
        assert_eq!(
            SDL_RenderCopyEx(
                harness.renderer,
                sprite_texture,
                ptr::null(),
                &rotated_dest,
                45.0,
                ptr::null(),
                0,
            ),
            0
        );
        assert_eq!(SDL_RenderSetScale(harness.renderer, 1.0, 1.0), 0);

        let streaming = SDL_CreateTexture(
            harness.renderer,
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888,
            SDL_TextureAccess_SDL_TEXTUREACCESS_STREAMING as i32,
            16,
            16,
        );
        assert!(!streaming.is_null(), "{}", testutils::current_error());
        let mut streaming_pixels = vec![0u8; 16 * 16 * 4];
        for chunk in streaming_pixels.chunks_exact_mut(4) {
            chunk.copy_from_slice(&[0x00, 0xff, 0x00, 0xff]);
        }
        assert_eq!(
            SDL_UpdateTexture(
                streaming,
                ptr::null(),
                streaming_pixels.as_ptr().cast(),
                16 * 4
            ),
            0,
            "{}",
            testutils::current_error()
        );
        let streaming_dest = SDL_Rect {
            x: 40,
            y: 40,
            w: 16,
            h: 16,
        };
        assert_eq!(
            SDL_RenderCopy(harness.renderer, streaming, ptr::null(), &streaming_dest),
            0
        );

        let rgb = std::slice::from_raw_parts(
            (*yuv_source).pixels.cast::<u8>(),
            ((*yuv_source).pitch * (*yuv_source).h) as usize,
        );
        let mut iyuv =
            vec![0u8; yuv_buffer_len((*yuv_source).w as usize, (*yuv_source).h as usize)];
        let mut nv12 = vec![0u8; iyuv.len()];
        assert!(testyuv_cvt::convert_rgb_to_yuv(
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_IYUV,
            rgb,
            (*yuv_source).pitch as usize,
            &mut iyuv,
            (*yuv_source).w as usize,
            (*yuv_source).h as usize,
            SDL_YUV_CONVERSION_MODE_SDL_YUV_CONVERSION_BT601,
            false,
            100,
        ));
        assert!(testyuv_cvt::convert_rgb_to_yuv(
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV12,
            rgb,
            (*yuv_source).pitch as usize,
            &mut nv12,
            (*yuv_source).w as usize,
            (*yuv_source).h as usize,
            SDL_YUV_CONVERSION_MODE_SDL_YUV_CONVERSION_BT601,
            false,
            100,
        ));

        let iyuv_texture = SDL_CreateTexture(
            harness.renderer,
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_IYUV,
            SDL_TextureAccess_SDL_TEXTUREACCESS_STREAMING as i32,
            (*yuv_source).w,
            (*yuv_source).h,
        );
        let nv12_texture = SDL_CreateTexture(
            harness.renderer,
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV12,
            SDL_TextureAccess_SDL_TEXTUREACCESS_STREAMING as i32,
            (*yuv_source).w,
            (*yuv_source).h,
        );
        assert!(!iyuv_texture.is_null(), "{}", testutils::current_error());
        assert!(!nv12_texture.is_null(), "{}", testutils::current_error());

        let w = (*yuv_source).w as usize;
        let h = (*yuv_source).h as usize;
        let chroma_w = w.div_ceil(2);
        let chroma_h = h.div_ceil(2);
        let y_len = w * h;
        let u_len = chroma_w * chroma_h;

        assert_eq!(
            SDL_UpdateYUVTexture(
                iyuv_texture,
                ptr::null(),
                iyuv.as_ptr(),
                w as i32,
                iyuv[y_len..].as_ptr(),
                chroma_w as i32,
                iyuv[y_len + u_len..].as_ptr(),
                chroma_w as i32,
            ),
            0,
            "{}",
            testutils::current_error()
        );
        assert_eq!(
            SDL_UpdateNVTexture(
                nv12_texture,
                ptr::null(),
                nv12.as_ptr(),
                w as i32,
                nv12[y_len..].as_ptr(),
                w as i32,
            ),
            0,
            "{}",
            testutils::current_error()
        );

        let iyuv_dest = SDL_Rect {
            x: 0,
            y: 48,
            w: 32,
            h: 32,
        };
        let nv12_dest = SDL_Rect {
            x: 32,
            y: 48,
            w: 32,
            h: 32,
        };
        assert_eq!(
            SDL_RenderCopy(harness.renderer, iyuv_texture, ptr::null(), &iyuv_dest),
            0
        );
        assert_eq!(
            SDL_RenderCopy(harness.renderer, nv12_texture, ptr::null(), &nv12_dest),
            0
        );

        let pixel = read_renderer_pixel(harness.renderer, harness.width, harness.height, 8, 56);
        assert!(pixel.0 > 0 || pixel.1 > 0 || pixel.2 > 0);

        SDL_DestroyTexture(nv12_texture);
        SDL_DestroyTexture(iyuv_texture);
        SDL_DestroyTexture(streaming);
        SDL_DestroyTexture(sprite_texture);
        SDL_DestroyTexture(sample_texture);
        SDL_FreeSurface(yuv_source);
        SDL_FreeSurface(sprite);
        SDL_FreeSurface(sample);
    }
}

#[test]
fn testgl2_testgles_testgles2_testgles2_sdf_and_testshader_ports_validate_gl_gles_egl_resource_paths(
) {
    let _serial = testutils::serial_lock();
    let Some(_display) = testutils::acquire_x11_display() else {
        return;
    };
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_VIDEO);

    assert!(Path::new(&testutils::get_resource_filename(
        None,
        "testgles2_sdf_img_normal.bmp"
    ))
    .exists());
    assert!(Path::new(&testutils::get_resource_filename(
        None,
        "testgles2_sdf_img_sdf.bmp"
    ))
    .exists());

    if let Ok(egl) = EglLibrary::load(None) {
        let name = CString::new("eglGetDisplay").unwrap();
        assert!(!egl.get_proc_address(name.as_c_str()).is_null());
    }

    unsafe {
        let gl_window = SDL_CreateWindow(
            testutils::cstring("gl-port").as_ptr(),
            32,
            32,
            64,
            64,
            SDL_WindowFlags_SDL_WINDOW_OPENGL | SDL_WindowFlags_SDL_WINDOW_HIDDEN,
        );
        assert!(!gl_window.is_null(), "{}", testutils::current_error());

        if SDL_GL_LoadLibrary(ptr::null()) == 0 {
            let proc_name = testutils::cstring("glClear");
            assert!(!SDL_GL_GetProcAddress(proc_name.as_ptr()).is_null());

            let context = SDL_GL_CreateContext(gl_window);
            if !context.is_null() {
                assert_eq!(SDL_GL_MakeCurrent(gl_window, context), 0);
                let (mut w, mut h) = (0, 0);
                SDL_GL_GetDrawableSize(gl_window, &mut w, &mut h);
                assert!(w > 0 && h > 0);
                SDL_GL_DeleteContext(context);
            }
            SDL_GL_UnloadLibrary();
        }
        SDL_DestroyWindow(gl_window);

        SDL_GL_ResetAttributes();
        assert_eq!(
            SDL_GL_SetAttribute(
                SDL_GLattr_SDL_GL_CONTEXT_PROFILE_MASK,
                SDL_GLprofile_SDL_GL_CONTEXT_PROFILE_ES as i32,
            ),
            0
        );
        assert_eq!(
            SDL_GL_SetAttribute(SDL_GLattr_SDL_GL_CONTEXT_MAJOR_VERSION, 2),
            0
        );
        assert_eq!(
            SDL_GL_SetAttribute(SDL_GLattr_SDL_GL_CONTEXT_MINOR_VERSION, 0),
            0
        );

        let gles_window = SDL_CreateWindow(
            testutils::cstring("gles-port").as_ptr(),
            40,
            40,
            64,
            64,
            SDL_WindowFlags_SDL_WINDOW_OPENGL | SDL_WindowFlags_SDL_WINDOW_HIDDEN,
        );
        assert!(!gles_window.is_null(), "{}", testutils::current_error());
        if SDL_GL_LoadLibrary(ptr::null()) == 0 {
            let context = SDL_GL_CreateContext(gles_window);
            if !context.is_null() {
                SDL_GL_DeleteContext(context);
            }
            SDL_GL_UnloadLibrary();
        }
        SDL_DestroyWindow(gles_window);
    }
}

#[test]
fn testvulkan_port_queries_extensions_and_drawable_size_when_loader_is_available() {
    let _serial = testutils::serial_lock();
    let Some(_display) = testutils::acquire_x11_display() else {
        return;
    };
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_VIDEO);

    unsafe {
        let window = SDL_CreateWindow(
            testutils::cstring("vulkan-port").as_ptr(),
            32,
            32,
            64,
            64,
            SDL_WindowFlags_SDL_WINDOW_VULKAN | SDL_WindowFlags_SDL_WINDOW_HIDDEN,
        );
        if window.is_null() {
            return;
        }

        if SDL_Vulkan_LoadLibrary(ptr::null()) != 0 {
            SDL_DestroyWindow(window);
            return;
        }

        assert!(!SDL_Vulkan_GetVkGetInstanceProcAddr().is_null());

        let mut count = 0u32;
        if SDL_Vulkan_GetInstanceExtensions(window, &mut count, ptr::null_mut()) != 0 {
            assert!(count > 0);
            let mut names = vec![ptr::null(); count as usize];
            assert_ne!(
                SDL_Vulkan_GetInstanceExtensions(window, &mut count, names.as_mut_ptr()),
                0
            );
        }

        let (mut w, mut h) = (0, 0);
        SDL_Vulkan_GetDrawableSize(window, &mut w, &mut h);
        assert!(w > 0 && h > 0);

        SDL_Vulkan_UnloadLibrary();
        SDL_DestroyWindow(window);
    }
}
