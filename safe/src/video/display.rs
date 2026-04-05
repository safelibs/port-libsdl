use std::sync::OnceLock;

use crate::abi::generated_types::{
    SDL_DisplayMode, SDL_DisplayOrientation, SDL_Point, SDL_Rect, SDL_bool, SDL_INIT_VIDEO,
};

struct DisplayApi {
    init_subsystem: unsafe extern "C" fn(u32) -> libc::c_int,
    quit_subsystem: unsafe extern "C" fn(u32),
    get_num_video_drivers: unsafe extern "C" fn() -> libc::c_int,
    get_video_driver: unsafe extern "C" fn(libc::c_int) -> *const libc::c_char,
    get_current_video_driver: unsafe extern "C" fn() -> *const libc::c_char,
    video_init: unsafe extern "C" fn(*const libc::c_char) -> libc::c_int,
    video_quit: unsafe extern "C" fn(),
    get_num_video_displays: unsafe extern "C" fn() -> libc::c_int,
    get_display_name: unsafe extern "C" fn(libc::c_int) -> *const libc::c_char,
    get_display_bounds: unsafe extern "C" fn(libc::c_int, *mut SDL_Rect) -> libc::c_int,
    get_display_usable_bounds: unsafe extern "C" fn(libc::c_int, *mut SDL_Rect) -> libc::c_int,
    get_display_dpi: unsafe extern "C" fn(libc::c_int, *mut f32, *mut f32, *mut f32) -> libc::c_int,
    get_display_orientation: unsafe extern "C" fn(libc::c_int) -> SDL_DisplayOrientation,
    get_num_display_modes: unsafe extern "C" fn(libc::c_int) -> libc::c_int,
    get_display_mode:
        unsafe extern "C" fn(libc::c_int, libc::c_int, *mut SDL_DisplayMode) -> libc::c_int,
    get_desktop_display_mode:
        unsafe extern "C" fn(libc::c_int, *mut SDL_DisplayMode) -> libc::c_int,
    get_current_display_mode:
        unsafe extern "C" fn(libc::c_int, *mut SDL_DisplayMode) -> libc::c_int,
    get_closest_display_mode: unsafe extern "C" fn(
        libc::c_int,
        *const SDL_DisplayMode,
        *mut SDL_DisplayMode,
    ) -> *mut SDL_DisplayMode,
    get_point_display_index: unsafe extern "C" fn(*const SDL_Point) -> libc::c_int,
    get_rect_display_index: unsafe extern "C" fn(*const SDL_Rect) -> libc::c_int,
    is_screen_saver_enabled: unsafe extern "C" fn() -> SDL_bool,
    enable_screen_saver: unsafe extern "C" fn(),
    disable_screen_saver: unsafe extern "C" fn(),
}

fn api() -> &'static DisplayApi {
    static API: OnceLock<DisplayApi> = OnceLock::new();
    API.get_or_init(|| DisplayApi {
        init_subsystem: crate::video::load_symbol(b"SDL_InitSubSystem\0"),
        quit_subsystem: crate::video::load_symbol(b"SDL_QuitSubSystem\0"),
        get_num_video_drivers: crate::video::load_symbol(b"SDL_GetNumVideoDrivers\0"),
        get_video_driver: crate::video::load_symbol(b"SDL_GetVideoDriver\0"),
        get_current_video_driver: crate::video::load_symbol(b"SDL_GetCurrentVideoDriver\0"),
        video_init: crate::video::load_symbol(b"SDL_VideoInit\0"),
        video_quit: crate::video::load_symbol(b"SDL_VideoQuit\0"),
        get_num_video_displays: crate::video::load_symbol(b"SDL_GetNumVideoDisplays\0"),
        get_display_name: crate::video::load_symbol(b"SDL_GetDisplayName\0"),
        get_display_bounds: crate::video::load_symbol(b"SDL_GetDisplayBounds\0"),
        get_display_usable_bounds: crate::video::load_symbol(b"SDL_GetDisplayUsableBounds\0"),
        get_display_dpi: crate::video::load_symbol(b"SDL_GetDisplayDPI\0"),
        get_display_orientation: crate::video::load_symbol(b"SDL_GetDisplayOrientation\0"),
        get_num_display_modes: crate::video::load_symbol(b"SDL_GetNumDisplayModes\0"),
        get_display_mode: crate::video::load_symbol(b"SDL_GetDisplayMode\0"),
        get_desktop_display_mode: crate::video::load_symbol(b"SDL_GetDesktopDisplayMode\0"),
        get_current_display_mode: crate::video::load_symbol(b"SDL_GetCurrentDisplayMode\0"),
        get_closest_display_mode: crate::video::load_symbol(b"SDL_GetClosestDisplayMode\0"),
        get_point_display_index: crate::video::load_symbol(b"SDL_GetPointDisplayIndex\0"),
        get_rect_display_index: crate::video::load_symbol(b"SDL_GetRectDisplayIndex\0"),
        is_screen_saver_enabled: crate::video::load_symbol(b"SDL_IsScreenSaverEnabled\0"),
        enable_screen_saver: crate::video::load_symbol(b"SDL_EnableScreenSaver\0"),
        disable_screen_saver: crate::video::load_symbol(b"SDL_DisableScreenSaver\0"),
    })
}

pub(crate) fn init_video_subsystem() -> Result<(), ()> {
    crate::video::clear_real_error();
    let rc = unsafe { (api().init_subsystem)(SDL_INIT_VIDEO) };
    if rc == 0 {
        Ok(())
    } else {
        Err(())
    }
}

pub(crate) fn quit_video_subsystem() {
    unsafe {
        (api().quit_subsystem)(SDL_INIT_VIDEO);
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetNumVideoDrivers() -> libc::c_int {
    crate::video::clear_real_error();
    (api().get_num_video_drivers)()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetVideoDriver(index: libc::c_int) -> *const libc::c_char {
    crate::video::clear_real_error();
    (api().get_video_driver)(index)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetCurrentVideoDriver() -> *const libc::c_char {
    crate::video::clear_real_error();
    (api().get_current_video_driver)()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_VideoInit(driver_name: *const libc::c_char) -> libc::c_int {
    crate::video::clear_real_error();
    (api().video_init)(driver_name)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_VideoQuit() {
    crate::video::clear_real_error();
    (api().video_quit)();
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetNumVideoDisplays() -> libc::c_int {
    crate::video::clear_real_error();
    (api().get_num_video_displays)()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetDisplayName(displayIndex: libc::c_int) -> *const libc::c_char {
    crate::video::clear_real_error();
    (api().get_display_name)(displayIndex)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetDisplayBounds(
    displayIndex: libc::c_int,
    rect: *mut SDL_Rect,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().get_display_bounds)(displayIndex, rect)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetDisplayUsableBounds(
    displayIndex: libc::c_int,
    rect: *mut SDL_Rect,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().get_display_usable_bounds)(displayIndex, rect)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetDisplayDPI(
    displayIndex: libc::c_int,
    ddpi: *mut f32,
    hdpi: *mut f32,
    vdpi: *mut f32,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().get_display_dpi)(displayIndex, ddpi, hdpi, vdpi)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetDisplayOrientation(
    displayIndex: libc::c_int,
) -> SDL_DisplayOrientation {
    crate::video::clear_real_error();
    (api().get_display_orientation)(displayIndex)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetNumDisplayModes(displayIndex: libc::c_int) -> libc::c_int {
    crate::video::clear_real_error();
    (api().get_num_display_modes)(displayIndex)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetDisplayMode(
    displayIndex: libc::c_int,
    modeIndex: libc::c_int,
    mode: *mut SDL_DisplayMode,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().get_display_mode)(displayIndex, modeIndex, mode)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetDesktopDisplayMode(
    displayIndex: libc::c_int,
    mode: *mut SDL_DisplayMode,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().get_desktop_display_mode)(displayIndex, mode)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetCurrentDisplayMode(
    displayIndex: libc::c_int,
    mode: *mut SDL_DisplayMode,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().get_current_display_mode)(displayIndex, mode)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetClosestDisplayMode(
    displayIndex: libc::c_int,
    mode: *const SDL_DisplayMode,
    closest: *mut SDL_DisplayMode,
) -> *mut SDL_DisplayMode {
    crate::video::clear_real_error();
    (api().get_closest_display_mode)(displayIndex, mode, closest)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetPointDisplayIndex(point: *const SDL_Point) -> libc::c_int {
    crate::video::clear_real_error();
    (api().get_point_display_index)(point)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetRectDisplayIndex(rect: *const SDL_Rect) -> libc::c_int {
    crate::video::clear_real_error();
    (api().get_rect_display_index)(rect)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_IsScreenSaverEnabled() -> SDL_bool {
    crate::video::clear_real_error();
    (api().is_screen_saver_enabled)()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_EnableScreenSaver() {
    crate::video::clear_real_error();
    (api().enable_screen_saver)();
}

#[no_mangle]
pub unsafe extern "C" fn SDL_DisableScreenSaver() {
    crate::video::clear_real_error();
    (api().disable_screen_saver)();
}
