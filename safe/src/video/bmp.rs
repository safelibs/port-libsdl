use crate::abi::generated_types::{SDL_RWops, SDL_Surface};
use crate::core::error::set_error_message;
use crate::core::rwops::{
    SDL_RWclose, SDL_RWseek, SDL_RWsize, SDL_RWtell, SDL_ReadLE16, SDL_ReadLE32,
};
use crate::security::checked_math;
use crate::video::surface::{
    apply_math_error, clear_real_error, real_sdl, sync_error_from_real, validate_surface_storage,
};

const BI_RGB: u32 = 0;
const BI_BITFIELDS: u32 = 3;

unsafe fn restore_position(src: *mut SDL_RWops, position: i64) -> bool {
    SDL_RWseek(src, position, libc::SEEK_SET) == position
}

unsafe fn validate_bmp_stream(src: *mut SDL_RWops) -> Result<(), &'static str> {
    if src.is_null() {
        return Err("src");
    }

    let start = SDL_RWtell(src);
    if start < 0 {
        return Err("Error seeking in datastream");
    }

    let stream_size = SDL_RWsize(src);
    let magic = SDL_ReadLE16(src);
    if magic != 0x4D42 {
        let _ = restore_position(src, start);
        return Err("File is not a Windows BMP file");
    }

    let _file_size = SDL_ReadLE32(src);
    let _reserved1 = SDL_ReadLE16(src);
    let _reserved2 = SDL_ReadLE16(src);
    let pixel_offset = SDL_ReadLE32(src);
    let dib_size = SDL_ReadLE32(src);

    if dib_size < 12 {
        let _ = restore_position(src, start);
        return Err("Truncated BMP header");
    }

    if dib_size == 12 {
        let width = SDL_ReadLE16(src) as i32;
        let height = SDL_ReadLE16(src) as i32;
        let _planes = SDL_ReadLE16(src);
        let bits_per_pixel = SDL_ReadLE16(src);
        if checked_math::validate_bmp_dimensions(width, height, bits_per_pixel).is_err() {
            let _ = restore_position(src, start);
            return Err("Invalid BMP dimensions");
        }
    } else {
        let width = SDL_ReadLE32(src) as i32;
        let signed_height = SDL_ReadLE32(src) as i32;
        let _planes = SDL_ReadLE16(src);
        let bits_per_pixel = SDL_ReadLE16(src);
        let compression = SDL_ReadLE32(src);
        let image_size = SDL_ReadLE32(src) as usize;
        let _xppm = SDL_ReadLE32(src);
        let _yppm = SDL_ReadLE32(src);
        let _used = SDL_ReadLE32(src);
        let _important = SDL_ReadLE32(src);

        if width <= 0 || signed_height == i32::MIN {
            let _ = restore_position(src, start);
            return Err("Invalid BMP dimensions");
        }

        let height = signed_height.abs();
        if matches!(compression, BI_RGB | BI_BITFIELDS) {
            let (_, _, expected_size) =
                checked_math::validate_bmp_dimensions(width, height, bits_per_pixel).map_err(
                    |_| {
                        let _ = restore_position(src, start);
                        "Invalid BMP dimensions"
                    },
                )?;
            if image_size != 0 && image_size < expected_size {
                let _ = restore_position(src, start);
                return Err("Invalid BMP row data");
            }
        }
    }

    if stream_size >= 0 && (pixel_offset as i64) > stream_size {
        let _ = restore_position(src, start);
        return Err("Truncated BMP pixel data");
    }

    if !restore_position(src, start) {
        return Err("Error seeking in datastream");
    }
    Ok(())
}

fn finish_bmp_failure(
    src: *mut SDL_RWops,
    freesrc: libc::c_int,
    message: &str,
) -> *mut SDL_Surface {
    if freesrc != 0 && !src.is_null() {
        unsafe {
            let _ = SDL_RWclose(src);
        }
    }
    let _ = set_error_message(message);
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_LoadBMP_RW(
    src: *mut SDL_RWops,
    freesrc: libc::c_int,
) -> *mut SDL_Surface {
    match validate_bmp_stream(src) {
        Ok(()) => {}
        Err("src") => return finish_bmp_failure(src, freesrc, "Parameter 'src' is invalid"),
        Err(message) => return finish_bmp_failure(src, freesrc, message),
    }

    clear_real_error();
    let surface = (real_sdl().load_bmp_rw)(src, freesrc);
    if surface.is_null() {
        let _ = sync_error_from_real("Couldn't load BMP");
    }
    surface
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SaveBMP_RW(
    surface: *mut SDL_Surface,
    dst: *mut SDL_RWops,
    freedst: libc::c_int,
) -> libc::c_int {
    if dst.is_null() {
        return crate::core::error::invalid_param_error("dst");
    }
    if let Err(error) = validate_surface_storage(surface) {
        if freedst != 0 {
            let _ = SDL_RWclose(dst);
        }
        return apply_math_error(error);
    }

    clear_real_error();
    let result = (real_sdl().save_bmp_rw)(surface, dst, freedst);
    if result < 0 {
        let _ = sync_error_from_real("Couldn't save BMP");
    }
    result
}
