use crate::abi::generated_types::{SDL_Rect, SDL_Surface, Uint32};
use crate::security::checked_math::{self, MathError};
use crate::video::surface::{
    apply_math_error, format_descriptor, real_sdl, sync_error_from_real, validate_surface_storage,
};

unsafe fn validate_blit_surface(surface: *mut SDL_Surface) -> Result<(), MathError> {
    let _ = validate_surface_storage(surface)?;
    Ok(())
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
        return crate::core::error::invalid_param_error("src");
    }
    if dst.is_null() {
        return crate::core::error::invalid_param_error("dst");
    }
    if let Some(descriptor) = format_descriptor(src_format) {
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
    if let Some(descriptor) = format_descriptor(dst_format) {
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

    crate::video::surface::clear_real_error();
    let result = (real_sdl().convert_pixels)(
        width, height, src_format, src, src_pitch, dst_format, dst, dst_pitch,
    );
    if result < 0 {
        let _ = sync_error_from_real("Couldn't convert pixels");
    }
    result
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
        return crate::core::error::invalid_param_error("src");
    }
    if dst.is_null() {
        return crate::core::error::invalid_param_error("dst");
    }
    if let Some(descriptor) = format_descriptor(src_format) {
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
    if let Some(descriptor) = format_descriptor(dst_format) {
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

    crate::video::surface::clear_real_error();
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
    if src.is_null() {
        return crate::core::error::invalid_param_error("src");
    }
    if dst.is_null() {
        return crate::core::error::invalid_param_error("dst");
    }
    if let Err(error) = validate_blit_surface(src) {
        return apply_math_error(error);
    }
    if let Err(error) = validate_blit_surface(dst) {
        return apply_math_error(error);
    }

    crate::video::surface::clear_real_error();
    let result = (real_sdl().upper_blit)(src, srcrect, dst, dstrect);
    if result < 0 {
        let _ = sync_error_from_real("Couldn't blit surface");
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn SDL_LowerBlit(
    src: *mut SDL_Surface,
    srcrect: *mut SDL_Rect,
    dst: *mut SDL_Surface,
    dstrect: *mut SDL_Rect,
) -> libc::c_int {
    if src.is_null() {
        return crate::core::error::invalid_param_error("src");
    }
    if dst.is_null() {
        return crate::core::error::invalid_param_error("dst");
    }
    if let Err(error) = validate_blit_surface(src) {
        return apply_math_error(error);
    }
    if let Err(error) = validate_blit_surface(dst) {
        return apply_math_error(error);
    }

    crate::video::surface::clear_real_error();
    let result = (real_sdl().lower_blit)(src, srcrect, dst, dstrect);
    if result < 0 {
        let _ = sync_error_from_real("Couldn't blit surface");
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SoftStretch(
    src: *mut SDL_Surface,
    srcrect: *const SDL_Rect,
    dst: *mut SDL_Surface,
    dstrect: *const SDL_Rect,
) -> libc::c_int {
    if src.is_null() {
        return crate::core::error::invalid_param_error("src");
    }
    if dst.is_null() {
        return crate::core::error::invalid_param_error("dst");
    }
    if let Err(error) = validate_blit_surface(src) {
        return apply_math_error(error);
    }
    if let Err(error) = validate_blit_surface(dst) {
        return apply_math_error(error);
    }

    crate::video::surface::clear_real_error();
    let result = (real_sdl().soft_stretch)(src, srcrect, dst, dstrect);
    if result < 0 {
        let _ = sync_error_from_real("Couldn't stretch surface");
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SoftStretchLinear(
    src: *mut SDL_Surface,
    srcrect: *const SDL_Rect,
    dst: *mut SDL_Surface,
    dstrect: *const SDL_Rect,
) -> libc::c_int {
    if src.is_null() {
        return crate::core::error::invalid_param_error("src");
    }
    if dst.is_null() {
        return crate::core::error::invalid_param_error("dst");
    }
    if let Err(error) = validate_blit_surface(src) {
        return apply_math_error(error);
    }
    if let Err(error) = validate_blit_surface(dst) {
        return apply_math_error(error);
    }

    crate::video::surface::clear_real_error();
    let result = (real_sdl().soft_stretch_linear)(src, srcrect, dst, dstrect);
    if result < 0 {
        let _ = sync_error_from_real("Couldn't stretch surface");
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn SDL_UpperBlitScaled(
    src: *mut SDL_Surface,
    srcrect: *const SDL_Rect,
    dst: *mut SDL_Surface,
    dstrect: *mut SDL_Rect,
) -> libc::c_int {
    if src.is_null() {
        return crate::core::error::invalid_param_error("src");
    }
    if dst.is_null() {
        return crate::core::error::invalid_param_error("dst");
    }
    if let Err(error) = validate_blit_surface(src) {
        return apply_math_error(error);
    }
    if let Err(error) = validate_blit_surface(dst) {
        return apply_math_error(error);
    }

    crate::video::surface::clear_real_error();
    let result = (real_sdl().upper_blit_scaled)(src, srcrect, dst, dstrect);
    if result < 0 {
        let _ = sync_error_from_real("Couldn't scale-blit surface");
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn SDL_LowerBlitScaled(
    src: *mut SDL_Surface,
    srcrect: *mut SDL_Rect,
    dst: *mut SDL_Surface,
    dstrect: *mut SDL_Rect,
) -> libc::c_int {
    if src.is_null() {
        return crate::core::error::invalid_param_error("src");
    }
    if dst.is_null() {
        return crate::core::error::invalid_param_error("dst");
    }
    if let Err(error) = validate_blit_surface(src) {
        return apply_math_error(error);
    }
    if let Err(error) = validate_blit_surface(dst) {
        return apply_math_error(error);
    }

    crate::video::surface::clear_real_error();
    let result = (real_sdl().lower_blit_scaled)(src, srcrect, dst, dstrect);
    if result < 0 {
        let _ = sync_error_from_real("Couldn't scale-blit surface");
    }
    result
}
