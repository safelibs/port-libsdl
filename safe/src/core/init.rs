use crate::abi::generated_types::Uint32;

crate::forward_sdl! {
    fn SDL_Init(flags: Uint32) -> libc::c_int;
    fn SDL_InitSubSystem(flags: Uint32) -> libc::c_int;
    fn SDL_QuitSubSystem(flags: Uint32);
    fn SDL_WasInit(flags: Uint32) -> Uint32;
    fn SDL_Quit();
}
