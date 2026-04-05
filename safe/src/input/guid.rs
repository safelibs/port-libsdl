use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::ptr;

use crate::abi::generated_types::{SDL_JoystickGUID, Uint16, SDL_GUID};

const HEX: &[u8; 16] = b"0123456789abcdef";

#[no_mangle]
pub unsafe extern "C" fn SDL_GUIDToString(guid: SDL_GUID, pszGUID: *mut c_char, cbGUID: c_int) {
    if pszGUID.is_null() || cbGUID <= 0 {
        return;
    }

    let mut out = pszGUID.cast::<u8>();
    let max_bytes = ((cbGUID - 1) / 2).max(0) as usize;
    for byte in guid.data.into_iter().take(max_bytes) {
        *out = HEX[(byte >> 4) as usize];
        out = out.add(1);
        *out = HEX[(byte & 0x0f) as usize];
        out = out.add(1);
    }
    *out.cast::<c_char>() = 0;
}

fn nibble(value: u8) -> u8 {
    match value {
        b'0'..=b'9' => value - b'0',
        b'A'..=b'F' => value - b'A' + 0x0a,
        b'a'..=b'f' => value - b'a' + 0x0a,
        _ => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GUIDFromString(pchGUID: *const c_char) -> SDL_GUID {
    let mut guid = SDL_GUID { data: [0; 16] };
    if pchGUID.is_null() {
        return guid;
    }

    let bytes = CStr::from_ptr(pchGUID).to_bytes();
    let len = bytes.len() & !1usize;
    for (index, chunk) in bytes[..len]
        .chunks_exact(2)
        .take(guid.data.len())
        .enumerate()
    {
        guid.data[index] = (nibble(chunk[0]) << 4) | nibble(chunk[1]);
    }
    guid
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickGetGUIDString(
    guid: SDL_JoystickGUID,
    pszGUID: *mut c_char,
    cbGUID: c_int,
) {
    SDL_GUIDToString(guid, pszGUID, cbGUID);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickGetGUIDFromString(pchGUID: *const c_char) -> SDL_JoystickGUID {
    SDL_GUIDFromString(pchGUID)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetJoystickGUIDInfo(
    guid: SDL_JoystickGUID,
    vendor: *mut Uint16,
    product: *mut Uint16,
    version: *mut Uint16,
    crc16: *mut Uint16,
) {
    let (vendor_value, product_value, version_value, crc_value) = super::decode_guid_info(guid);
    if !vendor.is_null() {
        ptr::write(vendor, vendor_value);
    }
    if !product.is_null() {
        ptr::write(product, product_value);
    }
    if !version.is_null() {
        ptr::write(version, version_value);
    }
    if !crc16.is_null() {
        ptr::write(crc16, crc_value);
    }
}
