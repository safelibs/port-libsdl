use std::collections::HashMap;
use std::ffi::CStr;
use std::sync::{Mutex, OnceLock};

use crate::abi::generated_types::{
    SDL_LogCategory_SDL_LOG_CATEGORY_APPLICATION, SDL_LogCategory_SDL_LOG_CATEGORY_ASSERT,
    SDL_LogCategory_SDL_LOG_CATEGORY_CUSTOM, SDL_LogCategory_SDL_LOG_CATEGORY_TEST,
    SDL_LogOutputFunction, SDL_LogPriority, SDL_LogPriority_SDL_LOG_PRIORITY_DEBUG,
    SDL_LogPriority_SDL_LOG_PRIORITY_ERROR, SDL_LogPriority_SDL_LOG_PRIORITY_INFO,
    SDL_LogPriority_SDL_LOG_PRIORITY_VERBOSE, SDL_LogPriority_SDL_LOG_PRIORITY_WARN,
    SDL_LogPriority_SDL_NUM_LOG_PRIORITIES, SDL_HINT_LOGGING,
};

struct LogState {
    forced_priority: Option<SDL_LogPriority>,
    per_category: HashMap<libc::c_int, SDL_LogPriority>,
    callback: SDL_LogOutputFunction,
    userdata: *mut libc::c_void,
}

unsafe impl Send for LogState {}

fn log_state() -> &'static Mutex<LogState> {
    static LOG_STATE: OnceLock<Mutex<LogState>> = OnceLock::new();
    LOG_STATE.get_or_init(|| {
        Mutex::new(LogState {
            forced_priority: None,
            per_category: HashMap::new(),
            callback: Some(default_log_output),
            userdata: std::ptr::null_mut(),
        })
    })
}

fn lock_log_state() -> std::sync::MutexGuard<'static, LogState> {
    match log_state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

unsafe extern "C" fn default_log_output(
    _userdata: *mut libc::c_void,
    _category: libc::c_int,
    _priority: SDL_LogPriority,
    message: *const libc::c_char,
) {
    if message.is_null() {
        return;
    }
    eprintln!("{}", CStr::from_ptr(message).to_string_lossy());
}

fn default_priority(category: libc::c_int) -> SDL_LogPriority {
    match category {
        x if x == SDL_LogCategory_SDL_LOG_CATEGORY_APPLICATION as libc::c_int => {
            SDL_LogPriority_SDL_LOG_PRIORITY_INFO
        }
        x if x == SDL_LogCategory_SDL_LOG_CATEGORY_ASSERT as libc::c_int => {
            SDL_LogPriority_SDL_LOG_PRIORITY_WARN
        }
        x if x == SDL_LogCategory_SDL_LOG_CATEGORY_TEST as libc::c_int => {
            SDL_LogPriority_SDL_LOG_PRIORITY_VERBOSE
        }
        _ => SDL_LogPriority_SDL_LOG_PRIORITY_ERROR,
    }
}

fn parse_priority(value: &str) -> Option<SDL_LogPriority> {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "1" | "verbose" => Some(SDL_LogPriority_SDL_LOG_PRIORITY_VERBOSE),
        "2" | "debug" => Some(SDL_LogPriority_SDL_LOG_PRIORITY_DEBUG),
        "3" | "info" => Some(SDL_LogPriority_SDL_LOG_PRIORITY_INFO),
        "4" | "warn" | "warning" => Some(SDL_LogPriority_SDL_LOG_PRIORITY_WARN),
        "5" | "error" => Some(SDL_LogPriority_SDL_LOG_PRIORITY_ERROR),
        "6" | "critical" => {
            Some(crate::abi::generated_types::SDL_LogPriority_SDL_LOG_PRIORITY_CRITICAL)
        }
        "0" | "quiet" => Some(SDL_LogPriority_SDL_NUM_LOG_PRIORITIES),
        _ => None,
    }
}

fn parse_category(value: &str) -> Option<libc::c_int> {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "*" => Some(-1),
        "app" | "application" => Some(SDL_LogCategory_SDL_LOG_CATEGORY_APPLICATION as libc::c_int),
        "error" => {
            Some(crate::abi::generated_types::SDL_LogCategory_SDL_LOG_CATEGORY_ERROR as libc::c_int)
        }
        "assert" => Some(SDL_LogCategory_SDL_LOG_CATEGORY_ASSERT as libc::c_int),
        "system" => Some(
            crate::abi::generated_types::SDL_LogCategory_SDL_LOG_CATEGORY_SYSTEM as libc::c_int,
        ),
        "audio" => {
            Some(crate::abi::generated_types::SDL_LogCategory_SDL_LOG_CATEGORY_AUDIO as libc::c_int)
        }
        "video" => {
            Some(crate::abi::generated_types::SDL_LogCategory_SDL_LOG_CATEGORY_VIDEO as libc::c_int)
        }
        "render" => Some(
            crate::abi::generated_types::SDL_LogCategory_SDL_LOG_CATEGORY_RENDER as libc::c_int,
        ),
        "input" => {
            Some(crate::abi::generated_types::SDL_LogCategory_SDL_LOG_CATEGORY_INPUT as libc::c_int)
        }
        "test" => Some(SDL_LogCategory_SDL_LOG_CATEGORY_TEST as libc::c_int),
        _ => normalized.parse::<libc::c_int>().ok(),
    }
}

fn priority_from_hint(category: libc::c_int) -> Option<SDL_LogPriority> {
    unsafe {
        let hint = crate::core::hints::SDL_GetHint(SDL_HINT_LOGGING.as_ptr().cast());
        if hint.is_null() {
            return None;
        }
        let hint = CStr::from_ptr(hint).to_string_lossy();
        if !hint.contains('=') {
            return parse_priority(&hint);
        }
        let mut wildcard = None;
        for clause in hint.split(',') {
            let (raw_category, raw_priority) = clause.split_once('=')?;
            let parsed_category = parse_category(raw_category)?;
            let parsed_priority = parse_priority(raw_priority)?;
            if parsed_category == category {
                return Some(parsed_priority);
            }
            if parsed_category == -1 {
                wildcard = Some(parsed_priority);
            }
        }
        wildcard
    }
}

pub(crate) fn log_quit() {
    let mut state = lock_log_state();
    state.forced_priority = None;
    state.per_category.clear();
    state.callback = Some(default_log_output);
    state.userdata = std::ptr::null_mut();
}

#[no_mangle]
pub unsafe extern "C" fn SDL_LogSetAllPriority(priority: SDL_LogPriority) {
    let mut state = lock_log_state();
    state.per_category.clear();
    state.forced_priority = Some(priority);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_LogSetPriority(category: libc::c_int, priority: SDL_LogPriority) {
    let mut state = lock_log_state();
    state.per_category.insert(category, priority);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_LogGetPriority(category: libc::c_int) -> SDL_LogPriority {
    let state = lock_log_state();
    if let Some(priority) = state.per_category.get(&category).copied() {
        return priority;
    }
    if let Some(priority) = state.forced_priority {
        return priority;
    }
    drop(state);

    priority_from_hint(category).unwrap_or_else(|| {
        if category >= SDL_LogCategory_SDL_LOG_CATEGORY_CUSTOM as libc::c_int {
            SDL_LogPriority_SDL_LOG_PRIORITY_INFO
        } else {
            default_priority(category)
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn SDL_LogResetPriorities() {
    let mut state = lock_log_state();
    state.per_category.clear();
    state.forced_priority = None;
}

#[no_mangle]
pub unsafe extern "C" fn SDL_LogGetOutputFunction(
    callback: *mut SDL_LogOutputFunction,
    userdata: *mut *mut libc::c_void,
) {
    let state = lock_log_state();
    if !callback.is_null() {
        *callback = state.callback;
    }
    if !userdata.is_null() {
        *userdata = state.userdata;
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_LogSetOutputFunction(
    callback: SDL_LogOutputFunction,
    userdata: *mut libc::c_void,
) {
    let mut state = lock_log_state();
    state.callback = callback;
    state.userdata = userdata;
}

unsafe extern "C" {
    fn SDL_SetError();
    fn SDL_Log();
    fn SDL_LogVerbose();
    fn SDL_LogDebug();
    fn SDL_LogInfo();
    fn SDL_LogWarn();
    fn SDL_LogError();
    fn SDL_LogCritical();
    fn SDL_LogMessage();
    fn SDL_LogMessageV();
    fn SDL_vsnprintf();
    fn SDL_snprintf();
    fn SDL_vasprintf();
    fn SDL_asprintf();
    fn SDL_vsscanf();
    fn SDL_sscanf();
}

#[used]
static FORCE_PHASE2_VARIADIC_SHIMS_LINK: [unsafe extern "C" fn(); 16] = [
    SDL_SetError,
    SDL_Log,
    SDL_LogVerbose,
    SDL_LogDebug,
    SDL_LogInfo,
    SDL_LogWarn,
    SDL_LogError,
    SDL_LogCritical,
    SDL_LogMessage,
    SDL_LogMessageV,
    SDL_vsnprintf,
    SDL_snprintf,
    SDL_vasprintf,
    SDL_asprintf,
    SDL_vsscanf,
    SDL_sscanf,
];
