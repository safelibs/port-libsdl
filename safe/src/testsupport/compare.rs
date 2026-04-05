use std::sync::atomic::{AtomicI32, Ordering};

use crate::abi::generated_types::{self as sdl, SDL_Surface, Uint32, Uint8};

static COMPARE_SURFACE_COUNT: AtomicI32 = AtomicI32::new(0);

unsafe fn get_pixel(p: *const Uint8, bytes_per_pixel: usize) -> Uint32 {
    let mut value = 0u32;
    for index in 0..bytes_per_pixel {
        let byte = *p.add(index) as u32;
        #[cfg(target_endian = "big")]
        {
            let shift = ((std::mem::size_of::<u32>() - bytes_per_pixel + index) * 8) as u32;
            value |= byte << shift;
        }
        #[cfg(target_endian = "little")]
        {
            value |= byte << (index * 8);
        }
    }
    value
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_CompareSurfaces(
    surface: *mut SDL_Surface,
    referenceSurface: *mut SDL_Surface,
    mut allowable_error: libc::c_int,
) -> libc::c_int {
    if surface.is_null() || referenceSurface.is_null() {
        return -1;
    }
    if (*surface).w != (*referenceSurface).w || (*surface).h != (*referenceSurface).h {
        return -2;
    }
    if allowable_error < 0 {
        allowable_error = 0;
    }

    sdl::SDL_LockSurface(surface);
    sdl::SDL_LockSurface(referenceSurface);

    let mut failures = 0;
    let mut sample = (0, 0, 0);
    let bpp = (*(*surface).format).BytesPerPixel as usize;
    let ref_bpp = (*(*referenceSurface).format).BytesPerPixel as usize;
    for y in 0..(*surface).h {
        for x in 0..(*surface).w {
            let pixel_ptr = (*surface)
                .pixels
                .cast::<Uint8>()
                .add((y * (*surface).pitch + x * bpp as i32) as usize);
            let ref_ptr = (*referenceSurface)
                .pixels
                .cast::<Uint8>()
                .add((y * (*referenceSurface).pitch + x * ref_bpp as i32) as usize);
            let pixel = get_pixel(pixel_ptr, bpp);
            let reference = get_pixel(ref_ptr, ref_bpp);
            let mut r = 0u8;
            let mut g = 0u8;
            let mut b = 0u8;
            let mut a = 0u8;
            let mut rr = 0u8;
            let mut rg = 0u8;
            let mut rb = 0u8;
            let mut ra = 0u8;
            sdl::SDL_GetRGBA(pixel, (*surface).format, &mut r, &mut g, &mut b, &mut a);
            sdl::SDL_GetRGBA(
                reference,
                (*referenceSurface).format,
                &mut rr,
                &mut rg,
                &mut rb,
                &mut ra,
            );
            let dist = (r as i32 - rr as i32).pow(2)
                + (g as i32 - rg as i32).pow(2)
                + (b as i32 - rb as i32).pow(2);
            if dist > allowable_error {
                failures += 1;
                if failures == 1 {
                    sample = (x, y, dist);
                }
            }
        }
    }

    sdl::SDL_UnlockSurface(surface);
    sdl::SDL_UnlockSurface(referenceSurface);

    COMPARE_SURFACE_COUNT.fetch_add(1, Ordering::Relaxed);
    if failures != 0 {
        let image_filename = format!(
            "CompareSurfaces{:04}_TestOutput.bmp",
            COMPARE_SURFACE_COUNT.load(Ordering::Relaxed)
        );
        let reference_filename = format!(
            "CompareSurfaces{:04}_Reference.bmp",
            COMPARE_SURFACE_COUNT.load(Ordering::Relaxed)
        );
        let message = std::ffi::CString::new(format!(
            "Comparison of pixels with allowable error of {allowable_error} failed {failures} times."
        ))
        .unwrap();
        crate::testsupport::log::SDLTest_LogErrorFromBuffer(message.as_ptr());
        let message = std::ffi::CString::new(format!(
            "First detected occurrence at position {},{} with a squared RGB-difference of {}.",
            sample.0, sample.1, sample.2
        ))
        .unwrap();
        crate::testsupport::log::SDLTest_LogErrorFromBuffer(message.as_ptr());
        crate::testsupport::with_c_buffer(&image_filename, |filename| {
            crate::testsupport::with_c_buffer("wb", |mode| unsafe {
                let rw = sdl::SDL_RWFromFile(filename, mode);
                if !rw.is_null() {
                    sdl::SDL_SaveBMP_RW(surface, rw, 1);
                }
            });
        });
        crate::testsupport::with_c_buffer(&reference_filename, |reference_name| {
            crate::testsupport::with_c_buffer("wb", |mode| unsafe {
                let rw = sdl::SDL_RWFromFile(reference_name, mode);
                if !rw.is_null() {
                    sdl::SDL_SaveBMP_RW(referenceSurface, rw, 1);
                }
            });
        });
        let message = std::ffi::CString::new(format!(
            "Surfaces from failed comparison saved as '{}' and '{}'",
            image_filename, reference_filename
        ))
        .unwrap();
        crate::testsupport::log::SDLTest_LogErrorFromBuffer(message.as_ptr());
    }

    failures
}
