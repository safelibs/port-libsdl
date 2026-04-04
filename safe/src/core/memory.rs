use crate::abi::generated_types::{
    SDL_calloc_func, SDL_free_func, SDL_malloc_func, SDL_realloc_func,
};

crate::forward_sdl! {
    fn SDL_malloc(size: usize) -> *mut libc::c_void;
    fn SDL_calloc(nmemb: usize, size: usize) -> *mut libc::c_void;
    fn SDL_realloc(mem: *mut libc::c_void, size: usize) -> *mut libc::c_void;
    fn SDL_free(mem: *mut libc::c_void);
    fn SDL_GetOriginalMemoryFunctions(
        malloc_func: *mut SDL_malloc_func,
        calloc_func: *mut SDL_calloc_func,
        realloc_func: *mut SDL_realloc_func,
        free_func: *mut SDL_free_func
    );
    fn SDL_GetMemoryFunctions(
        malloc_func: *mut SDL_malloc_func,
        calloc_func: *mut SDL_calloc_func,
        realloc_func: *mut SDL_realloc_func,
        free_func: *mut SDL_free_func
    );
    fn SDL_SetMemoryFunctions(
        malloc_func: SDL_malloc_func,
        calloc_func: SDL_calloc_func,
        realloc_func: SDL_realloc_func,
        free_func: SDL_free_func
    ) -> libc::c_int;
    fn SDL_GetNumAllocations() -> libc::c_int;
}
