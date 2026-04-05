#![allow(clippy::all)]

#[path = "common/testutils.rs"]
mod testutils;

use std::mem::MaybeUninit;
use std::ptr;

use safe_sdl::abi::generated_types::{SDL_AudioSpec, AUDIO_S16LSB};
use safe_sdl::audio::wave::{SDL_FreeWAV, SDL_LoadWAV_RW};
use safe_sdl::core::rwops::SDL_RWFromConstMem;

const SDL_SAMPLE_WAV: &[u8] = include_bytes!("../../original/test/sample.wav");

fn chunk(id: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(8 + payload.len() + (payload.len() & 1));
    bytes.extend_from_slice(id);
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(payload);
    if payload.len() % 2 != 0 {
        bytes.push(0);
    }
    bytes
}

fn riff_wave(chunks: &[Vec<u8>]) -> Vec<u8> {
    let payload_len = chunks.iter().map(Vec::len).sum::<usize>();
    let mut bytes = Vec::with_capacity(12 + payload_len);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(4u32 + payload_len as u32).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    for chunk in chunks {
        bytes.extend_from_slice(chunk);
    }
    bytes
}

fn ima_adpcm_fmt(
    channels: u16,
    sample_rate: u32,
    block_align: u16,
    samples_per_block: u16,
) -> Vec<u8> {
    let avg_bytes_per_second = if samples_per_block == 0 {
        0
    } else {
        sample_rate
            .saturating_mul(block_align as u32)
            .saturating_div(samples_per_block as u32)
    };
    let mut fmt = Vec::with_capacity(20);
    fmt.extend_from_slice(&0x0011u16.to_le_bytes());
    fmt.extend_from_slice(&channels.to_le_bytes());
    fmt.extend_from_slice(&sample_rate.to_le_bytes());
    fmt.extend_from_slice(&avg_bytes_per_second.to_le_bytes());
    fmt.extend_from_slice(&block_align.to_le_bytes());
    fmt.extend_from_slice(&4u16.to_le_bytes());
    fmt.extend_from_slice(&2u16.to_le_bytes());
    fmt.extend_from_slice(&samples_per_block.to_le_bytes());
    fmt
}

unsafe fn load_wave(bytes: &[u8]) -> (*mut SDL_AudioSpec, *mut u8, u32) {
    let rw = SDL_RWFromConstMem(bytes.as_ptr().cast(), bytes.len() as i32);
    assert!(!rw.is_null(), "{}", testutils::current_error());

    let mut spec = MaybeUninit::<SDL_AudioSpec>::zeroed();
    let mut audio = ptr::null_mut();
    let mut len = 0u32;
    let result = SDL_LoadWAV_RW(rw, 1, spec.as_mut_ptr(), &mut audio, &mut len);
    (result, audio, len)
}

#[test]
fn wave_ima_adpcm_decoder_accepts_a_minimal_valid_block() {
    let _serial = testutils::serial_lock();

    let fmt = ima_adpcm_fmt(1, 22_050, 8, 9);
    let data = [0u8; 8];
    let wave = riff_wave(&[
        chunk(b"fmt ", &fmt),
        chunk(b"fact", &9u32.to_le_bytes()),
        chunk(b"data", &data),
    ]);

    unsafe {
        let (result, audio, len) = load_wave(&wave);
        assert!(!result.is_null(), "{}", testutils::current_error());
        let spec = *result;
        assert_eq!(spec.format, AUDIO_S16LSB as _);
        assert_eq!(spec.channels, 1);
        assert_eq!(len, 18);
        SDL_FreeWAV(audio);
    }
}

#[test]
fn wave_ima_adpcm_decoder_rejects_malformed_block_sizes_and_truncation() {
    let _serial = testutils::serial_lock();

    let malformed_block = riff_wave(&[
        chunk(b"fmt ", &ima_adpcm_fmt(2, 44_100, 4, 9)),
        chunk(b"data", &[0u8; 4]),
    ]);
    let truncated_block = riff_wave(&[
        chunk(b"fmt ", &ima_adpcm_fmt(1, 22_050, 8, 9)),
        chunk(b"data", &[0u8; 7]),
    ]);

    unsafe {
        let (result, _, _) = load_wave(&malformed_block);
        assert!(result.is_null());
        assert!(!testutils::current_error().is_empty());

        let (result, _, _) = load_wave(&truncated_block);
        assert!(result.is_null());
        assert!(!testutils::current_error().is_empty());
    }
}

#[test]
fn wave_ima_adpcm_decoder_rejects_impossible_sample_counts_before_allocation() {
    let _serial = testutils::serial_lock();

    let impossible_samples = riff_wave(&[
        chunk(b"fmt ", &ima_adpcm_fmt(1, 22_050, 8, u16::MAX)),
        chunk(b"data", &[0u8; 8]),
    ]);
    let invalid_fact = riff_wave(&[
        chunk(b"fmt ", &ima_adpcm_fmt(1, 22_050, 8, 9)),
        chunk(b"fact", &65_535u32.to_le_bytes()),
        chunk(b"data", &[0u8; 8]),
    ]);

    unsafe {
        let (result, _, _) = load_wave(&impossible_samples);
        assert!(
            result.is_null(),
            "CVE-2019-13626 class regression: impossible decompression request must fail"
        );
        assert!(!testutils::current_error().is_empty());

        let (result, audio, len) = load_wave(&invalid_fact);
        assert!(
            !result.is_null(),
            "oversized fact chunks must not trigger a decompression-sized allocation"
        );
        assert_eq!(len, 18);
        SDL_FreeWAV(audio);
    }
}

#[test]
fn wave_ms_adpcm_decoder_accepts_checked_in_sample_asset() {
    let _serial = testutils::serial_lock();

    unsafe {
        let (result, audio, len) = load_wave(SDL_SAMPLE_WAV);
        assert!(!result.is_null(), "{}", testutils::current_error());

        let spec = *result;
        assert_eq!(spec.format, AUDIO_S16LSB as _);
        assert_eq!(spec.channels, 1);
        assert_eq!(spec.freq, 22_050);
        assert!(len > 0);

        SDL_FreeWAV(audio);
    }
}
