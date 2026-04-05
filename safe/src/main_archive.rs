use crate::abi::generated_types::{
    SDL_bool_SDL_FALSE, SDL_version, SDL_MAJOR_VERSION, SDL_MINOR_VERSION, SDL_PATCHLEVEL,
};
use std::sync::OnceLock;

static REVISION: &[u8] = b"SDL-release-2.30.0-0-g859844eae (Ubuntu 2.30.0+dfsg-1ubuntu3.1)\0";
static LEGACY_VERSION_HINT: &[u8] = b"SDL_LEGACY_VERSION\0";
static CACHED_VERSION: OnceLock<[u8; 3]> = OnceLock::new();

fn resolved_version() -> [u8; 3] {
    let mut version = [
        SDL_MAJOR_VERSION as u8,
        SDL_MINOR_VERSION as u8,
        SDL_PATCHLEVEL as u8,
    ];

    let legacy_version = unsafe {
        crate::core::hints::SDL_GetHintBoolean(
            LEGACY_VERSION_HINT.as_ptr().cast(),
            SDL_bool_SDL_FALSE,
        ) != 0
    };

    if legacy_version {
        version[2] = version[1];
        version[1] = 0;
    }

    version
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetMainReady() {
    crate::core::init::mark_main_ready();
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetVersion(ver: *mut SDL_version) {
    if ver.is_null() {
        return;
    }

    let [major, minor, patch] = *CACHED_VERSION.get_or_init(resolved_version);
    (*ver).major = major;
    (*ver).minor = minor;
    (*ver).patch = patch;
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetRevision() -> *const libc::c_char {
    REVISION.as_ptr().cast()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetRevisionNumber() -> libc::c_int {
    0
}
