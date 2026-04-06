use std::ffi::{CStr, CString, OsStr};
use std::fs;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub mod blit;
pub mod bmp;
pub mod clipboard;
pub mod display;
pub mod dummy;
pub mod egl;
pub mod messagebox;
pub mod offscreen;
pub mod pixels;
pub mod rect;
pub mod shape;
pub mod surface;
pub mod syswm;
pub mod vulkan;
pub mod window;

pub mod linux {
    pub mod ime;
    pub mod kmsdrm;
    pub mod wayland;
    pub mod x11;
}

const DEFAULT_REAL_SDL_CANDIDATES: [&[u8]; 4] = [
    b"/lib/x86_64-linux-gnu/libSDL2-2.0.so.0.0.0\0",
    b"/lib/x86_64-linux-gnu/libSDL2-2.0.so.0\0",
    b"/usr/lib/x86_64-linux-gnu/libSDL2-2.0.so.0.0.0\0",
    b"/usr/lib/x86_64-linux-gnu/libSDL2-2.0.so.0\0",
];
const SAFE_RUNTIME_DISABLE_ENV: &str = "SAFE_SDL_DISABLE_REAL_RUNTIME";

pub(crate) fn real_sdl_dlopen_candidates() -> Vec<CString> {
    if real_runtime_autoload_disabled() {
        return Vec::new();
    }
    let mut candidates = Vec::with_capacity(DEFAULT_REAL_SDL_CANDIDATES.len() + 1);
    if let Some(path) = std::env::var_os("SAFE_SDL_REAL_SDL_PATH") {
        if !path.is_empty() {
            if let Ok(path) = CString::new(path.as_os_str().as_bytes()) {
                candidates.push(path);
            }
        }
    }
    candidates.extend(
        DEFAULT_REAL_SDL_CANDIDATES
            .iter()
            .map(|candidate| CString::from_vec_with_nul((*candidate).to_vec()).unwrap()),
    );
    candidates
}

fn open_real_sdl() -> *mut libc::c_void {
    try_open_real_sdl_with_flags(real_sdl_dlopen_flags())
        .unwrap_or_else(|| panic!("unable to load the SDL2 compatibility runtime"))
}

pub(crate) fn open_real_sdl_with_flags(flags: libc::c_int) -> *mut libc::c_void {
    try_open_real_sdl_with_flags(flags)
        .unwrap_or_else(|| panic!("unable to load the SDL2 compatibility runtime"))
}

pub(crate) fn try_open_real_sdl_with_flags(flags: libc::c_int) -> Option<*mut libc::c_void> {
    for candidate in real_sdl_dlopen_candidates() {
        if should_skip_real_sdl_candidate(candidate.as_c_str()) {
            continue;
        }
        let handle = unsafe { libc::dlopen(candidate.as_ptr(), flags) };
        if !handle.is_null() {
            return Some(handle);
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn real_sdl_dlopen_flags() -> libc::c_int {
    libc::RTLD_LOCAL | libc::RTLD_NOW | libc::RTLD_DEEPBIND
}

#[cfg(not(target_os = "linux"))]
fn real_sdl_dlopen_flags() -> libc::c_int {
    libc::RTLD_LOCAL | libc::RTLD_NOW
}

pub(crate) fn real_sdl_handle() -> *mut libc::c_void {
    try_real_sdl_handle().unwrap_or_else(|| panic!("unable to load the SDL2 compatibility runtime"))
}

pub(crate) fn try_real_sdl_handle() -> Option<*mut libc::c_void> {
    real_sdl_handle_slot()
        .get_or_init(|| {
            try_open_real_sdl_with_flags(real_sdl_dlopen_flags()).map(|handle| handle as usize)
        })
        .as_ref()
        .map(|handle| *handle as *mut libc::c_void)
}

pub(crate) fn real_sdl_is_available() -> bool {
    try_real_sdl_handle().is_some()
}

fn loaded_real_sdl_handle() -> Option<*mut libc::c_void> {
    real_sdl_handle_slot()
        .get()
        .copied()
        .flatten()
        .map(|handle| handle as *mut libc::c_void)
}

pub(crate) fn real_sdl_is_loaded() -> bool {
    loaded_real_sdl_handle().is_some()
}

fn real_sdl_handle_slot() -> &'static OnceLock<Option<usize>> {
    static HANDLE: OnceLock<Option<usize>> = OnceLock::new();
    &HANDLE
}

fn should_skip_real_sdl_candidate(candidate: &CStr) -> bool {
    let bytes = candidate.to_bytes();
    if !bytes.starts_with(b"/") {
        return false;
    }
    let self_path = current_library_path().and_then(|path| fs::canonicalize(path).ok());
    let candidate_path = fs::canonicalize(Path::new(OsStr::from_bytes(bytes))).ok();
    matches!((&self_path, &candidate_path), (Some(self_path), Some(candidate_path)) if self_path == candidate_path)
        || candidate_path
            .as_deref()
            .map(candidate_looks_like_safe_sdl)
            .unwrap_or(false)
}

fn real_runtime_autoload_disabled() -> bool {
    matches!(
        std::env::var_os(SAFE_RUNTIME_DISABLE_ENV),
        Some(value) if !value.is_empty() && value != "0"
    )
}

fn candidate_looks_like_safe_sdl(path: &Path) -> bool {
    const SAFE_MARKERS: [&[u8]; 2] = [
        b"safe_sdl_store_error_message",
        b"safe-sdl bootstrap stub called",
    ];

    let Ok(bytes) = fs::read(path) else {
        return false;
    };
    SAFE_MARKERS
        .iter()
        .any(|marker| bytes.windows(marker.len()).any(|window| window == *marker))
}

fn current_library_path() -> Option<PathBuf> {
    let mut info = std::mem::MaybeUninit::<libc::Dl_info>::uninit();
    let rc = unsafe {
        libc::dladdr(
            current_library_path as *const () as *const libc::c_void,
            info.as_mut_ptr(),
        )
    };
    if rc == 0 {
        return None;
    }
    let info = unsafe { info.assume_init() };
    if info.dli_fname.is_null() {
        return None;
    }
    let bytes = unsafe { CStr::from_ptr(info.dli_fname) }
        .to_bytes()
        .to_vec();
    Some(PathBuf::from(std::ffi::OsString::from_vec(bytes)))
}

pub(crate) fn load_symbol<T>(name: &[u8]) -> T {
    let symbol = unsafe { libc::dlsym(real_sdl_handle(), name.as_ptr().cast()) };
    assert!(
        !symbol.is_null(),
        "missing host SDL2 symbol {}",
        String::from_utf8_lossy(&name[..name.len().saturating_sub(1)])
    );
    unsafe { std::mem::transmute_copy(&symbol) }
}

pub(crate) fn try_load_symbol<T>(name: &[u8]) -> Option<T> {
    let handle = try_real_sdl_handle()?;
    let symbol = unsafe { libc::dlsym(handle, name.as_ptr().cast()) };
    if symbol.is_null() {
        None
    } else {
        Some(unsafe { std::mem::transmute_copy(&symbol) })
    }
}

type GetErrorFn = unsafe extern "C" fn() -> *const libc::c_char;
type ClearErrorFn = unsafe extern "C" fn();

fn real_get_error() -> GetErrorFn {
    static FN: OnceLock<GetErrorFn> = OnceLock::new();
    *FN.get_or_init(|| load_symbol(b"SDL_GetError\0"))
}

fn real_clear_error_fn() -> ClearErrorFn {
    static FN: OnceLock<ClearErrorFn> = OnceLock::new();
    *FN.get_or_init(|| load_symbol(b"SDL_ClearError\0"))
}

pub(crate) fn clear_real_error() {
    if loaded_real_sdl_handle().is_some() {
        unsafe {
            real_clear_error_fn()();
        }
    }
}

pub(crate) fn real_error_ptr() -> *const libc::c_char {
    if loaded_real_sdl_handle().is_some() {
        unsafe { real_get_error()() }
    } else {
        std::ptr::null()
    }
}
