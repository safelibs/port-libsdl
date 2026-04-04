use crate::abi::generated_types::{
    SDL_version, SDL_MAJOR_VERSION, SDL_MINOR_VERSION, SDL_PATCHLEVEL,
};

static REVISION: &[u8; 20] = b"safe-sdl-phase-2-rs\0";

#[no_mangle]
pub unsafe extern "C" fn SDL_SetMainReady() {
    crate::core::init::mark_main_ready();
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetVersion(ver: *mut SDL_version) {
    if ver.is_null() {
        return;
    }
    (*ver).major = SDL_MAJOR_VERSION as u8;
    (*ver).minor = SDL_MINOR_VERSION as u8;
    (*ver).patch = SDL_PATCHLEVEL as u8;
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetRevision() -> *const libc::c_char {
    REVISION.as_ptr().cast()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetRevisionNumber() -> libc::c_int {
    0
}
