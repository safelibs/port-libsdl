#![cfg(feature = "host-video-tests")]
#![allow(clippy::all)]

#[path = "common/testutils.rs"]
mod testutils;

use safe_sdl::core::rwops::SDL_RWFromConstMem;
use safe_sdl::video::bmp::SDL_LoadBMP_RW;

fn make_bmp(
    width: i32,
    height: i32,
    bits_per_pixel: u16,
    image_size: u32,
    pixel_offset: u32,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"BM");
    bytes.extend_from_slice(&(54u32 + image_size).to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&pixel_offset.to_le_bytes());
    bytes.extend_from_slice(&40u32.to_le_bytes());
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&height.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&bits_per_pixel.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&image_size.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.resize(54usize.saturating_add(image_size as usize), 0);
    bytes
}

unsafe fn load_from_bytes(bytes: &[u8]) -> *mut safe_sdl::abi::generated_types::SDL_Surface {
    let rw = SDL_RWFromConstMem(bytes.as_ptr().cast(), bytes.len() as i32);
    assert!(!rw.is_null(), "{}", testutils::current_error());
    SDL_LoadBMP_RW(rw, 1)
}

#[test]
fn bmp_parser_rejects_truncated_headers() {
    let _serial = testutils::serial_lock();

    unsafe {
        assert!(load_from_bytes(b"BM").is_null());
        assert!(!testutils::current_error().is_empty());

        let truncated = make_bmp(2, 2, 32, 16, 54);
        assert!(load_from_bytes(&truncated[..20]).is_null());
        assert!(!testutils::current_error().is_empty());
    }
}

#[test]
fn bmp_parser_rejects_oversized_dimensions_and_row_stride_claims() {
    let _serial = testutils::serial_lock();

    unsafe {
        let oversized = make_bmp(i32::MAX, i32::MAX, 32, 0, 54);
        assert!(load_from_bytes(&oversized).is_null());
        assert!(!testutils::current_error().is_empty());

        let malicious_stride = make_bmp(4096, 4096, 32, 4, 54);
        assert!(load_from_bytes(&malicious_stride).is_null());
        assert!(!testutils::current_error().is_empty());
    }
}

#[test]
fn bmp_parser_rejects_offsets_past_end_of_stream() {
    let _serial = testutils::serial_lock();

    unsafe {
        let bad_offset = make_bmp(4, 4, 32, 16, 4096);
        assert!(load_from_bytes(&bad_offset).is_null());
        assert!(!testutils::current_error().is_empty());
    }
}

#[test]
fn bmp_parser_rejects_truncated_pixel_payloads() {
    let _serial = testutils::serial_lock();

    unsafe {
        let truncated = make_bmp(4, 4, 32, 64, 54);
        assert!(load_from_bytes(&truncated[..70]).is_null());
        assert!(!testutils::current_error().is_empty());
    }
}
