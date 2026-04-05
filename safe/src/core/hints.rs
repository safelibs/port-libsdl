use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::sync::{Mutex, OnceLock};

use crate::abi::generated_types::{
    SDL_HintCallback, SDL_HintPriority, SDL_HintPriority_SDL_HINT_DEFAULT,
    SDL_HintPriority_SDL_HINT_NORMAL, SDL_HintPriority_SDL_HINT_OVERRIDE, SDL_bool,
};
use crate::core::system::bool_to_sdl;

#[derive(Clone, Copy)]
struct HintWatcher {
    callback: SDL_HintCallback,
    userdata: *mut libc::c_void,
}

unsafe impl Send for HintWatcher {}

struct HintEntry {
    value: Option<CString>,
    priority: SDL_HintPriority,
    watchers: Vec<HintWatcher>,
}

fn hints() -> &'static Mutex<HashMap<String, HintEntry>> {
    static HINTS: OnceLock<Mutex<HashMap<String, HintEntry>>> = OnceLock::new();
    HINTS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_hints() -> std::sync::MutexGuard<'static, HashMap<String, HintEntry>> {
    match hints().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn env_value(name: &CStr) -> Option<CString> {
    unsafe {
        let value = libc::getenv(name.as_ptr());
        if value.is_null() {
            None
        } else {
            Some(CStr::from_ptr(value).to_owned())
        }
    }
}

fn c_string_from_ptr(value: *const libc::c_char) -> Option<CString> {
    if value.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(value).to_owned() })
    }
}

fn effective_value(entry: &HintEntry, env: Option<&CString>) -> Option<CString> {
    if env.is_none() || entry.priority == SDL_HintPriority_SDL_HINT_OVERRIDE {
        entry.value.clone()
    } else {
        env.cloned()
    }
}

fn call_watchers(
    watchers: &[HintWatcher],
    name: &CStr,
    old_value: Option<&CString>,
    new_value: Option<&CString>,
) {
    let old_ptr = old_value
        .map(|value| value.as_ptr())
        .unwrap_or(std::ptr::null());
    let new_ptr = new_value
        .map(|value| value.as_ptr())
        .unwrap_or(std::ptr::null());
    for watcher in watchers {
        if let Some(callback) = watcher.callback {
            unsafe {
                callback(watcher.userdata, name.as_ptr(), old_ptr, new_ptr);
            }
        }
    }
}

fn parse_bool_string(value: Option<&CStr>, default_value: SDL_bool) -> SDL_bool {
    match value.and_then(|value| value.to_str().ok()) {
        None | Some("") => default_value,
        Some("0") => crate::abi::generated_types::SDL_bool_SDL_FALSE,
        Some(text) if text.eq_ignore_ascii_case("false") => {
            crate::abi::generated_types::SDL_bool_SDL_FALSE
        }
        Some(_) => crate::abi::generated_types::SDL_bool_SDL_TRUE,
    }
}

fn callbacks_match(lhs: SDL_HintCallback, rhs: SDL_HintCallback) -> bool {
    match (lhs, rhs) {
        (Some(lhs), Some(rhs)) => std::ptr::fn_addr_eq(lhs, rhs),
        (None, None) => true,
        _ => false,
    }
}

type RealSetHintWithPriorityFn =
    unsafe extern "C" fn(*const libc::c_char, *const libc::c_char, SDL_HintPriority) -> SDL_bool;
type RealResetHintFn = unsafe extern "C" fn(*const libc::c_char) -> SDL_bool;
type RealResetHintsFn = unsafe extern "C" fn();
type RealClearHintsFn = unsafe extern "C" fn();

fn real_set_hint_with_priority_fn() -> RealSetHintWithPriorityFn {
    static FN: OnceLock<RealSetHintWithPriorityFn> = OnceLock::new();
    *FN.get_or_init(|| crate::video::load_symbol(b"SDL_SetHintWithPriority\0"))
}

fn real_reset_hint_fn() -> RealResetHintFn {
    static FN: OnceLock<RealResetHintFn> = OnceLock::new();
    *FN.get_or_init(|| crate::video::load_symbol(b"SDL_ResetHint\0"))
}

fn real_reset_hints_fn() -> RealResetHintsFn {
    static FN: OnceLock<RealResetHintsFn> = OnceLock::new();
    *FN.get_or_init(|| crate::video::load_symbol(b"SDL_ResetHints\0"))
}

fn real_clear_hints_fn() -> RealClearHintsFn {
    static FN: OnceLock<RealClearHintsFn> = OnceLock::new();
    *FN.get_or_init(|| crate::video::load_symbol(b"SDL_ClearHints\0"))
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetHintWithPriority(
    name: *const libc::c_char,
    value: *const libc::c_char,
    priority: SDL_HintPriority,
) -> SDL_bool {
    if name.is_null() {
        return crate::abi::generated_types::SDL_bool_SDL_FALSE;
    }
    let name = CStr::from_ptr(name);
    let key = name.to_string_lossy().into_owned();
    let env = env_value(name);
    if env.is_some() && priority < SDL_HintPriority_SDL_HINT_OVERRIDE {
        return crate::abi::generated_types::SDL_bool_SDL_FALSE;
    }

    let new_value = c_string_from_ptr(value);
    let (watchers, old_effective, new_effective) = {
        let mut state = lock_hints();
        let entry = state.entry(key).or_insert_with(|| HintEntry {
            value: None,
            priority: SDL_HintPriority_SDL_HINT_DEFAULT,
            watchers: Vec::new(),
        });

        if priority < entry.priority {
            return crate::abi::generated_types::SDL_bool_SDL_FALSE;
        }

        let old_effective = effective_value(entry, env.as_ref());
        entry.value = new_value.clone();
        entry.priority = priority;
        let new_effective = effective_value(entry, env.as_ref());
        (entry.watchers.clone(), old_effective, new_effective)
    };

    if old_effective.as_ref().map(|value| value.as_bytes())
        != new_effective.as_ref().map(|value| value.as_bytes())
    {
        call_watchers(
            &watchers,
            name,
            old_effective.as_ref(),
            new_effective.as_ref(),
        );
    }

    if crate::video::real_sdl_is_loaded() {
        crate::video::clear_real_error();
        let _ = real_set_hint_with_priority_fn()(name.as_ptr(), value, priority);
    }

    crate::abi::generated_types::SDL_bool_SDL_TRUE
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetHint(
    name: *const libc::c_char,
    value: *const libc::c_char,
) -> SDL_bool {
    SDL_SetHintWithPriority(name, value, SDL_HintPriority_SDL_HINT_NORMAL)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ResetHint(name: *const libc::c_char) -> SDL_bool {
    if name.is_null() {
        return crate::abi::generated_types::SDL_bool_SDL_FALSE;
    }
    let name = CStr::from_ptr(name);
    let key = name.to_string_lossy().into_owned();
    let env = env_value(name);
    let (watchers, old_effective, new_effective, found) = {
        let mut state = lock_hints();
        if let Some(entry) = state.get_mut(&key) {
            let old_effective = effective_value(entry, env.as_ref());
            entry.value = None;
            entry.priority = SDL_HintPriority_SDL_HINT_DEFAULT;
            let new_effective = effective_value(entry, env.as_ref());
            (entry.watchers.clone(), old_effective, new_effective, true)
        } else {
            (Vec::new(), None, None, false)
        }
    };

    if found
        && old_effective.as_ref().map(|value| value.as_bytes())
            != new_effective.as_ref().map(|value| value.as_bytes())
    {
        call_watchers(
            &watchers,
            name,
            old_effective.as_ref(),
            new_effective.as_ref(),
        );
    }

    if crate::video::real_sdl_is_loaded() {
        crate::video::clear_real_error();
        let _ = real_reset_hint_fn()(name.as_ptr());
    }

    bool_to_sdl(found)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ResetHints() {
    let updates = {
        let mut state = lock_hints();
        state
            .iter_mut()
            .map(|(key, entry)| {
                let name = CString::new(key.as_str()).unwrap_or_default();
                let env = env_value(&name);
                let old_effective = effective_value(entry, env.as_ref());
                entry.value = None;
                entry.priority = SDL_HintPriority_SDL_HINT_DEFAULT;
                let new_effective = effective_value(entry, env.as_ref());
                (name, entry.watchers.clone(), old_effective, new_effective)
            })
            .collect::<Vec<_>>()
    };

    for (name, watchers, old_effective, new_effective) in updates {
        if old_effective.as_ref().map(|value| value.as_bytes())
            != new_effective.as_ref().map(|value| value.as_bytes())
        {
            call_watchers(
                &watchers,
                &name,
                old_effective.as_ref(),
                new_effective.as_ref(),
            );
        }
    }

    if crate::video::real_sdl_is_loaded() {
        crate::video::clear_real_error();
        real_reset_hints_fn()();
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetHint(name: *const libc::c_char) -> *const libc::c_char {
    if name.is_null() {
        return std::ptr::null();
    }
    let name = CStr::from_ptr(name);
    let key = name.to_string_lossy();
    let env = libc::getenv(name.as_ptr());
    let state = lock_hints();
    if let Some(entry) = state.get(key.as_ref()) {
        if env.is_null() || entry.priority == SDL_HintPriority_SDL_HINT_OVERRIDE {
            return entry
                .value
                .as_ref()
                .map(|value| value.as_ptr())
                .unwrap_or(std::ptr::null());
        }
    }
    env
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetHintBoolean(
    name: *const libc::c_char,
    default_value: SDL_bool,
) -> SDL_bool {
    let hint = SDL_GetHint(name);
    if hint.is_null() {
        return default_value;
    }
    parse_bool_string(Some(CStr::from_ptr(hint)), default_value)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_AddHintCallback(
    name: *const libc::c_char,
    callback: SDL_HintCallback,
    userdata: *mut libc::c_void,
) {
    if name.is_null() || *name == 0 {
        let _ = crate::core::error::invalid_param_error("name");
        return;
    }
    if callback.is_none() {
        let _ = crate::core::error::invalid_param_error("callback");
        return;
    }

    let name_cstr = CStr::from_ptr(name);
    let key = name_cstr.to_string_lossy().into_owned();
    let current_value = {
        let mut state = lock_hints();
        let entry = state.entry(key).or_insert_with(|| HintEntry {
            value: None,
            priority: SDL_HintPriority_SDL_HINT_DEFAULT,
            watchers: Vec::new(),
        });
        entry.watchers.retain(|watcher| {
            !callbacks_match(watcher.callback, callback) || watcher.userdata != userdata
        });
        entry.watchers.push(HintWatcher { callback, userdata });
        effective_value(entry, env_value(name_cstr).as_ref())
    };

    if let Some(callback) = callback {
        callback(
            userdata,
            name,
            current_value
                .as_ref()
                .map(|value| value.as_ptr())
                .unwrap_or(std::ptr::null()),
            current_value
                .as_ref()
                .map(|value| value.as_ptr())
                .unwrap_or(std::ptr::null()),
        );
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_DelHintCallback(
    name: *const libc::c_char,
    callback: SDL_HintCallback,
    userdata: *mut libc::c_void,
) {
    if name.is_null() {
        return;
    }
    let key = CStr::from_ptr(name).to_string_lossy().into_owned();
    if let Some(entry) = lock_hints().get_mut(&key) {
        entry.watchers.retain(|watcher| {
            !callbacks_match(watcher.callback, callback) || watcher.userdata != userdata
        });
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ClearHints() {
    lock_hints().clear();
    if crate::video::real_sdl_is_loaded() {
        crate::video::clear_real_error();
        real_clear_hints_fn()();
    }
}
