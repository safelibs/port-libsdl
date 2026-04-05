use crate::abi::generated_types::{SDL_Finger, SDL_TouchID};

#[no_mangle]
pub unsafe extern "C" fn SDL_GetNumTouchDevices() -> libc::c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetTouchDevice(_index: libc::c_int) -> SDL_TouchID {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetTouchFinger(
    _touchID: SDL_TouchID,
    _index: libc::c_int,
) -> *mut SDL_Finger {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetNumTouchFingers(_touchID: SDL_TouchID) -> libc::c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetTouchName(_index: libc::c_int) -> *const libc::c_char {
    std::ptr::null()
}
