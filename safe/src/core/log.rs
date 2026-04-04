use crate::abi::generated_types::{SDL_LogOutputFunction, SDL_LogPriority};

crate::forward_sdl! {
    fn SDL_LogSetAllPriority(priority: SDL_LogPriority);
    fn SDL_LogSetPriority(category: libc::c_int, priority: SDL_LogPriority);
    fn SDL_LogGetPriority(category: libc::c_int) -> SDL_LogPriority;
    fn SDL_LogResetPriorities();
    fn SDL_LogGetOutputFunction(
        callback: *mut SDL_LogOutputFunction,
        userdata: *mut *mut libc::c_void
    );
    fn SDL_LogSetOutputFunction(callback: SDL_LogOutputFunction, userdata: *mut libc::c_void);
}
