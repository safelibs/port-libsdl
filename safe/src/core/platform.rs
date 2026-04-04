static PLATFORM: &[u8; 6] = b"Linux\0";

#[no_mangle]
pub unsafe extern "C" fn SDL_GetPlatform() -> *const libc::c_char {
    PLATFORM.as_ptr().cast()
}
