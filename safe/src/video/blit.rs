use std::sync::OnceLock;

use crate::abi::generated_types::{
    SDL_PixelFormat, SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_IYUV, SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV12,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV21, SDL_PixelFormatEnum_SDL_PIXELFORMAT_UYVY,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_YUY2, SDL_PixelFormatEnum_SDL_PIXELFORMAT_YV12,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_YVYU, SDL_Rect, SDL_Surface, Uint32,
    SDL_YUV_CONVERSION_MODE, SDL_YUV_CONVERSION_MODE_SDL_YUV_CONVERSION_AUTOMATIC,
    SDL_YUV_CONVERSION_MODE_SDL_YUV_CONVERSION_BT601,
    SDL_YUV_CONVERSION_MODE_SDL_YUV_CONVERSION_BT709,
    SDL_YUV_CONVERSION_MODE_SDL_YUV_CONVERSION_JPEG,
};
use crate::core::error::{invalid_param_error, out_of_memory_error, set_error_message};
use crate::security::checked_math::{self, MathError};
use crate::video::surface::{
    apply_math_error, blit_surface_pixels, clear_real_error, format_descriptor, full_surface_rect,
    intersect_rects, is_registered_surface, real_sdl, scale_surface_pixels_nearest,
    sync_error_from_real, validate_surface_storage, FormatDescriptor,
};

type SetYuvConversionModeFn = unsafe extern "C" fn(SDL_YUV_CONVERSION_MODE);
type GetYuvConversionModeFn = unsafe extern "C" fn() -> SDL_YUV_CONVERSION_MODE;
type GetYuvConversionModeForResolutionFn =
    unsafe extern "C" fn(libc::c_int, libc::c_int) -> SDL_YUV_CONVERSION_MODE;

struct AllocatedFormat {
    raw: *mut SDL_PixelFormat,
    descriptor: FormatDescriptor,
}

impl AllocatedFormat {
    unsafe fn new(pixel_format: Uint32) -> Result<Self, libc::c_int> {
        let raw = crate::video::pixels::SDL_AllocFormat(pixel_format);
        if raw.is_null() {
            return Err(set_error_message("Couldn't allocate pixel format"));
        }

        Ok(Self {
            raw,
            descriptor: FormatDescriptor {
                bits_per_pixel: (*raw).BitsPerPixel,
                bytes_per_pixel: (*raw).BytesPerPixel,
            },
        })
    }
}

impl Drop for AllocatedFormat {
    fn drop(&mut self) {
        unsafe {
            if !self.raw.is_null() {
                crate::video::pixels::SDL_FreeFormat(self.raw);
            }
        }
    }
}

#[derive(Clone, Copy)]
struct YuvLayout {
    y_pitch: usize,
    chroma_pitch: usize,
    plane_size: usize,
    chroma_plane_size: usize,
    total_size: usize,
}

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

fn is_yuv_format(format: Uint32) -> bool {
    matches!(
        format,
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_YV12
            | SDL_PixelFormatEnum_SDL_PIXELFORMAT_IYUV
            | SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV12
            | SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV21
            | SDL_PixelFormatEnum_SDL_PIXELFORMAT_YUY2
            | SDL_PixelFormatEnum_SDL_PIXELFORMAT_UYVY
            | SDL_PixelFormatEnum_SDL_PIXELFORMAT_YVYU
    )
}

fn clip_byte(value: f32) -> u8 {
    let value = value.clamp(0.0, 255.0);
    (value + 0.5).floor() as u8
}

fn clip_unit_interval(value: f32) -> f32 {
    value.clamp(0.0, 255.0)
}

fn effective_yuv_mode(width: libc::c_int, height: libc::c_int) -> SDL_YUV_CONVERSION_MODE {
    let mode = unsafe { get_yuv_conversion_mode_for_resolution_fn()(width, height) };
    if mode == SDL_YUV_CONVERSION_MODE_SDL_YUV_CONVERSION_AUTOMATIC {
        if height <= 576 {
            SDL_YUV_CONVERSION_MODE_SDL_YUV_CONVERSION_BT601
        } else {
            SDL_YUV_CONVERSION_MODE_SDL_YUV_CONVERSION_BT709
        }
    } else {
        mode
    }
}

fn rgb_to_yuv(rgb: [u8; 3], mode: SDL_YUV_CONVERSION_MODE) -> [u8; 3] {
    if mode == SDL_YUV_CONVERSION_MODE_SDL_YUV_CONVERSION_JPEG {
        let y = 0.299 * rgb[0] as f32 + 0.587 * rgb[1] as f32 + 0.114 * rgb[2] as f32;
        let u = ((rgb[2] as f32 - y) * 0.565 + 128.0).floor();
        let v = ((rgb[0] as f32 - y) * 0.713 + 128.0).floor();
        [y.floor() as u8, u as u8, v as u8]
    } else {
        let (kr, kb) = if mode == SDL_YUV_CONVERSION_MODE_SDL_YUV_CONVERSION_BT709 {
            (0.2126, 0.0722)
        } else {
            (0.299, 0.114)
        };
        let r = rgb[0] as f32;
        let g = rgb[1] as f32;
        let b = rgb[2] as f32;
        let luma = kr * r + kb * b + (1.0 - kr - kb) * g;
        let y = (219.0 * luma / 255.0 + 16.0 + 0.5).floor();
        let u =
            clip_unit_interval((112.0 * (b - luma) / ((1.0 - kb) * 255.0) + 128.0 + 0.5).floor());
        let v =
            clip_unit_interval((112.0 * (r - luma) / ((1.0 - kr) * 255.0) + 128.0 + 0.5).floor());
        [y as u8, u as u8, v as u8]
    }
}

fn yuv_to_rgb(y: u8, u: u8, v: u8, mode: SDL_YUV_CONVERSION_MODE) -> (u8, u8, u8) {
    if mode == SDL_YUV_CONVERSION_MODE_SDL_YUV_CONVERSION_JPEG {
        let y = y as f32;
        let u = u as f32 - 128.0;
        let v = v as f32 - 128.0;
        let r = y + 1.402 * v;
        let g = y - 0.344_136 * u - 0.714_136 * v;
        let b = y + 1.772 * u;
        (clip_byte(r), clip_byte(g), clip_byte(b))
    } else {
        let (kr, kb) = if mode == SDL_YUV_CONVERSION_MODE_SDL_YUV_CONVERSION_BT709 {
            (0.2126, 0.0722)
        } else {
            (0.299, 0.114)
        };
        let luma = 255.0 * (y as f32 - 16.0) / 219.0;
        let pb = (u as f32 - 128.0) * ((1.0 - kb) * 255.0 / 112.0);
        let pr = (v as f32 - 128.0) * ((1.0 - kr) * 255.0 / 112.0);
        let r = luma + pr;
        let b = luma + pb;
        let g = (luma - kr * r - kb * b) / (1.0 - kr - kb);
        (clip_byte(r), clip_byte(g), clip_byte(b))
    }
}

fn validate_yuv_layout(
    format: Uint32,
    width: libc::c_int,
    height: libc::c_int,
    pitch: libc::c_int,
) -> Result<YuvLayout, MathError> {
    let width = checked_math::nonnegative_to_usize("width", width)?;
    let height = checked_math::nonnegative_to_usize("height", height)?;
    if pitch == 0 {
        return Err(MathError::InvalidParam("pitch"));
    }
    let y_pitch = usize::try_from(pitch).map_err(|_| MathError::InvalidParam("pitch"))?;

    match format {
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_YV12
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_IYUV
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV12
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV21 => {
            if width > 0 && y_pitch < width {
                return Err(MathError::InvalidParam("pitch"));
            }
        }
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_YUY2
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_UYVY
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_YVYU => {
            let pair_width = width.div_ceil(2);
            let minimal_pitch =
                checked_math::checked_mul_usize(pair_width, 4, "blit copy length overflow")?;
            if width > 0 && y_pitch < minimal_pitch {
                return Err(MathError::InvalidParam("pitch"));
            }
        }
        _ => return Err(MathError::InvalidParam("format")),
    }

    let plane_size = checked_math::checked_mul_usize(height, y_pitch, "blit copy length overflow")?;
    let chroma_h = height.div_ceil(2);
    let (chroma_pitch, chroma_plane_size, total_size) = match format {
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_YV12 | SDL_PixelFormatEnum_SDL_PIXELFORMAT_IYUV => {
            let chroma_pitch = y_pitch.div_ceil(2);
            let chroma_plane_size = checked_math::checked_mul_usize(
                chroma_h,
                chroma_pitch,
                "blit copy length overflow",
            )?;
            let total_size = checked_math::checked_add_usize(
                plane_size,
                checked_math::checked_mul_usize(chroma_plane_size, 2, "blit copy length overflow")?,
                "blit copy length overflow",
            )?;
            (chroma_pitch, chroma_plane_size, total_size)
        }
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV12 | SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV21 => {
            let chroma_pitch = checked_math::checked_mul_usize(
                y_pitch.div_ceil(2),
                2,
                "blit copy length overflow",
            )?;
            let chroma_plane_size = checked_math::checked_mul_usize(
                chroma_h,
                chroma_pitch,
                "blit copy length overflow",
            )?;
            let total_size = checked_math::checked_add_usize(
                plane_size,
                chroma_plane_size,
                "blit copy length overflow",
            )?;
            (chroma_pitch, chroma_plane_size, total_size)
        }
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_YUY2
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_UYVY
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_YVYU => (y_pitch, 0, plane_size),
        _ => unreachable!(),
    };

    Ok(YuvLayout {
        y_pitch,
        chroma_pitch,
        plane_size,
        chroma_plane_size,
        total_size,
    })
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

unsafe fn write_raw_pixel(ptr: *mut u8, bytes_per_pixel: u8, pixel: Uint32) {
    match bytes_per_pixel {
        1 => ptr.write(pixel as u8),
        2 => ptr.cast::<u16>().write_unaligned(pixel as u16),
        3 => {
            let bytes = pixel.to_le_bytes();
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, 3);
        }
        4 => ptr.cast::<Uint32>().write_unaligned(pixel),
        _ => {}
    }
}

fn allocate_rgba_pixels(width: usize, height: usize) -> Result<Vec<[u8; 4]>, libc::c_int> {
    let count = match checked_math::checked_mul_usize(width, height, "blit copy length overflow") {
        Ok(count) => count,
        Err(error) => return Err(apply_math_error(error)),
    };
    let mut pixels = Vec::new();
    if pixels.try_reserve_exact(count).is_err() {
        return Err(out_of_memory_error());
    }
    pixels.resize(count, [0; 4]);
    Ok(pixels)
}

unsafe fn fill_rgba_from_rgb(
    format: &AllocatedFormat,
    src: *const u8,
    src_pitch: usize,
    width: usize,
    height: usize,
    rgba: &mut [[u8; 4]],
) -> Result<(), libc::c_int> {
    let bytes_per_pixel = format.descriptor.bytes_per_pixel;
    if !(1..=4).contains(&bytes_per_pixel) {
        return Err(set_error_message("Unsupported pixel format"));
    }

    for y in 0..height {
        let src_row = src.add(y * src_pitch);
        for x in 0..width {
            let src_pixel = src_row.add(x * bytes_per_pixel as usize);
            let pixel = raw_pixel(src_pixel, bytes_per_pixel);
            let (mut r, mut g, mut b, mut a) = (0, 0, 0, 0);
            (real_sdl().get_rgba)(pixel, format.raw, &mut r, &mut g, &mut b, &mut a);
            rgba[y * width + x] = [r, g, b, a];
        }
    }
    Ok(())
}

unsafe fn write_rgba_to_rgb(
    format: &AllocatedFormat,
    dst: *mut u8,
    dst_pitch: usize,
    width: usize,
    height: usize,
    rgba: &[[u8; 4]],
) -> Result<(), libc::c_int> {
    let bytes_per_pixel = format.descriptor.bytes_per_pixel;
    if !(1..=4).contains(&bytes_per_pixel) {
        return Err(set_error_message("Unsupported pixel format"));
    }

    for y in 0..height {
        let dst_row = dst.add(y * dst_pitch);
        for x in 0..width {
            let pixel = rgba[y * width + x];
            let mapped = (real_sdl().map_rgba)(format.raw, pixel[0], pixel[1], pixel[2], pixel[3]);
            let dst_pixel = dst_row.add(x * bytes_per_pixel as usize);
            write_raw_pixel(dst_pixel, bytes_per_pixel, mapped);
        }
    }
    Ok(())
}

unsafe fn fill_rgba_from_yuv(
    format: Uint32,
    src: *const u8,
    width: usize,
    height: usize,
    layout: YuvLayout,
    mode: SDL_YUV_CONVERSION_MODE,
    rgba: &mut [[u8; 4]],
) {
    for y in 0..height {
        for x in 0..width {
            let (yy, uu, vv) = match format {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_YV12 => {
                    let y_plane = src;
                    let v_plane = src.add(layout.plane_size);
                    let u_plane = v_plane.add(layout.chroma_plane_size);
                    (
                        *y_plane.add(y * layout.y_pitch + x),
                        *u_plane.add((y / 2) * layout.chroma_pitch + (x / 2)),
                        *v_plane.add((y / 2) * layout.chroma_pitch + (x / 2)),
                    )
                }
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_IYUV => {
                    let y_plane = src;
                    let u_plane = src.add(layout.plane_size);
                    let v_plane = u_plane.add(layout.chroma_plane_size);
                    (
                        *y_plane.add(y * layout.y_pitch + x),
                        *u_plane.add((y / 2) * layout.chroma_pitch + (x / 2)),
                        *v_plane.add((y / 2) * layout.chroma_pitch + (x / 2)),
                    )
                }
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV12 => {
                    let y_plane = src;
                    let uv_plane = src.add(layout.plane_size);
                    let chroma = uv_plane.add((y / 2) * layout.chroma_pitch + (x / 2) * 2);
                    (
                        *y_plane.add(y * layout.y_pitch + x),
                        *chroma,
                        *chroma.add(1),
                    )
                }
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV21 => {
                    let y_plane = src;
                    let vu_plane = src.add(layout.plane_size);
                    let chroma = vu_plane.add((y / 2) * layout.chroma_pitch + (x / 2) * 2);
                    (
                        *y_plane.add(y * layout.y_pitch + x),
                        *chroma.add(1),
                        *chroma,
                    )
                }
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_YUY2 => {
                    let packed = src.add(y * layout.y_pitch + (x / 2) * 4);
                    let y_value = if x & 1 == 0 { *packed } else { *packed.add(2) };
                    (y_value, *packed.add(1), *packed.add(3))
                }
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_UYVY => {
                    let packed = src.add(y * layout.y_pitch + (x / 2) * 4);
                    let y_value = if x & 1 == 0 {
                        *packed.add(1)
                    } else {
                        *packed.add(3)
                    };
                    (y_value, *packed, *packed.add(2))
                }
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_YVYU => {
                    let packed = src.add(y * layout.y_pitch + (x / 2) * 4);
                    let y_value = if x & 1 == 0 { *packed } else { *packed.add(2) };
                    (y_value, *packed.add(3), *packed.add(1))
                }
                _ => unreachable!(),
            };
            let (r, g, b) = yuv_to_rgb(yy, uu, vv, mode);
            rgba[y * width + x] = [r, g, b, 255];
        }
    }
}

unsafe fn write_rgba_to_yuv(
    format: Uint32,
    dst: *mut u8,
    width: usize,
    height: usize,
    layout: YuvLayout,
    mode: SDL_YUV_CONVERSION_MODE,
    rgba: &[[u8; 4]],
) {
    match format {
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_YV12
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_IYUV
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV12
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV21 => {
            let y_plane = dst;
            match format {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_YV12 => {
                    let v_plane = dst.add(layout.plane_size);
                    let u_plane = v_plane.add(layout.chroma_plane_size);
                    for by in (0..height).step_by(2) {
                        for bx in (0..width).step_by(2) {
                            let mut samples = 0usize;
                            let mut u_total = 0u32;
                            let mut v_total = 0u32;
                            for dy in 0..2 {
                                if by + dy >= height {
                                    continue;
                                }
                                for dx in 0..2 {
                                    if bx + dx >= width {
                                        continue;
                                    }
                                    let pixel = rgba[(by + dy) * width + (bx + dx)];
                                    let yuv = rgb_to_yuv([pixel[0], pixel[1], pixel[2]], mode);
                                    *y_plane.add((by + dy) * layout.y_pitch + (bx + dx)) = yuv[0];
                                    u_total += yuv[1] as u32;
                                    v_total += yuv[2] as u32;
                                    samples += 1;
                                }
                            }
                            let u_value = ((u_total as f32 / samples as f32) + 0.5).floor() as u8;
                            let v_value = ((v_total as f32 / samples as f32) + 0.5).floor() as u8;
                            let chroma_index = (by / 2) * layout.chroma_pitch + (bx / 2);
                            *u_plane.add(chroma_index) = u_value;
                            *v_plane.add(chroma_index) = v_value;
                        }
                    }
                }
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_IYUV => {
                    let u_plane = dst.add(layout.plane_size);
                    let v_plane = u_plane.add(layout.chroma_plane_size);
                    for by in (0..height).step_by(2) {
                        for bx in (0..width).step_by(2) {
                            let mut samples = 0usize;
                            let mut u_total = 0u32;
                            let mut v_total = 0u32;
                            for dy in 0..2 {
                                if by + dy >= height {
                                    continue;
                                }
                                for dx in 0..2 {
                                    if bx + dx >= width {
                                        continue;
                                    }
                                    let pixel = rgba[(by + dy) * width + (bx + dx)];
                                    let yuv = rgb_to_yuv([pixel[0], pixel[1], pixel[2]], mode);
                                    *y_plane.add((by + dy) * layout.y_pitch + (bx + dx)) = yuv[0];
                                    u_total += yuv[1] as u32;
                                    v_total += yuv[2] as u32;
                                    samples += 1;
                                }
                            }
                            let u_value = ((u_total as f32 / samples as f32) + 0.5).floor() as u8;
                            let v_value = ((v_total as f32 / samples as f32) + 0.5).floor() as u8;
                            let chroma_index = (by / 2) * layout.chroma_pitch + (bx / 2);
                            *u_plane.add(chroma_index) = u_value;
                            *v_plane.add(chroma_index) = v_value;
                        }
                    }
                }
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV12
                | SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV21 => {
                    let uv_plane = dst.add(layout.plane_size);
                    let swap_uv = format == SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV21;
                    for by in (0..height).step_by(2) {
                        for bx in (0..width).step_by(2) {
                            let mut samples = 0usize;
                            let mut u_total = 0u32;
                            let mut v_total = 0u32;
                            for dy in 0..2 {
                                if by + dy >= height {
                                    continue;
                                }
                                for dx in 0..2 {
                                    if bx + dx >= width {
                                        continue;
                                    }
                                    let pixel = rgba[(by + dy) * width + (bx + dx)];
                                    let yuv = rgb_to_yuv([pixel[0], pixel[1], pixel[2]], mode);
                                    *y_plane.add((by + dy) * layout.y_pitch + (bx + dx)) = yuv[0];
                                    u_total += yuv[1] as u32;
                                    v_total += yuv[2] as u32;
                                    samples += 1;
                                }
                            }
                            let u_value = ((u_total as f32 / samples as f32) + 0.5).floor() as u8;
                            let v_value = ((v_total as f32 / samples as f32) + 0.5).floor() as u8;
                            let chroma =
                                uv_plane.add((by / 2) * layout.chroma_pitch + (bx / 2) * 2);
                            if swap_uv {
                                *chroma = v_value;
                                *chroma.add(1) = u_value;
                            } else {
                                *chroma = u_value;
                                *chroma.add(1) = v_value;
                            }
                        }
                    }
                }
                _ => unreachable!(),
            }
        }
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_YUY2
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_UYVY
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_YVYU => {
            for y in 0..height {
                let dst_row = dst.add(y * layout.y_pitch);
                for x in (0..width).step_by(2) {
                    let left = rgba[y * width + x];
                    let right = rgba[y * width + (x + 1).min(width.saturating_sub(1))];
                    let yuv1 = rgb_to_yuv([left[0], left[1], left[2]], mode);
                    let yuv2 = rgb_to_yuv([right[0], right[1], right[2]], mode);
                    let u = ((yuv1[1] as f32 + yuv2[1] as f32) / 2.0 + 0.5).floor() as u8;
                    let v = ((yuv1[2] as f32 + yuv2[2] as f32) / 2.0 + 0.5).floor() as u8;
                    let packed = dst_row.add((x / 2) * 4);
                    match format {
                        SDL_PixelFormatEnum_SDL_PIXELFORMAT_YUY2 => {
                            *packed = yuv1[0];
                            *packed.add(1) = u;
                            *packed.add(2) = yuv2[0];
                            *packed.add(3) = v;
                        }
                        SDL_PixelFormatEnum_SDL_PIXELFORMAT_UYVY => {
                            *packed = u;
                            *packed.add(1) = yuv1[0];
                            *packed.add(2) = v;
                            *packed.add(3) = yuv2[0];
                        }
                        SDL_PixelFormatEnum_SDL_PIXELFORMAT_YVYU => {
                            *packed = yuv1[0];
                            *packed.add(1) = v;
                            *packed.add(2) = yuv2[0];
                            *packed.add(3) = u;
                        }
                        _ => unreachable!(),
                    }
                }
            }
        }
        _ => unreachable!(),
    }
}

unsafe fn copy_same_rgb_format(
    src: *const u8,
    src_pitch: usize,
    dst: *mut u8,
    dst_pitch: usize,
    row_bytes: usize,
    height: usize,
) {
    for row in 0..height {
        std::ptr::copy(
            src.add(row * src_pitch),
            dst.add(row * dst_pitch),
            row_bytes,
        );
    }
}

unsafe fn copy_same_yuv_format(
    format: Uint32,
    src: *const u8,
    src_layout: YuvLayout,
    dst: *mut u8,
    dst_layout: YuvLayout,
    width: usize,
    height: usize,
) {
    match format {
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_YV12 | SDL_PixelFormatEnum_SDL_PIXELFORMAT_IYUV => {
            for row in 0..height {
                std::ptr::copy(
                    src.add(row * src_layout.y_pitch),
                    dst.add(row * dst_layout.y_pitch),
                    width,
                );
            }

            let src_u_offset = if format == SDL_PixelFormatEnum_SDL_PIXELFORMAT_YV12 {
                src_layout.plane_size + src_layout.chroma_plane_size
            } else {
                src_layout.plane_size
            };
            let src_v_offset = if format == SDL_PixelFormatEnum_SDL_PIXELFORMAT_YV12 {
                src_layout.plane_size
            } else {
                src_layout.plane_size + src_layout.chroma_plane_size
            };
            let dst_u_offset = if format == SDL_PixelFormatEnum_SDL_PIXELFORMAT_YV12 {
                dst_layout.plane_size + dst_layout.chroma_plane_size
            } else {
                dst_layout.plane_size
            };
            let dst_v_offset = if format == SDL_PixelFormatEnum_SDL_PIXELFORMAT_YV12 {
                dst_layout.plane_size
            } else {
                dst_layout.plane_size + dst_layout.chroma_plane_size
            };
            let chroma_h = height.div_ceil(2);
            let chroma_w = width.div_ceil(2);
            for row in 0..chroma_h {
                std::ptr::copy(
                    src.add(src_u_offset + row * src_layout.chroma_pitch),
                    dst.add(dst_u_offset + row * dst_layout.chroma_pitch),
                    chroma_w,
                );
                std::ptr::copy(
                    src.add(src_v_offset + row * src_layout.chroma_pitch),
                    dst.add(dst_v_offset + row * dst_layout.chroma_pitch),
                    chroma_w,
                );
            }
        }
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV12 | SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV21 => {
            for row in 0..height {
                std::ptr::copy(
                    src.add(row * src_layout.y_pitch),
                    dst.add(row * dst_layout.y_pitch),
                    width,
                );
            }
            let chroma_h = height.div_ceil(2);
            let chroma_w = width.div_ceil(2) * 2;
            for row in 0..chroma_h {
                std::ptr::copy(
                    src.add(src_layout.plane_size + row * src_layout.chroma_pitch),
                    dst.add(dst_layout.plane_size + row * dst_layout.chroma_pitch),
                    chroma_w,
                );
            }
        }
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_YUY2
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_UYVY
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_YVYU => {
            let row_bytes = width.div_ceil(2) * 4;
            for row in 0..height {
                std::ptr::copy(
                    src.add(row * src_layout.y_pitch),
                    dst.add(row * dst_layout.y_pitch),
                    row_bytes,
                );
            }
        }
        _ => unreachable!(),
    }
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

unsafe fn should_use_real_blit(src: *mut SDL_Surface, dst: *mut SDL_Surface) -> bool {
    !(is_registered_surface(src) && is_registered_surface(dst))
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
    if src_pitch == 0 {
        return invalid_param_error("src_pitch");
    }
    if dst.is_null() {
        return invalid_param_error("dst");
    }
    if dst_pitch == 0 {
        return invalid_param_error("dst_pitch");
    }

    let width_usize = match checked_math::nonnegative_to_usize("width", width) {
        Ok(value) => value,
        Err(error) => return apply_math_error(error),
    };
    let height_usize = match checked_math::nonnegative_to_usize("height", height) {
        Ok(value) => value,
        Err(error) => return apply_math_error(error),
    };

    let src_is_yuv = is_yuv_format(src_format);
    let dst_is_yuv = is_yuv_format(dst_format);

    let mut src_row_bytes = 0usize;
    let src_layout = if src_is_yuv {
        match validate_yuv_layout(src_format, width, height, src_pitch) {
            Ok(layout) => Some(layout),
            Err(error) => return apply_math_error(error),
        }
    } else {
        let descriptor = match format_descriptor(src_format) {
            Some(descriptor) => descriptor,
            None => return set_error_message("Unsupported pixel format"),
        };
        match checked_math::validate_copy_layout(
            width,
            height,
            descriptor.bits_per_pixel,
            descriptor.bytes_per_pixel,
            src_pitch,
        ) {
            Ok((row_bytes, _)) => {
                src_row_bytes = row_bytes;
                None
            }
            Err(error) => return apply_math_error(error),
        }
    };

    let mut dst_row_bytes = 0usize;
    let dst_layout = if dst_is_yuv {
        match validate_yuv_layout(dst_format, width, height, dst_pitch) {
            Ok(layout) => Some(layout),
            Err(error) => return apply_math_error(error),
        }
    } else {
        let descriptor = match format_descriptor(dst_format) {
            Some(descriptor) => descriptor,
            None => return set_error_message("Unsupported pixel format"),
        };
        match checked_math::validate_copy_layout(
            width,
            height,
            descriptor.bits_per_pixel,
            descriptor.bytes_per_pixel,
            dst_pitch,
        ) {
            Ok((row_bytes, _)) => {
                dst_row_bytes = row_bytes;
                None
            }
            Err(error) => return apply_math_error(error),
        }
    };

    if width_usize == 0 || height_usize == 0 {
        return 0;
    }

    let src_pitch = match usize::try_from(src_pitch) {
        Ok(value) => value,
        Err(_) => return invalid_param_error("src_pitch"),
    };
    let dst_pitch = match usize::try_from(dst_pitch) {
        Ok(value) => value,
        Err(_) => return invalid_param_error("dst_pitch"),
    };

    if src_format == dst_format {
        if src == dst.cast_const() && src_pitch == dst_pitch {
            return 0;
        }
        if src_is_yuv {
            copy_same_yuv_format(
                src_format,
                src.cast(),
                src_layout.unwrap(),
                dst.cast(),
                dst_layout.unwrap(),
                width_usize,
                height_usize,
            );
        } else {
            copy_same_rgb_format(
                src.cast(),
                src_pitch,
                dst.cast(),
                dst_pitch,
                src_row_bytes.min(dst_row_bytes),
                height_usize,
            );
        }
        return 0;
    }

    let mut rgba = match allocate_rgba_pixels(width_usize, height_usize) {
        Ok(pixels) => pixels,
        Err(code) => return code,
    };
    let mode = effective_yuv_mode(width, height);

    if src_is_yuv {
        fill_rgba_from_yuv(
            src_format,
            src.cast(),
            width_usize,
            height_usize,
            src_layout.unwrap(),
            mode,
            &mut rgba,
        );
    } else {
        let src_format = match AllocatedFormat::new(src_format) {
            Ok(format) => format,
            Err(code) => return code,
        };
        if let Err(code) = fill_rgba_from_rgb(
            &src_format,
            src.cast(),
            src_pitch,
            width_usize,
            height_usize,
            &mut rgba,
        ) {
            return code;
        }
    }

    if dst_is_yuv {
        write_rgba_to_yuv(
            dst_format,
            dst.cast(),
            width_usize,
            height_usize,
            dst_layout.unwrap(),
            mode,
            &rgba,
        );
    } else {
        let dst_format = match AllocatedFormat::new(dst_format) {
            Ok(format) => format,
            Err(code) => return code,
        };
        if let Err(code) = write_rgba_to_rgb(
            &dst_format,
            dst.cast(),
            dst_pitch,
            width_usize,
            height_usize,
            &rgba,
        ) {
            return code;
        }
    }

    0
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
    if src_pitch == 0 {
        return invalid_param_error("src_pitch");
    }
    if dst.is_null() {
        return invalid_param_error("dst");
    }
    if dst_pitch == 0 {
        return invalid_param_error("dst_pitch");
    }
    if src_format != SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888 {
        return invalid_param_error("src_format");
    }
    if dst_format != SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888 {
        return invalid_param_error("dst_format");
    }

    if let Err(error) = checked_math::validate_copy_layout(width, height, 32, 4, src_pitch) {
        return apply_math_error(error);
    }
    if let Err(error) = checked_math::validate_copy_layout(width, height, 32, 4, dst_pitch) {
        return apply_math_error(error);
    }

    let width_usize = match checked_math::nonnegative_to_usize("width", width) {
        Ok(value) => value,
        Err(error) => return apply_math_error(error),
    };
    let height_usize = match checked_math::nonnegative_to_usize("height", height) {
        Ok(value) => value,
        Err(error) => return apply_math_error(error),
    };
    if width_usize == 0 || height_usize == 0 {
        return 0;
    }

    let pixel_count = match checked_math::checked_mul_usize(
        width_usize,
        height_usize,
        "blit copy length overflow",
    ) {
        Ok(count) => count,
        Err(error) => return apply_math_error(error),
    };
    let mut pixels = Vec::new();
    if pixels.try_reserve_exact(pixel_count).is_err() {
        return out_of_memory_error();
    }
    pixels.resize(pixel_count, 0u32);

    let src_pitch = match usize::try_from(src_pitch) {
        Ok(value) => value,
        Err(_) => return invalid_param_error("src_pitch"),
    };
    let dst_pitch = match usize::try_from(dst_pitch) {
        Ok(value) => value,
        Err(_) => return invalid_param_error("dst_pitch"),
    };

    for y in 0..height_usize {
        let src_row = src.cast::<u8>().add(y * src_pitch);
        for x in 0..width_usize {
            let src_pixel = src_row.add(x * 4).cast::<u32>().read_unaligned();
            let src_a = (src_pixel >> 24) & 0xff;
            let src_r = (src_pixel >> 16) & 0xff;
            let src_g = (src_pixel >> 8) & 0xff;
            let src_b = src_pixel & 0xff;
            let dst_r = (src_a * src_r) / 255;
            let dst_g = (src_a * src_g) / 255;
            let dst_b = (src_a * src_b) / 255;
            pixels[y * width_usize + x] = (src_a << 24) | (dst_r << 16) | (dst_g << 8) | dst_b;
        }
    }

    for y in 0..height_usize {
        let dst_row = dst.cast::<u8>().add(y * dst_pitch);
        for x in 0..width_usize {
            dst_row
                .add(x * 4)
                .cast::<u32>()
                .write_unaligned(pixels[y * width_usize + x]);
        }
    }

    0
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

    if should_use_real_blit(src, dst) {
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

    if should_use_real_blit(src, dst) {
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

    if should_use_real_blit(src, dst) {
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

    if should_use_real_blit(src, dst) {
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
