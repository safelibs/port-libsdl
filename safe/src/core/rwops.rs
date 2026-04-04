use crate::abi::generated_types::{SDL_RWops, SDL_bool, Sint64, Uint16, Uint32, Uint64, Uint8};

crate::forward_sdl! {
    fn SDL_RWFromFile(file: *const libc::c_char, mode: *const libc::c_char) -> *mut SDL_RWops;
    fn SDL_RWFromFP(fp: *mut libc::c_void, autoclose: SDL_bool) -> *mut SDL_RWops;
    fn SDL_RWFromMem(mem: *mut libc::c_void, size: libc::c_int) -> *mut SDL_RWops;
    fn SDL_RWFromConstMem(mem: *const libc::c_void, size: libc::c_int) -> *mut SDL_RWops;
    fn SDL_AllocRW() -> *mut SDL_RWops;
    fn SDL_FreeRW(area: *mut SDL_RWops);
    fn SDL_RWsize(context: *mut SDL_RWops) -> Sint64;
    fn SDL_RWseek(context: *mut SDL_RWops, offset: Sint64, whence: libc::c_int) -> Sint64;
    fn SDL_RWtell(context: *mut SDL_RWops) -> Sint64;
    fn SDL_RWread(
        context: *mut SDL_RWops,
        ptr: *mut libc::c_void,
        size: usize,
        maxnum: usize
    ) -> usize;
    fn SDL_RWwrite(
        context: *mut SDL_RWops,
        ptr: *const libc::c_void,
        size: usize,
        num: usize
    ) -> usize;
    fn SDL_RWclose(context: *mut SDL_RWops) -> libc::c_int;
    fn SDL_LoadFile_RW(src: *mut SDL_RWops, datasize: *mut usize, freesrc: libc::c_int) -> *mut libc::c_void;
    fn SDL_LoadFile(file: *const libc::c_char, datasize: *mut usize) -> *mut libc::c_void;
    fn SDL_ReadU8(src: *mut SDL_RWops) -> Uint8;
    fn SDL_ReadLE16(src: *mut SDL_RWops) -> Uint16;
    fn SDL_ReadBE16(src: *mut SDL_RWops) -> Uint16;
    fn SDL_ReadLE32(src: *mut SDL_RWops) -> Uint32;
    fn SDL_ReadBE32(src: *mut SDL_RWops) -> Uint32;
    fn SDL_ReadLE64(src: *mut SDL_RWops) -> Uint64;
    fn SDL_ReadBE64(src: *mut SDL_RWops) -> Uint64;
    fn SDL_WriteU8(dst: *mut SDL_RWops, value: Uint8) -> usize;
    fn SDL_WriteLE16(dst: *mut SDL_RWops, value: Uint16) -> usize;
    fn SDL_WriteBE16(dst: *mut SDL_RWops, value: Uint16) -> usize;
    fn SDL_WriteLE32(dst: *mut SDL_RWops, value: Uint32) -> usize;
    fn SDL_WriteBE32(dst: *mut SDL_RWops, value: Uint32) -> usize;
    fn SDL_WriteLE64(dst: *mut SDL_RWops, value: Uint64) -> usize;
    fn SDL_WriteBE64(dst: *mut SDL_RWops, value: Uint64) -> usize;
}
