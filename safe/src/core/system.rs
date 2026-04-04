use std::ptr::addr_of_mut;
use std::sync::atomic::{fence, AtomicI32, AtomicPtr, Ordering};

use crate::abi::generated_types::{
    SDL_SpinLock, SDL_atomic_t, SDL_bool, SDL_bool_SDL_FALSE, SDL_bool_SDL_TRUE,
};

pub(crate) fn bool_to_sdl(value: bool) -> SDL_bool {
    if value {
        SDL_bool_SDL_TRUE
    } else {
        SDL_bool_SDL_FALSE
    }
}

pub(crate) fn sdl_to_bool(value: SDL_bool) -> bool {
    value != SDL_bool_SDL_FALSE
}

pub(crate) fn last_os_error_message(prefix: &str) -> String {
    let error = std::io::Error::last_os_error();
    if let Some(raw) = error.raw_os_error() {
        format!("{prefix}: {error} (errno {raw})")
    } else {
        format!("{prefix}: {error}")
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_AtomicTryLock(lock: *mut SDL_SpinLock) -> SDL_bool {
    if lock.is_null() {
        return SDL_bool_SDL_FALSE;
    }
    let atomic = AtomicI32::from_ptr(lock);
    bool_to_sdl(
        atomic
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_ok(),
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDL_AtomicLock(lock: *mut SDL_SpinLock) {
    if lock.is_null() {
        return;
    }
    let atomic = AtomicI32::from_ptr(lock);
    while atomic
        .compare_exchange_weak(0, 1, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        std::hint::spin_loop();
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_AtomicUnlock(lock: *mut SDL_SpinLock) {
    if lock.is_null() {
        return;
    }
    let atomic = AtomicI32::from_ptr(lock);
    atomic.store(0, Ordering::Release);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_AtomicCAS(
    a: *mut SDL_atomic_t,
    oldval: libc::c_int,
    newval: libc::c_int,
) -> SDL_bool {
    if a.is_null() {
        return SDL_bool_SDL_FALSE;
    }
    let atomic = AtomicI32::from_ptr(addr_of_mut!((*a).value));
    bool_to_sdl(
        atomic
            .compare_exchange(oldval, newval, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok(),
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDL_AtomicSet(a: *mut SDL_atomic_t, v: libc::c_int) -> libc::c_int {
    if a.is_null() {
        return 0;
    }
    let atomic = AtomicI32::from_ptr(addr_of_mut!((*a).value));
    atomic.swap(v, Ordering::SeqCst)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_AtomicGet(a: *mut SDL_atomic_t) -> libc::c_int {
    if a.is_null() {
        return 0;
    }
    let atomic = AtomicI32::from_ptr(addr_of_mut!((*a).value));
    atomic.load(Ordering::SeqCst)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_AtomicAdd(a: *mut SDL_atomic_t, v: libc::c_int) -> libc::c_int {
    if a.is_null() {
        return 0;
    }
    let atomic = AtomicI32::from_ptr(addr_of_mut!((*a).value));
    atomic.fetch_add(v, Ordering::SeqCst)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_AtomicCASPtr(
    a: *mut *mut libc::c_void,
    oldval: *mut libc::c_void,
    newval: *mut libc::c_void,
) -> SDL_bool {
    if a.is_null() {
        return SDL_bool_SDL_FALSE;
    }
    let atomic = AtomicPtr::from_ptr(a);
    bool_to_sdl(
        atomic
            .compare_exchange(oldval, newval, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok(),
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDL_AtomicSetPtr(
    a: *mut *mut libc::c_void,
    v: *mut libc::c_void,
) -> *mut libc::c_void {
    if a.is_null() {
        return std::ptr::null_mut();
    }
    let atomic = AtomicPtr::from_ptr(a);
    atomic.swap(v, Ordering::SeqCst)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_AtomicGetPtr(a: *mut *mut libc::c_void) -> *mut libc::c_void {
    if a.is_null() {
        return std::ptr::null_mut();
    }
    let atomic = AtomicPtr::from_ptr(a);
    atomic.load(Ordering::SeqCst)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_MemoryBarrierReleaseFunction() {
    fence(Ordering::Release);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_MemoryBarrierAcquireFunction() {
    fence(Ordering::Acquire);
}
