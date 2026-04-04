use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use crate::abi::generated_types::{
    SDL_AssertData, SDL_AssertState, SDL_AssertState_SDL_ASSERTION_ALWAYS_IGNORE,
    SDL_AssertState_SDL_ASSERTION_IGNORE, SDL_AssertionHandler,
};

struct AssertionState {
    handler: SDL_AssertionHandler,
    userdata: *mut libc::c_void,
    report_head: *mut SDL_AssertData,
    seen: HashSet<usize>,
}

unsafe impl Send for AssertionState {}

fn assertion_state() -> &'static Mutex<AssertionState> {
    static ASSERT_STATE: OnceLock<Mutex<AssertionState>> = OnceLock::new();
    ASSERT_STATE.get_or_init(|| {
        Mutex::new(AssertionState {
            handler: Some(default_assertion_handler),
            userdata: std::ptr::null_mut(),
            report_head: std::ptr::null_mut(),
            seen: HashSet::new(),
        })
    })
}

fn lock_assertion_state() -> std::sync::MutexGuard<'static, AssertionState> {
    match assertion_state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

unsafe extern "C" fn default_assertion_handler(
    _data: *const SDL_AssertData,
    _userdata: *mut libc::c_void,
) -> SDL_AssertState {
    SDL_AssertState_SDL_ASSERTION_IGNORE
}

pub(crate) fn assertions_quit() {
    unsafe {
        SDL_ResetAssertionReport();
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ReportAssertion(
    data: *mut SDL_AssertData,
    func: *const libc::c_char,
    file: *const libc::c_char,
    line: libc::c_int,
) -> SDL_AssertState {
    if data.is_null() {
        return SDL_AssertState_SDL_ASSERTION_IGNORE;
    }

    let (handler, userdata) = {
        let mut state = lock_assertion_state();
        (*data).function = func;
        (*data).filename = file;
        (*data).linenum = line;
        (*data).trigger_count = (*data).trigger_count.saturating_add(1);
        if state.seen.insert(data as usize) {
            (*data).next = state.report_head.cast_const();
            state.report_head = data;
        }
        (state.handler, state.userdata)
    };

    let result = if (*data).always_ignore != 0 {
        SDL_AssertState_SDL_ASSERTION_ALWAYS_IGNORE
    } else if let Some(handler) = handler {
        handler(data.cast_const(), userdata)
    } else {
        SDL_AssertState_SDL_ASSERTION_IGNORE
    };

    if result == SDL_AssertState_SDL_ASSERTION_ALWAYS_IGNORE {
        (*data).always_ignore = 1;
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetAssertionHandler(
    handler: SDL_AssertionHandler,
    userdata: *mut libc::c_void,
) {
    let mut state = lock_assertion_state();
    state.handler = handler.or(Some(default_assertion_handler));
    state.userdata = userdata;
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetDefaultAssertionHandler() -> SDL_AssertionHandler {
    Some(default_assertion_handler)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetAssertionHandler(
    puserdata: *mut *mut libc::c_void,
) -> SDL_AssertionHandler {
    let state = lock_assertion_state();
    if !puserdata.is_null() {
        *puserdata = state.userdata;
    }
    state.handler
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetAssertionReport() -> *const SDL_AssertData {
    lock_assertion_state().report_head.cast_const()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ResetAssertionReport() {
    let mut state = lock_assertion_state();
    let mut current = state.report_head;
    while !current.is_null() {
        let next = (*current).next as *mut SDL_AssertData;
        (*current).always_ignore = 0;
        (*current).trigger_count = 0;
        (*current).next = std::ptr::null();
        current = next;
    }
    state.report_head = std::ptr::null_mut();
    state.seen.clear();
}
