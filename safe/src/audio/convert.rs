use std::collections::HashMap;
use std::slice;
use std::sync::{Mutex, OnceLock};

use crate::abi::generated_types::{
    SDL_AudioCVT, SDL_AudioFormat, AUDIO_F32LSB, AUDIO_F32MSB, AUDIO_S16LSB, AUDIO_S16MSB,
    AUDIO_S32LSB, AUDIO_S32MSB, AUDIO_S8, AUDIO_U16LSB, AUDIO_U16MSB, AUDIO_U8, SDL_MIX_MAXVOLUME,
};
use crate::audio::{bytes_per_sample, frame_size, normalize_audio_spec};

const AUDIO_U8_FMT: SDL_AudioFormat = AUDIO_U8 as SDL_AudioFormat;
const AUDIO_S8_FMT: SDL_AudioFormat = AUDIO_S8 as SDL_AudioFormat;
const AUDIO_U16LSB_FMT: SDL_AudioFormat = AUDIO_U16LSB as SDL_AudioFormat;
const AUDIO_U16MSB_FMT: SDL_AudioFormat = AUDIO_U16MSB as SDL_AudioFormat;
const AUDIO_S16LSB_FMT: SDL_AudioFormat = AUDIO_S16LSB as SDL_AudioFormat;
const AUDIO_S16MSB_FMT: SDL_AudioFormat = AUDIO_S16MSB as SDL_AudioFormat;
const AUDIO_S32LSB_FMT: SDL_AudioFormat = AUDIO_S32LSB as SDL_AudioFormat;
const AUDIO_S32MSB_FMT: SDL_AudioFormat = AUDIO_S32MSB as SDL_AudioFormat;
const AUDIO_F32LSB_FMT: SDL_AudioFormat = AUDIO_F32LSB as SDL_AudioFormat;
const AUDIO_F32MSB_FMT: SDL_AudioFormat = AUDIO_F32MSB as SDL_AudioFormat;

#[derive(Clone, Copy)]
struct CvtSpec {
    src_channels: u8,
    src_rate: i32,
    dst_channels: u8,
    dst_rate: i32,
}

fn cvt_specs() -> &'static Mutex<HashMap<usize, CvtSpec>> {
    static SPECS: OnceLock<Mutex<HashMap<usize, CvtSpec>>> = OnceLock::new();
    SPECS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Default)]
pub(crate) struct AudioConvertScratch {
    decoded: Vec<f32>,
    channel_converted: Vec<f32>,
    resampled: Vec<f32>,
    pub(crate) encoded: Vec<u8>,
}

fn decode_sample(format: SDL_AudioFormat, bytes: &[u8]) -> f32 {
    match format {
        AUDIO_U8_FMT => (bytes[0] as f32 - 128.0) / 128.0,
        AUDIO_S8_FMT => i8::from_ne_bytes([bytes[0]]) as f32 / 128.0,
        AUDIO_U16LSB_FMT => (u16::from_le_bytes([bytes[0], bytes[1]]) as f32 - 32768.0) / 32768.0,
        AUDIO_U16MSB_FMT => (u16::from_be_bytes([bytes[0], bytes[1]]) as f32 - 32768.0) / 32768.0,
        AUDIO_S16LSB_FMT => i16::from_le_bytes([bytes[0], bytes[1]]) as f32 / 32768.0,
        AUDIO_S16MSB_FMT => i16::from_be_bytes([bytes[0], bytes[1]]) as f32 / 32768.0,
        AUDIO_S32LSB_FMT => {
            i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f32 / 2147483648.0
        }
        AUDIO_S32MSB_FMT => {
            i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f32 / 2147483648.0
        }
        AUDIO_F32LSB_FMT => f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        AUDIO_F32MSB_FMT => f32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        _ => 0.0,
    }
}

fn encode_sample(format: SDL_AudioFormat, sample: f32, bytes: &mut [u8]) {
    let sample = sample.clamp(-1.0, 1.0);
    match format {
        AUDIO_U8_FMT => bytes[0] = ((sample * 128.0) + 128.0).round().clamp(0.0, 255.0) as u8,
        AUDIO_S8_FMT => bytes[0] = ((sample * 127.0).round().clamp(-128.0, 127.0) as i8) as u8,
        AUDIO_U16LSB_FMT | AUDIO_U16MSB_FMT => {
            let value = ((sample * 32768.0) + 32768.0).round().clamp(0.0, 65535.0) as u16;
            let encoded = if format == AUDIO_U16LSB_FMT {
                value.to_le_bytes()
            } else {
                value.to_be_bytes()
            };
            bytes.copy_from_slice(&encoded);
        }
        AUDIO_S16LSB_FMT | AUDIO_S16MSB_FMT => {
            let value = (sample * 32767.0)
                .round()
                .clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            let encoded = if format == AUDIO_S16LSB_FMT {
                value.to_le_bytes()
            } else {
                value.to_be_bytes()
            };
            bytes.copy_from_slice(&encoded);
        }
        AUDIO_S32LSB_FMT | AUDIO_S32MSB_FMT => {
            let value = (sample * 2147483647.0)
                .round()
                .clamp(i32::MIN as f32, i32::MAX as f32) as i32;
            let encoded = if format == AUDIO_S32LSB_FMT {
                value.to_le_bytes()
            } else {
                value.to_be_bytes()
            };
            bytes.copy_from_slice(&encoded);
        }
        AUDIO_F32LSB_FMT | AUDIO_F32MSB_FMT => {
            let value = if sample.is_finite() { sample } else { 0.0 };
            let encoded = if format == AUDIO_F32LSB_FMT {
                value.to_le_bytes()
            } else {
                value.to_be_bytes()
            };
            bytes.copy_from_slice(&encoded);
        }
        _ => {}
    }
}

fn decode_interleaved_to_f32_into(
    input: &[u8],
    format: SDL_AudioFormat,
    channels: u8,
    output: &mut Vec<f32>,
) -> Result<(), &'static str> {
    let sample_size = bytes_per_sample(format).ok_or("Unsupported audio format")?;
    let frame_size = frame_size(format, channels).ok_or("Unsupported audio format")?;
    if frame_size == 0 || input.len() % frame_size != 0 {
        return Err("Audio buffer has an incomplete frame");
    }

    output.clear();
    output.reserve(input.len() / sample_size);
    for chunk in input.chunks_exact(sample_size) {
        let sample = decode_sample(format, chunk);
        output.push(if sample.is_finite() { sample } else { 0.0 });
    }
    Ok(())
}

fn convert_channels_into(
    input: &[f32],
    src_channels: usize,
    dst_channels: usize,
    output: &mut Vec<f32>,
) {
    if src_channels == dst_channels {
        output.clear();
        output.extend_from_slice(input);
        return;
    }

    let frames = input.len() / src_channels;
    output.clear();
    output.resize(frames * dst_channels, 0.0);
    for frame in 0..frames {
        let src = &input[frame * src_channels..(frame + 1) * src_channels];
        let dst = &mut output[frame * dst_channels..(frame + 1) * dst_channels];

        if dst_channels == 1 {
            dst[0] = src.iter().copied().sum::<f32>() / src_channels as f32;
            continue;
        }

        if src_channels == 1 {
            dst.fill(src[0]);
            continue;
        }

        let copy_count = src_channels.min(dst_channels);
        dst[..copy_count].copy_from_slice(&src[..copy_count]);
        if dst_channels > src_channels {
            for sample in &mut dst[src_channels..] {
                *sample = 0.0;
            }
        }
    }
}

fn encode_interleaved_from_f32_to_buffer(
    input: &[f32],
    format: SDL_AudioFormat,
    output: &mut Vec<u8>,
) -> Result<(), &'static str> {
    let sample_size = bytes_per_sample(format).ok_or("Unsupported audio format")?;
    output.clear();
    output.resize(input.len() * sample_size, 0);
    for (sample, chunk) in input.iter().zip(output.chunks_exact_mut(sample_size)) {
        encode_sample(format, *sample, chunk);
    }
    Ok(())
}

pub(crate) fn convert_audio_buffer_into(
    input: &[u8],
    src_format: SDL_AudioFormat,
    src_channels: u8,
    src_rate: i32,
    dst_format: SDL_AudioFormat,
    dst_channels: u8,
    dst_rate: i32,
) -> Result<AudioConvertScratch, &'static str> {
    let mut scratch = AudioConvertScratch::default();
    convert_audio_buffer_reuse(
        input,
        src_format,
        src_channels,
        src_rate,
        dst_format,
        dst_channels,
        dst_rate,
        &mut scratch,
    )?;
    Ok(scratch)
}

pub(crate) fn convert_audio_buffer_reuse(
    input: &[u8],
    src_format: SDL_AudioFormat,
    src_channels: u8,
    src_rate: i32,
    dst_format: SDL_AudioFormat,
    dst_channels: u8,
    dst_rate: i32,
    scratch: &mut AudioConvertScratch,
) -> Result<(), &'static str> {
    if src_format == dst_format && src_channels == dst_channels && src_rate == dst_rate {
        scratch.encoded.clear();
        scratch.encoded.extend_from_slice(input);
        return Ok(());
    }

    decode_interleaved_to_f32_into(input, src_format, src_channels, &mut scratch.decoded)?;

    let mut converted = &scratch.decoded[..];
    if src_rate != dst_rate && dst_channels > src_channels {
        crate::audio::resample::resample_interleaved_f32_into(
            &scratch.decoded,
            src_channels as usize,
            src_rate,
            dst_rate,
            &mut scratch.resampled,
        );
        converted = &scratch.resampled;
        if src_channels != dst_channels {
            convert_channels_into(
                converted,
                src_channels as usize,
                dst_channels as usize,
                &mut scratch.channel_converted,
            );
            converted = &scratch.channel_converted;
        }
    } else {
        if src_channels != dst_channels {
            convert_channels_into(
                &scratch.decoded,
                src_channels as usize,
                dst_channels as usize,
                &mut scratch.channel_converted,
            );
            converted = &scratch.channel_converted;
        }
        if src_rate != dst_rate {
            crate::audio::resample::resample_interleaved_f32_into(
                converted,
                dst_channels as usize,
                src_rate,
                dst_rate,
                &mut scratch.resampled,
            );
            converted = &scratch.resampled;
        }
    }

    encode_interleaved_from_f32_to_buffer(converted, dst_format, &mut scratch.encoded)
}

pub(crate) fn convert_audio_buffer(
    input: &[u8],
    src_format: SDL_AudioFormat,
    src_channels: u8,
    src_rate: i32,
    dst_format: SDL_AudioFormat,
    dst_channels: u8,
    dst_rate: i32,
) -> Result<Vec<u8>, &'static str> {
    Ok(convert_audio_buffer_into(
        input,
        src_format,
        src_channels,
        src_rate,
        dst_format,
        dst_channels,
        dst_rate,
    )?
    .encoded)
}

fn len_ratio(
    src_format: SDL_AudioFormat,
    src_channels: u8,
    src_rate: i32,
    dst_format: SDL_AudioFormat,
    dst_channels: u8,
    dst_rate: i32,
) -> Result<f64, &'static str> {
    let src_frame = frame_size(src_format, src_channels).ok_or("Unsupported audio format")?;
    let dst_frame = frame_size(dst_format, dst_channels).ok_or("Unsupported audio format")?;
    Ok((dst_frame as f64 * dst_rate as f64) / (src_frame as f64 * src_rate as f64))
}

fn len_mult(
    src_format: SDL_AudioFormat,
    src_channels: u8,
    src_rate: i32,
    dst_format: SDL_AudioFormat,
    dst_channels: u8,
    dst_rate: i32,
) -> Result<i32, &'static str> {
    let src_frame = frame_size(src_format, src_channels).ok_or("Unsupported audio format")?;
    let dst_frame = frame_size(dst_format, dst_channels).ok_or("Unsupported audio format")?;
    let numerator = dst_frame as u128 * dst_rate as u128;
    let denominator = src_frame as u128 * src_rate as u128;
    let ratio = numerator.div_ceil(denominator);
    let ratio = ratio.saturating_add(1);
    i32::try_from(ratio).map_err(|_| "Audio conversion expansion is too large")
}

#[no_mangle]
pub unsafe extern "C" fn SDL_BuildAudioCVT(
    cvt: *mut SDL_AudioCVT,
    src_format: SDL_AudioFormat,
    src_channels: u8,
    src_rate: i32,
    dst_format: SDL_AudioFormat,
    dst_channels: u8,
    dst_rate: i32,
) -> libc::c_int {
    if cvt.is_null() {
        return crate::core::error::invalid_param_error("cvt");
    }
    if src_channels == 0 || dst_channels == 0 || src_rate <= 0 || dst_rate <= 0 {
        return crate::core::error::set_error_message("Invalid audio conversion specification");
    }

    let mut src_spec = crate::abi::generated_types::SDL_AudioSpec {
        freq: src_rate,
        format: src_format,
        channels: src_channels,
        silence: 0,
        samples: 4096,
        padding: 0,
        size: 0,
        callback: None,
        userdata: std::ptr::null_mut(),
    };
    if normalize_audio_spec(&mut src_spec).is_err() {
        return crate::core::error::set_error_message("Invalid source audio specification");
    }

    let mut dst_spec = crate::abi::generated_types::SDL_AudioSpec {
        freq: dst_rate,
        format: dst_format,
        channels: dst_channels,
        silence: 0,
        samples: 4096,
        padding: 0,
        size: 0,
        callback: None,
        userdata: std::ptr::null_mut(),
    };
    if normalize_audio_spec(&mut dst_spec).is_err() {
        return crate::core::error::set_error_message("Invalid destination audio specification");
    }

    let needed = src_format != dst_format || src_channels != dst_channels || src_rate != dst_rate;
    let mult = match len_mult(
        src_format,
        src_channels,
        src_rate,
        dst_format,
        dst_channels,
        dst_rate,
    ) {
        Ok(value) => value,
        Err(message) => return crate::core::error::set_error_message(message),
    };
    let ratio = match len_ratio(
        src_format,
        src_channels,
        src_rate,
        dst_format,
        dst_channels,
        dst_rate,
    ) {
        Ok(value) => value,
        Err(message) => return crate::core::error::set_error_message(message),
    };

    (*cvt).needed = needed as libc::c_int;
    (*cvt).src_format = src_format;
    (*cvt).dst_format = dst_format;
    (*cvt).rate_incr = dst_rate as f64 / src_rate as f64;
    (*cvt).buf = std::ptr::null_mut();
    (*cvt).len = 0;
    (*cvt).len_cvt = 0;
    (*cvt).len_mult = mult.max(1);
    (*cvt).len_ratio = ratio.max(1.0 / (*cvt).len_mult as f64);
    (*cvt).filters = [None; 10];
    (*cvt).filter_index = 0;

    match cvt_specs().lock() {
        Ok(mut guard) => {
            guard.insert(
                cvt as usize,
                CvtSpec {
                    src_channels,
                    src_rate,
                    dst_channels,
                    dst_rate,
                },
            );
        }
        Err(poisoned) => {
            let mut guard = poisoned.into_inner();
            guard.insert(
                cvt as usize,
                CvtSpec {
                    src_channels,
                    src_rate,
                    dst_channels,
                    dst_rate,
                },
            );
        }
    }

    if needed {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ConvertAudio(cvt: *mut SDL_AudioCVT) -> libc::c_int {
    if cvt.is_null() {
        return crate::core::error::invalid_param_error("cvt");
    }
    if (*cvt).len < 0 {
        return crate::core::error::set_error_message("Audio conversion length is invalid");
    }
    if (*cvt).len > 0 && (*cvt).buf.is_null() {
        return crate::core::error::set_error_message("Audio conversion buffer is invalid");
    }

    let src_len = (*cvt).len as usize;
    if (*cvt).needed == 0 {
        (*cvt).len_cvt = (*cvt).len;
        return 0;
    }

    let spec = match cvt_specs().lock() {
        Ok(guard) => guard.get(&(cvt as usize)).copied(),
        Err(poisoned) => poisoned.into_inner().get(&(cvt as usize)).copied(),
    };
    let Some(spec) = spec else {
        return crate::core::error::set_error_message("Audio conversion was not initialized");
    };

    let input = if src_len == 0 {
        &[][..]
    } else {
        slice::from_raw_parts((*cvt).buf, src_len)
    };
    let output = match convert_audio_buffer(
        input,
        (*cvt).src_format,
        spec.src_channels,
        spec.src_rate,
        (*cvt).dst_format,
        spec.dst_channels,
        spec.dst_rate.max(1),
    ) {
        Ok(bytes) => bytes,
        Err(message) => return crate::core::error::set_error_message(message),
    };

    let capacity = src_len.saturating_mul((*cvt).len_mult.max(1) as usize);
    if output.len() > capacity {
        return crate::core::error::set_error_message("Audio conversion buffer is too small");
    }

    if !output.is_empty() {
        slice::from_raw_parts_mut((*cvt).buf, output.len()).copy_from_slice(&output);
    }
    (*cvt).len_cvt = output.len() as libc::c_int;
    0
}

fn mix_sample(dst: f32, src: f32, volume: i32) -> f32 {
    let gain = (volume.clamp(0, SDL_MIX_MAXVOLUME as i32) as f32) / SDL_MIX_MAXVOLUME as f32;
    (dst + src * gain).clamp(-1.0, 1.0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_MixAudioFormat(
    dst: *mut u8,
    src: *const u8,
    format: SDL_AudioFormat,
    len: u32,
    volume: i32,
) {
    if len == 0 || dst.is_null() || src.is_null() {
        return;
    }

    let sample_size = match bytes_per_sample(format) {
        Some(value) => value,
        None => return,
    };
    let sample_count = len as usize / sample_size;
    let dst_bytes = slice::from_raw_parts_mut(dst, sample_count * sample_size);
    let src_bytes = slice::from_raw_parts(src, sample_count * sample_size);

    for index in 0..sample_count {
        let byte_start = index * sample_size;
        let byte_end = byte_start + sample_size;
        let mixed = mix_sample(
            decode_sample(format, &dst_bytes[byte_start..byte_end]),
            decode_sample(format, &src_bytes[byte_start..byte_end]),
            volume,
        );
        encode_sample(format, mixed, &mut dst_bytes[byte_start..byte_end]);
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_MixAudio(dst: *mut u8, src: *const u8, len: u32, volume: i32) {
    let format = crate::audio::device::legacy_output_format().unwrap_or(AUDIO_U8_FMT);
    SDL_MixAudioFormat(dst, src, format, len, volume);
}
