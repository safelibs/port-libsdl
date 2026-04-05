#![cfg(feature = "host-video-tests")]
#![allow(clippy::all)]

#[path = "common/testutils.rs"]
mod testutils;

use std::mem::MaybeUninit;

use safe_sdl::abi::generated_types::{SDL_WindowFlags_SDL_WINDOW_HIDDEN, SDL_INIT_VIDEO};
use safe_sdl::video::clipboard::{SDL_GetClipboardText, SDL_SetClipboardText};
use safe_sdl::video::display::SDL_GetCurrentVideoDriver;
use safe_sdl::video::syswm::{SDL_GetWindowWMInfo, SDL_SysWMinfo, SDL_SYSWM_X11};
use safe_sdl::video::window::{
    SDL_CreateWindow, SDL_DestroyWindow, SDL_GetWindowSurface, SDL_UpdateWindowSurface,
};

#[test]
fn xvfb_backed_x11_window_smoke_replaces_manual_window_demos() {
    let _serial = testutils::serial_lock();
    let Some(_display) = testutils::acquire_x11_display() else {
        return;
    };
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_VIDEO);

    unsafe {
        assert_eq!(testutils::string_from_c(SDL_GetCurrentVideoDriver()), "x11");
        let title = testutils::cstring("xvfb-smoke");
        let window = SDL_CreateWindow(
            title.as_ptr(),
            24,
            24,
            320,
            200,
            SDL_WindowFlags_SDL_WINDOW_HIDDEN,
        );
        assert!(!window.is_null(), "{}", testutils::current_error());

        let surface = SDL_GetWindowSurface(window);
        assert!(!surface.is_null(), "{}", testutils::current_error());
        assert_eq!(
            SDL_UpdateWindowSurface(window),
            0,
            "{}",
            testutils::current_error()
        );

        let clipboard = testutils::cstring("xvfb clipboard");
        assert_eq!(
            SDL_SetClipboardText(clipboard.as_ptr()),
            0,
            "{}",
            testutils::current_error()
        );
        assert_eq!(
            testutils::string_from_c(SDL_GetClipboardText()),
            "xvfb clipboard"
        );

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
        assert_eq!(info.assume_init().subsystem, SDL_SYSWM_X11);

        SDL_DestroyWindow(window);
    }
}
