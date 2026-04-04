use crate::abi::generated_types::SDL_errorcode;

crate::forward_sdl! {
    fn SDL_GetError() -> *const libc::c_char;
    fn SDL_GetErrorMsg(errstr: *mut libc::c_char, maxlen: libc::c_int) -> *mut libc::c_char;
    fn SDL_ClearError();
    fn SDL_Error(code: SDL_errorcode) -> libc::c_int;
}
