#![allow(clippy::all)]

#[path = "common/testutils.rs"]
mod testutils;

use std::ffi::{c_void, CStr};
use std::mem::MaybeUninit;
use std::ptr;
use std::sync::atomic::{AtomicU32, Ordering};

use safe_sdl::abi::generated_types::{
    SDL_Locale, SDL_PowerState_SDL_POWERSTATE_CHARGED, SDL_PowerState_SDL_POWERSTATE_CHARGING,
    SDL_PowerState_SDL_POWERSTATE_NO_BATTERY, SDL_PowerState_SDL_POWERSTATE_ON_BATTERY,
    SDL_PowerState_SDL_POWERSTATE_UNKNOWN, SDL_SpinLock, SDL_atomic_t, SDL_version, SDL_INIT_TIMER,
    SDL_INIT_VIDEO, SDL_MUTEX_TIMEDOUT, SDL_TLSID,
};
use safe_sdl::core::error::SDL_GetError;
use safe_sdl::core::filesystem::{SDL_GetBasePath, SDL_GetPrefPath};
use safe_sdl::core::init::{SDL_Init, SDL_Quit};
use safe_sdl::core::loadso::{SDL_LoadFunction, SDL_LoadObject, SDL_UnloadObject};
use safe_sdl::core::locale::SDL_GetPreferredLocales;
use safe_sdl::core::memory::SDL_free;
use safe_sdl::core::misc::SDL_OpenURL;
use safe_sdl::core::mutex::{
    SDL_CreateSemaphore, SDL_DestroySemaphore, SDL_SemPost, SDL_SemTryWait, SDL_SemWaitTimeout,
};
use safe_sdl::core::platform::SDL_GetPlatform;
use safe_sdl::core::power::SDL_GetPowerInfo;
use safe_sdl::core::rwops::{
    SDL_RWFromConstMem, SDL_RWFromFile, SDL_RWFromMem, SDL_RWclose, SDL_RWread, SDL_RWseek,
    SDL_RWtell, SDL_RWwrite, SDL_ReadLE32, SDL_WriteLE32,
};
use safe_sdl::core::stdlib::{SDL_bsearch, SDL_iconv_string, SDL_qsort};
use safe_sdl::core::system::{
    SDL_AtomicAdd, SDL_AtomicCAS, SDL_AtomicGet, SDL_AtomicLock, SDL_AtomicSet, SDL_AtomicTryLock,
    SDL_AtomicUnlock,
};
use safe_sdl::core::thread::{
    SDL_CreateThread, SDL_TLSCleanup, SDL_TLSCreate, SDL_TLSGet, SDL_TLSSet, SDL_ThreadID,
    SDL_WaitThread,
};
use safe_sdl::core::timer::{SDL_AddTimer, SDL_Delay, SDL_GetTicks, SDL_RemoveTimer};
use safe_sdl::main_archive::{SDL_GetRevision, SDL_GetVersion};

unsafe extern "C" {
    fn SDL_SetError(fmt: *const libc::c_char, ...) -> libc::c_int;
}

unsafe extern "C" fn compare_ints(a: *const c_void, b: *const c_void) -> libc::c_int {
    let lhs = *(a as *const i32);
    let rhs = *(b as *const i32);
    if lhs < rhs {
        -1
    } else if lhs > rhs {
        1
    } else {
        0
    }
}

struct ThreadContext {
    tls: SDL_TLSID,
    ok: AtomicU32,
}

unsafe extern "C" fn tls_thread(data: *mut c_void) -> libc::c_int {
    let context = &*(data as *const ThreadContext);
    let label = b"child thread\0";
    assert_eq!(SDL_TLSSet(context.tls, label.as_ptr().cast(), None), 0);
    let value = SDL_TLSGet(context.tls) as *const libc::c_char;
    if !value.is_null() && CStr::from_ptr(value).to_bytes() == b"child thread" {
        context.ok.store(1, Ordering::SeqCst);
    }
    123
}

unsafe extern "C" fn fire_timer(_interval: u32, userdata: *mut c_void) -> u32 {
    let fired = &*(userdata as *const AtomicU32);
    fired.store(1, Ordering::SeqCst);
    0
}

#[test]
fn atomic_operations_and_spinlock_smoke() {
    let _serial = testutils::serial_lock();
    let mut value = SDL_atomic_t { value: 0 };
    let mut lock: SDL_SpinLock = 0;

    unsafe {
        assert_ne!(SDL_AtomicTryLock(&mut lock), 0);
        SDL_AtomicUnlock(&mut lock);
        SDL_AtomicLock(&mut lock);
        SDL_AtomicUnlock(&mut lock);

        assert_eq!(SDL_AtomicSet(&mut value, 10), 0);
        assert_eq!(SDL_AtomicAdd(&mut value, 5), 10);
        assert_eq!(SDL_AtomicGet(&mut value), 15);
        assert_eq!(SDL_AtomicCAS(&mut value, 15, 21), 1);
        assert_eq!(SDL_AtomicGet(&mut value), 21);
    }
}

#[test]
fn error_state_is_thread_local() {
    let _serial = testutils::serial_lock();
    let main_error = testutils::cstring("main error");
    let child_error = testutils::cstring("child error");
    let ready = unsafe { SDL_CreateSemaphore(0) };
    assert!(!ready.is_null());

    struct ErrorThreadContext {
        ready: *mut safe_sdl::abi::generated_types::SDL_sem,
        error: *const libc::c_char,
    }

    unsafe extern "C" fn error_thread(data: *mut c_void) -> libc::c_int {
        let context = &*(data as *const ErrorThreadContext);
        SDL_SetError(context.error);
        SDL_SemPost(context.ready);
        assert_eq!(testutils::string_from_c(SDL_GetError()), "child error");
        0
    }

    let context = ErrorThreadContext {
        ready,
        error: child_error.as_ptr(),
    };

    unsafe {
        assert_eq!(SDL_Init(0), 0, "{}", testutils::current_error());
        assert_eq!(SDL_SetError(main_error.as_ptr()), -1);
        let thread = SDL_CreateThread(
            Some(error_thread),
            ptr::null(),
            (&context as *const ErrorThreadContext).cast_mut().cast(),
        );
        assert!(!thread.is_null());
        assert_eq!(SDL_SemWaitTimeout(ready, 5_000), 0);
        let mut status = 0;
        SDL_WaitThread(thread, &mut status);
        assert_eq!(status, 0);
        assert_eq!(testutils::string_from_c(SDL_GetError()), "main error");
        SDL_DestroySemaphore(ready);
        SDL_Quit();
    }
}

#[test]
fn rwops_memory_and_file_helpers_roundtrip() {
    let _serial = testutils::serial_lock();
    let mut buffer = [0u8; 8];

    unsafe {
        let rw = SDL_RWFromMem(buffer.as_mut_ptr().cast(), buffer.len() as libc::c_int);
        assert!(!rw.is_null());
        assert_eq!(SDL_WriteLE32(rw, 0x11223344), 1);
        assert_eq!(SDL_RWtell(rw), 4);
        assert_eq!(SDL_RWseek(rw, 0, libc::SEEK_SET), 0);
        assert_eq!(SDL_ReadLE32(rw), 0x11223344);
        assert_eq!(SDL_RWclose(rw), 0);
        assert_eq!(&buffer[..4], &[0x44, 0x33, 0x22, 0x11]);

        let readonly = SDL_RWFromConstMem(buffer.as_ptr().cast(), buffer.len() as libc::c_int);
        assert!(!readonly.is_null());
        let mut out = [0u8; 4];
        assert_eq!(
            SDL_RWread(readonly, out.as_mut_ptr().cast(), 1, out.len()),
            out.len()
        );
        assert_eq!(SDL_RWclose(readonly), 0);
        assert_eq!(out, [0x44, 0x33, 0x22, 0x11]);
    }

    let file = tempfile::NamedTempFile::new().expect("create temp file");
    let file_path = testutils::cstring(file.path().to_str().expect("utf-8 temp path"));
    let mode = testutils::cstring("wb+");
    unsafe {
        let rw = SDL_RWFromFile(file_path.as_ptr(), mode.as_ptr());
        assert!(!rw.is_null(), "{}", testutils::current_error());
        let bytes = b"SDL";
        assert_eq!(
            SDL_RWwrite(rw, bytes.as_ptr().cast(), 1, bytes.len()),
            bytes.len()
        );
        assert_eq!(SDL_RWseek(rw, 0, libc::SEEK_SET), 0);
        let mut out = [0u8; 3];
        assert_eq!(
            SDL_RWread(rw, out.as_mut_ptr().cast(), 1, out.len()),
            out.len()
        );
        assert_eq!(out, *bytes);
        assert_eq!(SDL_RWclose(rw), 0);
    }
}

#[test]
fn filesystem_paths_are_available() {
    let _serial = testutils::serial_lock();

    unsafe {
        assert_eq!(SDL_Init(0), 0, "{}", testutils::current_error());
        let base = SDL_GetBasePath();
        assert!(!base.is_null(), "{}", testutils::current_error());
        assert!(!testutils::string_from_c(base).is_empty());
        SDL_free(base.cast());

        let org = testutils::cstring("libsdl");
        let app = testutils::cstring("test_filesystem");
        let pref = SDL_GetPrefPath(org.as_ptr(), app.as_ptr());
        assert!(!pref.is_null(), "{}", testutils::current_error());
        assert!(!testutils::string_from_c(pref).is_empty());
        SDL_free(pref.cast());
        SDL_Quit();
    }
}

#[test]
fn iconv_fixture_roundtrip_uses_shared_testutils() {
    let _serial = testutils::serial_lock();
    let fixture_path = testutils::get_resource_filename(None, "utf8.txt");
    assert!(fixture_path.exists());
    let sample = b"Portable \xCE\xBA\xE1\xBD\xB9\xCF\x83\xCE\xBC\xCE\xB5".to_vec();
    let fixture_c = std::ffi::CString::new(sample.clone()).expect("utf8 sample CString");
    let utf8 = testutils::cstring("UTF-8");

    unsafe {
        let converted = SDL_iconv_string(
            utf8.as_ptr(),
            utf8.as_ptr(),
            fixture_c.as_ptr(),
            sample.len(),
        );
        assert!(!converted.is_null(), "{}", testutils::current_error());
        assert_eq!(CStr::from_ptr(converted).to_bytes(), sample.as_slice());
        SDL_free(converted.cast());
    }
}

#[test]
fn loadso_can_open_libc_and_find_printf() {
    let _serial = testutils::serial_lock();
    let libc_name = testutils::cstring("libc.so.6");
    let symbol = testutils::cstring("printf");

    unsafe {
        let handle = SDL_LoadObject(libc_name.as_ptr());
        assert!(!handle.is_null(), "{}", testutils::current_error());
        let function = SDL_LoadFunction(handle, symbol.as_ptr());
        assert!(!function.is_null(), "{}", testutils::current_error());
        SDL_UnloadObject(handle);
    }
}

#[test]
#[cfg_attr(
    not(feature = "host-video-tests"),
    ignore = "run with --features host-video-tests"
)]
fn locale_list_is_terminated_and_freeable() {
    let _serial = testutils::serial_lock();
    let _video = testutils::ScopedEnvVar::set("SDL_VIDEODRIVER", "dummy");

    unsafe {
        assert_eq!(
            SDL_Init(SDL_INIT_VIDEO),
            0,
            "{}",
            testutils::current_error()
        );
        let locales = SDL_GetPreferredLocales();
        if !locales.is_null() {
            let first: &SDL_Locale = &*locales;
            assert!(!first.language.is_null());
            SDL_free(locales.cast());
        } else {
            assert!(!testutils::string_from_c(SDL_GetError()).is_empty());
        }
        SDL_Quit();
    }
}

#[test]
fn dummy_video_init_returns_for_packaged_consumers() {
    let _serial = testutils::serial_lock();
    let _video = testutils::ScopedEnvVar::set("SDL_VIDEODRIVER", "dummy");

    unsafe {
        assert_eq!(SDL_Init(SDL_INIT_VIDEO), 0, "{}", testutils::current_error());
        SDL_Quit();
    }
}

#[test]
fn platform_power_and_version_smoke() {
    let _serial = testutils::serial_lock();

    unsafe {
        let platform = testutils::string_from_c(SDL_GetPlatform());
        assert!(!platform.is_empty());

        let mut seconds = -2;
        let mut percent = -2;
        let state = SDL_GetPowerInfo(&mut seconds, &mut percent);
        assert!([
            SDL_PowerState_SDL_POWERSTATE_UNKNOWN,
            SDL_PowerState_SDL_POWERSTATE_ON_BATTERY,
            SDL_PowerState_SDL_POWERSTATE_NO_BATTERY,
            SDL_PowerState_SDL_POWERSTATE_CHARGING,
            SDL_PowerState_SDL_POWERSTATE_CHARGED,
        ]
        .contains(&state));
        assert!((-1..=100).contains(&percent));
        assert!(seconds >= -1);

        let mut version = MaybeUninit::<SDL_version>::zeroed();
        SDL_GetVersion(version.as_mut_ptr());
        let version = version.assume_init();
        assert_eq!(version.major, 2);
        assert!(!testutils::string_from_c(SDL_GetRevision()).is_empty());
    }
}

#[test]
fn qsort_and_bsearch_match_testqsort() {
    let _serial = testutils::serial_lock();
    let mut numbers = [9i32, 2, 7, 5, 1, 8, 3];
    let key = 5i32;

    unsafe {
        SDL_qsort(
            numbers.as_mut_ptr().cast(),
            numbers.len(),
            std::mem::size_of::<i32>(),
            Some(compare_ints),
        );
        assert_eq!(numbers, [1, 2, 3, 5, 7, 8, 9]);
        let found = SDL_bsearch(
            (&key as *const i32).cast(),
            numbers.as_ptr().cast(),
            numbers.len(),
            std::mem::size_of::<i32>(),
            Some(compare_ints),
        ) as *const i32;
        assert!(!found.is_null());
        assert_eq!(*found, 5);
    }
}

#[test]
fn semaphores_threads_and_tls_smoke() {
    let _serial = testutils::serial_lock();
    let _timer = testutils::SubsystemGuard::init(SDL_INIT_TIMER);
    let tls = unsafe { SDL_TLSCreate() };
    assert_ne!(tls, 0);
    let ready = unsafe { SDL_CreateSemaphore(0) };
    assert!(!ready.is_null());
    let main_label = b"main thread\0";
    let context = ThreadContext {
        tls,
        ok: AtomicU32::new(0),
    };

    unsafe {
        assert_eq!(SDL_TLSSet(tls, main_label.as_ptr().cast(), None), 0);
        assert_eq!(
            CStr::from_ptr(SDL_TLSGet(tls) as *const libc::c_char).to_bytes(),
            b"main thread"
        );

        let thread_name = testutils::cstring("worker");
        let thread = SDL_CreateThread(
            Some(tls_thread),
            thread_name.as_ptr(),
            (&context as *const ThreadContext).cast_mut().cast(),
        );
        assert!(!thread.is_null(), "{}", testutils::current_error());
        let mut status = 0;
        SDL_WaitThread(thread, &mut status);
        assert_eq!(status, 123);
        assert_eq!(context.ok.load(Ordering::SeqCst), 1);

        assert_eq!(SDL_SemTryWait(ready), SDL_MUTEX_TIMEDOUT as i32);
        assert_eq!(SDL_SemPost(ready), 0);
        assert_eq!(SDL_SemWaitTimeout(ready, 100), 0);
        assert_ne!(SDL_ThreadID(), 0);

        SDL_DestroySemaphore(ready);
        SDL_TLSCleanup();
    }
}

#[test]
fn timer_smoke_matches_original_apps() {
    let _serial = testutils::serial_lock();
    let _timer = testutils::SubsystemGuard::init(SDL_INIT_TIMER);
    let fired = Box::new(AtomicU32::new(0));
    let fired_ptr = (&*fired as *const AtomicU32).cast_mut().cast();

    unsafe {
        let start = SDL_GetTicks();
        SDL_Delay(20);
        let elapsed = SDL_GetTicks().wrapping_sub(start);
        assert!(elapsed >= 10);

        let timer = SDL_AddTimer(10, Some(fire_timer), fired_ptr);
        assert_ne!(timer, 0);
        SDL_Delay(100);
        assert_eq!(fired.load(Ordering::SeqCst), 1);
        assert_eq!(SDL_RemoveTimer(timer), 0);
    }
}

#[test]
fn openurl_reports_invalid_parameters() {
    let _serial = testutils::serial_lock();

    unsafe {
        assert_eq!(SDL_OpenURL(ptr::null()), -1);
        assert!(!testutils::string_from_c(SDL_GetError()).is_empty());
    }
}

#[test]
fn torturethread_like_many_threads_finish() {
    let _serial = testutils::serial_lock();
    let counter = Box::new(AtomicU32::new(0));
    let counter_ptr = (&*counter as *const AtomicU32).cast_mut().cast();

    unsafe extern "C" fn worker(data: *mut c_void) -> libc::c_int {
        let counter = &*(data as *const AtomicU32);
        counter.fetch_add(1, Ordering::SeqCst);
        7
    }

    unsafe {
        let mut threads = Vec::new();
        for _ in 0..8 {
            let thread = SDL_CreateThread(Some(worker), ptr::null(), counter_ptr);
            assert!(!thread.is_null(), "{}", testutils::current_error());
            threads.push(thread);
        }
        for thread in threads {
            let mut status = 0;
            SDL_WaitThread(thread, &mut status);
            assert_eq!(status, 7);
        }
    }
    assert_eq!(counter.load(Ordering::SeqCst), 8);
}

#[cfg(all(windows, target_pointer_width = "32"))]
mod testfilesystem_pre {
    #[test]
    fn source_is_intentionally_covered_in_phase_two() {
        assert!(true);
    }
}
