#![allow(dead_code)]
#![allow(non_upper_case_globals)]
#![allow(clippy::all)]

use safe_sdl::abi::generated_types::{
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_IYUV, SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV12,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV21, SDL_PixelFormatEnum_SDL_PIXELFORMAT_UYVY,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_YUY2, SDL_PixelFormatEnum_SDL_PIXELFORMAT_YV12,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_YVYU, Uint32, SDL_YUV_CONVERSION_MODE,
    SDL_YUV_CONVERSION_MODE_SDL_YUV_CONVERSION_BT709,
    SDL_YUV_CONVERSION_MODE_SDL_YUV_CONVERSION_JPEG,
};

fn clip3(min: f32, max: f32, value: f32) -> f32 {
    value.clamp(min, max)
}

fn rgb_to_yuv(
    rgb: &[u8],
    mode: SDL_YUV_CONVERSION_MODE,
    monochrome: bool,
    luminance: i32,
) -> [u8; 3] {
    let mut yuv = if mode == SDL_YUV_CONVERSION_MODE_SDL_YUV_CONVERSION_JPEG {
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
        let l = kr * r + kb * b + (1.0 - kr - kb) * g;
        let y = (219.0 * l / 255.0 + 16.0 + 0.5).floor();
        let u = clip3(
            0.0,
            255.0,
            (112.0 * (b - l) / ((1.0 - kb) * 255.0) + 128.0 + 0.5).floor(),
        );
        let v = clip3(
            0.0,
            255.0,
            (112.0 * (r - l) / ((1.0 - kr) * 255.0) + 128.0 + 0.5).floor(),
        );
        [y as u8, u as u8, v as u8]
    };

    if monochrome {
        yuv[1] = 128;
        yuv[2] = 128;
    }

    if luminance != 100 {
        yuv[0] = ((yuv[0] as i32 * luminance) / 100).min(255) as u8;
    }

    yuv
}

fn pixel_offset(pitch: usize, x: usize, y: usize) -> usize {
    y * pitch + x * 3
}

fn convert_rgb_to_planar_2x2(
    format: Uint32,
    src: &[u8],
    pitch: usize,
    out: &mut [u8],
    w: usize,
    h: usize,
    mode: SDL_YUV_CONVERSION_MODE,
    monochrome: bool,
    luminance: i32,
) {
    let plane_size = w * h;
    let chroma_w = w.div_ceil(2);
    let chroma_h = h.div_ceil(2);
    let chroma_plane_size = chroma_w * chroma_h;

    let (u_offset, v_offset, uv_step, nv_swapped) = match format {
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_YV12 => {
            (plane_size + chroma_plane_size, plane_size, 1usize, false)
        }
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_IYUV => {
            (plane_size, plane_size + chroma_plane_size, 1usize, false)
        }
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV12 => (plane_size, plane_size + 1, 2usize, false),
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV21 => (plane_size + 1, plane_size, 2usize, true),
        _ => unreachable!("unsupported planar YUV format"),
    };

    for y in (0..h).step_by(2) {
        for x in (0..w).step_by(2) {
            let mut block = [[0u8; 3]; 4];
            let mut samples = 0usize;
            let mut u_total = 0u32;
            let mut v_total = 0u32;

            for dy in 0..2 {
                if y + dy >= h {
                    continue;
                }
                for dx in 0..2 {
                    if x + dx >= w {
                        continue;
                    }
                    let src_offset = pixel_offset(pitch, x + dx, y + dy);
                    let yuv = rgb_to_yuv(
                        &src[src_offset..src_offset + 3],
                        mode,
                        monochrome,
                        luminance,
                    );
                    block[samples] = yuv;
                    out[(y + dy) * w + (x + dx)] = yuv[0];
                    u_total += yuv[1] as u32;
                    v_total += yuv[2] as u32;
                    samples += 1;
                }
            }

            let u_value = ((u_total as f32 / samples as f32) + 0.5).floor() as u8;
            let v_value = ((v_total as f32 / samples as f32) + 0.5).floor() as u8;
            let chroma_index = (y / 2) * chroma_w + (x / 2);

            if uv_step == 1 {
                out[u_offset + chroma_index] = u_value;
                out[v_offset + chroma_index] = v_value;
            } else {
                let base = plane_size + chroma_index * uv_step;
                if nv_swapped {
                    out[base] = v_value;
                    out[base + 1] = u_value;
                } else {
                    out[base] = u_value;
                    out[base + 1] = v_value;
                }
            }
        }
    }
}

fn convert_rgb_to_packed4(
    format: Uint32,
    src: &[u8],
    pitch: usize,
    out: &mut [u8],
    w: usize,
    h: usize,
    mode: SDL_YUV_CONVERSION_MODE,
    monochrome: bool,
    luminance: i32,
) {
    let yuv_pitch = calculate_yuv_pitch(format, w as i32) as usize;
    for y in 0..h {
        for x in (0..w).step_by(2) {
            let left = pixel_offset(pitch, x, y);
            let right_x = (x + 1).min(w - 1);
            let right = pixel_offset(pitch, right_x, y);

            let yuv1 = rgb_to_yuv(&src[left..left + 3], mode, monochrome, luminance);
            let yuv2 = rgb_to_yuv(&src[right..right + 3], mode, monochrome, luminance);
            let u = ((yuv1[1] as f32 + yuv2[1] as f32) / 2.0 + 0.5).floor() as u8;
            let v = ((yuv1[2] as f32 + yuv2[2] as f32) / 2.0 + 0.5).floor() as u8;

            let base = y * yuv_pitch + (x / 2) * 4;
            match format {
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_YUY2 => {
                    out[base] = yuv1[0];
                    out[base + 1] = u;
                    out[base + 2] = yuv2[0];
                    out[base + 3] = v;
                }
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_UYVY => {
                    out[base] = u;
                    out[base + 1] = yuv1[0];
                    out[base + 2] = v;
                    out[base + 3] = yuv2[0];
                }
                SDL_PixelFormatEnum_SDL_PIXELFORMAT_YVYU => {
                    out[base] = yuv1[0];
                    out[base + 1] = v;
                    out[base + 2] = yuv2[0];
                    out[base + 3] = u;
                }
                _ => unreachable!("unsupported packed YUV format"),
            }
        }
    }
}

pub fn convert_rgb_to_yuv(
    format: Uint32,
    src: &[u8],
    pitch: usize,
    out: &mut [u8],
    w: usize,
    h: usize,
    mode: SDL_YUV_CONVERSION_MODE,
    monochrome: bool,
    luminance: i32,
) -> bool {
    match format {
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_YV12
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_IYUV
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV12
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV21 => {
            convert_rgb_to_planar_2x2(format, src, pitch, out, w, h, mode, monochrome, luminance);
            true
        }
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_YUY2
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_UYVY
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_YVYU => {
            convert_rgb_to_packed4(format, src, pitch, out, w, h, mode, monochrome, luminance);
            true
        }
        _ => false,
    }
}

pub fn calculate_yuv_pitch(format: Uint32, width: i32) -> i32 {
    match format {
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_YV12
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_IYUV
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV12
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV21 => width,
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_YUY2
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_UYVY
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_YVYU => 4 * ((width + 1) / 2),
        _ => 0,
    }
}
