crate::forward_sdl! {
    fn SDL_GetBasePath() -> *mut libc::c_char;
    fn SDL_GetPrefPath(org: *const libc::c_char, app: *const libc::c_char) -> *mut libc::c_char;
}
