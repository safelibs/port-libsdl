use std::os::raw::c_int;

use crate::testsupport::SDLTest_Crc32Context;

const CRC32_POLY: u32 = 0xEDB8_8320;

#[no_mangle]
pub unsafe extern "C" fn SDLTest_Crc32Init(crcContext: *mut SDLTest_Crc32Context) -> c_int {
    if crcContext.is_null() {
        return -1;
    }
    for i in 0..256usize {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                (c >> 1) ^ CRC32_POLY
            } else {
                c >> 1
            };
        }
        (*crcContext).crc32_table[i] = c;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_Crc32CalcStart(
    crcContext: *mut SDLTest_Crc32Context,
    crc32: *mut u32,
) -> c_int {
    if crcContext.is_null() || crc32.is_null() {
        if !crc32.is_null() {
            *crc32 = 0;
        }
        return -1;
    }
    *crc32 = 0xffff_ffff;
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_Crc32CalcBuffer(
    crcContext: *mut SDLTest_Crc32Context,
    inBuf: *mut u8,
    mut inLen: u32,
    crc32: *mut u32,
) -> c_int {
    if crcContext.is_null() || inBuf.is_null() || crc32.is_null() {
        if !crc32.is_null() && crcContext.is_null() {
            *crc32 = 0;
        }
        return -1;
    }
    let mut crc = *crc32;
    let mut ptr = inBuf;
    while inLen > 0 {
        crc = ((crc >> 8) & 0x00ff_ffff)
            ^ (*crcContext).crc32_table[((crc ^ (*ptr as u32)) & 0xff) as usize];
        ptr = ptr.add(1);
        inLen -= 1;
    }
    *crc32 = crc;
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_Crc32CalcEnd(
    crcContext: *mut SDLTest_Crc32Context,
    crc32: *mut u32,
) -> c_int {
    if crcContext.is_null() || crc32.is_null() {
        if !crc32.is_null() {
            *crc32 = 0;
        }
        return -1;
    }
    *crc32 = !*crc32;
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_Crc32Calc(
    crcContext: *mut SDLTest_Crc32Context,
    inBuf: *mut u8,
    inLen: u32,
    crc32: *mut u32,
) -> c_int {
    if SDLTest_Crc32CalcStart(crcContext, crc32) != 0 {
        return -1;
    }
    if SDLTest_Crc32CalcBuffer(crcContext, inBuf, inLen, crc32) != 0 {
        return -1;
    }
    SDLTest_Crc32CalcEnd(crcContext, crc32)
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_Crc32Done(crcContext: *mut SDLTest_Crc32Context) -> c_int {
    if crcContext.is_null() {
        -1
    } else {
        0
    }
}
