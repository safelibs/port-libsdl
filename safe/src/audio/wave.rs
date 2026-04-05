use std::slice;

use crate::abi::generated_types::{
    SDL_AudioSpec, SDL_RWops, Uint8, AUDIO_F32LSB, AUDIO_S16LSB, AUDIO_S32LSB, AUDIO_U8,
};
use crate::audio::normalize_audio_spec;
use crate::core::memory::{SDL_free, SDL_malloc};
use crate::core::rwops::SDL_LoadFile_RW;

const RIFF: &[u8; 4] = b"RIFF";
const WAVE: &[u8; 4] = b"WAVE";
const FMT: &[u8; 4] = b"fmt ";
const DATA: &[u8; 4] = b"data";
const FACT: &[u8; 4] = b"fact";

const WAVE_FORMAT_PCM: u16 = 0x0001;
const WAVE_FORMAT_IEEE_FLOAT: u16 = 0x0003;
const WAVE_FORMAT_IMA_ADPCM: u16 = 0x0011;
const WAVE_FORMAT_EXTENSIBLE: u16 = 0xFFFE;
const AUDIO_U8_FMT: crate::abi::generated_types::SDL_AudioFormat = AUDIO_U8 as _;
const AUDIO_S16LSB_FMT: crate::abi::generated_types::SDL_AudioFormat = AUDIO_S16LSB as _;
const AUDIO_S32LSB_FMT: crate::abi::generated_types::SDL_AudioFormat = AUDIO_S32LSB as _;
const AUDIO_F32LSB_FMT: crate::abi::generated_types::SDL_AudioFormat = AUDIO_F32LSB as _;

#[derive(Clone, Copy)]
struct WaveFormat {
    format_tag: u16,
    channels: u16,
    frequency: u32,
    block_align: u16,
    bits_per_sample: u16,
    samples_per_block: u16,
}

fn le_u16(bytes: &[u8], offset: usize) -> Result<u16, &'static str> {
    let end = offset.checked_add(2).ok_or("Truncated WAVE header")?;
    bytes
        .get(offset..end)
        .map(|slice| u16::from_le_bytes([slice[0], slice[1]]))
        .ok_or("Truncated WAVE header")
}

fn le_u32(bytes: &[u8], offset: usize) -> Result<u32, &'static str> {
    let end = offset.checked_add(4).ok_or("Truncated WAVE header")?;
    bytes
        .get(offset..end)
        .map(|slice| u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
        .ok_or("Truncated WAVE header")
}

fn checked_mul(lhs: usize, rhs: usize, message: &'static str) -> Result<usize, &'static str> {
    lhs.checked_mul(rhs).ok_or(message)
}

fn checked_add(lhs: usize, rhs: usize, message: &'static str) -> Result<usize, &'static str> {
    lhs.checked_add(rhs).ok_or(message)
}

fn parse_format(fmt: &[u8]) -> Result<WaveFormat, &'static str> {
    if fmt.len() < 16 {
        return Err("Invalid WAVE fmt chunk length");
    }

    let mut format_tag = le_u16(fmt, 0)?;
    let channels = le_u16(fmt, 2)?;
    let frequency = le_u32(fmt, 4)?;
    let block_align = le_u16(fmt, 12)?;
    let bits_per_sample = le_u16(fmt, 14)?;
    let extension_size = if fmt.len() >= 18 {
        le_u16(fmt, 16)? as usize
    } else {
        0
    };
    let mut samples_per_block = if fmt.len() >= 20 { le_u16(fmt, 18)? } else { 0 };

    if channels == 0 || channels > 8 {
        return Err("Invalid number of channels in WAVE file");
    }
    if frequency == 0 {
        return Err("Invalid sample rate in WAVE file");
    }
    if block_align == 0 {
        return Err("Invalid block alignment in WAVE file");
    }

    if format_tag == WAVE_FORMAT_EXTENSIBLE {
        if fmt.len() < 40 || extension_size < 22 {
            return Err("Invalid WAVE extensible format header");
        }
        format_tag = le_u16(fmt, 24)?;
        if samples_per_block == 0 && matches!(format_tag, WAVE_FORMAT_IMA_ADPCM) {
            samples_per_block = le_u16(fmt, 18)?;
        }
    }

    Ok(WaveFormat {
        format_tag,
        channels,
        frequency,
        block_align,
        bits_per_sample,
        samples_per_block,
    })
}

fn spec_for_format(format: WaveFormat) -> Result<SDL_AudioSpec, &'static str> {
    let audio_format = match (format.format_tag, format.bits_per_sample) {
        (WAVE_FORMAT_PCM, 8) => AUDIO_U8_FMT,
        (WAVE_FORMAT_PCM, 16) => AUDIO_S16LSB_FMT,
        (WAVE_FORMAT_PCM, 32) => AUDIO_S32LSB_FMT,
        (WAVE_FORMAT_IEEE_FLOAT, 32) => AUDIO_F32LSB_FMT,
        (WAVE_FORMAT_IMA_ADPCM, _) => AUDIO_S16LSB_FMT,
        _ => return Err("Unsupported WAVE encoding"),
    };

    let mut spec = SDL_AudioSpec {
        freq: format.frequency as i32,
        format: audio_format,
        channels: format.channels as u8,
        silence: 0,
        samples: 4096,
        padding: 0,
        size: 0,
        callback: None,
        userdata: std::ptr::null_mut(),
    };
    normalize_audio_spec(&mut spec).map_err(|_| "Invalid WAVE audio specification")?;
    Ok(spec)
}

fn copy_pcm_payload(format: WaveFormat, data: &[u8]) -> Result<Vec<u8>, &'static str> {
    let frame_size = match (format.format_tag, format.bits_per_sample) {
        (WAVE_FORMAT_PCM, 8) => format.channels as usize,
        (WAVE_FORMAT_PCM, 16) => {
            checked_mul(format.channels as usize, 2, "WAVE data is too large")?
        }
        (WAVE_FORMAT_PCM, 32) | (WAVE_FORMAT_IEEE_FLOAT, 32) => {
            checked_mul(format.channels as usize, 4, "WAVE data is too large")?
        }
        _ => return Err("Unsupported WAVE encoding"),
    };

    if frame_size == 0 || format.block_align as usize != frame_size || data.len() % frame_size != 0
    {
        return Err("Invalid WAVE PCM data alignment");
    }

    Ok(data.to_vec())
}

fn ima_step(index: &mut i8, last_sample: i16, nibble: u8) -> i16 {
    const INDEX_TABLE: [i8; 16] = [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];
    const STEP_TABLE: [u16; 89] = [
        7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60,
        66, 73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371,
        408, 449, 494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878,
        2066, 2272, 2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845,
        8630, 9493, 10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086,
        29794, 32767,
    ];

    let clamped_index = (*index).clamp(0, 88);
    let step = STEP_TABLE[clamped_index as usize] as i32;
    *index = clamped_index.saturating_add(INDEX_TABLE[nibble as usize]);

    let mut delta = step >> 3;
    if nibble & 0x04 != 0 {
        delta += step;
    }
    if nibble & 0x02 != 0 {
        delta += step >> 1;
    }
    if nibble & 0x01 != 0 {
        delta += step >> 2;
    }
    if nibble & 0x08 != 0 {
        delta = -delta;
    }

    let sample = (last_sample as i32 + delta).clamp(i16::MIN as i32, i16::MAX as i32);
    sample as i16
}

fn decode_ima_adpcm(
    format: WaveFormat,
    data: &[u8],
    fact_samples: Option<u32>,
) -> Result<Vec<u8>, &'static str> {
    let channels = format.channels as usize;
    if channels == 0 || channels > 2 {
        return Err("Unsupported number of IMA ADPCM channels");
    }
    if format.bits_per_sample != 4 {
        return Err("Invalid IMA ADPCM bits per sample");
    }

    let header_size = checked_mul(channels, 4, "IMA ADPCM block size overflow")?;
    let block_align = format.block_align as usize;
    if block_align < header_size || block_align % 4 != 0 {
        return Err("Invalid IMA ADPCM block size");
    }
    if data.is_empty() {
        return Ok(Vec::new());
    }
    if data.len() % block_align != 0 {
        return Err("Truncated IMA ADPCM block");
    }

    let block_data_samples = ((block_align - header_size) * 8)
        / checked_mul(channels, 4, "IMA ADPCM sample-count overflow")?;
    let samples_per_block = if format.samples_per_block == 0 {
        block_data_samples + 1
    } else {
        format.samples_per_block as usize
    };
    if samples_per_block == 0 || block_data_samples < samples_per_block.saturating_sub(1) {
        return Err("Invalid number of samples per IMA ADPCM block");
    }

    let block_count = data.len() / block_align;
    let mut total_frames = checked_mul(
        block_count,
        samples_per_block,
        "IMA ADPCM sample-count overflow",
    )?;
    if let Some(fact_samples) = fact_samples {
        let fact_samples = fact_samples as usize;
        if fact_samples > total_frames {
            return Err("Invalid WAVE fact chunk sample count");
        }
        total_frames = fact_samples;
    }

    let total_samples = checked_mul(total_frames, channels, "IMA ADPCM sample-count overflow")?;
    let output_bytes = checked_mul(total_samples, 2, "IMA ADPCM output size overflow")?;
    if output_bytes > u32::MAX as usize {
        return Err("WAVE file too large");
    }

    let mut decoded_samples = Vec::<i16>::with_capacity(total_samples);

    for block_index in 0..block_count {
        let block = &data[block_index * block_align..(block_index + 1) * block_align];
        if decoded_samples.len() / channels >= total_frames {
            break;
        }

        let mut last_samples = vec![0i16; channels];
        let mut indices = vec![0i8; channels];
        for channel in 0..channels {
            let offset = channel * 4;
            last_samples[channel] = i16::from_le_bytes([block[offset], block[offset + 1]]);
            indices[channel] = block[offset + 2] as i8;
        }

        if decoded_samples.len() / channels < total_frames {
            decoded_samples.extend_from_slice(&last_samples);
        }

        let mut position = header_size;
        let mut block_frames_left = total_frames.saturating_sub(decoded_samples.len() / channels);
        block_frames_left = block_frames_left.min(samples_per_block.saturating_sub(1));

        while block_frames_left > 0 {
            let subblock_samples = block_frames_left.min(8);
            let mut frame_buffer = vec![0i16; subblock_samples * channels];

            for channel in 0..channels {
                let next = checked_add(position, 4, "Truncated IMA ADPCM block")?;
                if next > block.len() {
                    return Err("Truncated IMA ADPCM block");
                }
                let packed = &block[position..next];
                position = next;

                let mut sample = last_samples[channel];
                for sample_index in 0..subblock_samples {
                    let byte = packed[sample_index / 2];
                    let nibble = if sample_index % 2 == 0 {
                        byte & 0x0f
                    } else {
                        byte >> 4
                    };
                    sample = ima_step(&mut indices[channel], sample, nibble);
                    frame_buffer[sample_index * channels + channel] = sample;
                }
                last_samples[channel] = sample;
            }

            decoded_samples.extend_from_slice(&frame_buffer);
            block_frames_left -= subblock_samples;
        }
    }

    decoded_samples.truncate(total_samples);
    let mut output = Vec::with_capacity(output_bytes);
    for sample in decoded_samples {
        output.extend_from_slice(&sample.to_le_bytes());
    }
    Ok(output)
}

fn parse_wave(bytes: &[u8]) -> Result<(SDL_AudioSpec, Vec<u8>), &'static str> {
    if bytes.len() < 12 || &bytes[..4] != RIFF || &bytes[8..12] != WAVE {
        return Err("File is not a WAVE file");
    }

    let mut fmt_chunk = None;
    let mut data_chunk = None;
    let mut fact_samples = None;
    let mut offset = 12usize;

    while let Some(header_end) = offset.checked_add(8) {
        if header_end > bytes.len() {
            break;
        }
        let chunk_id = &bytes[offset..offset + 4];
        let chunk_size = le_u32(bytes, offset + 4)? as usize;
        let data_start = header_end;
        let data_end = checked_add(data_start, chunk_size, "Truncated WAVE chunk")?;
        if data_end > bytes.len() {
            return Err("Truncated WAVE chunk");
        }

        if chunk_id == FMT && fmt_chunk.is_none() {
            fmt_chunk = Some(&bytes[data_start..data_end]);
        } else if chunk_id == DATA && data_chunk.is_none() {
            data_chunk = Some(&bytes[data_start..data_end]);
        } else if chunk_id == FACT && fact_samples.is_none() {
            if chunk_size < 4 {
                return Err("Invalid WAVE fact chunk");
            }
            fact_samples = Some(le_u32(bytes, data_start)?);
        }

        offset = checked_add(data_end, chunk_size & 1, "Truncated WAVE chunk")?;
    }

    let fmt_chunk = fmt_chunk.ok_or("Missing fmt chunk in WAVE file")?;
    let data_chunk = data_chunk.ok_or("Missing data chunk in WAVE file")?;
    let format = parse_format(fmt_chunk)?;
    let spec = spec_for_format(format)?;

    let audio = match format.format_tag {
        WAVE_FORMAT_PCM | WAVE_FORMAT_IEEE_FLOAT => copy_pcm_payload(format, data_chunk)?,
        WAVE_FORMAT_IMA_ADPCM => decode_ima_adpcm(format, data_chunk, fact_samples)?,
        _ => return Err("Unsupported WAVE encoding"),
    };

    Ok((spec, audio))
}

#[no_mangle]
pub unsafe extern "C" fn SDL_LoadWAV_RW(
    src: *mut SDL_RWops,
    freesrc: libc::c_int,
    spec: *mut SDL_AudioSpec,
    audio_buf: *mut *mut Uint8,
    audio_len: *mut u32,
) -> *mut SDL_AudioSpec {
    if src.is_null() {
        let _ = crate::core::error::invalid_param_error("src");
        return std::ptr::null_mut();
    }
    if spec.is_null() {
        let _ = crate::core::error::invalid_param_error("spec");
        return std::ptr::null_mut();
    }
    if audio_buf.is_null() {
        let _ = crate::core::error::invalid_param_error("audio_buf");
        return std::ptr::null_mut();
    }
    if audio_len.is_null() {
        let _ = crate::core::error::invalid_param_error("audio_len");
        return std::ptr::null_mut();
    }

    let mut size = 0usize;
    let loaded = SDL_LoadFile_RW(src, &mut size, freesrc) as *mut u8;
    if loaded.is_null() {
        return std::ptr::null_mut();
    }

    let bytes = slice::from_raw_parts(loaded, size).to_vec();
    SDL_free(loaded.cast());

    let (parsed_spec, audio) = match parse_wave(&bytes) {
        Ok(value) => value,
        Err(message) => {
            let _ = crate::core::error::set_error_message(message);
            return std::ptr::null_mut();
        }
    };

    *spec = parsed_spec;
    if audio.is_empty() {
        *audio_buf = std::ptr::null_mut();
        *audio_len = 0;
        return spec;
    }

    let allocation = SDL_malloc(audio.len()) as *mut u8;
    if allocation.is_null() {
        let _ = crate::core::error::out_of_memory_error();
        return std::ptr::null_mut();
    }
    slice::from_raw_parts_mut(allocation, audio.len()).copy_from_slice(&audio);
    *audio_buf = allocation;
    *audio_len = audio.len() as u32;
    spec
}

#[no_mangle]
pub unsafe extern "C" fn SDL_FreeWAV(audio_buf: *mut Uint8) {
    SDL_free(audio_buf.cast());
}
