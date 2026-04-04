use std::ffi::c_void;
use std::sync::OnceLock;

const SYSTEM_SDL_CANDIDATES: &[&str] = &[
    "/usr/lib/x86_64-linux-gnu/libSDL2-2.0.so.0",
    "/lib/x86_64-linux-gnu/libSDL2-2.0.so.0",
];

static SYSTEM_SDL_HANDLE: OnceLock<usize> = OnceLock::new();

fn fatal_load(message: &str) -> ! {
    eprintln!("safe-sdl phase-2 forwarding error: {message}");
    std::process::abort();
}

fn open_system_sdl() -> *mut c_void {
    let handle = SYSTEM_SDL_HANDLE.get_or_init(|| {
        let override_path = std::env::var("SAFE_SDL_SYSTEM_LIBSDL2").ok();
        let mut candidates = Vec::new();
        if let Some(path) = override_path.as_deref() {
            candidates.push(path.to_string());
        }
        candidates.extend(SYSTEM_SDL_CANDIDATES.iter().map(|path| path.to_string()));

        for candidate in candidates {
            let c_candidate = match std::ffi::CString::new(candidate.clone()) {
                Ok(value) => value,
                Err(_) => continue,
            };
            let handle =
                unsafe { libc::dlopen(c_candidate.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
            if !handle.is_null() {
                return handle as usize;
            }
        }

        let detail = unsafe {
            let ptr = libc::dlerror();
            if ptr.is_null() {
                "unknown loader error".to_string()
            } else {
                std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
            }
        };
        fatal_load(&format!("unable to open the system SDL runtime: {detail}"));
    });

    *handle as *mut c_void
}

pub(crate) unsafe fn load_symbol<T: Copy>(name: &'static [u8]) -> T {
    debug_assert_eq!(std::mem::size_of::<T>(), std::mem::size_of::<*mut c_void>());

    let symbol = libc::dlsym(open_system_sdl(), name.as_ptr().cast());
    if symbol.is_null() {
        let symbol_name = String::from_utf8_lossy(&name[..name.len().saturating_sub(1)]);
        fatal_load(&format!("missing symbol {symbol_name} in the system SDL runtime"));
    }

    std::mem::transmute_copy(&symbol)
}

unsafe extern "C" {
    fn safe_sdl_set_error_message(message: *const libc::c_char);
}

#[used]
static FORCE_PHASE2_VARIADIC_SHIMS_LINK: unsafe extern "C" fn(*const libc::c_char) =
    safe_sdl_set_error_message;

#[allow(dead_code)]
pub(crate) fn set_error_message(message: &str) {
    if let Ok(c_message) = std::ffi::CString::new(message) {
        unsafe { safe_sdl_set_error_message(c_message.as_ptr()) };
    }
}

#[macro_export]
macro_rules! forward_sdl {
    ($(fn $name:ident($($arg:ident : $ty:ty),* $(,)?) $(-> $ret:ty)?;)+) => {
        $(
            #[no_mangle]
            pub unsafe extern "C" fn $name($($arg: $ty),*) $(-> $ret)? {
                type FnTy = unsafe extern "C" fn($($ty),*) $(-> $ret)?;
                let real: FnTy = $crate::core::system::load_symbol(concat!(stringify!($name), "\0").as_bytes());
                real($($arg),*)
            }
        )+
    };
}

use crate::abi::generated_types::{SDL_SpinLock, SDL_atomic_t, SDL_bool};

crate::forward_sdl! {
    fn SDL_AtomicTryLock(lock: *mut SDL_SpinLock) -> SDL_bool;
    fn SDL_AtomicLock(lock: *mut SDL_SpinLock);
    fn SDL_AtomicUnlock(lock: *mut SDL_SpinLock);
    fn SDL_AtomicCAS(a: *mut SDL_atomic_t, oldval: libc::c_int, newval: libc::c_int) -> SDL_bool;
    fn SDL_AtomicSet(a: *mut SDL_atomic_t, v: libc::c_int) -> libc::c_int;
    fn SDL_AtomicGet(a: *mut SDL_atomic_t) -> libc::c_int;
    fn SDL_AtomicAdd(a: *mut SDL_atomic_t, v: libc::c_int) -> libc::c_int;
    fn SDL_AtomicCASPtr(a: *mut *mut libc::c_void, oldval: *mut libc::c_void, newval: *mut libc::c_void) -> SDL_bool;
    fn SDL_AtomicSetPtr(a: *mut *mut libc::c_void, v: *mut libc::c_void) -> *mut libc::c_void;
    fn SDL_AtomicGetPtr(a: *mut *mut libc::c_void) -> *mut libc::c_void;
}
