use std::sync::OnceLock;

use crate::abi::generated_types::{SDL_Finger, SDL_TouchID};

struct TouchApi {
    get_num_touch_devices: unsafe extern "C" fn() -> libc::c_int,
    get_touch_device: unsafe extern "C" fn(libc::c_int) -> SDL_TouchID,
    get_touch_finger: unsafe extern "C" fn(SDL_TouchID, libc::c_int) -> *mut SDL_Finger,
    get_num_touch_fingers: unsafe extern "C" fn(SDL_TouchID) -> libc::c_int,
    get_touch_name: unsafe extern "C" fn(libc::c_int) -> *const libc::c_char,
}

fn api() -> &'static TouchApi {
    static API: OnceLock<TouchApi> = OnceLock::new();
    API.get_or_init(|| TouchApi {
        get_num_touch_devices: crate::video::load_symbol(b"SDL_GetNumTouchDevices\0"),
        get_touch_device: crate::video::load_symbol(b"SDL_GetTouchDevice\0"),
        get_touch_finger: crate::video::load_symbol(b"SDL_GetTouchFinger\0"),
        get_num_touch_fingers: crate::video::load_symbol(b"SDL_GetNumTouchFingers\0"),
        get_touch_name: crate::video::load_symbol(b"SDL_GetTouchName\0"),
    })
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetNumTouchDevices() -> libc::c_int {
    crate::video::clear_real_error();
    (api().get_num_touch_devices)()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetTouchDevice(index: libc::c_int) -> SDL_TouchID {
    crate::video::clear_real_error();
    (api().get_touch_device)(index)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetTouchFinger(
    touchID: SDL_TouchID,
    index: libc::c_int,
) -> *mut SDL_Finger {
    crate::video::clear_real_error();
    (api().get_touch_finger)(touchID, index)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetNumTouchFingers(touchID: SDL_TouchID) -> libc::c_int {
    crate::video::clear_real_error();
    (api().get_num_touch_fingers)(touchID)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetTouchName(index: libc::c_int) -> *const libc::c_char {
    crate::video::clear_real_error();
    (api().get_touch_name)(index)
}
