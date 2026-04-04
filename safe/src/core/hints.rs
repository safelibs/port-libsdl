use crate::abi::generated_types::{SDL_HintCallback, SDL_HintPriority, SDL_bool};

crate::forward_sdl! {
    fn SDL_SetHintWithPriority(
        name: *const libc::c_char,
        value: *const libc::c_char,
        priority: SDL_HintPriority
    ) -> SDL_bool;
    fn SDL_SetHint(name: *const libc::c_char, value: *const libc::c_char) -> SDL_bool;
    fn SDL_ResetHint(name: *const libc::c_char) -> SDL_bool;
    fn SDL_ResetHints();
    fn SDL_GetHint(name: *const libc::c_char) -> *const libc::c_char;
    fn SDL_GetHintBoolean(name: *const libc::c_char, default_value: SDL_bool) -> SDL_bool;
    fn SDL_AddHintCallback(
        name: *const libc::c_char,
        callback: SDL_HintCallback,
        userdata: *mut libc::c_void
    );
    fn SDL_DelHintCallback(
        name: *const libc::c_char,
        callback: SDL_HintCallback,
        userdata: *mut libc::c_void
    );
    fn SDL_ClearHints();
}
