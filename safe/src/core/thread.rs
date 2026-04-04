use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::thread::{Builder, JoinHandle};

use crate::abi::generated_types::{
    SDL_Thread, SDL_ThreadFunction, SDL_ThreadPriority,
    SDL_ThreadPriority_SDL_THREAD_PRIORITY_HIGH, SDL_ThreadPriority_SDL_THREAD_PRIORITY_LOW,
    SDL_ThreadPriority_SDL_THREAD_PRIORITY_TIME_CRITICAL, SDL_threadID, Sint64, SDL_TLSID,
};

struct ThreadTlsValue {
    value: *mut libc::c_void,
    destructor: Option<unsafe extern "C" fn(*mut libc::c_void)>,
}

struct ThreadHandle {
    name: CString,
    id: AtomicUsize,
    join: Option<JoinHandle<libc::c_int>>,
}

thread_local! {
    static TLS_MAP: RefCell<HashMap<SDL_TLSID, ThreadTlsValue>> = RefCell::new(HashMap::new());
}

fn next_tls_id() -> &'static AtomicU32 {
    static NEXT_TLS_ID: AtomicU32 = AtomicU32::new(1);
    &NEXT_TLS_ID
}

fn current_thread_id() -> SDL_threadID {
    unsafe { libc::syscall(libc::SYS_gettid) as SDL_threadID }
}

pub(crate) fn cleanup_current_thread_tls() {
    TLS_MAP.with(|map| {
        let mut map = map.borrow_mut();
        let values = map.drain().map(|(_, value)| value).collect::<Vec<_>>();
        drop(map);
        for value in values {
            if let Some(destructor) = value.destructor {
                unsafe { destructor(value.value) };
            }
        }
    });
}

fn priority_to_nice(priority: libc::c_int) -> libc::c_int {
    match priority as u32 {
        SDL_ThreadPriority_SDL_THREAD_PRIORITY_LOW => 19,
        SDL_ThreadPriority_SDL_THREAD_PRIORITY_HIGH => -10,
        SDL_ThreadPriority_SDL_THREAD_PRIORITY_TIME_CRITICAL => -20,
        _ => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CreateThread(
    fn_: SDL_ThreadFunction,
    name: *const libc::c_char,
    data: *mut libc::c_void,
) -> *mut SDL_Thread {
    SDL_CreateThreadWithStackSize(fn_, name, 0, data)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CreateThreadWithStackSize(
    fn_: SDL_ThreadFunction,
    name: *const libc::c_char,
    stacksize: usize,
    data: *mut libc::c_void,
) -> *mut SDL_Thread {
    let Some(fn_) = fn_ else {
        let _ = crate::core::error::invalid_param_error("fn");
        return std::ptr::null_mut();
    };

    let thread_name = if name.is_null() {
        CString::default()
    } else {
        CStr::from_ptr(name).to_owned()
    };
    let builder = if thread_name.as_bytes().is_empty() {
        Builder::new()
    } else {
        Builder::new().name(thread_name.to_string_lossy().into_owned())
    };
    let builder = if stacksize > 0 {
        builder.stack_size(stacksize)
    } else {
        builder
    };

    let mut handle = Box::new(ThreadHandle {
        name: thread_name,
        id: AtomicUsize::new(0),
        join: None,
    });
    let raw_handle = &mut *handle as *mut ThreadHandle;

    let raw_handle = raw_handle as usize;
    let data = data as usize;
    match builder.spawn(move || {
        let thread = unsafe { &*(raw_handle as *mut ThreadHandle) };
        thread
            .id
            .store(current_thread_id() as usize, Ordering::Release);
        let status = fn_(data as *mut libc::c_void);
        cleanup_current_thread_tls();
        status
    }) {
        Ok(join) => {
            handle.join = Some(join);
            Box::into_raw(handle) as *mut SDL_Thread
        }
        Err(error) => {
            let _ =
                crate::core::error::set_error_message(&format!("Couldn't create thread: {error}"));
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetThreadName(thread: *mut SDL_Thread) -> *const libc::c_char {
    if thread.is_null() {
        return std::ptr::null();
    }
    let thread = &*(thread as *mut ThreadHandle);
    if thread.name.as_bytes().is_empty() {
        std::ptr::null()
    } else {
        thread.name.as_ptr()
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ThreadID() -> SDL_threadID {
    current_thread_id()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetThreadID(thread: *mut SDL_Thread) -> SDL_threadID {
    if thread.is_null() {
        return current_thread_id();
    }
    let thread = &*(thread as *mut ThreadHandle);
    thread.id.load(Ordering::Acquire) as SDL_threadID
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetThreadPriority(priority: SDL_ThreadPriority) -> libc::c_int {
    SDL_LinuxSetThreadPriorityAndPolicy(
        current_thread_id() as Sint64,
        priority as libc::c_int,
        libc::SCHED_OTHER,
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDL_WaitThread(thread: *mut SDL_Thread, status: *mut libc::c_int) {
    if thread.is_null() {
        return;
    }
    let mut handle = Box::from_raw(thread as *mut ThreadHandle);
    let result = handle.join.take().map(|join| join.join());
    if !status.is_null() {
        *status = match result {
            Some(Ok(value)) => value,
            _ => 0,
        };
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_DetachThread(thread: *mut SDL_Thread) {
    if thread.is_null() {
        return;
    }
    let mut handle = Box::from_raw(thread as *mut ThreadHandle);
    let _ = handle.join.take();
}

#[no_mangle]
pub unsafe extern "C" fn SDL_TLSCreate() -> SDL_TLSID {
    next_tls_id().fetch_add(1, Ordering::Relaxed)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_TLSGet(id: SDL_TLSID) -> *mut libc::c_void {
    TLS_MAP.with(|map| {
        map.borrow()
            .get(&id)
            .map(|value| value.value)
            .unwrap_or(std::ptr::null_mut())
    })
}

#[no_mangle]
pub unsafe extern "C" fn SDL_TLSSet(
    id: SDL_TLSID,
    value: *const libc::c_void,
    destructor: Option<unsafe extern "C" fn(arg1: *mut libc::c_void)>,
) -> libc::c_int {
    if id == 0 {
        return crate::core::error::invalid_param_error("id");
    }

    let old = TLS_MAP.with(|map| {
        map.borrow_mut().insert(
            id,
            ThreadTlsValue {
                value: value.cast_mut(),
                destructor,
            },
        )
    });
    if let Some(old) = old {
        if let Some(old_destructor) = old.destructor {
            old_destructor(old.value);
        }
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_TLSCleanup() {
    cleanup_current_thread_tls();
}

#[no_mangle]
pub unsafe extern "C" fn SDL_LinuxSetThreadPriority(
    threadID: Sint64,
    priority: libc::c_int,
) -> libc::c_int {
    if libc::setpriority(libc::PRIO_PROCESS, threadID as u32, priority) == 0 {
        0
    } else {
        crate::core::error::set_error_message("setpriority() failed")
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_LinuxSetThreadPriorityAndPolicy(
    threadID: Sint64,
    sdlPriority: libc::c_int,
    schedPolicy: libc::c_int,
) -> libc::c_int {
    let os_priority = priority_to_nice(sdlPriority);
    if schedPolicy == libc::SCHED_FIFO || schedPolicy == libc::SCHED_RR {
        let param = libc::sched_param {
            sched_priority: match sdlPriority as u32 {
                SDL_ThreadPriority_SDL_THREAD_PRIORITY_LOW => 1,
                SDL_ThreadPriority_SDL_THREAD_PRIORITY_HIGH => 50,
                SDL_ThreadPriority_SDL_THREAD_PRIORITY_TIME_CRITICAL => 99,
                _ => 25,
            },
        };
        if libc::sched_setscheduler(threadID as i32, schedPolicy, &param) == 0 {
            return 0;
        }
    } else if libc::setpriority(libc::PRIO_PROCESS, threadID as u32, os_priority) == 0 {
        return 0;
    }
    crate::core::error::set_error_message("setpriority() failed")
}
