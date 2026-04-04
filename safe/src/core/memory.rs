use std::ptr;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::abi::generated_types::{
    SDL_calloc_func, SDL_free_func, SDL_malloc_func, SDL_realloc_func,
};

#[derive(Clone, Copy)]
struct MemoryFunctions {
    malloc_func: SDL_malloc_func,
    calloc_func: SDL_calloc_func,
    realloc_func: SDL_realloc_func,
    free_func: SDL_free_func,
}

unsafe extern "C" fn builtin_malloc(size: usize) -> *mut libc::c_void {
    libc::malloc(size)
}

unsafe extern "C" fn builtin_calloc(nmemb: usize, size: usize) -> *mut libc::c_void {
    libc::calloc(nmemb, size)
}

unsafe extern "C" fn builtin_realloc(mem: *mut libc::c_void, size: usize) -> *mut libc::c_void {
    libc::realloc(mem, size)
}

unsafe extern "C" fn builtin_free(mem: *mut libc::c_void) {
    libc::free(mem);
}

fn default_memory_functions() -> MemoryFunctions {
    MemoryFunctions {
        malloc_func: Some(builtin_malloc),
        calloc_func: Some(builtin_calloc),
        realloc_func: Some(builtin_realloc),
        free_func: Some(builtin_free),
    }
}

fn memory_functions() -> &'static Mutex<MemoryFunctions> {
    static MEMORY_FUNCTIONS: OnceLock<Mutex<MemoryFunctions>> = OnceLock::new();
    MEMORY_FUNCTIONS.get_or_init(|| Mutex::new(default_memory_functions()))
}

fn allocation_count() -> &'static AtomicI32 {
    static COUNT: AtomicI32 = AtomicI32::new(0);
    &COUNT
}

fn lock_memory_functions() -> std::sync::MutexGuard<'static, MemoryFunctions> {
    match memory_functions().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

pub(crate) unsafe fn alloc_bytes(bytes: &[u8]) -> *mut libc::c_char {
    let ptr = SDL_malloc(bytes.len()) as *mut libc::c_char;
    if ptr.is_null() {
        return ptr;
    }
    ptr::copy_nonoverlapping(bytes.as_ptr().cast::<libc::c_char>(), ptr, bytes.len());
    ptr
}

pub(crate) unsafe fn alloc_c_string(value: &str) -> *mut libc::c_char {
    let mut bytes = value.as_bytes().to_vec();
    bytes.push(0);
    alloc_bytes(&bytes)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_malloc(size: usize) -> *mut libc::c_void {
    let functions = *lock_memory_functions();
    let ptr = functions
        .malloc_func
        .expect("SDL_malloc callback should always be present")(size);
    if !ptr.is_null() {
        allocation_count().fetch_add(1, Ordering::Relaxed);
    }
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn SDL_calloc(nmemb: usize, size: usize) -> *mut libc::c_void {
    let functions = *lock_memory_functions();
    let ptr = functions
        .calloc_func
        .expect("SDL_calloc callback should always be present")(nmemb, size);
    if !ptr.is_null() {
        allocation_count().fetch_add(1, Ordering::Relaxed);
    }
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn SDL_realloc(mem: *mut libc::c_void, size: usize) -> *mut libc::c_void {
    let functions = *lock_memory_functions();
    let ptr = functions
        .realloc_func
        .expect("SDL_realloc callback should always be present")(mem, size);

    if mem.is_null() && !ptr.is_null() {
        allocation_count().fetch_add(1, Ordering::Relaxed);
    } else if !mem.is_null() && size == 0 && !ptr.is_null() {
        allocation_count().fetch_sub(1, Ordering::Relaxed);
    }

    ptr
}

#[no_mangle]
pub unsafe extern "C" fn SDL_free(mem: *mut libc::c_void) {
    if mem.is_null() {
        return;
    }
    let functions = *lock_memory_functions();
    functions
        .free_func
        .expect("SDL_free callback should always be present")(mem);
    allocation_count().fetch_sub(1, Ordering::Relaxed);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetOriginalMemoryFunctions(
    malloc_func: *mut SDL_malloc_func,
    calloc_func: *mut SDL_calloc_func,
    realloc_func: *mut SDL_realloc_func,
    free_func: *mut SDL_free_func,
) {
    let defaults = default_memory_functions();
    if !malloc_func.is_null() {
        *malloc_func = defaults.malloc_func;
    }
    if !calloc_func.is_null() {
        *calloc_func = defaults.calloc_func;
    }
    if !realloc_func.is_null() {
        *realloc_func = defaults.realloc_func;
    }
    if !free_func.is_null() {
        *free_func = defaults.free_func;
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetMemoryFunctions(
    malloc_func: *mut SDL_malloc_func,
    calloc_func: *mut SDL_calloc_func,
    realloc_func: *mut SDL_realloc_func,
    free_func: *mut SDL_free_func,
) {
    let current = *lock_memory_functions();
    if !malloc_func.is_null() {
        *malloc_func = current.malloc_func;
    }
    if !calloc_func.is_null() {
        *calloc_func = current.calloc_func;
    }
    if !realloc_func.is_null() {
        *realloc_func = current.realloc_func;
    }
    if !free_func.is_null() {
        *free_func = current.free_func;
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetMemoryFunctions(
    malloc_func: SDL_malloc_func,
    calloc_func: SDL_calloc_func,
    realloc_func: SDL_realloc_func,
    free_func: SDL_free_func,
) -> libc::c_int {
    if malloc_func.is_none() {
        return crate::core::error::invalid_param_error("malloc_func");
    }
    if calloc_func.is_none() {
        return crate::core::error::invalid_param_error("calloc_func");
    }
    if realloc_func.is_none() {
        return crate::core::error::invalid_param_error("realloc_func");
    }
    if free_func.is_none() {
        return crate::core::error::invalid_param_error("free_func");
    }

    *lock_memory_functions() = MemoryFunctions {
        malloc_func,
        calloc_func,
        realloc_func,
        free_func,
    };
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetNumAllocations() -> libc::c_int {
    allocation_count().load(Ordering::Relaxed)
}
