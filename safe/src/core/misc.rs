crate::forward_sdl! {
    fn SDL_OpenURL(url: *const libc::c_char) -> libc::c_int;
}
