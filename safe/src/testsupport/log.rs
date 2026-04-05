use std::ffi::CString;
use std::mem::MaybeUninit;
use std::os::raw::c_char;
use std::ptr;

use crate::abi::generated_types::{
    self as sdl, SDL_LogCategory_SDL_LOG_CATEGORY_TEST, SDL_LogPriority,
    SDL_LogPriority_SDL_LOG_PRIORITY_ERROR, SDL_LogPriority_SDL_LOG_PRIORITY_INFO,
};

unsafe fn timestamp_string() -> String {
    let now = libc::time(ptr::null_mut());
    let mut tm = MaybeUninit::<libc::tm>::zeroed();
    if libc::localtime_r(&now, tm.as_mut_ptr()).is_null() {
        return String::new();
    }
    let mut buffer = [0 as c_char; 64];
    let format = b"%x %X\0";
    let written = libc::strftime(
        buffer.as_mut_ptr(),
        buffer.len(),
        format.as_ptr().cast(),
        tm.as_ptr(),
    );
    if written == 0 {
        String::new()
    } else {
        std::ffi::CStr::from_ptr(buffer.as_ptr())
            .to_string_lossy()
            .into_owned()
    }
}

unsafe fn log_with_priority(priority: SDL_LogPriority, message: *const c_char) {
    let timestamp = timestamp_string();
    let timestamp = CString::new(timestamp).unwrap_or_else(|_| CString::new("").unwrap());
    let info_format = b" %s: %s\0";
    let error_format = b"%s: %s\0";
    let format = if priority == SDL_LogPriority_SDL_LOG_PRIORITY_INFO {
        info_format.as_ptr()
    } else {
        error_format.as_ptr()
    };
    sdl::SDL_LogMessage(
        SDL_LogCategory_SDL_LOG_CATEGORY_TEST as libc::c_int,
        priority,
        format.cast(),
        timestamp.as_ptr(),
        message,
    );
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_LogFromBuffer(message: *const c_char) {
    log_with_priority(SDL_LogPriority_SDL_LOG_PRIORITY_INFO, message);
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_LogErrorFromBuffer(message: *const c_char) {
    log_with_priority(SDL_LogPriority_SDL_LOG_PRIORITY_ERROR, message);
}
