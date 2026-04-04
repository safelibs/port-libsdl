crate::forward_sdl! {
    fn SDL_LoadObject(sofile: *const libc::c_char) -> *mut libc::c_void;
    fn SDL_LoadFunction(handle: *mut libc::c_void, name: *const libc::c_char) -> *mut libc::c_void;
    fn SDL_UnloadObject(handle: *mut libc::c_void);
}
