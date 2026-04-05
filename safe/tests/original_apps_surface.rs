#![allow(clippy::all)]

#[path = "common/testutils.rs"]
mod testutils;

use std::ptr;

use safe_sdl::abi::generated_types::SDL_Rect;
use safe_sdl::core::rwops::{SDL_RWFromFile, SDL_RWclose, SDL_RWread, SDL_RWseek, SDL_RWwrite};
use safe_sdl::video::rect::{SDL_IntersectRect, SDL_IntersectRectAndLine};

#[test]
fn testfile_like_rwops_modes_and_seeks_match_original_app_expectations() {
    let _serial = testutils::serial_lock();
    let tempdir = tempfile::tempdir().expect("create tempdir");
    let path = tempdir.path().join("sdldata1");
    let path_c = testutils::cstring(path.to_str().expect("utf-8 temp path"));

    unsafe {
        assert!(SDL_RWFromFile(ptr::null(), ptr::null()).is_null());

        let write_mode = testutils::cstring("wb");
        let rw = SDL_RWFromFile(path_c.as_ptr(), write_mode.as_ptr());
        assert!(!rw.is_null(), "{}", testutils::current_error());
        assert_eq!(SDL_RWwrite(rw, b"1234567890".as_ptr().cast(), 1, 10), 10);
        assert_eq!(SDL_RWwrite(rw, b"1234567890".as_ptr().cast(), 1, 10), 10);
        assert_eq!(SDL_RWwrite(rw, b"1234567".as_ptr().cast(), 1, 7), 7);
        assert_eq!(SDL_RWseek(rw, 0, libc::SEEK_SET), 0);
        let mut denied = [0u8; 1];
        assert_eq!(SDL_RWread(rw, denied.as_mut_ptr().cast(), 1, 1), 0);
        assert_eq!(SDL_RWclose(rw), 0);

        let read_mode = testutils::cstring("rb");
        let rw = SDL_RWFromFile(path_c.as_ptr(), read_mode.as_ptr());
        assert!(!rw.is_null(), "{}", testutils::current_error());
        assert_eq!(SDL_RWseek(rw, -7, libc::SEEK_END), 20);
        let mut tail = [0u8; 7];
        assert_eq!(
            SDL_RWread(rw, tail.as_mut_ptr().cast(), 1, tail.len()),
            tail.len()
        );
        assert_eq!(&tail, b"1234567");
        assert_eq!(SDL_RWseek(rw, -27, libc::SEEK_CUR), 0);
        let mut chunk = [0u8; 20];
        assert_eq!(SDL_RWread(rw, chunk.as_mut_ptr().cast(), 10, 3), 2);
        assert_eq!(&chunk[..20], b"12345678901234567890");
        assert_eq!(SDL_RWclose(rw), 0);
    }
}

#[test]
fn testintersections_like_line_and_rect_math_matches_original_app_render_logic() {
    let _serial = testutils::serial_lock();

    unsafe {
        let rect_a = SDL_Rect {
            x: 10,
            y: 10,
            w: 50,
            h: 30,
        };
        let rect_b = SDL_Rect {
            x: 40,
            y: 0,
            w: 30,
            h: 40,
        };
        let mut overlap = SDL_Rect {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        };
        assert_ne!(SDL_IntersectRect(&rect_a, &rect_b, &mut overlap), 0);
        assert_eq!(
            (overlap.x, overlap.y, overlap.w, overlap.h),
            (40, 10, 20, 30)
        );

        let mut x1 = 0;
        let mut y1 = 0;
        let mut x2 = 100;
        let mut y2 = 60;
        assert_ne!(
            SDL_IntersectRectAndLine(&rect_a, &mut x1, &mut y1, &mut x2, &mut y2),
            0
        );
        assert_eq!((x1, y1, x2, y2), (16, 10, 59, 35));
    }
}
