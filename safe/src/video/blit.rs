use std::sync::OnceLock;

use crate::abi::generated_types::{SDL_Rect, SDL_Surface, Uint32, SDL_YUV_CONVERSION_MODE};
use crate::core::error::{invalid_param_error, set_error_message};
use crate::security::checked_math::{self, MathError};
use crate::video::surface::{
    apply_math_error, blit_surface_pixels, clear_real_error, full_surface_rect, intersect_rects,
    is_registered_surface, real_sdl, scale_surface_pixels_nearest, sync_error_from_real,
    validate_surface_storage,
};

type SetYuvConversionModeFn = unsafe extern "C" fn(SDL_YUV_CONVERSION_MODE);
type GetYuvConversionModeFn = unsafe extern "C" fn() -> SDL_YUV_CONVERSION_MODE;
type GetYuvConversionModeForResolutionFn =
    unsafe extern "C" fn(libc::c_int, libc::c_int) -> SDL_YUV_CONVERSION_MODE;

fn set_yuv_conversion_mode_fn() -> SetYuvConversionModeFn {
    static FN: OnceLock<SetYuvConversionModeFn> = OnceLock::new();
    *FN.get_or_init(|| crate::video::load_symbol(b"SDL_SetYUVConversionMode\0"))
}

fn get_yuv_conversion_mode_fn() -> GetYuvConversionModeFn {
    static FN: OnceLock<GetYuvConversionModeFn> = OnceLock::new();
    *FN.get_or_init(|| crate::video::load_symbol(b"SDL_GetYUVConversionMode\0"))
}

fn get_yuv_conversion_mode_for_resolution_fn() -> GetYuvConversionModeForResolutionFn {
    static FN: OnceLock<GetYuvConversionModeForResolutionFn> = OnceLock::new();
    *FN.get_or_init(|| crate::video::load_symbol(b"SDL_GetYUVConversionModeForResolution\0"))
}

unsafe fn validate_blit_surface(surface: *mut SDL_Surface) -> Result<(), MathError> {
    let _ = validate_surface_storage(surface)?;
    Ok(())
}

unsafe fn ensure_blit_ready(
    src: *mut SDL_Surface,
    dst: *mut SDL_Surface,
) -> Result<(), libc::c_int> {
    if src.is_null() {
        return Err(invalid_param_error("src"));
    }
    if dst.is_null() {
        return Err(invalid_param_error("dst"));
    }
    if let Err(error) = validate_blit_surface(src) {
        return Err(apply_math_error(error));
    }
    if let Err(error) = validate_blit_surface(dst) {
        return Err(apply_math_error(error));
    }
    if (*src).locked != 0 || (*dst).locked != 0 {
        return Err(set_error_message("Surfaces must not be locked during blit"));
    }
    Ok(())
}

unsafe fn upper_blit_rects(
    src: *mut SDL_Surface,
    srcrect: *const SDL_Rect,
    dst: *mut SDL_Surface,
    dstrect: *mut SDL_Rect,
) -> Result<(SDL_Rect, SDL_Rect), libc::c_int> {
    let mut src_region = full_surface_rect(src);
    let mut dst_region = SDL_Rect {
        x: if dstrect.is_null() { 0 } else { (*dstrect).x },
        y: if dstrect.is_null() { 0 } else { (*dstrect).y },
        w: 0,
        h: 0,
    };

    if !srcrect.is_null() {
        match intersect_rects(&src_region, &*srcrect) {
            Some(clipped) => {
                dst_region.x += clipped.x - (*srcrect).x;
                dst_region.y += clipped.y - (*srcrect).y;
                src_region = clipped;
            }
            None => {
                if !dstrect.is_null() {
                    (*dstrect).w = 0;
                    (*dstrect).h = 0;
                }
                return Err(0);
            }
        }
    }

    dst_region.w = src_region.w;
    dst_region.h = src_region.h;

    match intersect_rects(&dst_region, &(*dst).clip_rect) {
        Some(clipped) => {
            src_region.x += clipped.x - dst_region.x;
            src_region.y += clipped.y - dst_region.y;
            src_region.w = clipped.w;
            src_region.h = clipped.h;
            dst_region = clipped;
        }
        None => {
            if !dstrect.is_null() {
                (*dstrect).w = 0;
                (*dstrect).h = 0;
            }
            return Err(0);
        }
    }

    if dst_region.w <= 0 || dst_region.h <= 0 {
        if !dstrect.is_null() {
            (*dstrect).w = 0;
            (*dstrect).h = 0;
        }
        return Err(0);
    }

    if !dstrect.is_null() {
        *dstrect = dst_region;
    }
    Ok((src_region, dst_region))
}

unsafe fn upper_blit_scaled_rects(
    src: *mut SDL_Surface,
    srcrect: *const SDL_Rect,
    dst: *mut SDL_Surface,
    dstrect: *mut SDL_Rect,
) -> Result<(SDL_Rect, SDL_Rect), libc::c_int> {
    let src_region = if srcrect.is_null() {
        full_surface_rect(src)
    } else {
        *srcrect
    };
    let dst_region = if dstrect.is_null() {
        full_surface_rect(dst)
    } else {
        *dstrect
    };

    if src_region.w <= 0 || src_region.h <= 0 || dst_region.w <= 0 || dst_region.h <= 0 {
        if !dstrect.is_null() {
            (*dstrect).w = 0;
            (*dstrect).h = 0;
        }
        return Err(0);
    }

    let src_bounds = full_surface_rect(src);
    let dst_clip = (*dst).clip_rect;

    let scale_x = dst_region.w as f64 / src_region.w as f64;
    let scale_y = dst_region.h as f64 / src_region.h as f64;

    let mut src_x0 = src_region.x as f64;
    let mut src_y0 = src_region.y as f64;
    let mut src_x1 = (src_region.x + src_region.w) as f64;
    let mut src_y1 = (src_region.y + src_region.h) as f64;
    let mut dst_x0 = dst_region.x as f64;
    let mut dst_y0 = dst_region.y as f64;
    let mut dst_x1 = (dst_region.x + dst_region.w) as f64;
    let mut dst_y1 = (dst_region.y + dst_region.h) as f64;

    if src_x0 < 0.0 {
        dst_x0 += (-src_x0) * scale_x;
        src_x0 = 0.0;
    }
    if src_y0 < 0.0 {
        dst_y0 += (-src_y0) * scale_y;
        src_y0 = 0.0;
    }
    if src_x1 > src_bounds.w as f64 {
        dst_x1 -= (src_x1 - src_bounds.w as f64) * scale_x;
        src_x1 = src_bounds.w as f64;
    }
    if src_y1 > src_bounds.h as f64 {
        dst_y1 -= (src_y1 - src_bounds.h as f64) * scale_y;
        src_y1 = src_bounds.h as f64;
    }

    if dst_x0 < dst_clip.x as f64 {
        src_x0 += (dst_clip.x as f64 - dst_x0) / scale_x;
        dst_x0 = dst_clip.x as f64;
    }
    if dst_y0 < dst_clip.y as f64 {
        src_y0 += (dst_clip.y as f64 - dst_y0) / scale_y;
        dst_y0 = dst_clip.y as f64;
    }
    if dst_x1 > (dst_clip.x + dst_clip.w) as f64 {
        src_x1 -= (dst_x1 - (dst_clip.x + dst_clip.w) as f64) / scale_x;
        dst_x1 = (dst_clip.x + dst_clip.w) as f64;
    }
    if dst_y1 > (dst_clip.y + dst_clip.h) as f64 {
        src_y1 -= (dst_y1 - (dst_clip.y + dst_clip.h) as f64) / scale_y;
        dst_y1 = (dst_clip.y + dst_clip.h) as f64;
    }

    let src_final = SDL_Rect {
        x: src_x0.round() as libc::c_int,
        y: src_y0.round() as libc::c_int,
        w: (src_x1 - src_x0).round() as libc::c_int,
        h: (src_y1 - src_y0).round() as libc::c_int,
    };
    let dst_final = SDL_Rect {
        x: dst_x0.round() as libc::c_int,
        y: dst_y0.round() as libc::c_int,
        w: (dst_x1 - dst_x0).round() as libc::c_int,
        h: (dst_y1 - dst_y0).round() as libc::c_int,
    };

    if src_final.w <= 0 || src_final.h <= 0 || dst_final.w <= 0 || dst_final.h <= 0 {
        if !dstrect.is_null() {
            (*dstrect).w = 0;
            (*dstrect).h = 0;
        }
        return Err(0);
    }

    if !dstrect.is_null() {
        *dstrect = dst_final;
    }
    Ok((src_final, dst_final))
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ConvertPixels(
    width: libc::c_int,
    height: libc::c_int,
    src_format: Uint32,
    src: *const libc::c_void,
    src_pitch: libc::c_int,
    dst_format: Uint32,
    dst: *mut libc::c_void,
    dst_pitch: libc::c_int,
) -> libc::c_int {
    if src.is_null() {
        return invalid_param_error("src");
    }
    if dst.is_null() {
        return invalid_param_error("dst");
    }
    if let Some(descriptor) = crate::video::surface::format_descriptor(src_format) {
        if let Err(error) = checked_math::validate_copy_layout(
            width,
            height,
            descriptor.bits_per_pixel,
            descriptor.bytes_per_pixel,
            src_pitch,
        ) {
            return apply_math_error(error);
        }
    }
    if let Some(descriptor) = crate::video::surface::format_descriptor(dst_format) {
        if let Err(error) = checked_math::validate_copy_layout(
            width,
            height,
            descriptor.bits_per_pixel,
            descriptor.bytes_per_pixel,
            dst_pitch,
        ) {
            return apply_math_error(error);
        }
    }

    clear_real_error();
    let result = (real_sdl().convert_pixels)(
        width, height, src_format, src, src_pitch, dst_format, dst, dst_pitch,
    );
    if result < 0 {
        let _ = sync_error_from_real("Couldn't convert pixels");
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetYUVConversionMode(mode: SDL_YUV_CONVERSION_MODE) {
    crate::video::clear_real_error();
    set_yuv_conversion_mode_fn()(mode);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetYUVConversionMode() -> SDL_YUV_CONVERSION_MODE {
    crate::video::clear_real_error();
    get_yuv_conversion_mode_fn()()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetYUVConversionModeForResolution(
    width: libc::c_int,
    height: libc::c_int,
) -> SDL_YUV_CONVERSION_MODE {
    crate::video::clear_real_error();
    get_yuv_conversion_mode_for_resolution_fn()(width, height)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_PremultiplyAlpha(
    width: libc::c_int,
    height: libc::c_int,
    src_format: Uint32,
    src: *const libc::c_void,
    src_pitch: libc::c_int,
    dst_format: Uint32,
    dst: *mut libc::c_void,
    dst_pitch: libc::c_int,
) -> libc::c_int {
    if src.is_null() {
        return invalid_param_error("src");
    }
    if dst.is_null() {
        return invalid_param_error("dst");
    }
    if let Some(descriptor) = crate::video::surface::format_descriptor(src_format) {
        if let Err(error) = checked_math::validate_copy_layout(
            width,
            height,
            descriptor.bits_per_pixel,
            descriptor.bytes_per_pixel,
            src_pitch,
        ) {
            return apply_math_error(error);
        }
    }
    if let Some(descriptor) = crate::video::surface::format_descriptor(dst_format) {
        if let Err(error) = checked_math::validate_copy_layout(
            width,
            height,
            descriptor.bits_per_pixel,
            descriptor.bytes_per_pixel,
            dst_pitch,
        ) {
            return apply_math_error(error);
        }
    }

    clear_real_error();
    let result = (real_sdl().premultiply_alpha)(
        width, height, src_format, src, src_pitch, dst_format, dst, dst_pitch,
    );
    if result < 0 {
        let _ = sync_error_from_real("Couldn't premultiply alpha");
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn SDL_UpperBlit(
    src: *mut SDL_Surface,
    srcrect: *const SDL_Rect,
    dst: *mut SDL_Surface,
    dstrect: *mut SDL_Rect,
) -> libc::c_int {
    if let Err(code) = ensure_blit_ready(src, dst) {
        return code;
    }

    let (src_region, dst_region) = match upper_blit_rects(src, srcrect, dst, dstrect) {
        Ok(rects) => rects,
        Err(code) => return code,
    };

    if !(is_registered_surface(src) && is_registered_surface(dst)) {
        clear_real_error();
        let result = (real_sdl().upper_blit)(src, srcrect, dst, dstrect);
        if result < 0 {
            let _ = sync_error_from_real("Couldn't blit surface");
        }
        return result;
    }

    match blit_surface_pixels(src, &src_region, dst, &dst_region) {
        Ok(()) => 0,
        Err(error) => apply_math_error(error),
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_LowerBlit(
    src: *mut SDL_Surface,
    srcrect: *mut SDL_Rect,
    dst: *mut SDL_Surface,
    dstrect: *mut SDL_Rect,
) -> libc::c_int {
    if let Err(code) = ensure_blit_ready(src, dst) {
        return code;
    }
    if srcrect.is_null() {
        return invalid_param_error("srcrect");
    }
    if dstrect.is_null() {
        return invalid_param_error("dstrect");
    }

    if !(is_registered_surface(src) && is_registered_surface(dst)) {
        clear_real_error();
        let result = (real_sdl().lower_blit)(src, srcrect, dst, dstrect);
        if result < 0 {
            let _ = sync_error_from_real("Couldn't blit surface");
        }
        return result;
    }

    match blit_surface_pixels(src, &*srcrect, dst, &*dstrect) {
        Ok(()) => 0,
        Err(error) => apply_math_error(error),
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SoftStretch(
    src: *mut SDL_Surface,
    srcrect: *const SDL_Rect,
    dst: *mut SDL_Surface,
    dstrect: *const SDL_Rect,
) -> libc::c_int {
    if let Err(code) = ensure_blit_ready(src, dst) {
        return code;
    }

    let src_region = if srcrect.is_null() {
        full_surface_rect(src)
    } else {
        *srcrect
    };
    let dst_region = if dstrect.is_null() {
        full_surface_rect(dst)
    } else {
        *dstrect
    };

    if !(is_registered_surface(src) && is_registered_surface(dst)) {
        clear_real_error();
        let result = (real_sdl().soft_stretch)(src, srcrect, dst, dstrect);
        if result < 0 {
            let _ = sync_error_from_real("Couldn't stretch surface");
        }
        return result;
    }

    match scale_surface_pixels_nearest(src, &src_region, dst, &dst_region) {
        Ok(()) => 0,
        Err(error) => apply_math_error(error),
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SoftStretchLinear(
    src: *mut SDL_Surface,
    srcrect: *const SDL_Rect,
    dst: *mut SDL_Surface,
    dstrect: *const SDL_Rect,
) -> libc::c_int {
    SDL_SoftStretch(src, srcrect, dst, dstrect)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_UpperBlitScaled(
    src: *mut SDL_Surface,
    srcrect: *const SDL_Rect,
    dst: *mut SDL_Surface,
    dstrect: *mut SDL_Rect,
) -> libc::c_int {
    if let Err(code) = ensure_blit_ready(src, dst) {
        return code;
    }

    let (src_region, dst_region) = match upper_blit_scaled_rects(src, srcrect, dst, dstrect) {
        Ok(rects) => rects,
        Err(code) => return code,
    };

    if !(is_registered_surface(src) && is_registered_surface(dst)) {
        clear_real_error();
        let result = (real_sdl().upper_blit_scaled)(src, srcrect, dst, dstrect);
        if result < 0 {
            let _ = sync_error_from_real("Couldn't scale-blit surface");
        }
        return result;
    }

    match scale_surface_pixels_nearest(src, &src_region, dst, &dst_region) {
        Ok(()) => 0,
        Err(error) => apply_math_error(error),
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_LowerBlitScaled(
    src: *mut SDL_Surface,
    srcrect: *mut SDL_Rect,
    dst: *mut SDL_Surface,
    dstrect: *mut SDL_Rect,
) -> libc::c_int {
    if let Err(code) = ensure_blit_ready(src, dst) {
        return code;
    }
    if srcrect.is_null() {
        return invalid_param_error("srcrect");
    }
    if dstrect.is_null() {
        return invalid_param_error("dstrect");
    }

    if !(is_registered_surface(src) && is_registered_surface(dst)) {
        clear_real_error();
        let result = (real_sdl().lower_blit_scaled)(src, srcrect, dst, dstrect);
        if result < 0 {
            let _ = sync_error_from_real("Couldn't scale-blit surface");
        }
        return result;
    }

    match scale_surface_pixels_nearest(src, &*srcrect, dst, &*dstrect) {
        Ok(()) => 0,
        Err(error) => apply_math_error(error),
    }
}
