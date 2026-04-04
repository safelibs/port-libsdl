use crate::abi::generated_types::{SDL_cond, SDL_mutex, SDL_sem, Uint32};

crate::forward_sdl! {
    fn SDL_CreateMutex() -> *mut SDL_mutex;
    fn SDL_LockMutex(mutex: *mut SDL_mutex) -> libc::c_int;
    fn SDL_TryLockMutex(mutex: *mut SDL_mutex) -> libc::c_int;
    fn SDL_UnlockMutex(mutex: *mut SDL_mutex) -> libc::c_int;
    fn SDL_DestroyMutex(mutex: *mut SDL_mutex);
    fn SDL_CreateSemaphore(initial_value: Uint32) -> *mut SDL_sem;
    fn SDL_DestroySemaphore(sem: *mut SDL_sem);
    fn SDL_SemWait(sem: *mut SDL_sem) -> libc::c_int;
    fn SDL_SemTryWait(sem: *mut SDL_sem) -> libc::c_int;
    fn SDL_SemWaitTimeout(sem: *mut SDL_sem, timeout: Uint32) -> libc::c_int;
    fn SDL_SemPost(sem: *mut SDL_sem) -> libc::c_int;
    fn SDL_SemValue(sem: *mut SDL_sem) -> Uint32;
    fn SDL_CreateCond() -> *mut SDL_cond;
    fn SDL_DestroyCond(cond: *mut SDL_cond);
    fn SDL_CondSignal(cond: *mut SDL_cond) -> libc::c_int;
    fn SDL_CondBroadcast(cond: *mut SDL_cond) -> libc::c_int;
    fn SDL_CondWait(cond: *mut SDL_cond, mutex: *mut SDL_mutex) -> libc::c_int;
    fn SDL_CondWaitTimeout(cond: *mut SDL_cond, mutex: *mut SDL_mutex, ms: Uint32) -> libc::c_int;
}
