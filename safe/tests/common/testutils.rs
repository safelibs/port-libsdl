#![allow(dead_code)]

use std::ffi::{CStr, CString};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use safe_sdl::abi::generated_types::Uint32;
use safe_sdl::core::error::SDL_GetError;
use safe_sdl::core::init::{SDL_InitSubSystem, SDL_QuitSubSystem};

static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub fn serial_lock() -> MutexGuard<'static, ()> {
    match TEST_LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("safe crate lives under repo root")
        .to_path_buf()
}

pub fn resource_path(file: &str) -> PathBuf {
    repo_root().join("original/test").join(file)
}

pub fn get_nearby_filename(file: &str) -> PathBuf {
    let path = resource_path(file);
    if path.exists() {
        path
    } else {
        repo_root().join(file)
    }
}

pub fn get_resource_filename(user_specified: Option<&str>, default_name: &str) -> PathBuf {
    user_specified
        .map(PathBuf::from)
        .unwrap_or_else(|| get_nearby_filename(default_name))
}

pub fn load_utf8_fixture() -> Vec<u8> {
    std::fs::read(resource_path("utf8.txt")).expect("read utf8.txt fixture")
}

pub fn cstring(value: &str) -> CString {
    CString::new(value).expect("CString value")
}

pub unsafe fn string_from_c(ptr: *const libc::c_char) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

pub fn current_error() -> String {
    unsafe { string_from_c(SDL_GetError()) }
}

pub fn c_ptr(bytes: &[u8]) -> *const libc::c_char {
    bytes.as_ptr().cast()
}

pub struct ScopedEnvVar {
    key: String,
    previous: Option<String>,
}

impl ScopedEnvVar {
    pub fn set(key: &str, value: &str) -> Self {
        let previous = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self {
            key: key.to_string(),
            previous,
        }
    }
}

impl Drop for ScopedEnvVar {
    fn drop(&mut self) {
        if let Some(value) = &self.previous {
            std::env::set_var(&self.key, value);
        } else {
            std::env::remove_var(&self.key);
        }
    }
}

pub struct SubsystemGuard {
    flags: Uint32,
}

impl SubsystemGuard {
    pub fn init(flags: Uint32) -> Self {
        let rc = unsafe { SDL_InitSubSystem(flags) };
        assert_eq!(
            rc,
            0,
            "SDL_InitSubSystem({flags:#x}) failed: {}",
            current_error()
        );
        Self { flags }
    }
}

impl Drop for SubsystemGuard {
    fn drop(&mut self) {
        unsafe { SDL_QuitSubSystem(self.flags) };
    }
}
