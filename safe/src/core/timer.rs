use crate::abi::generated_types::{SDL_TimerCallback, SDL_TimerID, SDL_bool, Uint32, Uint64};

crate::forward_sdl! {
    fn SDL_GetTicks() -> Uint32;
    fn SDL_GetTicks64() -> Uint64;
    fn SDL_GetPerformanceCounter() -> Uint64;
    fn SDL_GetPerformanceFrequency() -> Uint64;
    fn SDL_Delay(ms: Uint32);
    fn SDL_AddTimer(interval: Uint32, callback: SDL_TimerCallback, param: *mut libc::c_void) -> SDL_TimerID;
    fn SDL_RemoveTimer(id: SDL_TimerID) -> SDL_bool;
}
