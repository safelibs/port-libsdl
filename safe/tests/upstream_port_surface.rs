#![allow(clippy::all)]

#[path = "common/testutils.rs"]
mod testutils;

use std::mem::MaybeUninit;
use std::path::Path;
use std::ptr;

use safe_sdl::abi::generated_types::{
    SDL_BlendMode_SDL_BLENDMODE_BLEND, SDL_Color, SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB2101010,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888, SDL_PixelFormatEnum_SDL_PIXELFORMAT_INDEX8,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB565, SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBA8888,
    SDL_Point, SDL_Rect, Uint32, SDL_RWOPS_MEMORY, SDL_RWOPS_MEMORY_RO,
};
use safe_sdl::core::error::SDL_ClearError;
use safe_sdl::core::rwops::{
    SDL_RWFromConstMem, SDL_RWFromFile, SDL_RWFromMem, SDL_RWclose, SDL_RWread, SDL_RWseek,
    SDL_RWtell, SDL_RWwrite, SDL_ReadLE32,
};
use safe_sdl::video::blit::{
    SDL_ConvertPixels, SDL_PremultiplyAlpha, SDL_SoftStretch, SDL_UpperBlit, SDL_UpperBlitScaled,
};
use safe_sdl::video::bmp::{SDL_LoadBMP_RW, SDL_SaveBMP_RW};
use safe_sdl::video::pixels::{
    SDL_AllocFormat, SDL_AllocPalette, SDL_CalculateGammaRamp, SDL_FreeFormat, SDL_FreePalette,
    SDL_GetPixelFormatName, SDL_GetRGBA, SDL_MapRGBA, SDL_MasksToPixelFormatEnum,
    SDL_PixelFormatEnumToMasks, SDL_SetPaletteColors, SDL_SetPixelFormatPalette,
};
use safe_sdl::video::rect::{
    SDL_EnclosePoints, SDL_HasIntersection, SDL_IntersectRect, SDL_IntersectRectAndLine,
    SDL_UnionRect,
};
use safe_sdl::video::surface::{
    SDL_ConvertSurfaceFormat, SDL_CreateRGBSurfaceWithFormat, SDL_DuplicateSurface, SDL_FillRect,
    SDL_FreeSurface, SDL_GetColorKey, SDL_GetSurfaceAlphaMod, SDL_GetSurfaceBlendMode,
    SDL_GetSurfaceColorMod, SDL_LockSurface, SDL_SetClipRect, SDL_SetColorKey,
    SDL_SetSurfaceAlphaMod, SDL_SetSurfaceBlendMode, SDL_SetSurfaceColorMod, SDL_UnlockSurface,
};

#[link(name = "SDL2_test")]
unsafe extern "C" {
    fn SDLTest_CompareSurfaces(
        surface: *mut safe_sdl::abi::generated_types::SDL_Surface,
        referenceSurface: *mut safe_sdl::abi::generated_types::SDL_Surface,
        allowable_error: i32,
    ) -> i32;
    fn SDLTest_ImageBlit() -> *mut safe_sdl::abi::generated_types::SDL_Surface;
    fn SDLTest_ImageFace() -> *mut safe_sdl::abi::generated_types::SDL_Surface;
}

unsafe fn create_argb8888_surface(
    width: i32,
    height: i32,
) -> *mut safe_sdl::abi::generated_types::SDL_Surface {
    SDL_CreateRGBSurfaceWithFormat(
        0,
        width,
        height,
        32,
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888,
    )
}

unsafe fn read_pixel32(
    surface: *mut safe_sdl::abi::generated_types::SDL_Surface,
    x: usize,
    y: usize,
) -> Uint32 {
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
    let value = *(row.add(x * 4).cast::<Uint32>());
    SDL_UnlockSurface(surface);
    value
}

unsafe fn load_bmp(path: &Path) -> *mut safe_sdl::abi::generated_types::SDL_Surface {
    let path_c = testutils::cstring(path.to_str().expect("utf-8 fixture path"));
    let mode = testutils::cstring("rb");
    let rw = SDL_RWFromFile(path_c.as_ptr(), mode.as_ptr());
    assert!(!rw.is_null(), "{}", testutils::current_error());
    SDL_LoadBMP_RW(rw, 1)
}

unsafe fn save_bmp(surface: *mut safe_sdl::abi::generated_types::SDL_Surface, path: &Path) -> i32 {
    let path_c = testutils::cstring(path.to_str().expect("utf-8 temp path"));
    let mode = testutils::cstring("wb");
    let rw = SDL_RWFromFile(path_c.as_ptr(), mode.as_ptr());
    assert!(!rw.is_null(), "{}", testutils::current_error());
    SDL_SaveBMP_RW(surface, rw, 1)
}

#[test]
fn pixels_alloc_name_masks_palette_and_rgba_roundtrip() {
    let _serial = testutils::serial_lock();

    unsafe {
        let format = SDL_AllocFormat(SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888);
        assert!(!format.is_null(), "{}", testutils::current_error());
        assert_eq!(
            testutils::string_from_c(SDL_GetPixelFormatName((*format).format)),
            "SDL_PIXELFORMAT_ARGB8888"
        );

        let pixel = SDL_MapRGBA(format, 0x11, 0x22, 0x33, 0x44);
        let mut r = 0;
        let mut g = 0;
        let mut b = 0;
        let mut a = 0;
        SDL_GetRGBA(pixel, format, &mut r, &mut g, &mut b, &mut a);
        assert_eq!((r, g, b, a), (0x11, 0x22, 0x33, 0x44));

        let argb2101010 = SDL_AllocFormat(SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB2101010);
        assert!(!argb2101010.is_null(), "{}", testutils::current_error());
        let pixel_2101010 = SDL_MapRGBA(argb2101010, 0x12, 0x34, 0x56, 0xff);
        SDL_GetRGBA(pixel_2101010, argb2101010, &mut r, &mut g, &mut b, &mut a);
        assert_eq!((r, g, b, a), (0x12, 0x34, 0x56, 0xff));

        let mut bpp = 0;
        let mut rmask = 0;
        let mut gmask = 0;
        let mut bmask = 0;
        let mut amask = 0;
        assert_ne!(
            SDL_PixelFormatEnumToMasks(
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB565,
                &mut bpp,
                &mut rmask,
                &mut gmask,
                &mut bmask,
                &mut amask,
            ),
            0
        );
        assert_eq!(bpp, 16);
        assert_eq!(
            SDL_MasksToPixelFormatEnum(bpp, rmask, gmask, bmask, amask),
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB565
        );

        let palette_format = SDL_AllocFormat(SDL_PixelFormatEnum_SDL_PIXELFORMAT_INDEX8);
        assert!(!palette_format.is_null(), "{}", testutils::current_error());
        let palette = SDL_AllocPalette(2);
        assert!(!palette.is_null(), "{}", testutils::current_error());
        let colors = [
            SDL_Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            SDL_Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
        ];
        assert_eq!(
            SDL_SetPaletteColors(palette, colors.as_ptr(), 0, colors.len() as i32),
            0
        );
        assert_eq!(SDL_SetPixelFormatPalette(palette_format, palette), 0);

        SDL_FreePalette(palette);
        SDL_FreeFormat(palette_format);
        SDL_FreeFormat(argb2101010);
        SDL_FreeFormat(format);
    }
}

#[test]
fn rect_intersection_union_enclose_and_line_clipping_match_upstream_cases() {
    let _serial = testutils::serial_lock();

    unsafe {
        let a = SDL_Rect {
            x: 0,
            y: 0,
            w: 32,
            h: 32,
        };
        let b = SDL_Rect {
            x: 16,
            y: 8,
            w: 32,
            h: 16,
        };
        let mut intersection = MaybeUninit::<SDL_Rect>::zeroed();
        assert_ne!(SDL_IntersectRect(&a, &b, intersection.as_mut_ptr()), 0);
        let intersection = intersection.assume_init();
        assert_eq!(
            (
                intersection.x,
                intersection.y,
                intersection.w,
                intersection.h
            ),
            (16, 8, 16, 16)
        );

        let mut union = MaybeUninit::<SDL_Rect>::zeroed();
        SDL_UnionRect(&a, &b, union.as_mut_ptr());
        let union = union.assume_init();
        assert_eq!((union.x, union.y, union.w, union.h), (0, 0, 48, 32));

        let points = [
            SDL_Point { x: 4, y: 3 },
            SDL_Point { x: 20, y: 8 },
            SDL_Point { x: 10, y: 12 },
        ];
        let mut enclosed = MaybeUninit::<SDL_Rect>::zeroed();
        assert_ne!(
            SDL_EnclosePoints(
                points.as_ptr(),
                points.len() as i32,
                ptr::null(),
                enclosed.as_mut_ptr()
            ),
            0
        );
        let enclosed = enclosed.assume_init();
        assert_eq!(
            (enclosed.x, enclosed.y, enclosed.w, enclosed.h),
            (4, 3, 17, 10)
        );

        let rect = a;
        let (mut x1, mut y1, mut x2, mut y2) = (-32, 15, 64, 15);
        assert_ne!(
            SDL_IntersectRectAndLine(&rect, &mut x1, &mut y1, &mut x2, &mut y2),
            0
        );
        assert_eq!((x1, y1, x2, y2), (0, 15, 31, 15));
        assert_eq!((rect.x, rect.y, rect.w, rect.h), (0, 0, 32, 32));
    }
}

#[test]
fn pixels_and_rect_invalid_parameters_preserve_upstream_error_messages() {
    let _serial = testutils::serial_lock();

    unsafe {
        let mut ramp = [0xbeefu16; 256];
        SDL_ClearError();
        SDL_CalculateGammaRamp(-1.0, ramp.as_mut_ptr());
        assert_eq!(testutils::current_error(), "Parameter 'gamma' is invalid");
        assert!(ramp.iter().all(|&value| value == 0xbeef));

        SDL_ClearError();
        SDL_CalculateGammaRamp(0.5, ptr::null_mut());
        assert_eq!(testutils::current_error(), "Parameter 'ramp' is invalid");

        let a = SDL_Rect {
            x: 0,
            y: 0,
            w: 32,
            h: 32,
        };
        let b = SDL_Rect {
            x: 8,
            y: 8,
            w: 8,
            h: 8,
        };
        let points = [SDL_Point { x: 4, y: 6 }];

        SDL_ClearError();
        assert_eq!(SDL_HasIntersection(ptr::null(), &b), 0);
        assert_eq!(testutils::current_error(), "Parameter 'A' is invalid");

        SDL_ClearError();
        assert_eq!(SDL_IntersectRect(&a, &b, ptr::null_mut()), 0);
        assert_eq!(testutils::current_error(), "Parameter 'result' is invalid");

        let mut union = SDL_Rect {
            x: -1,
            y: -1,
            w: -1,
            h: -1,
        };
        SDL_ClearError();
        SDL_UnionRect(&a, ptr::null(), &mut union);
        assert_eq!(testutils::current_error(), "Parameter 'B' is invalid");

        SDL_ClearError();
        assert_eq!(
            SDL_EnclosePoints(ptr::null(), 1, ptr::null(), &mut union),
            0
        );
        assert_eq!(testutils::current_error(), "Parameter 'points' is invalid");

        SDL_ClearError();
        assert_eq!(
            SDL_EnclosePoints(points.as_ptr(), 0, ptr::null(), &mut union),
            0
        );
        assert_eq!(testutils::current_error(), "Parameter 'count' is invalid");

        let (x1, mut y1, mut x2, mut y2) = (4, 4, 40, 40);
        SDL_ClearError();
        assert_eq!(
            SDL_IntersectRectAndLine(&a, ptr::null_mut(), &mut y1, &mut x2, &mut y2),
            0
        );
        assert_eq!(testutils::current_error(), "Parameter 'X1' is invalid");
        assert_eq!((x1, y1, x2, y2), (4, 4, 40, 40));
    }
}

#[test]
fn rwops_mem_constmem_and_file_helpers_cover_upstream_read_write_semantics() {
    let _serial = testutils::serial_lock();

    unsafe {
        assert!(SDL_RWFromMem(ptr::null_mut(), 4).is_null());
        assert!(SDL_RWFromMem([0u8; 1].as_ptr() as *mut _, 0).is_null());

        let mut buffer = [0u8; 12];
        let rw = SDL_RWFromMem(buffer.as_mut_ptr().cast(), buffer.len() as i32);
        assert!(!rw.is_null(), "{}", testutils::current_error());
        assert_eq!((*rw).type_, SDL_RWOPS_MEMORY);
        assert_eq!(SDL_RWwrite(rw, b"Hello World!".as_ptr().cast(), 1, 12), 12);
        assert_eq!(SDL_RWtell(rw), 12);
        assert_eq!(SDL_RWseek(rw, 0, libc::SEEK_SET), 0);
        let mut out = [0u8; 12];
        assert_eq!(
            SDL_RWread(rw, out.as_mut_ptr().cast(), 1, out.len()),
            out.len()
        );
        assert_eq!(&out, b"Hello World!");
        assert_eq!(SDL_RWclose(rw), 0);

        let readonly = SDL_RWFromConstMem(buffer.as_ptr().cast(), buffer.len() as i32);
        assert!(!readonly.is_null(), "{}", testutils::current_error());
        assert_eq!((*readonly).type_, SDL_RWOPS_MEMORY_RO);
        assert_eq!(SDL_RWwrite(readonly, b"!".as_ptr().cast(), 1, 1), 0);
        assert_eq!(SDL_RWseek(readonly, 0, libc::SEEK_SET), 0);
        assert_eq!(SDL_ReadLE32(readonly), u32::from_le_bytes(*b"Hell"));
        assert_eq!(SDL_RWclose(readonly), 0);
    }

    let file = tempfile::NamedTempFile::new().expect("create temp file");
    let path_c = testutils::cstring(file.path().to_str().expect("utf-8 temp path"));
    let mode = testutils::cstring("wb+");
    unsafe {
        let rw = SDL_RWFromFile(path_c.as_ptr(), mode.as_ptr());
        assert!(!rw.is_null(), "{}", testutils::current_error());
        assert_eq!(
            (*rw).type_,
            safe_sdl::abi::generated_types::SDL_RWOPS_STDFILE
        );
        assert_eq!(SDL_RWwrite(rw, b"1234567".as_ptr().cast(), 1, 7), 7);
        assert_eq!(SDL_RWseek(rw, -4, libc::SEEK_END), 3);
        let mut out = [0u8; 4];
        assert_eq!(
            SDL_RWread(rw, out.as_mut_ptr().cast(), 1, out.len()),
            out.len()
        );
        assert_eq!(&out, b"4567");
        assert_eq!(SDL_RWclose(rw), 0);
    }
}

#[test]
fn surface_create_fill_duplicate_convert_and_state_modulation_work() {
    let _serial = testutils::serial_lock();

    unsafe {
        let surface = create_argb8888_surface(8, 8);
        assert!(!surface.is_null(), "{}", testutils::current_error());
        let red = SDL_MapRGBA((*surface).format, 255, 0, 0, 255);
        assert_eq!(SDL_FillRect(surface, ptr::null(), red), 0);
        assert_eq!(read_pixel32(surface, 0, 0), red);

        let duplicate = SDL_DuplicateSurface(surface);
        assert!(!duplicate.is_null(), "{}", testutils::current_error());
        assert_eq!(((*duplicate).w, (*duplicate).h), (8, 8));
        assert_eq!(read_pixel32(duplicate, 0, 0), red);

        let clip = SDL_Rect {
            x: 2,
            y: 2,
            w: 4,
            h: 4,
        };
        assert_ne!(SDL_SetClipRect(surface, &clip), 0);

        assert_eq!(SDL_SetColorKey(surface, 1, red), 0);
        let mut key = 0;
        assert_eq!(SDL_GetColorKey(surface, &mut key), 0);
        assert_eq!(key, red);

        assert_eq!(SDL_SetSurfaceColorMod(surface, 1, 2, 3), 0);
        assert_eq!(SDL_SetSurfaceAlphaMod(surface, 77), 0);
        assert_eq!(
            SDL_SetSurfaceBlendMode(surface, SDL_BlendMode_SDL_BLENDMODE_BLEND),
            0
        );

        let mut r = 0;
        let mut g = 0;
        let mut b = 0;
        let mut a = 0;
        let mut blend = 0;
        assert_eq!(SDL_GetSurfaceColorMod(surface, &mut r, &mut g, &mut b), 0);
        assert_eq!(SDL_GetSurfaceAlphaMod(surface, &mut a), 0);
        assert_eq!(SDL_GetSurfaceBlendMode(surface, &mut blend), 0);
        assert_eq!(
            (r, g, b, a, blend),
            (1, 2, 3, 77, SDL_BlendMode_SDL_BLENDMODE_BLEND)
        );

        let converted =
            SDL_ConvertSurfaceFormat(surface, SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBA8888, 0);
        assert!(!converted.is_null(), "{}", testutils::current_error());

        SDL_FreeSurface(converted);
        SDL_FreeSurface(duplicate);
        SDL_FreeSurface(surface);
    }
}

#[test]
fn surface_bmp_roundtrip_and_blit_paths_match_owned_surface_behavior() {
    let _serial = testutils::serial_lock();

    unsafe {
        let source = create_argb8888_surface(8, 8);
        let dest = create_argb8888_surface(16, 16);
        assert!(!source.is_null());
        assert!(!dest.is_null());

        let red = SDL_MapRGBA((*source).format, 255, 0, 0, 255);
        let black = SDL_MapRGBA((*dest).format, 0, 0, 0, 255);
        assert_eq!(SDL_FillRect(source, ptr::null(), red), 0);
        assert_eq!(SDL_FillRect(dest, ptr::null(), black), 0);

        let mut dstrect = SDL_Rect {
            x: 4,
            y: 4,
            w: 0,
            h: 0,
        };
        assert_eq!(SDL_UpperBlit(source, ptr::null(), dest, &mut dstrect), 0);
        assert_eq!(read_pixel32(dest, 4, 4), red);

        let src_rect = SDL_Rect {
            x: 0,
            y: 0,
            w: 8,
            h: 8,
        };
        let mut scaled_rect = SDL_Rect {
            x: 0,
            y: 0,
            w: 16,
            h: 16,
        };
        assert_eq!(
            SDL_UpperBlitScaled(source, &src_rect, dest, &mut scaled_rect),
            0
        );
        assert_eq!(SDL_SoftStretch(source, &src_rect, dest, &scaled_rect), 0);
        assert_eq!(read_pixel32(dest, 8, 8), red);

        let temp = tempfile::NamedTempFile::new().expect("create temp bmp");
        assert_eq!(
            save_bmp(source, temp.path()),
            0,
            "{}",
            testutils::current_error()
        );
        let loaded = load_bmp(temp.path());
        assert!(!loaded.is_null(), "{}", testutils::current_error());
        assert_eq!(((*loaded).w, (*loaded).h), (8, 8));
        let loaded_red = SDL_MapRGBA((*loaded).format, 255, 0, 0, 255);
        assert_eq!(read_pixel32(loaded, 0, 0), loaded_red);

        SDL_FreeSurface(loaded);
        SDL_FreeSurface(dest);
        SDL_FreeSurface(source);
    }
}

#[test]
fn surface_blit_matches_upstream_face_reference_when_source_uses_default_blend_mode() {
    let _serial = testutils::serial_lock();

    unsafe {
        let face = SDLTest_ImageFace();
        let expected = SDLTest_ImageBlit();
        assert!(!face.is_null(), "{}", testutils::current_error());
        assert!(!expected.is_null(), "{}", testutils::current_error());

        let dest = SDL_CreateRGBSurfaceWithFormat(
            0,
            (*expected).w,
            (*expected).h,
            (*(*expected).format).BitsPerPixel as i32,
            (*(*expected).format).format,
        );
        assert!(!dest.is_null(), "{}", testutils::current_error());

        let black = SDL_MapRGBA((*dest).format, 0, 0, 0, 255);
        assert_eq!(SDL_FillRect(dest, ptr::null(), black), 0);
        assert_eq!(SDL_SetSurfaceAlphaMod(face, 255), 0);
        assert_eq!(SDL_SetSurfaceColorMod(face, 255, 255, 255), 0);
        assert_eq!(SDL_SetColorKey(face, 0, 0), 0);

        let ni = (*dest).w - (*face).w;
        let nj = (*dest).h - (*face).h;
        for y in (0..=nj).step_by(4) {
            for x in (0..=ni).step_by(4) {
                let mut rect = SDL_Rect {
                    x,
                    y,
                    w: (*face).w,
                    h: (*face).h,
                };
                assert_eq!(SDL_UpperBlit(face, ptr::null(), dest, &mut rect), 0);
            }
        }

        assert_eq!(SDLTest_CompareSurfaces(dest, expected, 0), 0);

        SDL_FreeSurface(dest);
        SDL_FreeSurface(expected);
        SDL_FreeSurface(face);
    }
}

#[test]
fn surface_convert_handles_argb2101010_to_index8_without_crashing() {
    let _serial = testutils::serial_lock();

    unsafe {
        let face = SDLTest_ImageFace();
        assert!(!face.is_null(), "{}", testutils::current_error());

        let argb2101010 =
            SDL_ConvertSurfaceFormat(face, SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB2101010, 0);
        assert!(!argb2101010.is_null(), "{}", testutils::current_error());

        let index8 =
            SDL_ConvertSurfaceFormat(argb2101010, SDL_PixelFormatEnum_SDL_PIXELFORMAT_INDEX8, 0);
        assert!(!index8.is_null(), "{}", testutils::current_error());

        SDL_FreeSurface(index8);
        SDL_FreeSurface(argb2101010);
        SDL_FreeSurface(face);
    }
}

#[test]
fn upper_blit_paths_allow_upstream_clipping_to_smaller_destinations() {
    let _serial = testutils::serial_lock();

    unsafe {
        let source = create_argb8888_surface(16, 16);
        let dest = create_argb8888_surface(8, 8);
        assert!(!source.is_null(), "{}", testutils::current_error());
        assert!(!dest.is_null(), "{}", testutils::current_error());

        let red = SDL_MapRGBA((*source).format, 255, 0, 0, 255);
        let black = SDL_MapRGBA((*dest).format, 0, 0, 0, 255);
        assert_eq!(SDL_FillRect(source, ptr::null(), red), 0);
        assert_eq!(SDL_FillRect(dest, ptr::null(), black), 0);

        let mut dstrect = SDL_Rect {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        };
        assert_eq!(SDL_UpperBlit(source, ptr::null(), dest, &mut dstrect), 0);
        assert_eq!((dstrect.x, dstrect.y, dstrect.w, dstrect.h), (0, 0, 8, 8));
        assert_eq!(read_pixel32(dest, 7, 7), red);

        assert_eq!(SDL_FillRect(dest, ptr::null(), black), 0);
        let mut scaled = SDL_Rect {
            x: 0,
            y: 0,
            w: 16,
            h: 16,
        };
        assert_eq!(
            SDL_UpperBlitScaled(source, ptr::null(), dest, &mut scaled),
            0
        );
        assert_eq!((scaled.x, scaled.y, scaled.w, scaled.h), (0, 0, 8, 8));
        assert_eq!(read_pixel32(dest, 7, 7), red);

        SDL_FreeSurface(dest);
        SDL_FreeSurface(source);
    }
}

#[test]
fn surface_convert_pixels_copies_expected_content() {
    let _serial = testutils::serial_lock();

    unsafe {
        let src_surface = create_argb8888_surface(2, 1);
        assert!(!src_surface.is_null(), "{}", testutils::current_error());
        let pixels = (*src_surface).pixels.cast::<Uint32>();
        *pixels = SDL_MapRGBA((*src_surface).format, 10, 20, 30, 40);
        *pixels.add(1) = SDL_MapRGBA((*src_surface).format, 50, 60, 70, 80);

        let mut dst = [0u8; 8];
        assert_eq!(
            SDL_ConvertPixels(
                2,
                1,
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888,
                (*src_surface).pixels,
                (*src_surface).pitch,
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBA8888,
                dst.as_mut_ptr().cast(),
                8,
            ),
            0,
            "{}",
            testutils::current_error()
        );

        assert_ne!(u32::from_ne_bytes(dst[..4].try_into().unwrap()), 0);
        SDL_FreeSurface(src_surface);
    }
}

#[test]
fn surface_premultiply_alpha_matches_argb8888_reference() {
    let _serial = testutils::serial_lock();

    unsafe {
        let src_surface = create_argb8888_surface(2, 1);
        assert!(!src_surface.is_null(), "{}", testutils::current_error());
        let pixels = (*src_surface).pixels.cast::<Uint32>();
        *pixels = SDL_MapRGBA((*src_surface).format, 100, 50, 25, 128);
        *pixels.add(1) = SDL_MapRGBA((*src_surface).format, 255, 64, 32, 64);

        let mut dst = [0u8; 8];
        assert_eq!(
            SDL_PremultiplyAlpha(
                2,
                1,
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888,
                (*src_surface).pixels,
                (*src_surface).pitch,
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888,
                dst.as_mut_ptr().cast(),
                8,
            ),
            0,
            "{}",
            testutils::current_error()
        );

        let mut r = 0;
        let mut g = 0;
        let mut b = 0;
        let mut a = 0;
        SDL_GetRGBA(
            u32::from_ne_bytes(dst[..4].try_into().unwrap()),
            (*src_surface).format,
            &mut r,
            &mut g,
            &mut b,
            &mut a,
        );
        assert_eq!((r, g, b, a), (50, 25, 12, 128));

        SDL_GetRGBA(
            u32::from_ne_bytes(dst[4..8].try_into().unwrap()),
            (*src_surface).format,
            &mut r,
            &mut g,
            &mut b,
            &mut a,
        );
        assert_eq!((r, g, b, a), (64, 16, 8, 64));

        SDL_FreeSurface(src_surface);
    }
}
