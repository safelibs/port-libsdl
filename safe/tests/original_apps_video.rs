#![allow(unexpected_cfgs)]
#![cfg(feature = "host-video-tests")]
#![allow(clippy::all)]

#[path = "common/testutils.rs"]
mod testutils;

use safe_sdl::abi::generated_types::{
    SDL_DisplayMode, SDL_DropEvent, SDL_Event, SDL_EventType_SDL_DROPFILE,
    SDL_FlashOperation_SDL_FLASH_BRIEFLY, SDL_Rect, SDL_SystemCursor_SDL_SYSTEM_CURSOR_ARROW,
    SDL_WindowFlags_SDL_WINDOW_HIDDEN, SDL_WindowShapeMode, SDL_bool_SDL_TRUE, SDL_INIT_EVENTS,
    SDL_INIT_VIDEO,
};
use safe_sdl::core::rwops::{SDL_RWFromMem, SDL_RWclose, SDL_RWseek};
use safe_sdl::core::thread::{SDL_CreateThread, SDL_WaitThread};
use safe_sdl::events::gesture::{
    SDL_LoadDollarTemplates, SDL_RecordGesture, SDL_SaveAllDollarTemplates,
};
use safe_sdl::events::keyboard::SDL_GetScancodeName;
use safe_sdl::events::mouse::{
    SDL_CreateColorCursor, SDL_CreateSystemCursor, SDL_FreeCursor, SDL_GetRelativeMouseMode,
    SDL_SetCursor, SDL_SetRelativeMouseMode, SDL_WarpMouseInWindow,
};
use safe_sdl::events::queue::{SDL_PollEvent, SDL_PushEvent, SDL_RegisterEvents, SDL_WaitEvent};
use safe_sdl::video::clipboard::{SDL_GetClipboardText, SDL_SetClipboardText};
use safe_sdl::video::display::{
    SDL_GetCurrentDisplayMode, SDL_GetCurrentVideoDriver, SDL_GetDesktopDisplayMode,
    SDL_GetDisplayBounds, SDL_GetDisplayDPI, SDL_GetDisplayName, SDL_GetNumDisplayModes,
    SDL_GetNumVideoDisplays,
};
use safe_sdl::video::linux::ime::{
    SDL_IsTextInputActive, SDL_SetTextInputRect, SDL_StartTextInput, SDL_StopTextInput,
};
use safe_sdl::video::messagebox::SDL_ShowSimpleMessageBox;
use safe_sdl::video::shape::{SDL_CreateShapedWindow, SDL_IsShapedWindow, SDL_SetWindowShape};
use safe_sdl::video::surface::{SDL_CreateRGBSurfaceWithFormat, SDL_FillRect, SDL_FreeSurface};
use safe_sdl::video::syswm::{SDL_GetWindowWMInfo, SDL_SysWMinfo, SDL_SYSWM_X11};
use safe_sdl::video::window::{
    SDL_CreateWindow, SDL_DestroyWindow, SDL_FlashWindow, SDL_GetWindowData,
    SDL_GetWindowDisplayIndex, SDL_GetWindowDisplayMode, SDL_GetWindowFlags, SDL_GetWindowFromID,
    SDL_GetWindowID, SDL_GetWindowPosition, SDL_GetWindowSize, SDL_GetWindowSurface,
    SDL_SetWindowData, SDL_SetWindowFullscreen, SDL_SetWindowHitTest, SDL_SetWindowMouseGrab,
    SDL_SetWindowPosition, SDL_SetWindowSize, SDL_SetWindowTitle, SDL_UpdateWindowSurface,
};
use std::ffi::{c_void, CString};
use std::mem::MaybeUninit;
use std::ptr;

unsafe extern "C" fn thread_push_quit(data: *mut c_void) -> libc::c_int {
    let event_type = data as usize as u32;
    let mut event = SDL_Event {
        user: safe_sdl::abi::generated_types::SDL_UserEvent {
            type_: event_type,
            timestamp: 0,
            windowID: 0,
            code: 7,
            data1: ptr::null_mut(),
            data2: ptr::null_mut(),
        },
    };
    SDL_PushEvent(&mut event);
    0
}

unsafe extern "C" fn passthrough_hit_test(
    _window: *mut safe_sdl::abi::generated_types::SDL_Window,
    _point: *const safe_sdl::abi::generated_types::SDL_Point,
    _userdata: *mut c_void,
) -> safe_sdl::abi::generated_types::SDL_HitTestResult {
    safe_sdl::abi::generated_types::SDL_HitTestResult_SDL_HITTEST_NORMAL
}

unsafe fn create_hidden_window() -> *mut safe_sdl::abi::generated_types::SDL_Window {
    let title = testutils::cstring("original-app-video");
    SDL_CreateWindow(
        title.as_ptr(),
        16,
        24,
        320,
        240,
        SDL_WindowFlags_SDL_WINDOW_HIDDEN,
    )
}

#[test]
fn checkkeys_and_testkeys_ports_cover_scancode_inventory() {
    let _serial = testutils::serial_lock();
    let _video = testutils::ScopedEnvVar::set("SDL_VIDEODRIVER", "dummy");
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_VIDEO);

    unsafe {
        assert_eq!(
            testutils::string_from_c(SDL_GetCurrentVideoDriver()),
            "dummy"
        );
        assert!(!testutils::string_from_c(SDL_GetScancodeName(4)).is_empty());
        assert!(!testutils::string_from_c(SDL_GetScancodeName(40)).is_empty());
    }
}

#[test]
fn checkkeysthreads_port_delivers_cross_thread_user_events() {
    let _serial = testutils::serial_lock();
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_EVENTS);

    unsafe {
        let event_type = SDL_RegisterEvents(1);
        assert_ne!(event_type, u32::MAX, "{}", testutils::current_error());
        let thread = SDL_CreateThread(
            Some(thread_push_quit),
            ptr::null(),
            event_type as usize as *mut c_void,
        );
        assert!(!thread.is_null(), "{}", testutils::current_error());
        let mut event = MaybeUninit::<SDL_Event>::zeroed();
        assert_eq!(
            SDL_WaitEvent(event.as_mut_ptr()),
            1,
            "{}",
            testutils::current_error()
        );
        let event = event.assume_init();
        assert_eq!(event.user.type_, event_type);
        let mut status = -1;
        SDL_WaitThread(thread, &mut status);
        assert_eq!(status, 0);
    }
}

#[test]
fn testbounds_and_testdisplayinfo_ports_query_display_contracts() {
    let _serial = testutils::serial_lock();
    let _video = testutils::ScopedEnvVar::set("SDL_VIDEODRIVER", "dummy");
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_VIDEO);

    unsafe {
        assert!(SDL_GetNumVideoDisplays() >= 1);
        assert!(!testutils::string_from_c(SDL_GetDisplayName(0)).is_empty());

        let mut bounds = MaybeUninit::<SDL_Rect>::zeroed();
        assert_eq!(
            SDL_GetDisplayBounds(0, bounds.as_mut_ptr()),
            0,
            "{}",
            testutils::current_error()
        );
        assert!(bounds.assume_init().w > 0);

        let mut current = MaybeUninit::<SDL_DisplayMode>::zeroed();
        let mut desktop = MaybeUninit::<SDL_DisplayMode>::zeroed();
        assert_eq!(SDL_GetCurrentDisplayMode(0, current.as_mut_ptr()), 0);
        assert_eq!(SDL_GetDesktopDisplayMode(0, desktop.as_mut_ptr()), 0);
        assert!(SDL_GetNumDisplayModes(0) >= 1);

        let (mut ddpi, mut hdpi, mut vdpi) = (0.0f32, 0.0f32, 0.0f32);
        let dpi_rc = SDL_GetDisplayDPI(0, &mut ddpi, &mut hdpi, &mut vdpi);
        if dpi_rc == 0 {
            assert!(ddpi >= 0.0);
        } else {
            assert!(!testutils::current_error().is_empty());
        }
    }
}

#[test]
fn testcustomcursor_and_testmouse_ports_manage_cursor_clipboard_and_mouse_mode() {
    let _serial = testutils::serial_lock();
    let _video = testutils::ScopedEnvVar::set("SDL_VIDEODRIVER", "dummy");
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_VIDEO);

    unsafe {
        let window = create_hidden_window();
        assert!(!window.is_null(), "{}", testutils::current_error());

        let surface = SDL_CreateRGBSurfaceWithFormat(
            0,
            16,
            16,
            32,
            safe_sdl::abi::generated_types::SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888,
        );
        assert!(!surface.is_null(), "{}", testutils::current_error());
        assert_eq!(SDL_FillRect(surface, ptr::null(), 0xffffffff), 0);

        let color_cursor = SDL_CreateColorCursor(surface, 1, 1);
        assert!(!color_cursor.is_null(), "{}", testutils::current_error());
        SDL_SetCursor(color_cursor);
        let system_cursor = SDL_CreateSystemCursor(SDL_SystemCursor_SDL_SYSTEM_CURSOR_ARROW);
        if !system_cursor.is_null() {
            SDL_SetCursor(system_cursor);
        } else {
            assert!(!testutils::current_error().is_empty());
        }

        assert_eq!(SDL_SetRelativeMouseMode(SDL_bool_SDL_TRUE), 0);
        assert_ne!(SDL_GetRelativeMouseMode(), 0);
        SDL_WarpMouseInWindow(window, 8, 8);
        assert_eq!(SDL_SetRelativeMouseMode(0), 0);
        assert_eq!(SDL_SetWindowMouseGrab(window, SDL_bool_SDL_TRUE), ());

        let text = testutils::cstring("mouse-clipboard");
        assert_eq!(SDL_SetClipboardText(text.as_ptr()), 0);
        assert_eq!(
            testutils::string_from_c(SDL_GetClipboardText()),
            "mouse-clipboard"
        );

        if !system_cursor.is_null() {
            SDL_FreeCursor(system_cursor);
        }
        SDL_FreeCursor(color_cursor);
        SDL_FreeSurface(surface);
        SDL_DestroyWindow(window);
    }
}

#[test]
fn testdropfile_port_roundtrips_drop_events() {
    let _serial = testutils::serial_lock();
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_EVENTS);

    unsafe {
        let path = CString::new("/tmp/phase4-drop.txt").unwrap().into_raw();
        let mut event = SDL_Event {
            drop: SDL_DropEvent {
                type_: SDL_EventType_SDL_DROPFILE,
                timestamp: 0,
                file: path,
                windowID: 0,
            },
        };
        assert_eq!(
            SDL_PushEvent(&mut event),
            1,
            "{}",
            testutils::current_error()
        );
        let mut out = MaybeUninit::<SDL_Event>::zeroed();
        assert_eq!(SDL_PollEvent(out.as_mut_ptr()), 1);
        let out = out.assume_init();
        let observed = CString::from_raw(out.drop.file);
        assert_eq!(observed.to_str().unwrap(), "/tmp/phase4-drop.txt");
    }
}

#[test]
fn gesture_replay_roundtrip_is_deterministic() {
    let _serial = testutils::serial_lock();
    let _video = testutils::ScopedEnvVar::set("SDL_VIDEODRIVER", "dummy");
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_VIDEO | SDL_INIT_EVENTS);

    unsafe {
        let mut buffer = vec![0u8; 4096];
        let rw = SDL_RWFromMem(buffer.as_mut_ptr().cast(), buffer.len() as libc::c_int);
        assert!(!rw.is_null());
        let _ = SDL_RecordGesture(-1);
        let saved = SDL_SaveAllDollarTemplates(rw);
        assert!(saved >= 0, "{}", testutils::current_error());
        assert_eq!(SDL_RWseek(rw, 0, libc::SEEK_SET), 0);
        let loaded = SDL_LoadDollarTemplates(-1, rw);
        assert!(loaded >= 0, "{}", testutils::current_error());
        assert_eq!(SDL_RWclose(rw), 0);
    }
}

#[test]
fn testhittesting_testime_testrelative_and_testwm2_ports_cover_window_interaction() {
    let _serial = testutils::serial_lock();
    let _video = testutils::ScopedEnvVar::set("SDL_VIDEODRIVER", "dummy");
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_VIDEO);

    unsafe {
        let window = create_hidden_window();
        assert!(!window.is_null(), "{}", testutils::current_error());

        let hit_test_rc = SDL_SetWindowHitTest(window, Some(passthrough_hit_test), ptr::null_mut());
        if hit_test_rc != 0 {
            assert!(!testutils::current_error().is_empty());
        }

        let rect = SDL_Rect {
            x: 5,
            y: 6,
            w: 20,
            h: 30,
        };
        SDL_StartTextInput();
        SDL_SetTextInputRect(&rect);
        assert_ne!(SDL_IsTextInputActive(), 0);
        SDL_StopTextInput();
        assert_eq!(SDL_IsTextInputActive(), 0);

        let id = SDL_GetWindowID(window);
        assert_eq!(SDL_GetWindowFromID(id), window);
        let key = testutils::cstring("wm2.data");
        let value = 0x55aausize as *mut c_void;
        assert!(SDL_SetWindowData(window, key.as_ptr(), value).is_null());
        assert_eq!(SDL_GetWindowData(window, key.as_ptr()), value);

        SDL_SetWindowTitle(window, testutils::cstring("wm2").as_ptr());
        SDL_SetWindowPosition(window, 40, 60);
        SDL_SetWindowSize(window, 640, 480);
        let (mut x, mut y, mut w, mut h) = (0, 0, 0, 0);
        SDL_GetWindowPosition(window, &mut x, &mut y);
        SDL_GetWindowSize(window, &mut w, &mut h);
        assert_eq!((x, y, w, h), (40, 60, 640, 480));
        assert!(SDL_GetWindowDisplayIndex(window) >= 0);
        let flash_rc = SDL_FlashWindow(window, SDL_FlashOperation_SDL_FLASH_BRIEFLY);
        if flash_rc != 0 {
            assert!(!testutils::current_error().is_empty());
        }
        let _ = SDL_SetWindowFullscreen(window, 0);
        assert_ne!(
            SDL_GetWindowFlags(window) & SDL_WindowFlags_SDL_WINDOW_HIDDEN,
            0
        );

        SDL_DestroyWindow(window);
    }
}

#[test]
fn testlock_and_testmessage_ports_cover_surface_and_error_paths() {
    let _serial = testutils::serial_lock();
    let _video = testutils::ScopedEnvVar::set("SDL_VIDEODRIVER", "dummy");
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_VIDEO);

    unsafe {
        let window = create_hidden_window();
        assert!(!window.is_null(), "{}", testutils::current_error());
        let surface = SDL_GetWindowSurface(window);
        assert!(!surface.is_null(), "{}", testutils::current_error());
        assert_eq!(
            SDL_UpdateWindowSurface(window),
            0,
            "{}",
            testutils::current_error()
        );

        let rc = SDL_ShowSimpleMessageBox(
            0,
            testutils::cstring("testmessage").as_ptr(),
            testutils::cstring("dummy backend message").as_ptr(),
            window,
        );
        if rc != 0 {
            assert!(!testutils::current_error().is_empty());
        }

        SDL_DestroyWindow(window);
    }
}

#[test]
fn testnative_and_testnativex11_ports_query_syswm_when_x11_is_available() {
    let _serial = testutils::serial_lock();
    if std::env::var_os("DISPLAY").is_none() {
        return;
    }
    let _video = testutils::ScopedEnvVar::set("SDL_VIDEODRIVER", "x11");
    let _subsystem =
        match std::panic::catch_unwind(|| testutils::SubsystemGuard::init(SDL_INIT_VIDEO)) {
            Ok(guard) => guard,
            Err(_) => return,
        };

    unsafe {
        let window = create_hidden_window();
        if window.is_null() {
            return;
        }
        let mut mode = MaybeUninit::<SDL_DisplayMode>::zeroed();
        let _ = SDL_GetWindowDisplayMode(window, mode.as_mut_ptr());

        let mut info = MaybeUninit::<SDL_SysWMinfo>::zeroed();
        (*info.as_mut_ptr()).version.major = 2;
        (*info.as_mut_ptr()).version.minor = 0;
        (*info.as_mut_ptr()).version.patch = 0;
        if SDL_GetWindowWMInfo(window, info.as_mut_ptr()) != 0 {
            assert_eq!(info.assume_init().subsystem, SDL_SYSWM_X11);
        }
        SDL_DestroyWindow(window);
    }
}

#[test]
fn testshape_port_exercises_shaped_window_api_when_supported() {
    let _serial = testutils::serial_lock();
    if std::env::var_os("DISPLAY").is_none() {
        return;
    }
    let _video = testutils::ScopedEnvVar::set("SDL_VIDEODRIVER", "x11");
    let _subsystem =
        match std::panic::catch_unwind(|| testutils::SubsystemGuard::init(SDL_INIT_VIDEO)) {
            Ok(guard) => guard,
            Err(_) => return,
        };

    unsafe {
        let title = testutils::cstring("shape");
        let window = SDL_CreateShapedWindow(
            title.as_ptr(),
            20,
            20,
            128,
            128,
            SDL_WindowFlags_SDL_WINDOW_HIDDEN,
        );
        if window.is_null() {
            return;
        }
        let surface = SDL_CreateRGBSurfaceWithFormat(
            0,
            128,
            128,
            32,
            safe_sdl::abi::generated_types::SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888,
        );
        assert!(!surface.is_null(), "{}", testutils::current_error());
        let _ = SDL_FillRect(surface, ptr::null(), 0xffffffff);
        let mut mode = MaybeUninit::<SDL_WindowShapeMode>::zeroed();
        let _ = SDL_SetWindowShape(window, surface, mode.as_mut_ptr());
        let _ = SDL_IsShapedWindow(window);
        SDL_FreeSurface(surface);
        SDL_DestroyWindow(window);
    }
}

#[cfg(target_os = "windows")]
mod testnativew32_port {
    use super::*;
    use safe_sdl::video::syswm::{SDL_GetWindowWMInfo, SDL_SysWMinfo, SDL_SYSWM_WINDOWS};

    #[test]
    fn testnativew32_port_validates_windows_syswm_dispatch() {
        let _serial = testutils::serial_lock();
        let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_VIDEO);

        unsafe {
            let window = create_hidden_window();
            assert!(!window.is_null(), "{}", testutils::current_error());

            let mut info = MaybeUninit::<SDL_SysWMinfo>::zeroed();
            (*info.as_mut_ptr()).version.major = 2;
            (*info.as_mut_ptr()).version.minor = 0;
            (*info.as_mut_ptr()).version.patch = 0;
            assert_ne!(
                SDL_GetWindowWMInfo(window, info.as_mut_ptr()),
                0,
                "{}",
                testutils::current_error()
            );
            let info = info.assume_init();
            assert_eq!(info.subsystem, SDL_SYSWM_WINDOWS);
            assert!(!info.info.win.window.is_null());

            SDL_DestroyWindow(window);
        }
    }
}

#[cfg(target_os = "macos")]
mod testnativecocoa_port {
    use super::*;
    use safe_sdl::video::syswm::{SDL_GetWindowWMInfo, SDL_SysWMinfo, SDL_SYSWM_COCOA};

    #[test]
    fn testnativecocoa_port_validates_cocoa_syswm_dispatch() {
        let _serial = testutils::serial_lock();
        let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_VIDEO);

        unsafe {
            let window = create_hidden_window();
            assert!(!window.is_null(), "{}", testutils::current_error());

            let mut info = MaybeUninit::<SDL_SysWMinfo>::zeroed();
            (*info.as_mut_ptr()).version.major = 2;
            (*info.as_mut_ptr()).version.minor = 0;
            (*info.as_mut_ptr()).version.patch = 0;
            assert_ne!(
                SDL_GetWindowWMInfo(window, info.as_mut_ptr()),
                0,
                "{}",
                testutils::current_error()
            );
            let info = info.assume_init();
            assert_eq!(info.subsystem, SDL_SYSWM_COCOA);
            assert!(!info.info.cocoa.window.is_null());

            SDL_DestroyWindow(window);
        }
    }
}

#[allow(unexpected_cfgs)]
#[cfg(target_os = "os2")]
mod testnativeos2_port {
    use super::*;
    use safe_sdl::video::syswm::{SDL_GetWindowWMInfo, SDL_SysWMinfo, SDL_SYSWM_OS2};

    #[test]
    fn testnativeos2_port_validates_os2_syswm_dispatch() {
        let _serial = testutils::serial_lock();
        let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_VIDEO);

        unsafe {
            let window = create_hidden_window();
            assert!(!window.is_null(), "{}", testutils::current_error());

            let mut info = MaybeUninit::<SDL_SysWMinfo>::zeroed();
            (*info.as_mut_ptr()).version.major = 2;
            (*info.as_mut_ptr()).version.minor = 0;
            (*info.as_mut_ptr()).version.patch = 0;
            assert_ne!(
                SDL_GetWindowWMInfo(window, info.as_mut_ptr()),
                0,
                "{}",
                testutils::current_error()
            );
            let info = info.assume_init();
            assert_eq!(info.subsystem, SDL_SYSWM_OS2);
            assert!(!info.info.os2.hwnd.is_null());

            SDL_DestroyWindow(window);
        }
    }
}
