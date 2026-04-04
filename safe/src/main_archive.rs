use crate::abi::generated_types::SDL_version;

crate::forward_sdl! {
    fn SDL_SetMainReady();
    fn SDL_GetVersion(ver: *mut SDL_version);
    fn SDL_GetRevision() -> *const libc::c_char;
    fn SDL_GetRevisionNumber() -> libc::c_int;
}
