#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathError {
    NegativeParam(&'static str),
    InvalidParam(&'static str),
    Overflow(&'static str),
}

pub type MathResult<T> = Result<T, MathError>;

pub fn nonnegative_to_usize(name: &'static str, value: libc::c_int) -> MathResult<usize> {
    usize::try_from(value).map_err(|_| MathError::NegativeParam(name))
}

pub fn checked_add_usize(lhs: usize, rhs: usize, message: &'static str) -> MathResult<usize> {
    lhs.checked_add(rhs).ok_or(MathError::Overflow(message))
}

pub fn checked_mul_usize(lhs: usize, rhs: usize, message: &'static str) -> MathResult<usize> {
    lhs.checked_mul(rhs).ok_or(MathError::Overflow(message))
}

pub fn calculate_buffer_offset(
    row: usize,
    pitch: usize,
    column_bytes: usize,
    message: &'static str,
) -> MathResult<usize> {
    let row_offset = checked_mul_usize(row, pitch, message)?;
    checked_add_usize(row_offset, column_bytes, message)
}

pub fn calculate_pitch(
    width: libc::c_int,
    bits_per_pixel: u8,
    bytes_per_pixel: u8,
    minimal: bool,
) -> MathResult<usize> {
    let width = nonnegative_to_usize("width", width)?;
    let mut pitch = if bits_per_pixel >= 8 {
        if bytes_per_pixel == 0 {
            return Err(MathError::InvalidParam("format"));
        }
        checked_mul_usize(width, bytes_per_pixel as usize, "surface pitch overflow")?
    } else {
        if bits_per_pixel == 0 {
            return Err(MathError::InvalidParam("format"));
        }
        let bits = checked_mul_usize(width, bits_per_pixel as usize, "surface pitch overflow")?;
        checked_add_usize(bits, 7, "surface pitch overflow")? / 8
    };

    if !minimal {
        pitch = checked_add_usize(pitch, 3, "surface pitch overflow")? & !3usize;
    }
    Ok(pitch)
}

pub fn calculate_surface_allocation(
    width: libc::c_int,
    height: libc::c_int,
    bits_per_pixel: u8,
    bytes_per_pixel: u8,
) -> MathResult<(usize, usize)> {
    let height = nonnegative_to_usize("height", height)?;
    let pitch = calculate_pitch(width, bits_per_pixel, bytes_per_pixel, false)?;
    let size = checked_mul_usize(height, pitch, "surface allocation overflow")?;
    Ok((pitch, size))
}

pub fn validate_preallocated_surface(
    width: libc::c_int,
    height: libc::c_int,
    pitch: libc::c_int,
    bits_per_pixel: u8,
    bytes_per_pixel: u8,
) -> MathResult<usize> {
    let height = nonnegative_to_usize("height", height)?;
    let pitch = usize::try_from(pitch).map_err(|_| MathError::InvalidParam("pitch"))?;
    let minimal_pitch = calculate_pitch(width, bits_per_pixel, bytes_per_pixel, true)?;
    if pitch > 0 && pitch < minimal_pitch {
        return Err(MathError::InvalidParam("pitch"));
    }
    checked_mul_usize(height, pitch, "surface allocation overflow")
}

pub fn validate_surface_layout(
    width: libc::c_int,
    height: libc::c_int,
    pitch: libc::c_int,
    bits_per_pixel: u8,
    bytes_per_pixel: u8,
) -> MathResult<usize> {
    let height = nonnegative_to_usize("height", height)?;
    let pitch = usize::try_from(pitch).map_err(|_| MathError::InvalidParam("pitch"))?;
    let minimal_pitch = calculate_pitch(width, bits_per_pixel, bytes_per_pixel, true)?;
    if width > 0 && pitch < minimal_pitch {
        return Err(MathError::InvalidParam("pitch"));
    }
    checked_mul_usize(height, pitch, "surface allocation overflow")
}

pub fn validate_copy_layout(
    width: libc::c_int,
    height: libc::c_int,
    bits_per_pixel: u8,
    bytes_per_pixel: u8,
    pitch: libc::c_int,
) -> MathResult<(usize, usize)> {
    let height = nonnegative_to_usize("height", height)?;
    let row_bytes = calculate_pitch(width, bits_per_pixel, bytes_per_pixel, true)?;
    let pitch = usize::try_from(pitch).map_err(|_| MathError::InvalidParam("pitch"))?;
    if width > 0 && pitch < row_bytes {
        return Err(MathError::InvalidParam("pitch"));
    }
    let total = checked_mul_usize(height, row_bytes, "blit copy length overflow")?;
    Ok((row_bytes, total))
}

pub fn calculate_bmp_row_stride(width: usize, bits_per_pixel: u16) -> MathResult<usize> {
    if bits_per_pixel == 0 {
        return Err(MathError::InvalidParam("bits_per_pixel"));
    }
    let row_bits = checked_mul_usize(width, bits_per_pixel as usize, "BMP row stride overflow")?;
    let aligned_bits = checked_add_usize(row_bits, 31, "BMP row stride overflow")?;
    let dwords = aligned_bits / 32;
    checked_mul_usize(dwords, 4, "BMP row stride overflow")
}

pub fn validate_bmp_dimensions(
    width: libc::c_int,
    height: libc::c_int,
    bits_per_pixel: u16,
) -> MathResult<(usize, usize, usize)> {
    let width = nonnegative_to_usize("width", width)?;
    let height = nonnegative_to_usize("height", height)?;
    let row_stride = calculate_bmp_row_stride(width, bits_per_pixel)?;
    let image_size = checked_mul_usize(height, row_stride, "BMP image size overflow")?;
    Ok((width, height, image_size))
}
