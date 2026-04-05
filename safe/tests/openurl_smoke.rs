#![allow(clippy::all)]

#[path = "common/testutils.rs"]
mod testutils;

use std::ptr;

use safe_sdl::core::error::SDL_GetError;
use safe_sdl::core::misc::SDL_OpenURL;

#[test]
fn openurl_rejects_null_input() {
    let _serial = testutils::serial_lock();

    unsafe {
        assert_eq!(SDL_OpenURL(ptr::null()), -1);
        let error = testutils::string_from_c(SDL_GetError());
        assert!(!error.is_empty());
    }
}

#[test]
fn openurl_attempts_to_delegate_non_null_urls() {
    let _serial = testutils::serial_lock();
    let url = testutils::cstring("https://example.com/");

    unsafe {
        let rc = SDL_OpenURL(url.as_ptr());
        if rc != 0 {
            let error = testutils::string_from_c(SDL_GetError());
            assert!(!error.is_empty());
        }
    }
}
