use crate::abi::generated_types::SDL_PowerState;

crate::forward_sdl! {
    fn SDL_GetPowerInfo(seconds: *mut libc::c_int, percent: *mut libc::c_int) -> SDL_PowerState;
}
