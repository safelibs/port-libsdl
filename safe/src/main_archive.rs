use crate::abi::generated_types::{
    SDL_bool_SDL_FALSE, SDL_version, SDL_MAJOR_VERSION, SDL_MINOR_VERSION, SDL_PATCHLEVEL,
};

static REVISION: &[u8] = b"SDL-release-2.30.0-0-g859844eae (Ubuntu 2.30.0+dfsg-1ubuntu3.1)\0";
static LEGACY_VERSION_HINT: &[u8] = b"SDL_LEGACY_VERSION\0";

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

    if crate::core::hints::SDL_GetHintBoolean(
        LEGACY_VERSION_HINT.as_ptr().cast(),
        SDL_bool_SDL_FALSE,
    ) != 0
    {
        (*ver).patch = (*ver).minor;
        (*ver).minor = 0;
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetRevision() -> *const libc::c_char {
    REVISION.as_ptr().cast()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetRevisionNumber() -> libc::c_int {
    0
}
