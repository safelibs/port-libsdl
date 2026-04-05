use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

use crate::abi::generated_types::{
    self as sdl, SDL_calloc_func, SDL_free_func, SDL_malloc_func, SDL_realloc_func,
};

#[derive(Clone, Copy)]
struct OriginalMemoryFns {
    malloc_func: SDL_malloc_func,
    calloc_func: SDL_calloc_func,
    realloc_func: SDL_realloc_func,
    free_func: SDL_free_func,
}

struct TrackerState {
    originals: Option<OriginalMemoryFns>,
    previous_allocations: i32,
    allocations: BTreeMap<usize, usize>,
}

fn tracker_state() -> &'static Mutex<TrackerState> {
    static STATE: OnceLock<Mutex<TrackerState>> = OnceLock::new();
    STATE.get_or_init(|| {
        Mutex::new(TrackerState {
            originals: None,
            previous_allocations: 0,
            allocations: BTreeMap::new(),
        })
    })
}

fn lock_state() -> std::sync::MutexGuard<'static, TrackerState> {
    match tracker_state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

unsafe extern "C" fn tracked_malloc(size: usize) -> *mut libc::c_void {
    let originals = lock_state()
        .originals
        .expect("track allocations not initialized");
    let ptr = originals.malloc_func.expect("SDL_malloc callback")(size);
    if !ptr.is_null() {
        lock_state().allocations.insert(ptr as usize, size);
    }
    ptr
}

unsafe extern "C" fn tracked_calloc(nmemb: usize, size: usize) -> *mut libc::c_void {
    let originals = lock_state()
        .originals
        .expect("track allocations not initialized");
    let ptr = originals.calloc_func.expect("SDL_calloc callback")(nmemb, size);
    if !ptr.is_null() {
        lock_state()
            .allocations
            .insert(ptr as usize, nmemb.saturating_mul(size));
    }
    ptr
}

unsafe extern "C" fn tracked_realloc(ptr: *mut libc::c_void, size: usize) -> *mut libc::c_void {
    let originals = lock_state()
        .originals
        .expect("track allocations not initialized");
    let new_ptr = originals.realloc_func.expect("SDL_realloc callback")(ptr, size);
    let mut state = lock_state();
    if !ptr.is_null() {
        state.allocations.remove(&(ptr as usize));
    }
    if !new_ptr.is_null() {
        state.allocations.insert(new_ptr as usize, size);
    }
    new_ptr
}

unsafe extern "C" fn tracked_free(ptr: *mut libc::c_void) {
    if ptr.is_null() {
        return;
    }
    let originals = lock_state()
        .originals
        .expect("track allocations not initialized");
    let mut state = lock_state();
    if state.previous_allocations == 0 {
        debug_assert!(state.allocations.contains_key(&(ptr as usize)));
    }
    state.allocations.remove(&(ptr as usize));
    drop(state);
    originals.free_func.expect("SDL_free callback")(ptr);
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_TrackAllocations() -> libc::c_int {
    let mut state = lock_state();
    if state.originals.is_some() {
        return 0;
    }
    let mut malloc_func = None;
    let mut calloc_func = None;
    let mut realloc_func = None;
    let mut free_func = None;
    sdl::SDL_GetMemoryFunctions(
        &mut malloc_func,
        &mut calloc_func,
        &mut realloc_func,
        &mut free_func,
    );
    state.originals = Some(OriginalMemoryFns {
        malloc_func,
        calloc_func,
        realloc_func,
        free_func,
    });
    state.previous_allocations = sdl::SDL_GetNumAllocations();
    if state.previous_allocations != 0 {
        let message = std::ffi::CString::new(format!(
            "SDLTest_TrackAllocations(): There are {} previous allocations, disabling free() validation",
            state.previous_allocations
        ))
        .unwrap();
        crate::testsupport::log::SDLTest_LogFromBuffer(message.as_ptr());
    }
    drop(state);
    sdl::SDL_SetMemoryFunctions(
        Some(tracked_malloc),
        Some(tracked_calloc),
        Some(tracked_realloc),
        Some(tracked_free),
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_LogAllocations() {
    let state = lock_state();
    if state.originals.is_none() {
        return;
    }
    let count = state.allocations.len();
    let total: usize = state.allocations.values().sum();
    let summary = std::ffi::CString::new(format!(
        "Memory allocations: outstanding={count} total_bytes={total}"
    ))
    .unwrap();
    crate::testsupport::log::SDLTest_LogFromBuffer(summary.as_ptr());
}
