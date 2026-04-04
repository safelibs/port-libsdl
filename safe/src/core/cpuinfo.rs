use crate::abi::generated_types::SDL_bool;

crate::forward_sdl! {
    fn SDL_GetCPUCount() -> libc::c_int;
    fn SDL_GetCPUCacheLineSize() -> libc::c_int;
    fn SDL_HasRDTSC() -> SDL_bool;
    fn SDL_HasAltiVec() -> SDL_bool;
    fn SDL_HasMMX() -> SDL_bool;
    fn SDL_Has3DNow() -> SDL_bool;
    fn SDL_HasSSE() -> SDL_bool;
    fn SDL_HasSSE2() -> SDL_bool;
    fn SDL_HasSSE3() -> SDL_bool;
    fn SDL_HasSSE41() -> SDL_bool;
    fn SDL_HasSSE42() -> SDL_bool;
    fn SDL_HasAVX() -> SDL_bool;
    fn SDL_HasAVX2() -> SDL_bool;
    fn SDL_HasAVX512F() -> SDL_bool;
    fn SDL_HasARMSIMD() -> SDL_bool;
    fn SDL_HasNEON() -> SDL_bool;
    fn SDL_HasLSX() -> SDL_bool;
    fn SDL_HasLASX() -> SDL_bool;
    fn SDL_GetSystemRAM() -> libc::c_int;
    fn SDL_SIMDAlloc(len: usize) -> *mut libc::c_void;
    fn SDL_SIMDRealloc(mem: *mut libc::c_void, len: usize) -> *mut libc::c_void;
    fn SDL_SIMDFree(ptr: *mut libc::c_void);
    fn SDL_SIMDGetAlignment() -> usize;
}
