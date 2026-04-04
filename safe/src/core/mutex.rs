use std::mem::MaybeUninit;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::abi::generated_types::{SDL_cond, SDL_mutex, SDL_sem, Uint32, SDL_MUTEX_TIMEDOUT};

#[repr(C)]
struct SdlMutexImpl {
    raw: libc::pthread_mutex_t,
}

#[repr(C)]
struct SdlSemaphoreImpl {
    raw: libc::sem_t,
}

#[repr(C)]
struct SdlCondImpl {
    raw: libc::pthread_cond_t,
}

fn timespec_after(ms: Uint32) -> libc::timespec {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    let target = now + Duration::from_millis(ms as u64);
    libc::timespec {
        tv_sec: target.as_secs() as libc::time_t,
        tv_nsec: target.subsec_nanos() as libc::c_long,
    }
}

fn mutex_from_ptr<'a>(mutex: *mut SDL_mutex) -> Option<&'a mut SdlMutexImpl> {
    (!mutex.is_null()).then(|| unsafe { &mut *(mutex as *mut SdlMutexImpl) })
}

fn sem_from_ptr<'a>(sem: *mut SDL_sem) -> Option<&'a mut SdlSemaphoreImpl> {
    (!sem.is_null()).then(|| unsafe { &mut *(sem as *mut SdlSemaphoreImpl) })
}

fn cond_from_ptr<'a>(cond: *mut SDL_cond) -> Option<&'a mut SdlCondImpl> {
    (!cond.is_null()).then(|| unsafe { &mut *(cond as *mut SdlCondImpl) })
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CreateMutex() -> *mut SDL_mutex {
    let mut attr = MaybeUninit::<libc::pthread_mutexattr_t>::uninit();
    if libc::pthread_mutexattr_init(attr.as_mut_ptr()) != 0 {
        let _ = crate::core::error::set_error_message(&crate::core::system::last_os_error_message(
            "pthread_mutexattr_init() failed",
        ));
        return std::ptr::null_mut();
    }
    let attr = attr.assume_init_mut();
    libc::pthread_mutexattr_settype(attr, libc::PTHREAD_MUTEX_RECURSIVE);

    let mut boxed = Box::new(std::mem::zeroed::<SdlMutexImpl>());
    let rc = libc::pthread_mutex_init(&mut boxed.raw, attr);
    libc::pthread_mutexattr_destroy(attr);
    if rc != 0 {
        let _ =
            crate::core::error::set_error_message(&format!("pthread_mutex_init() failed: {rc}"));
        return std::ptr::null_mut();
    }
    Box::into_raw(boxed) as *mut SDL_mutex
}

#[no_mangle]
pub unsafe extern "C" fn SDL_LockMutex(mutex: *mut SDL_mutex) -> libc::c_int {
    let Some(mutex) = mutex_from_ptr(mutex) else {
        return crate::core::error::invalid_param_error("mutex");
    };
    let rc = libc::pthread_mutex_lock(&mut mutex.raw);
    if rc == 0 {
        0
    } else {
        crate::core::error::set_error_message(&format!("pthread_mutex_lock() failed: {rc}"))
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_TryLockMutex(mutex: *mut SDL_mutex) -> libc::c_int {
    let Some(mutex) = mutex_from_ptr(mutex) else {
        return crate::core::error::invalid_param_error("mutex");
    };
    match libc::pthread_mutex_trylock(&mut mutex.raw) {
        0 => 0,
        libc::EBUSY => SDL_MUTEX_TIMEDOUT as libc::c_int,
        rc => {
            crate::core::error::set_error_message(&format!("pthread_mutex_trylock() failed: {rc}"))
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_UnlockMutex(mutex: *mut SDL_mutex) -> libc::c_int {
    let Some(mutex) = mutex_from_ptr(mutex) else {
        return crate::core::error::invalid_param_error("mutex");
    };
    let rc = libc::pthread_mutex_unlock(&mut mutex.raw);
    if rc == 0 {
        0
    } else {
        crate::core::error::set_error_message(&format!("pthread_mutex_unlock() failed: {rc}"))
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_DestroyMutex(mutex: *mut SDL_mutex) {
    if let Some(mutex) = mutex_from_ptr(mutex) {
        let _ = libc::pthread_mutex_destroy(&mut mutex.raw);
        drop(Box::from_raw(mutex as *mut SdlMutexImpl));
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CreateSemaphore(initial_value: Uint32) -> *mut SDL_sem {
    let mut boxed = Box::new(std::mem::zeroed::<SdlSemaphoreImpl>());
    let rc = libc::sem_init(&mut boxed.raw, 0, initial_value);
    if rc != 0 {
        let _ = crate::core::error::set_error_message(&crate::core::system::last_os_error_message(
            "sem_init() failed",
        ));
        return std::ptr::null_mut();
    }
    Box::into_raw(boxed) as *mut SDL_sem
}

#[no_mangle]
pub unsafe extern "C" fn SDL_DestroySemaphore(sem: *mut SDL_sem) {
    if let Some(sem) = sem_from_ptr(sem) {
        let _ = libc::sem_destroy(&mut sem.raw);
        drop(Box::from_raw(sem as *mut SdlSemaphoreImpl));
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SemWait(sem: *mut SDL_sem) -> libc::c_int {
    let Some(sem) = sem_from_ptr(sem) else {
        return crate::core::error::invalid_param_error("sem");
    };
    loop {
        let rc = libc::sem_wait(&mut sem.raw);
        if rc == 0 {
            return 0;
        }
        if std::io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) {
            return crate::core::error::set_error_message(
                &crate::core::system::last_os_error_message("sem_wait() failed"),
            );
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SemTryWait(sem: *mut SDL_sem) -> libc::c_int {
    let Some(sem) = sem_from_ptr(sem) else {
        return crate::core::error::invalid_param_error("sem");
    };
    match libc::sem_trywait(&mut sem.raw) {
        0 => 0,
        _ if std::io::Error::last_os_error().raw_os_error() == Some(libc::EAGAIN) => {
            SDL_MUTEX_TIMEDOUT as libc::c_int
        }
        _ => crate::core::error::set_error_message(&crate::core::system::last_os_error_message(
            "sem_trywait() failed",
        )),
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SemWaitTimeout(sem: *mut SDL_sem, timeout: Uint32) -> libc::c_int {
    if timeout == u32::MAX {
        return SDL_SemWait(sem);
    }
    let Some(sem) = sem_from_ptr(sem) else {
        return crate::core::error::invalid_param_error("sem");
    };
    let mut ts = timespec_after(timeout);
    loop {
        let rc = libc::sem_timedwait(&mut sem.raw, &mut ts);
        if rc == 0 {
            return 0;
        }
        match std::io::Error::last_os_error().raw_os_error() {
            Some(libc::EINTR) => continue,
            Some(libc::ETIMEDOUT) => return SDL_MUTEX_TIMEDOUT as libc::c_int,
            _ => {
                return crate::core::error::set_error_message(
                    &crate::core::system::last_os_error_message("sem_timedwait() failed"),
                )
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SemPost(sem: *mut SDL_sem) -> libc::c_int {
    let Some(sem) = sem_from_ptr(sem) else {
        return crate::core::error::invalid_param_error("sem");
    };
    if libc::sem_post(&mut sem.raw) == 0 {
        0
    } else {
        crate::core::error::set_error_message(&crate::core::system::last_os_error_message(
            "sem_post() failed",
        ))
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SemValue(sem: *mut SDL_sem) -> Uint32 {
    let Some(sem) = sem_from_ptr(sem) else {
        return 0;
    };
    let mut value = 0;
    if libc::sem_getvalue(&mut sem.raw, &mut value) == 0 {
        value.max(0) as Uint32
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CreateCond() -> *mut SDL_cond {
    let mut boxed = Box::new(std::mem::zeroed::<SdlCondImpl>());
    if libc::pthread_cond_init(&mut boxed.raw, std::ptr::null()) != 0 {
        let _ = crate::core::error::set_error_message(&crate::core::system::last_os_error_message(
            "pthread_cond_init() failed",
        ));
        return std::ptr::null_mut();
    }
    Box::into_raw(boxed) as *mut SDL_cond
}

#[no_mangle]
pub unsafe extern "C" fn SDL_DestroyCond(cond: *mut SDL_cond) {
    if let Some(cond) = cond_from_ptr(cond) {
        let _ = libc::pthread_cond_destroy(&mut cond.raw);
        drop(Box::from_raw(cond as *mut SdlCondImpl));
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CondSignal(cond: *mut SDL_cond) -> libc::c_int {
    let Some(cond) = cond_from_ptr(cond) else {
        return crate::core::error::invalid_param_error("cond");
    };
    if libc::pthread_cond_signal(&mut cond.raw) == 0 {
        0
    } else {
        crate::core::error::set_error_message(&crate::core::system::last_os_error_message(
            "pthread_cond_signal() failed",
        ))
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CondBroadcast(cond: *mut SDL_cond) -> libc::c_int {
    let Some(cond) = cond_from_ptr(cond) else {
        return crate::core::error::invalid_param_error("cond");
    };
    if libc::pthread_cond_broadcast(&mut cond.raw) == 0 {
        0
    } else {
        crate::core::error::set_error_message(&crate::core::system::last_os_error_message(
            "pthread_cond_broadcast() failed",
        ))
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CondWait(cond: *mut SDL_cond, mutex: *mut SDL_mutex) -> libc::c_int {
    let Some(cond) = cond_from_ptr(cond) else {
        return crate::core::error::invalid_param_error("cond");
    };
    let Some(mutex) = mutex_from_ptr(mutex) else {
        return crate::core::error::invalid_param_error("mutex");
    };
    if libc::pthread_cond_wait(&mut cond.raw, &mut mutex.raw) == 0 {
        0
    } else {
        crate::core::error::set_error_message(&crate::core::system::last_os_error_message(
            "pthread_cond_wait() failed",
        ))
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CondWaitTimeout(
    cond: *mut SDL_cond,
    mutex: *mut SDL_mutex,
    ms: Uint32,
) -> libc::c_int {
    let Some(cond) = cond_from_ptr(cond) else {
        return crate::core::error::invalid_param_error("cond");
    };
    let Some(mutex) = mutex_from_ptr(mutex) else {
        return crate::core::error::invalid_param_error("mutex");
    };
    let mut ts = timespec_after(ms);
    match libc::pthread_cond_timedwait(&mut cond.raw, &mut mutex.raw, &mut ts) {
        0 => 0,
        libc::ETIMEDOUT => SDL_MUTEX_TIMEDOUT as libc::c_int,
        _ => crate::core::error::set_error_message(&crate::core::system::last_os_error_message(
            "pthread_cond_timedwait() failed",
        )),
    }
}
