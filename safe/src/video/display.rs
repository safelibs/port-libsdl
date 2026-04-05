use std::ffi::CStr;
use std::sync::{Mutex, OnceLock};

use crate::abi::generated_types::{
    SDL_DisplayMode, SDL_DisplayOrientation, SDL_Point, SDL_Rect, SDL_bool, SDL_HINT_VIDEODRIVER,
    SDL_INIT_VIDEO,
};

struct DisplayApi {
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum VideoBackend {
    Host,
    Dummy,
    Offscreen,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum VideoOrigin {
    Direct,
    Subsystem,
}

#[derive(Clone, Copy)]
struct ActiveVideo {
    backend: VideoBackend,
    origin: VideoOrigin,
    started_host_events: bool,
}

fn video_backend() -> &'static Mutex<Option<ActiveVideo>> {
    static BACKEND: OnceLock<Mutex<Option<ActiveVideo>>> = OnceLock::new();
    BACKEND.get_or_init(|| Mutex::new(None))
}

fn lock_video_backend() -> std::sync::MutexGuard<'static, Option<ActiveVideo>> {
    match video_backend().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn requested_driver_name(driver_name: *const libc::c_char) -> Option<String> {
    unsafe {
        let value = if driver_name.is_null() {
            crate::core::hints::SDL_GetHint(SDL_HINT_VIDEODRIVER.as_ptr().cast())
        } else {
            driver_name
        };
        if value.is_null() {
            return None;
        }
        CStr::from_ptr(value)
            .to_str()
            .ok()
            .and_then(|text| text.split(',').map(str::trim).find(|candidate| !candidate.is_empty()))
            .map(str::to_ascii_lowercase)
    }
}

fn backend_name(backend: VideoBackend) -> Option<*const libc::c_char> {
    match backend {
        VideoBackend::Host => None,
        VideoBackend::Dummy => Some(b"dummy\0".as_ptr().cast()),
        VideoBackend::Offscreen => Some(b"offscreen\0".as_ptr().cast()),
    }
}

fn selected_stub_backend(driver_name: *const libc::c_char) -> Option<VideoBackend> {
    match requested_driver_name(driver_name).as_deref() {
        Some("dummy") => Some(VideoBackend::Dummy),
        Some("offscreen") => Some(VideoBackend::Offscreen),
        _ => None,
    }
}

fn api() -> &'static DisplayApi {
    static API: OnceLock<DisplayApi> = OnceLock::new();
    API.get_or_init(|| DisplayApi {
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
    if let Some(backend) = selected_stub_backend(std::ptr::null()) {
        *lock_video_backend() = Some(ActiveVideo {
            backend,
            origin: VideoOrigin::Subsystem,
            started_host_events: false,
        });
        return Ok(());
    }

    let started_host_events = crate::events::queue::ensure_real_event_subsystem();
    crate::video::clear_real_error();
    let rc = unsafe { (api().video_init)(std::ptr::null()) };
    if rc == 0 {
        *lock_video_backend() = Some(ActiveVideo {
            backend: VideoBackend::Host,
            origin: VideoOrigin::Subsystem,
            started_host_events,
        });
        Ok(())
    } else {
        if started_host_events {
            crate::events::queue::quit_event_subsystem();
        }
        Err(())
    }
}

pub(crate) fn quit_video_subsystem() {
    let active = {
        let mut state = lock_video_backend();
        state.take()
    };
    let Some(active) = active else {
        return;
    };
    if active.backend != VideoBackend::Host {
        return;
    }
    unsafe {
        (api().video_quit)();
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
    if let Some(active) = *lock_video_backend() {
        if let Some(name) = backend_name(active.backend) {
            return name;
        }
    }
    crate::video::clear_real_error();
    (api().get_current_video_driver)()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_VideoInit(driver_name: *const libc::c_char) -> libc::c_int {
    if let Some(backend) = selected_stub_backend(driver_name) {
        *lock_video_backend() = Some(ActiveVideo {
            backend,
            origin: VideoOrigin::Direct,
            started_host_events: false,
        });
        return 0;
    }

    let started_host_events = crate::events::queue::ensure_real_event_subsystem();
    crate::video::clear_real_error();
    let rc = (api().video_init)(driver_name);
    if rc == 0 {
        *lock_video_backend() = Some(ActiveVideo {
            backend: VideoBackend::Host,
            origin: VideoOrigin::Direct,
            started_host_events,
        });
    } else if started_host_events {
        crate::events::queue::quit_event_subsystem();
    }
    rc
}

#[no_mangle]
pub unsafe extern "C" fn SDL_VideoQuit() {
    let active = {
        let mut state = lock_video_backend();
        state.take()
    };
    let Some(active) = active else {
        return;
    };
    if active.backend != VideoBackend::Host {
        return;
    }
    crate::video::clear_real_error();
    (api().video_quit)();
    if active.origin == VideoOrigin::Direct && active.started_host_events {
        crate::events::queue::quit_event_subsystem();
    }
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
