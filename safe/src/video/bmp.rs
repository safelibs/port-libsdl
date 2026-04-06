use crate::abi::generated_types::{
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888, SDL_RWops, SDL_Surface, Uint32,
};
use crate::core::error::{invalid_param_error, set_error_message};
use crate::core::rwops::{
    SDL_RWclose, SDL_RWread, SDL_RWseek, SDL_RWtell, SDL_RWwrite, SDL_WriteLE16, SDL_WriteLE32,
};
use crate::security::checked_math;
use crate::video::surface::{
    apply_math_error, create_owned_surface_with_format, validate_surface_storage,
};

const BI_RGB: u32 = 0;
const BI_BITFIELDS: u32 = 3;

fn finish_bmp_failure(
    stream: *mut SDL_RWops,
    free_stream: libc::c_int,
    message: &str,
) -> *mut SDL_Surface {
    if free_stream != 0 && !stream.is_null() {
        unsafe {
            let _ = SDL_RWclose(stream);
        }
    }
    let _ = set_error_message(message);
    std::ptr::null_mut()
}

fn close_on_result(
    stream: *mut SDL_RWops,
    free_stream: libc::c_int,
    result: libc::c_int,
) -> libc::c_int {
    if free_stream != 0 && !stream.is_null() {
        unsafe {
            let _ = SDL_RWclose(stream);
        }
    }
    result
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    bytes
        .get(offset..offset + 2)
        .map(|slice| u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    bytes
        .get(offset..offset + 4)
        .map(|slice| u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_i32(bytes: &[u8], offset: usize) -> Option<i32> {
    read_u32(bytes, offset).map(|value| value as i32)
}

unsafe fn read_remaining_stream(src: *mut SDL_RWops) -> Result<Vec<u8>, &'static str> {
    if src.is_null() {
        return Err("src");
    }

    let start = SDL_RWtell(src);
    if start < 0 {
        return Err("Error seeking in datastream");
    }
    let end = SDL_RWseek(src, 0, libc::SEEK_END);
    if end < start {
        let _ = SDL_RWseek(src, start, libc::SEEK_SET);
        return Err("Error seeking in datastream");
    }
    if SDL_RWseek(src, start, libc::SEEK_SET) != start {
        return Err("Error seeking in datastream");
    }

    let remaining = usize::try_from(end - start).map_err(|_| "Truncated BMP header")?;
    let mut bytes = vec![0u8; remaining];
    if remaining > 0 && SDL_RWread(src, bytes.as_mut_ptr().cast(), 1, remaining) != remaining {
        return Err("Truncated BMP pixel data");
    }
    Ok(bytes)
}

fn read_palette(
    bytes: &[u8],
    start: usize,
    entry_size: usize,
    count: usize,
) -> Result<Vec<(u8, u8, u8, u8)>, &'static str> {
    let total = checked_math::checked_mul_usize(count, entry_size, "BMP palette overflow")
        .map_err(|_| "Invalid BMP dimensions")?;
    let end = checked_math::checked_add_usize(start, total, "BMP palette overflow")
        .map_err(|_| "Invalid BMP dimensions")?;
    if end > bytes.len() {
        return Err("Truncated BMP header");
    }

    let mut palette = Vec::with_capacity(count);
    for index in 0..count {
        let offset = start + index * entry_size;
        if entry_size == 3 {
            palette.push((bytes[offset + 2], bytes[offset + 1], bytes[offset], 255));
        } else {
            palette.push((bytes[offset + 2], bytes[offset + 1], bytes[offset], 255));
        }
    }
    Ok(palette)
}

fn mask_component(value: u32, mask: u32) -> u8 {
    if mask == 0 {
        return 0;
    }
    let shift = mask.trailing_zeros();
    let max = mask >> shift;
    let component = (value & mask) >> shift;
    ((component * 255 + (max / 2)) / max) as u8
}

unsafe fn write_argb8888_pixel(
    surface: *mut SDL_Surface,
    x: usize,
    y: usize,
    rgba: (u8, u8, u8, u8),
) {
    let row = (*surface)
        .pixels
        .cast::<u8>()
        .add(y * (*surface).pitch as usize);
    let pixel =
        crate::video::pixels::SDL_MapRGBA((*surface).format, rgba.0, rgba.1, rgba.2, rgba.3);
    row.add(x * 4).cast::<Uint32>().write_unaligned(pixel);
}

unsafe fn decode_bmp(bytes: &[u8]) -> Result<*mut SDL_Surface, &'static str> {
    if bytes.len() < 14 {
        return Err("Truncated BMP header");
    }
    if &bytes[..2] != b"BM" {
        return Err("File is not a Windows BMP file");
    }

    let pixel_offset = read_u32(bytes, 10).ok_or("Truncated BMP header")? as usize;
    let dib_size = read_u32(bytes, 14).ok_or("Truncated BMP header")?;
    if dib_size < 12 {
        return Err("Truncated BMP header");
    }

    let (
        width,
        height,
        top_down,
        planes,
        bits_per_pixel,
        compression,
        colors_used,
        palette_entry_size,
        masks,
    ) = if dib_size == 12 {
        let width = read_u16(bytes, 18).ok_or("Truncated BMP header")? as i32;
        let height = read_u16(bytes, 20).ok_or("Truncated BMP header")? as i32;
        let planes = read_u16(bytes, 22).ok_or("Truncated BMP header")?;
        let bits_per_pixel = read_u16(bytes, 24).ok_or("Truncated BMP header")?;
        (
            width,
            height,
            false,
            planes,
            bits_per_pixel,
            BI_RGB,
            0,
            3usize,
            (0x00ff0000, 0x0000ff00, 0x000000ff, 0xff000000),
        )
    } else {
        let width = read_i32(bytes, 18).ok_or("Truncated BMP header")?;
        let signed_height = read_i32(bytes, 22).ok_or("Truncated BMP header")?;
        if signed_height == i32::MIN {
            return Err("Invalid BMP dimensions");
        }
        let planes = read_u16(bytes, 26).ok_or("Truncated BMP header")?;
        let bits_per_pixel = read_u16(bytes, 28).ok_or("Truncated BMP header")?;
        let compression = read_u32(bytes, 30).ok_or("Truncated BMP header")?;
        let colors_used = read_u32(bytes, 46).unwrap_or(0);
        let masks = if bits_per_pixel == 32 && compression == BI_BITFIELDS {
            if dib_size >= 56 {
                (
                    read_u32(bytes, 54).ok_or("Truncated BMP header")?,
                    read_u32(bytes, 58).ok_or("Truncated BMP header")?,
                    read_u32(bytes, 62).ok_or("Truncated BMP header")?,
                    read_u32(bytes, 66).unwrap_or(0),
                )
            } else {
                let mask_start =
                    checked_math::checked_add_usize(14, dib_size as usize, "BMP header overflow")
                        .map_err(|_| "Invalid BMP dimensions")?;
                (
                    read_u32(bytes, mask_start).ok_or("Truncated BMP header")?,
                    read_u32(bytes, mask_start + 4).ok_or("Truncated BMP header")?,
                    read_u32(bytes, mask_start + 8).ok_or("Truncated BMP header")?,
                    read_u32(bytes, mask_start + 12).unwrap_or(0),
                )
            }
        } else {
            (0x00ff0000, 0x0000ff00, 0x000000ff, 0xff000000)
        };

        (
            width,
            signed_height.abs(),
            signed_height < 0,
            planes,
            bits_per_pixel,
            compression,
            colors_used,
            4usize,
            masks,
        )
    };

    if width <= 0 || height < 0 {
        return Err("Invalid BMP dimensions");
    }
    if planes != 1 {
        return Err("Unsupported BMP format");
    }

    let (width_usize, height_usize, expected_image_size) =
        checked_math::validate_bmp_dimensions(width, height, bits_per_pixel)
            .map_err(|_| "Invalid BMP dimensions")?;
    let row_stride = checked_math::calculate_bmp_row_stride(width_usize, bits_per_pixel)
        .map_err(|_| "Invalid BMP dimensions")?;

    match bits_per_pixel {
        4 | 8 if compression != BI_RGB => return Err("Unsupported BMP format"),
        24 if compression != BI_RGB => return Err("Unsupported BMP format"),
        32 if compression != BI_RGB && compression != BI_BITFIELDS => {
            return Err("Unsupported BMP format")
        }
        4 | 8 | 24 | 32 => {}
        _ => return Err("Unsupported BMP format"),
    }

    let pixel_end = checked_math::checked_add_usize(
        pixel_offset,
        expected_image_size,
        "BMP image size overflow",
    )
    .map_err(|_| "Invalid BMP dimensions")?;
    if pixel_offset > bytes.len() || pixel_end > bytes.len() {
        return Err("Truncated BMP pixel data");
    }

    let palette_count = match bits_per_pixel {
        4 | 8 => {
            if colors_used == 0 {
                1usize << bits_per_pixel
            } else {
                usize::try_from(colors_used).map_err(|_| "Invalid BMP dimensions")?
            }
        }
        _ => 0,
    };
    let palette = if palette_count > 0 {
        let palette_start =
            checked_math::checked_add_usize(14, dib_size as usize, "BMP header overflow")
                .map_err(|_| "Invalid BMP dimensions")?;
        read_palette(bytes, palette_start, palette_entry_size, palette_count)?
    } else {
        Vec::new()
    };

    let surface = create_owned_surface_with_format(
        0,
        width,
        height,
        32,
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888,
    );
    if surface.is_null() {
        return Err("Couldn't create RGB surface");
    }

    for y in 0..height_usize {
        let src_y = if top_down { y } else { height_usize - 1 - y };
        let row_start = pixel_offset + src_y * row_stride;
        let row = &bytes[row_start..row_start + row_stride];

        for x in 0..width_usize {
            let rgba = match bits_per_pixel {
                4 => {
                    let byte = row[x / 2];
                    let index = if x % 2 == 0 { byte >> 4 } else { byte & 0x0f } as usize;
                    *palette.get(index).ok_or("Invalid BMP row data")?
                }
                8 => *palette.get(row[x] as usize).ok_or("Invalid BMP row data")?,
                24 => {
                    let offset = x * 3;
                    (row[offset + 2], row[offset + 1], row[offset], 255)
                }
                32 => {
                    let offset = x * 4;
                    let value = u32::from_le_bytes([
                        row[offset],
                        row[offset + 1],
                        row[offset + 2],
                        row[offset + 3],
                    ]);
                    (
                        mask_component(value, masks.0),
                        mask_component(value, masks.1),
                        mask_component(value, masks.2),
                        if masks.3 == 0 {
                            255
                        } else {
                            mask_component(value, masks.3)
                        },
                    )
                }
                _ => unreachable!(),
            };
            write_argb8888_pixel(surface, x, y, rgba);
        }
    }

    Ok(surface)
}

unsafe fn raw_pixel(ptr: *const u8, bytes_per_pixel: u8) -> Uint32 {
    match bytes_per_pixel {
        1 => ptr.read() as Uint32,
        2 => ptr.cast::<u16>().read_unaligned() as Uint32,
        3 => {
            let bytes = [ptr.read(), ptr.add(1).read(), ptr.add(2).read(), 0];
            Uint32::from_le_bytes(bytes)
        }
        4 => ptr.cast::<Uint32>().read_unaligned(),
        _ => 0,
    }
}

unsafe fn surface_rgba(
    surface: *mut SDL_Surface,
    bytes_per_pixel: u8,
    x: usize,
    y: usize,
) -> (u8, u8, u8, u8) {
    let row = (*surface)
        .pixels
        .cast::<u8>()
        .add(y * (*surface).pitch as usize);
    let pixel = raw_pixel(row.add(x * bytes_per_pixel as usize), bytes_per_pixel);
    let (mut r, mut g, mut b, mut a) = (0, 0, 0, 0);
    crate::video::pixels::SDL_GetRGBA(pixel, (*surface).format, &mut r, &mut g, &mut b, &mut a);
    (r, g, b, a)
}

fn write_all(dst: *mut SDL_RWops, bytes: &[u8]) -> Result<(), ()> {
    unsafe {
        if bytes.is_empty()
            || SDL_RWwrite(dst, bytes.as_ptr().cast(), 1, bytes.len()) == bytes.len()
        {
            Ok(())
        } else {
            Err(())
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_LoadBMP_RW(
    src: *mut SDL_RWops,
    freesrc: libc::c_int,
) -> *mut SDL_Surface {
    let bytes = match read_remaining_stream(src) {
        Ok(bytes) => bytes,
        Err("src") => return finish_bmp_failure(src, freesrc, "Parameter 'src' is invalid"),
        Err(message) => return finish_bmp_failure(src, freesrc, message),
    };

    let surface = match decode_bmp(&bytes) {
        Ok(surface) => surface,
        Err("Couldn't create RGB surface") => {
            if freesrc != 0 && !src.is_null() {
                let _ = SDL_RWclose(src);
            }
            return std::ptr::null_mut();
        }
        Err(message) => return finish_bmp_failure(src, freesrc, message),
    };

    if freesrc != 0 && !src.is_null() {
        let _ = SDL_RWclose(src);
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
        return invalid_param_error("dst");
    }

    let descriptor = match validate_surface_storage(surface) {
        Ok(descriptor) => descriptor,
        Err(error) => {
            if freedst != 0 {
                let _ = SDL_RWclose(dst);
            }
            return apply_math_error(error);
        }
    };

    let width = (*surface).w.max(0) as usize;
    let height = (*surface).h.max(0) as usize;
    let row_stride = match checked_math::calculate_bmp_row_stride(width, 24) {
        Ok(stride) => stride,
        Err(error) => {
            if freedst != 0 {
                let _ = SDL_RWclose(dst);
            }
            return apply_math_error(error);
        }
    };
    let image_size =
        match checked_math::checked_mul_usize(height, row_stride, "BMP image size overflow") {
            Ok(size) => size,
            Err(error) => {
                if freedst != 0 {
                    let _ = SDL_RWclose(dst);
                }
                return apply_math_error(error);
            }
        };
    let file_size =
        match checked_math::checked_add_usize(14 + 40, image_size, "BMP image size overflow") {
            Ok(size) => size,
            Err(error) => {
                if freedst != 0 {
                    let _ = SDL_RWclose(dst);
                }
                return apply_math_error(error);
            }
        };

    let ok = SDL_WriteLE16(dst, 0x4d42) == 1
        && SDL_WriteLE32(dst, file_size as u32) == 1
        && SDL_WriteLE16(dst, 0) == 1
        && SDL_WriteLE16(dst, 0) == 1
        && SDL_WriteLE32(dst, (14 + 40) as u32) == 1
        && SDL_WriteLE32(dst, 40) == 1
        && SDL_WriteLE32(dst, width as u32) == 1
        && SDL_WriteLE32(dst, height as u32) == 1
        && SDL_WriteLE16(dst, 1) == 1
        && SDL_WriteLE16(dst, 24) == 1
        && SDL_WriteLE32(dst, BI_RGB) == 1
        && SDL_WriteLE32(dst, image_size as u32) == 1
        && SDL_WriteLE32(dst, 0) == 1
        && SDL_WriteLE32(dst, 0) == 1
        && SDL_WriteLE32(dst, 0) == 1
        && SDL_WriteLE32(dst, 0) == 1;
    if !ok {
        let _ = set_error_message("Error writing to datastream");
        return close_on_result(dst, freedst, -1);
    }

    let mut row = vec![0u8; row_stride];
    for src_y in (0..height).rev() {
        row.fill(0);
        for x in 0..width {
            let rgba = surface_rgba(surface, descriptor.bytes_per_pixel, x, src_y);
            let offset = x * 3;
            row[offset] = rgba.2;
            row[offset + 1] = rgba.1;
            row[offset + 2] = rgba.0;
        }
        if write_all(dst, &row).is_err() {
            let _ = set_error_message("Error writing to datastream");
            return close_on_result(dst, freedst, -1);
        }
    }

    close_on_result(dst, freedst, 0)
}
