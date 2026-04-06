#![cfg(feature = "host-video-tests")]
#![allow(clippy::all)]

#[path = "common/testutils.rs"]
mod testutils;

use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::ptr;

use safe_sdl::abi::generated_types::{
    SDL_DisplayMode, SDL_Event, SDL_EventType_SDL_WINDOWEVENT, SDL_Keycode, SDL_Rect,
    SDL_SystemCursor_SDL_SYSTEM_CURSOR_ARROW, SDL_UserEvent,
    SDL_WindowEventID_SDL_WINDOWEVENT_FOCUS_GAINED, SDL_WindowFlags_SDL_WINDOW_FULLSCREEN_DESKTOP,
    SDL_WindowFlags_SDL_WINDOW_HIDDEN, SDL_bool_SDL_TRUE, SDL_eventaction_SDL_GETEVENT,
    SDL_DISABLE, SDL_ENABLE, SDL_HINT_GRAB_KEYBOARD, SDL_INIT_EVENTS, SDL_INIT_VIDEO, SDL_QUERY,
    SDL_WINDOWPOS_CENTERED_MASK,
};
use safe_sdl::core::error::SDL_ClearError;
use safe_sdl::core::hints::SDL_SetHint;
use safe_sdl::events::gesture::{
    SDL_LoadDollarTemplates, SDL_RecordGesture, SDL_SaveAllDollarTemplates,
};
use safe_sdl::events::keyboard::{
    SDL_GetKeyFromName, SDL_GetKeyFromScancode, SDL_GetKeyName, SDL_GetKeyboardState,
    SDL_GetScancodeFromKey, SDL_GetScancodeFromName, SDL_GetScancodeName,
};
use safe_sdl::events::mouse::{
    SDL_CreateSystemCursor, SDL_FreeCursor, SDL_GetCursor, SDL_GetRelativeMouseMode, SDL_SetCursor,
    SDL_SetRelativeMouseMode, SDL_ShowCursor,
};
use safe_sdl::events::queue::{
    SDL_AddEventWatch, SDL_DelEventWatch, SDL_EventState, SDL_PeepEvents, SDL_PollEvent,
    SDL_PushEvent, SDL_RegisterEvents, SDL_SetEventFilter,
};
use safe_sdl::video::clipboard::{
    SDL_GetClipboardText, SDL_GetPrimarySelectionText, SDL_HasClipboardText,
    SDL_HasPrimarySelectionText, SDL_SetClipboardText, SDL_SetPrimarySelectionText,
};
use safe_sdl::video::display::{
    SDL_GetClosestDisplayMode, SDL_GetCurrentVideoDriver, SDL_GetDisplayBounds, SDL_GetDisplayMode,
    SDL_GetDisplayName, SDL_GetNumDisplayModes, SDL_GetNumVideoDisplays, SDL_GetNumVideoDrivers,
    SDL_GetVideoDriver,
};
use safe_sdl::video::linux::ime::{
    SDL_IsTextInputActive, SDL_SetTextInputRect, SDL_StartTextInput, SDL_StopTextInput,
};
use safe_sdl::video::syswm::{
    SDL_GetWindowWMInfo, SDL_SysWMinfo, SDL_SYSWM_UNKNOWN, SDL_SYSWM_X11,
};
use safe_sdl::video::window::{
    SDL_CreateWindow, SDL_DestroyWindow, SDL_GetWindowBrightness, SDL_GetWindowData,
    SDL_GetWindowDisplayIndex, SDL_GetWindowFlags, SDL_GetWindowFromID, SDL_GetWindowGammaRamp,
    SDL_GetWindowGrab, SDL_GetWindowID, SDL_GetWindowKeyboardGrab, SDL_GetWindowMaximumSize,
    SDL_GetWindowMinimumSize, SDL_GetWindowMouseGrab, SDL_GetWindowPosition, SDL_GetWindowSize,
    SDL_RaiseWindow, SDL_SetWindowData, SDL_SetWindowFullscreen, SDL_SetWindowGrab,
    SDL_SetWindowKeyboardGrab, SDL_SetWindowMaximumSize, SDL_SetWindowMinimumSize,
    SDL_SetWindowMouseGrab, SDL_SetWindowPosition, SDL_SetWindowSize,
};

unsafe extern "C" fn count_events(userdata: *mut c_void, _event: *mut SDL_Event) -> libc::c_int {
    let counter = &mut *(userdata as *mut usize);
    *counter += 1;
    1
}

unsafe extern "C" fn allow_events(userdata: *mut c_void, _event: *mut SDL_Event) -> libc::c_int {
    let counter = &mut *(userdata as *mut usize);
    *counter += 1;
    1
}

unsafe fn create_hidden_window() -> *mut safe_sdl::abi::generated_types::SDL_Window {
    let title = testutils::cstring("video-events");
    SDL_CreateWindow(
        title.as_ptr(),
        32,
        48,
        320,
        240,
        SDL_WindowFlags_SDL_WINDOW_HIDDEN,
    )
}

#[test]
fn clipboard_roundtrip_matches_upstream_automation_expectations() {
    let _serial = testutils::serial_lock();
    let _video = testutils::ScopedEnvVar::set("SDL_VIDEODRIVER", "dummy");
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_VIDEO);

    unsafe {
        let clipboard = testutils::cstring("phase4 clipboard text");
        let primary = testutils::cstring("phase4 primary selection");
        assert_eq!(
            SDL_SetClipboardText(clipboard.as_ptr()),
            0,
            "{}",
            testutils::current_error()
        );
        assert_eq!(
            testutils::string_from_c(SDL_GetClipboardText()),
            "phase4 clipboard text"
        );
        assert_ne!(SDL_HasClipboardText(), 0);

        assert_eq!(
            SDL_SetPrimarySelectionText(primary.as_ptr()),
            0,
            "{}",
            testutils::current_error()
        );
        assert_eq!(
            testutils::string_from_c(SDL_GetPrimarySelectionText()),
            "phase4 primary selection"
        );
        assert_ne!(SDL_HasPrimarySelectionText(), 0);

        assert_eq!(SDL_SetClipboardText(ptr::null()), 0);
        assert_eq!(testutils::string_from_c(SDL_GetClipboardText()), "");
        assert_eq!(SDL_HasClipboardText(), 0);

        assert_eq!(SDL_SetPrimarySelectionText(ptr::null()), 0);
        assert_eq!(testutils::string_from_c(SDL_GetPrimarySelectionText()), "");
        assert_eq!(SDL_HasPrimarySelectionText(), 0);
    }
}

#[test]
fn event_queue_watch_filter_and_custom_events_roundtrip() {
    let _serial = testutils::serial_lock();
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_EVENTS);

    unsafe {
        let mut watched = 0usize;
        let mut filtered = 0usize;
        SDL_AddEventWatch(Some(count_events), (&mut watched as *mut usize).cast());
        SDL_SetEventFilter(Some(allow_events), (&mut filtered as *mut usize).cast());

        let event_type = SDL_RegisterEvents(1);
        assert_ne!(event_type, u32::MAX, "{}", testutils::current_error());

        let mut event = SDL_Event {
            user: SDL_UserEvent {
                type_: event_type,
                timestamp: 0,
                windowID: 0,
                code: 41,
                data1: 0x1234usize as *mut c_void,
                data2: 0x5678usize as *mut c_void,
            },
        };
        assert_eq!(
            SDL_PushEvent(&mut event),
            1,
            "{}",
            testutils::current_error()
        );

        let mut out = MaybeUninit::<SDL_Event>::zeroed();
        assert_eq!(
            SDL_PeepEvents(
                out.as_mut_ptr(),
                1,
                SDL_eventaction_SDL_GETEVENT,
                event_type,
                event_type
            ),
            1,
            "{}",
            testutils::current_error()
        );
        let out = out.assume_init();
        assert_eq!(out.user.type_, event_type);
        assert_eq!(out.user.code, 41);
        assert_eq!(watched, 1);
        assert_eq!(filtered, 1);

        assert_eq!(SDL_EventState(event_type, SDL_QUERY), SDL_ENABLE as u8);
        let _previous = SDL_EventState(event_type, SDL_DISABLE as i32);
        let mut none = MaybeUninit::<SDL_Event>::zeroed();
        assert_eq!(SDL_PollEvent(none.as_mut_ptr()), 0);

        SDL_DelEventWatch(Some(count_events), (&mut watched as *mut usize).cast());
        SDL_SetEventFilter(None, ptr::null_mut());
    }
}

#[test]
fn keyboard_mouse_and_text_input_state_match_upstream_queries() {
    let _serial = testutils::serial_lock();
    let _video = testutils::ScopedEnvVar::set("SDL_VIDEODRIVER", "dummy");
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_VIDEO);

    unsafe {
        let window = create_hidden_window();
        assert!(!window.is_null(), "{}", testutils::current_error());

        let mut numkeys = 0;
        let keyboard = SDL_GetKeyboardState(&mut numkeys);
        assert!(!keyboard.is_null());
        assert!(numkeys > 0);

        let name_a = testutils::cstring("A");
        let scancode = SDL_GetScancodeFromName(name_a.as_ptr());
        let key = SDL_GetKeyFromScancode(scancode);
        assert_ne!(key as SDL_Keycode, 0);
        assert_eq!(SDL_GetScancodeFromKey(key), scancode);
        assert_eq!(SDL_GetKeyFromName(name_a.as_ptr()), key);
        assert!(!testutils::string_from_c(SDL_GetScancodeName(scancode)).is_empty());
        assert!(!testutils::string_from_c(SDL_GetKeyName(key)).is_empty());

        let rect = SDL_Rect {
            x: 1,
            y: 2,
            w: 3,
            h: 4,
        };
        SDL_StartTextInput();
        assert_ne!(SDL_IsTextInputActive(), 0);
        SDL_SetTextInputRect(&rect);
        SDL_StopTextInput();
        assert_eq!(SDL_IsTextInputActive(), 0);

        let cursor = SDL_CreateSystemCursor(SDL_SystemCursor_SDL_SYSTEM_CURSOR_ARROW);
        if !cursor.is_null() {
            SDL_SetCursor(cursor);
            assert_eq!(SDL_GetCursor(), cursor);
        } else {
            assert!(!testutils::current_error().is_empty());
        }
        assert_eq!(SDL_ShowCursor(SDL_QUERY), 1);
        assert_eq!(SDL_SetRelativeMouseMode(SDL_bool_SDL_TRUE), 0);
        assert_ne!(SDL_GetRelativeMouseMode(), 0);
        assert_eq!(SDL_SetRelativeMouseMode(0), 0);
        if !cursor.is_null() {
            SDL_FreeCursor(cursor);
        }
        SDL_DestroyWindow(window);
    }
}

#[test]
fn video_driver_display_and_window_queries_cover_upstream_cases() {
    let _serial = testutils::serial_lock();
    let _video = testutils::ScopedEnvVar::set("SDL_VIDEODRIVER", "dummy");
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_VIDEO);

    unsafe {
        assert!(SDL_GetNumVideoDrivers() >= 1);
        assert_eq!(testutils::string_from_c(SDL_GetVideoDriver(0)), "x11");
        assert_eq!(
            testutils::string_from_c(SDL_GetCurrentVideoDriver()),
            "dummy"
        );
        assert!(SDL_GetNumVideoDisplays() >= 1);
        assert!(!testutils::string_from_c(SDL_GetDisplayName(0)).is_empty());

        let mut bounds = MaybeUninit::<SDL_Rect>::zeroed();
        assert_eq!(
            SDL_GetDisplayBounds(0, bounds.as_mut_ptr()),
            0,
            "{}",
            testutils::current_error()
        );
        let bounds = bounds.assume_init();
        assert!(bounds.w > 0);
        assert!(bounds.h > 0);

        let num_modes = SDL_GetNumDisplayModes(0);
        assert!(num_modes >= 1);
        let mut mode = MaybeUninit::<SDL_DisplayMode>::zeroed();
        assert_eq!(
            SDL_GetDisplayMode(0, 0, mode.as_mut_ptr()),
            0,
            "{}",
            testutils::current_error()
        );
        let mode = mode.assume_init();
        let mut closest = MaybeUninit::<SDL_DisplayMode>::zeroed();
        assert!(!SDL_GetClosestDisplayMode(0, &mode, closest.as_mut_ptr()).is_null());

        let window = create_hidden_window();
        assert!(!window.is_null(), "{}", testutils::current_error());
        let id = SDL_GetWindowID(window);
        assert_eq!(SDL_GetWindowFromID(id), window);
        assert_ne!(
            SDL_GetWindowFlags(window) & SDL_WindowFlags_SDL_WINDOW_HIDDEN,
            0
        );

        SDL_SetWindowPosition(window, 10, 20);
        let (mut x, mut y) = (0, 0);
        SDL_GetWindowPosition(window, &mut x, &mut y);
        assert_eq!((x, y), (10, 20));

        SDL_SetWindowSize(window, 400, 220);
        let (mut w, mut h) = (0, 0);
        SDL_GetWindowSize(window, &mut w, &mut h);
        assert_eq!((w, h), (400, 220));
        assert!(SDL_GetWindowDisplayIndex(window) >= 0);

        let key = testutils::cstring("phase4.window.data");
        let stored = 0xfeedusize as *mut c_void;
        assert!(SDL_SetWindowData(window, key.as_ptr(), stored).is_null());
        assert_eq!(SDL_GetWindowData(window, key.as_ptr()), stored);

        SDL_DestroyWindow(window);
    }
}

#[test]
fn syswm_and_gesture_plumbing_are_exercisable_without_manual_loops() {
    let _serial = testutils::serial_lock();
    let _video = testutils::ScopedEnvVar::set("SDL_VIDEODRIVER", "dummy");
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_VIDEO | SDL_INIT_EVENTS);

    unsafe {
        let window = create_hidden_window();
        assert!(!window.is_null(), "{}", testutils::current_error());

        let mut info = MaybeUninit::<SDL_SysWMinfo>::zeroed();
        (*info.as_mut_ptr()).version.major = 2;
        (*info.as_mut_ptr()).version.minor = 0;
        (*info.as_mut_ptr()).version.patch = 0;
        let got_wm_info = SDL_GetWindowWMInfo(window, info.as_mut_ptr());
        let info = info.assume_init();
        if got_wm_info != 0 {
            assert!(matches!(info.subsystem, SDL_SYSWM_X11 | SDL_SYSWM_UNKNOWN));
        }

        let mut stream_bytes = vec![0u8; 4096];
        let rw = safe_sdl::core::rwops::SDL_RWFromMem(
            stream_bytes.as_mut_ptr().cast(),
            stream_bytes.len() as libc::c_int,
        );
        assert!(!rw.is_null());
        let _ = SDL_RecordGesture(-1);
        let saved = SDL_SaveAllDollarTemplates(rw);
        assert!(saved >= 0, "{}", testutils::current_error());
        assert_eq!(safe_sdl::core::rwops::SDL_RWseek(rw, 0, libc::SEEK_SET), 0);
        let loaded = SDL_LoadDollarTemplates(-1, rw);
        assert!(loaded >= 0, "{}", testutils::current_error());
        assert_eq!(safe_sdl::core::rwops::SDL_RWclose(rw), 0);

        SDL_DestroyWindow(window);
    }
}

#[test]
fn dummy_window_stub_matches_upstream_grab_and_invalid_input_semantics() {
    let _serial = testutils::serial_lock();
    let _video = testutils::ScopedEnvVar::set("SDL_VIDEODRIVER", "dummy");
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_VIDEO | SDL_INIT_EVENTS);

    unsafe {
        let title = testutils::cstring("window-grab-regression");
        let window = SDL_CreateWindow(title.as_ptr(), 32, 48, 320, 240, 0);
        assert!(!window.is_null(), "{}", testutils::current_error());

        let mut event = MaybeUninit::<SDL_Event>::zeroed();
        while SDL_PollEvent(event.as_mut_ptr()) != 0 {}

        SDL_RaiseWindow(window);
        let mut saw_focus_gained = false;
        while SDL_PollEvent(event.as_mut_ptr()) != 0 {
            let event = event.assume_init();
            if event.type_ == SDL_EventType_SDL_WINDOWEVENT
                && event.window.event as u32 == SDL_WindowEventID_SDL_WINDOWEVENT_FOCUS_GAINED
            {
                saw_focus_gained = true;
            }
        }
        assert!(saw_focus_gained, "expected focus gained event after raise");

        SDL_SetWindowGrab(window, SDL_bool_SDL_TRUE);
        assert_ne!(SDL_GetWindowGrab(window), 0);
        assert_ne!(SDL_GetWindowMouseGrab(window), 0);
        assert_eq!(SDL_GetWindowKeyboardGrab(window), 0);

        let one = testutils::cstring("1");
        assert_ne!(
            SDL_SetHint(SDL_HINT_GRAB_KEYBOARD.as_ptr().cast(), one.as_ptr()),
            0
        );
        SDL_SetWindowGrab(window, SDL_bool_SDL_TRUE);
        assert_ne!(SDL_GetWindowGrab(window), 0);
        assert_ne!(SDL_GetWindowMouseGrab(window), 0);
        assert_ne!(SDL_GetWindowKeyboardGrab(window), 0);

        SDL_SetWindowGrab(window, 0);
        assert_eq!(SDL_GetWindowGrab(window), 0);
        assert_eq!(SDL_GetWindowMouseGrab(window), 0);
        assert_eq!(SDL_GetWindowKeyboardGrab(window), 0);

        SDL_SetWindowMouseGrab(window, SDL_bool_SDL_TRUE);
        SDL_SetWindowKeyboardGrab(window, SDL_bool_SDL_TRUE);
        SDL_SetWindowMouseGrab(window, 0);
        assert_eq!(SDL_GetWindowMouseGrab(window), 0);
        assert_ne!(SDL_GetWindowGrab(window), 0);
        SDL_SetWindowKeyboardGrab(window, 0);
        assert_eq!(SDL_GetWindowGrab(window), 0);

        assert_eq!(SDL_ShowCursor(SDL_QUERY), 1);
        assert_eq!(SDL_ShowCursor(SDL_DISABLE as i32), 1);
        assert_eq!(SDL_ShowCursor(SDL_QUERY), 0);
        assert_eq!(SDL_ShowCursor(SDL_ENABLE as i32), 0);
        assert_eq!(SDL_ShowCursor(SDL_QUERY), 1);

        let empty = testutils::cstring("");
        let payload = 0x1234usize as *mut c_void;
        SDL_ClearError();
        assert!(SDL_SetWindowData(window, empty.as_ptr(), payload).is_null());
        assert!(
            testutils::current_error().starts_with("Parameter"),
            "{}",
            testutils::current_error()
        );

        SDL_ClearError();
        assert!(SDL_GetWindowData(window, empty.as_ptr()).is_null());
        assert!(
            testutils::current_error().starts_with("Parameter"),
            "{}",
            testutils::current_error()
        );

        let mut red = [0u16; 256];
        let mut green = [0u16; 256];
        let mut blue = [0u16; 256];
        assert_eq!(
            SDL_GetWindowGammaRamp(
                window,
                red.as_mut_ptr(),
                green.as_mut_ptr(),
                blue.as_mut_ptr()
            ),
            0,
            "{}",
            testutils::current_error()
        );
        assert_eq!(red[0], 0);
        assert_eq!(red[255], 0xFFFF);

        SDL_ClearError();
        assert_eq!(SDL_GetWindowBrightness(ptr::null_mut()), 1.0);
        assert_eq!(testutils::current_error(), "Invalid window");

        let (mut w, mut h) = (0, 0);
        SDL_GetWindowSize(window, &mut w, &mut h);
        SDL_ClearError();
        SDL_SetWindowSize(window, 0, h);
        assert!(testutils::current_error().starts_with("Parameter"));
        let (mut new_w, mut new_h) = (0, 0);
        SDL_GetWindowSize(window, &mut new_w, &mut new_h);
        assert_eq!((new_w, new_h), (w, h));

        SDL_ClearError();
        SDL_SetWindowMinimumSize(window, 0, 1);
        assert!(testutils::current_error().starts_with("Parameter"));
        let (mut min_w, mut min_h) = (-1, -1);
        SDL_GetWindowMinimumSize(window, &mut min_w, &mut min_h);
        assert_eq!((min_w, min_h), (0, 0));

        SDL_ClearError();
        SDL_SetWindowMaximumSize(window, 0, 1);
        assert!(testutils::current_error().starts_with("Parameter"));
        let (mut max_w, mut max_h) = (-1, -1);
        SDL_GetWindowMaximumSize(window, &mut max_w, &mut max_h);
        assert_eq!((max_w, max_h), (0, 0));

        SDL_DestroyWindow(window);
    }
}

#[test]
fn dummy_window_stub_centers_and_restores_fullscreen_desktop_geometry() {
    let _serial = testutils::serial_lock();
    let _video = testutils::ScopedEnvVar::set("SDL_VIDEODRIVER", "dummy");
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_VIDEO | SDL_INIT_EVENTS);

    unsafe {
        let mut display = MaybeUninit::<SDL_Rect>::zeroed();
        assert_eq!(
            SDL_GetDisplayBounds(0, display.as_mut_ptr()),
            0,
            "{}",
            testutils::current_error()
        );
        let display = display.assume_init();

        let width = 320;
        let height = 180;
        let centered = SDL_WINDOWPOS_CENTERED_MASK as i32;
        let title = testutils::cstring("centered-window-regression");
        let window = SDL_CreateWindow(title.as_ptr(), centered, centered, width, height, 0);
        assert!(!window.is_null(), "{}", testutils::current_error());

        let (mut x, mut y) = (0, 0);
        let (mut w, mut h) = (0, 0);
        SDL_GetWindowPosition(window, &mut x, &mut y);
        SDL_GetWindowSize(window, &mut w, &mut h);
        let expected_x = display.x + ((display.w - width) / 2);
        let expected_y = display.y + ((display.h - height) / 2);
        assert_eq!((x, y, w, h), (expected_x, expected_y, width, height));

        assert_eq!(
            SDL_SetWindowFullscreen(window, SDL_WindowFlags_SDL_WINDOW_FULLSCREEN_DESKTOP),
            0,
            "{}",
            testutils::current_error()
        );
        SDL_GetWindowPosition(window, &mut x, &mut y);
        SDL_GetWindowSize(window, &mut w, &mut h);
        assert_eq!((x, y, w, h), (display.x, display.y, display.w, display.h));

        assert_eq!(
            SDL_SetWindowFullscreen(window, 0),
            0,
            "{}",
            testutils::current_error()
        );
        SDL_GetWindowPosition(window, &mut x, &mut y);
        SDL_GetWindowSize(window, &mut w, &mut h);
        assert_eq!((x, y, w, h), (expected_x, expected_y, width, height));

        SDL_DestroyWindow(window);
    }
}
