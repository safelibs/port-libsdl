use std::os::raw::{c_char, c_int};
use std::ptr;

use crate::abi::generated_types::{wchar_t, SDL_bool, SDL_hid_device, SDL_hid_device_info, Uint32};
use crate::core::error::{invalid_param_error, set_error_message};

fn unsupported() -> c_int {
    set_error_message("HIDAPI devices are not available in this runtime")
}

fn clear_wide_buffer(buffer: *mut wchar_t, maxlen: usize) {
    if !buffer.is_null() && maxlen > 0 {
        unsafe {
            *buffer = 0;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_init() -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_exit() -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_device_change_count() -> Uint32 {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_enumerate(
    _vendor_id: libc::c_ushort,
    _product_id: libc::c_ushort,
) -> *mut SDL_hid_device_info {
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_free_enumeration(_devs: *mut SDL_hid_device_info) {}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_open(
    _vendor_id: libc::c_ushort,
    _product_id: libc::c_ushort,
    _serial_number: *const wchar_t,
) -> *mut SDL_hid_device {
    let _ = unsupported();
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_open_path(
    _path: *const c_char,
    _exclusive: c_int,
) -> *mut SDL_hid_device {
    let _ = unsupported();
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_write(
    dev: *mut SDL_hid_device,
    _data: *const u8,
    _length: usize,
) -> c_int {
    if dev.is_null() {
        return invalid_param_error("dev");
    }
    unsupported()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_read_timeout(
    dev: *mut SDL_hid_device,
    _data: *mut u8,
    _length: usize,
    _milliseconds: c_int,
) -> c_int {
    if dev.is_null() {
        return invalid_param_error("dev");
    }
    unsupported()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_read(
    dev: *mut SDL_hid_device,
    _data: *mut u8,
    _length: usize,
) -> c_int {
    if dev.is_null() {
        return invalid_param_error("dev");
    }
    unsupported()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_set_nonblocking(
    dev: *mut SDL_hid_device,
    _nonblock: c_int,
) -> c_int {
    if dev.is_null() {
        return invalid_param_error("dev");
    }
    unsupported()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_send_feature_report(
    dev: *mut SDL_hid_device,
    _data: *const u8,
    _length: usize,
) -> c_int {
    if dev.is_null() {
        return invalid_param_error("dev");
    }
    unsupported()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_get_feature_report(
    dev: *mut SDL_hid_device,
    _data: *mut u8,
    _length: usize,
) -> c_int {
    if dev.is_null() {
        return invalid_param_error("dev");
    }
    unsupported()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_close(_dev: *mut SDL_hid_device) {}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_get_manufacturer_string(
    dev: *mut SDL_hid_device,
    string: *mut wchar_t,
    maxlen: usize,
) -> c_int {
    clear_wide_buffer(string, maxlen);
    if dev.is_null() {
        return invalid_param_error("dev");
    }
    unsupported()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_get_product_string(
    dev: *mut SDL_hid_device,
    string: *mut wchar_t,
    maxlen: usize,
) -> c_int {
    clear_wide_buffer(string, maxlen);
    if dev.is_null() {
        return invalid_param_error("dev");
    }
    unsupported()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_get_serial_number_string(
    dev: *mut SDL_hid_device,
    string: *mut wchar_t,
    maxlen: usize,
) -> c_int {
    clear_wide_buffer(string, maxlen);
    if dev.is_null() {
        return invalid_param_error("dev");
    }
    unsupported()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_get_indexed_string(
    dev: *mut SDL_hid_device,
    _string_index: c_int,
    string: *mut wchar_t,
    maxlen: usize,
) -> c_int {
    clear_wide_buffer(string, maxlen);
    if dev.is_null() {
        return invalid_param_error("dev");
    }
    unsupported()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_ble_scan(_active: SDL_bool) {}
