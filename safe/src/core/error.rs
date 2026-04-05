use std::ffi::CString;
use std::ptr;

use crate::abi::generated_types::{
    SDL_errorcode, SDL_errorcode_SDL_EFREAD, SDL_errorcode_SDL_EFSEEK, SDL_errorcode_SDL_EFWRITE,
    SDL_errorcode_SDL_ENOMEM, SDL_errorcode_SDL_UNSUPPORTED,
};

unsafe extern "C" {
    fn safe_sdl_store_error_message(message: *const libc::c_char);
    fn safe_sdl_get_error_message() -> *const libc::c_char;
    fn safe_sdl_clear_error_message();
    fn safe_sdl_error_is_active() -> libc::c_int;
}

fn write_error_string(errstr: *mut libc::c_char, maxlen: libc::c_int, text: *const libc::c_char) {
    if errstr.is_null() || maxlen <= 0 {
        return;
    }
    let maxlen = maxlen as usize;
    let src_len = if text.is_null() {
        0
    } else {
        unsafe { libc::strlen(text) }
    };
    let copy_len = src_len.min(maxlen.saturating_sub(1));
    if copy_len > 0 && !text.is_null() {
        unsafe {
            ptr::copy_nonoverlapping(text, errstr, copy_len);
        }
    }
    unsafe {
        *errstr.add(copy_len) = 0;
    }
}

pub(crate) fn set_error_message(message: &str) -> libc::c_int {
    let c_message = CString::new(message).unwrap_or_default();
    unsafe {
        safe_sdl_store_error_message(c_message.as_ptr());
    }
    -1
}

pub(crate) fn invalid_param_error(param: &str) -> libc::c_int {
    set_error_message(&format!("Parameter '{param}' is invalid"))
}

pub(crate) fn out_of_memory_error() -> libc::c_int {
    set_error_message("Out of memory")
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetError() -> *const libc::c_char {
    if safe_sdl_error_is_active() != 0 {
        safe_sdl_get_error_message()
    } else {
        let host = crate::video::real_error_ptr();
        if host.is_null() {
            b"\0".as_ptr().cast()
        } else {
            host
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetErrorMsg(
    errstr: *mut libc::c_char,
    maxlen: libc::c_int,
) -> *mut libc::c_char {
    let text = if safe_sdl_error_is_active() != 0 {
        safe_sdl_get_error_message()
    } else {
        let host = crate::video::real_error_ptr();
        if host.is_null() {
            b"\0".as_ptr().cast()
        } else {
            host
        }
    };
    write_error_string(errstr, maxlen, text);
    errstr
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ClearError() {
    safe_sdl_clear_error_message();
    crate::video::clear_real_error();
}

#[no_mangle]
pub unsafe extern "C" fn SDL_Error(code: SDL_errorcode) -> libc::c_int {
    match code {
        SDL_errorcode_SDL_ENOMEM => out_of_memory_error(),
        SDL_errorcode_SDL_EFREAD => set_error_message("Error reading from datastream"),
        SDL_errorcode_SDL_EFWRITE => set_error_message("Error writing to datastream"),
        SDL_errorcode_SDL_EFSEEK => set_error_message("Error seeking in datastream"),
        SDL_errorcode_SDL_UNSUPPORTED => set_error_message("That operation is not supported"),
        _ => set_error_message("Unknown SDL error"),
    }
}
