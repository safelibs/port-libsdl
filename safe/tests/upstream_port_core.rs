#![allow(clippy::all)]

#[path = "common/testutils.rs"]
mod testutils;

use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::ptr;
use std::sync::atomic::{AtomicU32, Ordering};

use safe_sdl::abi::generated_types::{
    SDL_HintPriority_SDL_HINT_DEFAULT, SDL_HintPriority_SDL_HINT_OVERRIDE,
    SDL_LogCategory_SDL_LOG_CATEGORY_APPLICATION, SDL_LogPriority,
    SDL_LogPriority_SDL_LOG_PRIORITY_DEBUG, SDL_LogPriority_SDL_LOG_PRIORITY_INFO,
    SDL_LogPriority_SDL_LOG_PRIORITY_WARN, SDL_PackedLayout_SDL_PACKEDLAYOUT_1010102,
    SDL_PackedOrder_SDL_PACKEDORDER_ABGR, SDL_PixelType_SDL_PIXELTYPE_PACKED32, SDL_version,
    SDL_INIT_AUDIO, SDL_INIT_EVENTS, SDL_INIT_TIMER, SDL_INIT_VIDEO,
};
use safe_sdl::audio::device::SDL_AudioQuit;
use safe_sdl::core::assert::{SDL_GetAssertionReport, SDL_ResetAssertionReport};
use safe_sdl::core::cpuinfo::{
    SDL_GetCPUCacheLineSize, SDL_GetCPUCount, SDL_GetSystemRAM, SDL_HasAVX, SDL_HasSSE,
    SDL_SIMDGetAlignment,
};
use safe_sdl::core::error::{SDL_ClearError, SDL_GetError};
use safe_sdl::core::hints::{
    SDL_AddHintCallback, SDL_DelHintCallback, SDL_GetHint, SDL_ResetHint, SDL_SetHint,
    SDL_SetHintWithPriority,
};
use safe_sdl::core::init::{SDL_InitSubSystem, SDL_Quit, SDL_QuitSubSystem, SDL_WasInit};
use safe_sdl::core::libm::{SDL_atan2, SDL_ceil, SDL_fabs, SDL_floor, SDL_log, SDL_pow, SDL_sqrt};
use safe_sdl::core::log::{
    SDL_LogGetOutputFunction, SDL_LogResetPriorities, SDL_LogSetOutputFunction,
};
use safe_sdl::core::stdlib::{
    SDL_bsearch, SDL_getenv, SDL_iconv_string, SDL_qsort, SDL_setenv, SDL_strcmp, SDL_strdup,
    SDL_strlcpy,
};
use safe_sdl::core::timer::{
    SDL_AddTimer, SDL_Delay, SDL_GetPerformanceCounter, SDL_GetPerformanceFrequency, SDL_GetTicks,
    SDL_RemoveTimer,
};
use safe_sdl::main_archive::{SDL_GetRevision, SDL_GetVersion};
use safe_sdl::video::pixels::{SDL_AllocFormat, SDL_GetPixelFormatName};

unsafe extern "C" {
    fn SDL_LogMessage(
        category: libc::c_int,
        priority: SDL_LogPriority,
        fmt: *const libc::c_char,
        ...
    );
    fn SDL_SetError(fmt: *const libc::c_char, ...) -> libc::c_int;
    fn SDL_snprintf(
        text: *mut libc::c_char,
        maxlen: usize,
        fmt: *const libc::c_char,
        ...
    ) -> libc::c_int;
}

unsafe extern "C" fn hint_callback(
    userdata: *mut c_void,
    _name: *const libc::c_char,
    _old: *const libc::c_char,
    hint: *const libc::c_char,
) {
    let values = &mut *(userdata as *mut Vec<String>);
    values.push(testutils::string_from_c(hint));
}

unsafe extern "C" fn count_log_messages(
    userdata: *mut c_void,
    _category: libc::c_int,
    _priority: SDL_LogPriority,
    _message: *const libc::c_char,
) {
    let count = &mut *(userdata as *mut usize);
    *count += 1;
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

unsafe extern "C" fn fire_once(_interval: u32, userdata: *mut c_void) -> u32 {
    let flag = &*(userdata as *const AtomicU32);
    flag.store(1, Ordering::SeqCst);
    0
}

fn invalid_pixel_format() -> u32 {
    (1u32 << 28)
        | (SDL_PixelType_SDL_PIXELTYPE_PACKED32 << 24)
        | (SDL_PackedOrder_SDL_PACKEDORDER_ABGR << 20)
        | ((SDL_PackedLayout_SDL_PACKEDLAYOUT_1010102 + 1) << 16)
        | (32 << 8)
        | 4
}

#[test]
#[cfg_attr(
    not(feature = "host-video-tests"),
    ignore = "run with --features host-video-tests"
)]
fn main_subsystem_refcount_and_dependency_cascade() {
    let _serial = testutils::serial_lock();
    let _video = testutils::ScopedEnvVar::set("SDL_VIDEODRIVER", "dummy");

    unsafe {
        SDL_Quit();

        assert_eq!(SDL_WasInit(SDL_INIT_TIMER), 0);
        assert_eq!(
            SDL_InitSubSystem(SDL_INIT_TIMER),
            0,
            "{}",
            testutils::current_error()
        );
        assert_ne!(SDL_WasInit(SDL_INIT_TIMER) & SDL_INIT_TIMER, 0);
        assert_eq!(
            SDL_InitSubSystem(SDL_INIT_TIMER),
            0,
            "{}",
            testutils::current_error()
        );
        SDL_QuitSubSystem(SDL_INIT_TIMER);
        assert_ne!(SDL_WasInit(SDL_INIT_TIMER) & SDL_INIT_TIMER, 0);
        SDL_QuitSubSystem(SDL_INIT_TIMER);
        assert_eq!(SDL_WasInit(SDL_INIT_TIMER) & SDL_INIT_TIMER, 0);

        assert_eq!(
            SDL_InitSubSystem(SDL_INIT_VIDEO),
            0,
            "{}",
            testutils::current_error()
        );
        let active = SDL_WasInit(SDL_INIT_VIDEO | SDL_INIT_EVENTS);
        assert_eq!(
            active & (SDL_INIT_VIDEO | SDL_INIT_EVENTS),
            SDL_INIT_VIDEO | SDL_INIT_EVENTS
        );
        SDL_QuitSubSystem(SDL_INIT_VIDEO);
        assert_eq!(SDL_WasInit(SDL_INIT_VIDEO | SDL_INIT_EVENTS), 0);

        SDL_Quit();
    }
}

#[test]
fn multi_flag_init_rolls_back_earlier_subsystems_on_failure() {
    let _serial = testutils::serial_lock();
    let _video = testutils::ScopedEnvVar::set("SDL_VIDEODRIVER", "safe-invalid-driver");

    unsafe {
        SDL_Quit();

        assert_eq!(
            SDL_InitSubSystem(SDL_INIT_EVENTS | SDL_INIT_TIMER | SDL_INIT_VIDEO),
            -1
        );
        assert_eq!(
            SDL_WasInit(SDL_INIT_EVENTS | SDL_INIT_TIMER | SDL_INIT_VIDEO),
            0,
            "failed multi-flag init should fully unwind prior refcount increments"
        );

        assert_eq!(
            SDL_InitSubSystem(SDL_INIT_TIMER),
            0,
            "{}",
            testutils::current_error()
        );
        SDL_QuitSubSystem(SDL_INIT_TIMER);
        SDL_Quit();
    }
}

#[test]
fn audio_init_implicitly_initializes_events_and_quit_cascades() {
    let _serial = testutils::serial_lock();
    let _audio = testutils::ScopedEnvVar::set("SDL_AUDIODRIVER", "dummy");

    unsafe {
        SDL_AudioQuit();
        SDL_QuitSubSystem(SDL_INIT_AUDIO | SDL_INIT_EVENTS);

        assert_eq!(SDL_WasInit(SDL_INIT_AUDIO | SDL_INIT_EVENTS), 0);
        assert_eq!(
            SDL_InitSubSystem(SDL_INIT_AUDIO),
            0,
            "{}",
            testutils::current_error()
        );

        let active = SDL_WasInit(SDL_INIT_AUDIO | SDL_INIT_EVENTS);
        assert_eq!(active & SDL_INIT_AUDIO, SDL_INIT_AUDIO);
        assert_eq!(active & SDL_INIT_EVENTS, SDL_INIT_EVENTS);

        SDL_QuitSubSystem(SDL_INIT_AUDIO);
        assert_eq!(SDL_WasInit(SDL_INIT_AUDIO | SDL_INIT_EVENTS), 0);
    }
}

#[test]
fn main_set_error_accepts_large_strings() {
    let _serial = testutils::serial_lock();
    let message = (0..1023)
        .map(|index| (b'a' + (index % 26) as u8) as char)
        .collect::<String>();
    let c_message = testutils::cstring(&message);

    unsafe {
        assert_eq!(SDL_SetError(c_message.as_ptr()), -1);
        assert_eq!(testutils::string_from_c(SDL_GetError()), message);
    }
}

#[test]
fn clear_error_clears_forwarded_host_error_state() {
    let _serial = testutils::serial_lock();

    unsafe {
        assert!(
            SDL_AllocFormat(invalid_pixel_format()).is_null(),
            "invalid format should fail"
        );
        assert!(
            !testutils::current_error().is_empty(),
            "invalid host call should set SDL error"
        );

        SDL_ClearError();
        assert_eq!(testutils::current_error(), "");
    }
}

#[test]
fn get_pixel_format_name_invalid_format_leaves_error_empty() {
    let _serial = testutils::serial_lock();

    unsafe {
        assert!(
            SDL_AllocFormat(invalid_pixel_format()).is_null(),
            "invalid format should fail"
        );
        SDL_ClearError();

        let name = testutils::string_from_c(SDL_GetPixelFormatName(invalid_pixel_format()));
        assert!(!name.is_empty());
        assert!(
            name.contains("UNKNOWN"),
            "unexpected pixel format name: {name}"
        );
        assert_eq!(testutils::current_error(), "");
    }
}

#[test]
fn hints_roundtrip_reset_and_callback() {
    let _serial = testutils::serial_lock();
    let hint_name = testutils::cstring("SDL_AUTOMATED_TEST_HINT_RS_CORE");
    let temp = testutils::cstring("temp");
    let override_value = testutils::cstring("override");
    let mut callback_values = Vec::new();

    unsafe {
        SDL_ResetHint(hint_name.as_ptr());
        assert_eq!(
            testutils::string_from_c(SDL_GetHint(hint_name.as_ptr())),
            ""
        );

        assert_ne!(SDL_SetHint(hint_name.as_ptr(), temp.as_ptr()), 0);
        assert_eq!(
            testutils::string_from_c(SDL_GetHint(hint_name.as_ptr())),
            "temp"
        );
    }

    let _env = testutils::ScopedEnvVar::set("SDL_AUTOMATED_TEST_HINT_RS_CORE", "original");

    unsafe {
        SDL_SetHintWithPriority(
            hint_name.as_ptr(),
            ptr::null(),
            SDL_HintPriority_SDL_HINT_DEFAULT,
        );
        assert_eq!(
            testutils::string_from_c(SDL_GetHint(hint_name.as_ptr())),
            "original"
        );

        SDL_AddHintCallback(
            hint_name.as_ptr(),
            Some(hint_callback),
            (&mut callback_values as *mut Vec<String>).cast(),
        );
        assert_ne!(
            SDL_SetHintWithPriority(
                hint_name.as_ptr(),
                override_value.as_ptr(),
                SDL_HintPriority_SDL_HINT_OVERRIDE,
            ),
            0
        );
        SDL_ResetHint(hint_name.as_ptr());
        SDL_DelHintCallback(
            hint_name.as_ptr(),
            Some(hint_callback),
            (&mut callback_values as *mut Vec<String>).cast(),
        );
    }

    assert!(callback_values.iter().any(|value| value == "override"));
    assert!(callback_values.iter().any(|value| value == "original"));
}

#[test]
fn log_hint_controls_delivery_threshold() {
    let _serial = testutils::serial_lock();
    let hint_name = testutils::c_ptr(safe_sdl::abi::generated_types::SDL_HINT_LOGGING);
    let info = SDL_LogPriority_SDL_LOG_PRIORITY_INFO;
    let debug = SDL_LogPriority_SDL_LOG_PRIORITY_DEBUG;
    let warn = SDL_LogPriority_SDL_LOG_PRIORITY_WARN;
    let app = SDL_LogCategory_SDL_LOG_CATEGORY_APPLICATION as libc::c_int;
    let mut original_callback = None;
    let mut original_userdata = ptr::null_mut();
    let mut count = 0usize;
    let debug_value = testutils::cstring("debug");
    let mixed_value = testutils::cstring("app=warn,*=info");
    let message = testutils::cstring("test");

    unsafe {
        SDL_LogGetOutputFunction(&mut original_callback, &mut original_userdata);
        SDL_LogSetOutputFunction(Some(count_log_messages), (&mut count as *mut usize).cast());

        SDL_SetHint(hint_name, ptr::null());
        SDL_LogResetPriorities();
        SDL_LogMessage(app, info, message.as_ptr());
        assert_eq!(count, 1);

        count = 0;
        SDL_LogMessage(app, debug, message.as_ptr());
        assert_eq!(count, 0);

        SDL_SetHint(hint_name, debug_value.as_ptr());
        count = 0;
        SDL_LogMessage(app, debug, message.as_ptr());
        assert_eq!(count, 1);

        SDL_SetHint(hint_name, mixed_value.as_ptr());
        count = 0;
        SDL_LogMessage(app, info, message.as_ptr());
        assert_eq!(count, 0);
        SDL_LogMessage(app, warn, message.as_ptr());
        assert_eq!(count, 1);

        SDL_LogSetOutputFunction(original_callback, original_userdata);
        SDL_SetHint(hint_name, ptr::null());
        SDL_LogResetPriorities();
    }
}

#[test]
fn platform_getters_are_sane() {
    let _serial = testutils::serial_lock();

    unsafe {
        let platform = testutils::string_from_c(safe_sdl::core::platform::SDL_GetPlatform());
        assert!(!platform.is_empty());
        assert!(SDL_GetCPUCount() > 0);
        assert!(SDL_GetCPUCacheLineSize() >= 0);
        assert!(matches!(SDL_HasSSE(), 0 | 1));
        assert!(matches!(SDL_HasAVX(), 0 | 1));
        assert!(SDL_GetSystemRAM() > 0);
        let alignment = SDL_SIMDGetAlignment();
        assert!(alignment >= std::mem::size_of::<*const c_void>());
        assert!(alignment.is_power_of_two());
        let revision = testutils::string_from_c(SDL_GetRevision());
        assert!(!revision.is_empty());

        SDL_ResetAssertionReport();
        assert!(SDL_GetAssertionReport().is_null());
    }
}

#[test]
fn get_version_caches_legacy_hint_on_first_call() {
    let _serial = testutils::serial_lock();
    let hint_name = testutils::cstring("SDL_LEGACY_VERSION");
    let legacy_value = testutils::cstring("1");

    unsafe {
        SDL_ResetHint(hint_name.as_ptr());

        let mut first = MaybeUninit::<SDL_version>::zeroed();
        SDL_GetVersion(first.as_mut_ptr());
        let first = first.assume_init();

        assert_eq!((first.major, first.minor, first.patch), (2, 30, 0));

        SDL_SetHint(hint_name.as_ptr(), legacy_value.as_ptr());

        let mut second = MaybeUninit::<SDL_version>::zeroed();
        SDL_GetVersion(second.as_mut_ptr());
        let second = second.assume_init();

        assert_eq!(
            (second.major, second.minor, second.patch),
            (first.major, first.minor, first.patch),
            "SDL_GetVersion should cache SDL_LEGACY_VERSION on first use"
        );

        SDL_ResetHint(hint_name.as_ptr());
    }
}

#[test]
fn math_wrappers_match_expected_values() {
    let _serial = testutils::serial_lock();
    let epsilon = 1.0e-10;

    unsafe {
        assert!((SDL_floor(1.75) - 1.0).abs() < epsilon);
        assert!((SDL_ceil(-1.25) - -1.0).abs() < epsilon);
        assert!((SDL_fabs(-3.5) - 3.5).abs() < epsilon);
        assert!((SDL_sqrt(81.0) - 9.0).abs() < epsilon);
        assert!((SDL_pow(2.0, 8.0) - 256.0).abs() < epsilon);
        assert!((SDL_log(std::f64::consts::E) - 1.0).abs() < epsilon);
        assert!((SDL_atan2(1.0, 1.0) - std::f64::consts::FRAC_PI_4).abs() < epsilon);
    }
}

#[test]
fn stdlib_helpers_cover_copy_format_search_and_iconv() {
    let _serial = testutils::serial_lock();
    let mut text = [0 as libc::c_char; 32];
    let foo = testutils::cstring("foo");
    let mut numbers = [9i32, 3, 5, 1, 7];
    let key = 5i32;
    let utf8 = b"Hello \xCE\xBA\xE1\xBD\xB9\xCF\x83\xCE\xBC\xCE\xB5".to_vec();
    let utf8_c = std::ffi::CString::new(utf8.clone()).expect("utf8 sample CString");
    let utf8_name = testutils::cstring("UTF-8");
    let env_name = testutils::cstring("SDL_AUTOMATED_TEST_ENV_RS");
    let env_value = testutils::cstring("expected");

    unsafe {
        assert_eq!(SDL_strlcpy(text.as_mut_ptr(), foo.as_ptr(), text.len()), 3);
        assert_eq!(testutils::string_from_c(text.as_ptr()), "foo");
        assert_eq!(
            SDL_snprintf(
                text.as_mut_ptr(),
                text.len(),
                testutils::cstring("%s %d").as_ptr(),
                foo.as_ptr(),
                42,
            ),
            6
        );
        assert_eq!(testutils::string_from_c(text.as_ptr()), "foo 42");

        SDL_qsort(
            numbers.as_mut_ptr().cast(),
            numbers.len(),
            std::mem::size_of::<i32>(),
            Some(compare_ints),
        );
        assert_eq!(numbers, [1, 3, 5, 7, 9]);
        let found = SDL_bsearch(
            (&key as *const i32).cast(),
            numbers.as_ptr().cast(),
            numbers.len(),
            std::mem::size_of::<i32>(),
            Some(compare_ints),
        ) as *const i32;
        assert!(!found.is_null());
        assert_eq!(*found, 5);

        assert_eq!(SDL_setenv(env_name.as_ptr(), env_value.as_ptr(), 1), 0);
        assert_eq!(
            testutils::string_from_c(SDL_getenv(env_name.as_ptr())),
            "expected"
        );

        let duplicated = SDL_strdup(foo.as_ptr());
        assert_eq!(testutils::string_from_c(duplicated), "foo");
        safe_sdl::core::memory::SDL_free(duplicated.cast());

        let converted = SDL_iconv_string(
            utf8_name.as_ptr(),
            utf8_name.as_ptr(),
            utf8_c.as_ptr(),
            utf8.len(),
        );
        assert!(!converted.is_null());
        assert_eq!(
            std::ffi::CStr::from_ptr(converted).to_bytes(),
            utf8.as_slice()
        );
        safe_sdl::core::memory::SDL_free(converted.cast());
        assert_eq!(
            SDL_strcmp(foo.as_ptr(), testutils::cstring("foo").as_ptr()),
            0
        );
    }
}

#[test]
fn timer_api_delays_and_callbacks() {
    let _serial = testutils::serial_lock();
    let _timer = testutils::SubsystemGuard::init(SDL_INIT_TIMER);
    let fired = Box::new(AtomicU32::new(0));
    let fired_ptr = (&*fired as *const AtomicU32).cast_mut().cast();

    unsafe {
        assert!(SDL_GetPerformanceCounter() > 0);
        assert!(SDL_GetPerformanceFrequency() > 0);

        let before = SDL_GetTicks();
        SDL_Delay(25);
        let after = SDL_GetTicks();
        assert!(after.wrapping_sub(before) >= 15);

        let long_timer = SDL_AddTimer(10_000, Some(fire_once), fired_ptr);
        assert_ne!(long_timer, 0);
        assert_ne!(SDL_RemoveTimer(long_timer), 0);
        assert_eq!(fired.load(Ordering::SeqCst), 0);

        let short_timer = SDL_AddTimer(10, Some(fire_once), fired_ptr);
        assert_ne!(short_timer, 0);
        SDL_Delay(100);
        assert_eq!(fired.load(Ordering::SeqCst), 1);
        assert_eq!(SDL_RemoveTimer(short_timer), 0);
    }
}
