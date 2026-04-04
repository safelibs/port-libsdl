crate::forward_sdl! {
    fn SDL_GetPlatform() -> *const libc::c_char;
}
