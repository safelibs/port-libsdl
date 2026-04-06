use std::ffi::CStr;
use std::sync::{Mutex, OnceLock};

use crate::abi::generated_types::{
    SDL_DisplayMode, SDL_DisplayOrientation, SDL_DisplayOrientation_SDL_ORIENTATION_UNKNOWN,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888, SDL_Point, SDL_Rect, SDL_bool,
    SDL_HINT_VIDEODRIVER, SDL_INIT_VIDEO,
};

const X11_NAME: &[u8] = b"x11\0";
const WAYLAND_NAME: &[u8] = b"wayland\0";
const KMSDRM_NAME: &[u8] = b"KMSDRM\0";
const OFFSCREEN_NAME: &[u8] = b"offscreen\0";
const DUMMY_NAME: &[u8] = b"dummy\0";
const EVDEV_NAME: &[u8] = b"evdev\0";

const X11_DISPLAY_NAME: &[u8] = b"Safe SDL X11 Display\0";
const WAYLAND_DISPLAY_NAME: &[u8] = b"Safe SDL Wayland Display\0";
const KMSDRM_DISPLAY_NAME: &[u8] = b"Safe SDL KMSDRM Display\0";
const OFFSCREEN_DISPLAY_NAME: &[u8] = b"Safe SDL Offscreen Display\0";
const DUMMY_DISPLAY_NAME: &[u8] = b"Safe SDL Dummy Display\0";
const EVDEV_DISPLAY_NAME: &[u8] = b"Safe SDL Evdev Display\0";

const STUB_DISPLAY_BOUNDS: SDL_Rect = SDL_Rect {
    x: 0,
    y: 0,
    w: 1024,
    h: 768,
};

const STUB_DISPLAY_MODE: SDL_DisplayMode = SDL_DisplayMode {
    format: SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888,
    w: 1024,
    h: 768,
    refresh_rate: 60,
    driverdata: std::ptr::null_mut(),
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum VideoBackendKind {
    Host,
    Offscreen,
    Dummy,
    Evdev,
}

#[derive(Clone, Copy)]
pub(crate) struct VideoDriverDescriptor {
    pub name: &'static str,
    pub name_bytes: &'static [u8],
    pub display_name_bytes: &'static [u8],
    pub description: &'static str,
    pub kind: VideoBackendKind,
}

impl VideoDriverDescriptor {
    pub(crate) fn is_host(self) -> bool {
        matches!(self.kind, VideoBackendKind::Host)
    }
}

fn driver_uses_host_runtime(driver: &VideoDriverDescriptor) -> bool {
    driver.is_host() && crate::video::real_sdl_is_available()
}

const DRIVER_REGISTRY: &[VideoDriverDescriptor] = &[
    VideoDriverDescriptor {
        name: "x11",
        name_bytes: X11_NAME,
        display_name_bytes: X11_DISPLAY_NAME,
        description: "SDL X11 video driver",
        kind: VideoBackendKind::Host,
    },
    VideoDriverDescriptor {
        name: "wayland",
        name_bytes: WAYLAND_NAME,
        display_name_bytes: WAYLAND_DISPLAY_NAME,
        description: "SDL Wayland video driver",
        kind: VideoBackendKind::Host,
    },
    VideoDriverDescriptor {
        name: "KMSDRM",
        name_bytes: KMSDRM_NAME,
        display_name_bytes: KMSDRM_DISPLAY_NAME,
        description: "KMS/DRM Video Driver",
        kind: VideoBackendKind::Host,
    },
    VideoDriverDescriptor {
        name: "offscreen",
        name_bytes: OFFSCREEN_NAME,
        display_name_bytes: OFFSCREEN_DISPLAY_NAME,
        description: "SDL offscreen video driver",
        kind: VideoBackendKind::Offscreen,
    },
    VideoDriverDescriptor {
        name: "dummy",
        name_bytes: DUMMY_NAME,
        display_name_bytes: DUMMY_DISPLAY_NAME,
        description: "SDL dummy video driver",
        kind: VideoBackendKind::Dummy,
    },
    VideoDriverDescriptor {
        name: "evdev",
        name_bytes: EVDEV_NAME,
        display_name_bytes: EVDEV_DISPLAY_NAME,
        description: "SDL dummy video driver with evdev",
        kind: VideoBackendKind::Evdev,
    },
];

struct HostDisplayApi {
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

fn load_host_symbol<T>(name: &[u8]) -> T {
    let symbol = unsafe { libc::dlsym(crate::video::real_sdl_handle(), name.as_ptr().cast()) };
    assert!(
        !symbol.is_null(),
        "missing host SDL2 symbol {}",
        String::from_utf8_lossy(&name[..name.len().saturating_sub(1)])
    );
    unsafe { std::mem::transmute_copy(&symbol) }
}

fn host_api() -> &'static HostDisplayApi {
    static API: OnceLock<HostDisplayApi> = OnceLock::new();
    API.get_or_init(|| HostDisplayApi {
        video_init: load_host_symbol(b"SDL_VideoInit\0"),
        video_quit: load_host_symbol(b"SDL_VideoQuit\0"),
        get_num_video_displays: load_host_symbol(b"SDL_GetNumVideoDisplays\0"),
        get_display_name: load_host_symbol(b"SDL_GetDisplayName\0"),
        get_display_bounds: load_host_symbol(b"SDL_GetDisplayBounds\0"),
        get_display_usable_bounds: load_host_symbol(b"SDL_GetDisplayUsableBounds\0"),
        get_display_dpi: load_host_symbol(b"SDL_GetDisplayDPI\0"),
        get_display_orientation: load_host_symbol(b"SDL_GetDisplayOrientation\0"),
        get_num_display_modes: load_host_symbol(b"SDL_GetNumDisplayModes\0"),
        get_display_mode: load_host_symbol(b"SDL_GetDisplayMode\0"),
        get_desktop_display_mode: load_host_symbol(b"SDL_GetDesktopDisplayMode\0"),
        get_current_display_mode: load_host_symbol(b"SDL_GetCurrentDisplayMode\0"),
        get_closest_display_mode: load_host_symbol(b"SDL_GetClosestDisplayMode\0"),
        get_point_display_index: load_host_symbol(b"SDL_GetPointDisplayIndex\0"),
        get_rect_display_index: load_host_symbol(b"SDL_GetRectDisplayIndex\0"),
        is_screen_saver_enabled: load_host_symbol(b"SDL_IsScreenSaverEnabled\0"),
        enable_screen_saver: load_host_symbol(b"SDL_EnableScreenSaver\0"),
        disable_screen_saver: load_host_symbol(b"SDL_DisableScreenSaver\0"),
    })
}

#[derive(Default)]
struct VideoState {
    current_driver_index: Option<usize>,
    screen_saver_enabled: bool,
}

fn video_state() -> &'static Mutex<VideoState> {
    static STATE: OnceLock<Mutex<VideoState>> = OnceLock::new();
    STATE.get_or_init(|| {
        Mutex::new(VideoState {
            current_driver_index: None,
            screen_saver_enabled: true,
        })
    })
}

fn lock_video_state() -> std::sync::MutexGuard<'static, VideoState> {
    match video_state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn driver_index_from_name(name: &str) -> Option<usize> {
    DRIVER_REGISTRY
        .iter()
        .position(|driver| driver.name.eq_ignore_ascii_case(name))
}

fn requested_driver_list(driver_name: *const libc::c_char) -> Option<Vec<String>> {
    unsafe {
        let value = if driver_name.is_null() {
            crate::core::hints::SDL_GetHint(SDL_HINT_VIDEODRIVER.as_ptr().cast())
        } else {
            driver_name
        };
        if value.is_null() {
            return None;
        }
        let names = CStr::from_ptr(value)
            .to_str()
            .ok()?
            .split(',')
            .map(str::trim)
            .filter(|candidate| !candidate.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        if names.is_empty() {
            None
        } else {
            Some(names)
        }
    }
}

fn candidate_driver_indices(driver_name: *const libc::c_char) -> Result<Vec<usize>, ()> {
    if let Some(requested) = requested_driver_list(driver_name) {
        let indices = requested
            .iter()
            .filter_map(|name| driver_index_from_name(name))
            .collect::<Vec<_>>();
        if indices.is_empty() {
            let _ = crate::core::error::set_error_message("No available video device");
            return Err(());
        }
        return Ok(indices);
    }

    Ok((0..DRIVER_REGISTRY.len()).collect())
}

fn current_driver_from_state(state: &VideoState) -> Option<&'static VideoDriverDescriptor> {
    state
        .current_driver_index
        .and_then(|index| DRIVER_REGISTRY.get(index))
}

fn reset_video_runtime_state() {
    crate::video::window::reset_video_state();
    crate::events::keyboard::reset_keyboard_state();
    crate::events::mouse::reset_mouse_state();
    crate::video::linux::ime::reset_text_input_state();
}

fn tear_down_locked(state: &mut VideoState) {
    if let Some(driver) = current_driver_from_state(state) {
        if driver_uses_host_runtime(driver) {
            crate::video::clear_real_error();
            unsafe {
                (host_api().video_quit)();
            }
        }
    }
    state.current_driver_index = None;
    state.screen_saver_enabled = true;
    reset_video_runtime_state();
}

fn try_activate_driver(index: usize) -> Result<(), ()> {
    let driver = DRIVER_REGISTRY[index];
    if driver.is_host() {
        if !crate::video::real_sdl_is_available() {
            return Err(());
        }
        crate::video::clear_real_error();
        let rc = unsafe { (host_api().video_init)(driver.name_bytes.as_ptr().cast()) };
        if rc != 0 {
            return Err(());
        }
    }

    let mut state = lock_video_state();
    state.current_driver_index = Some(index);
    state.screen_saver_enabled = true;
    Ok(())
}

fn init_video_internal(driver_name: *const libc::c_char) -> Result<(), ()> {
    let candidates = candidate_driver_indices(driver_name)?;

    {
        let mut state = lock_video_state();
        tear_down_locked(&mut state);
    }

    for index in candidates {
        if try_activate_driver(index).is_ok() {
            return Ok(());
        }
    }

    let _ = crate::core::error::set_error_message("No available video device");
    Err(())
}

fn stub_driver_active() -> Option<&'static VideoDriverDescriptor> {
    let state = lock_video_state();
    current_driver_from_state(&state).filter(|driver| !driver.is_host())
}

pub(crate) fn active_driver() -> Option<&'static VideoDriverDescriptor> {
    let state = lock_video_state();
    current_driver_from_state(&state)
}

pub(crate) fn require_video_driver() -> Result<&'static VideoDriverDescriptor, ()> {
    active_driver().ok_or_else(|| {
        let _ = crate::core::error::set_error_message("Video subsystem has not been initialized");
    })
}

pub(crate) fn current_driver_is_host() -> bool {
    active_driver()
        .map(driver_uses_host_runtime)
        .unwrap_or(false)
}

pub(crate) fn stub_display_mode() -> SDL_DisplayMode {
    STUB_DISPLAY_MODE
}

pub(crate) fn init_video_subsystem() -> Result<(), ()> {
    let _ = SDL_INIT_VIDEO;
    init_video_internal(std::ptr::null())
}

pub(crate) fn quit_video_subsystem() {
    let mut state = lock_video_state();
    tear_down_locked(&mut state);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetNumVideoDrivers() -> libc::c_int {
    DRIVER_REGISTRY.len() as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetVideoDriver(index: libc::c_int) -> *const libc::c_char {
    if index < 0 {
        return std::ptr::null();
    }
    DRIVER_REGISTRY
        .get(index as usize)
        .map(|driver| driver.name_bytes.as_ptr().cast())
        .unwrap_or(std::ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetCurrentVideoDriver() -> *const libc::c_char {
    active_driver()
        .map(|driver| driver.name_bytes.as_ptr().cast())
        .unwrap_or(std::ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_VideoInit(driver_name: *const libc::c_char) -> libc::c_int {
    match init_video_internal(driver_name) {
        Ok(()) => 0,
        Err(()) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_VideoQuit() {
    quit_video_subsystem();
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetNumVideoDisplays() -> libc::c_int {
    match require_video_driver() {
        Ok(driver) if driver_uses_host_runtime(driver) => {
            crate::video::clear_real_error();
            (host_api().get_num_video_displays)()
        }
        Ok(_) => 1,
        Err(()) => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetDisplayName(displayIndex: libc::c_int) -> *const libc::c_char {
    match require_video_driver() {
        Ok(driver) if driver_uses_host_runtime(driver) => {
            crate::video::clear_real_error();
            (host_api().get_display_name)(displayIndex)
        }
        Ok(driver) => {
            if displayIndex == 0 {
                driver.display_name_bytes.as_ptr().cast()
            } else {
                std::ptr::null()
            }
        }
        Err(()) => std::ptr::null(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetDisplayBounds(
    displayIndex: libc::c_int,
    rect: *mut SDL_Rect,
) -> libc::c_int {
    if rect.is_null() {
        return crate::core::error::invalid_param_error("rect");
    }
    match require_video_driver() {
        Ok(driver) if driver_uses_host_runtime(driver) => {
            crate::video::clear_real_error();
            (host_api().get_display_bounds)(displayIndex, rect)
        }
        Ok(_) if displayIndex == 0 => {
            *rect = STUB_DISPLAY_BOUNDS;
            0
        }
        Ok(_) => crate::core::error::set_error_message("Display index out of range"),
        Err(()) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetDisplayUsableBounds(
    displayIndex: libc::c_int,
    rect: *mut SDL_Rect,
) -> libc::c_int {
    if rect.is_null() {
        return crate::core::error::invalid_param_error("rect");
    }
    match require_video_driver() {
        Ok(driver) if driver_uses_host_runtime(driver) => {
            crate::video::clear_real_error();
            (host_api().get_display_usable_bounds)(displayIndex, rect)
        }
        Ok(_) if displayIndex == 0 => {
            *rect = STUB_DISPLAY_BOUNDS;
            0
        }
        Ok(_) => crate::core::error::set_error_message("Display index out of range"),
        Err(()) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetDisplayDPI(
    displayIndex: libc::c_int,
    ddpi: *mut f32,
    hdpi: *mut f32,
    vdpi: *mut f32,
) -> libc::c_int {
    match require_video_driver() {
        Ok(driver) if driver_uses_host_runtime(driver) => {
            crate::video::clear_real_error();
            (host_api().get_display_dpi)(displayIndex, ddpi, hdpi, vdpi)
        }
        Ok(_) if displayIndex == 0 => {
            if !ddpi.is_null() {
                *ddpi = 96.0;
            }
            if !hdpi.is_null() {
                *hdpi = 96.0;
            }
            if !vdpi.is_null() {
                *vdpi = 96.0;
            }
            0
        }
        Ok(_) => crate::core::error::set_error_message("Display index out of range"),
        Err(()) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetDisplayOrientation(
    displayIndex: libc::c_int,
) -> SDL_DisplayOrientation {
    match require_video_driver() {
        Ok(driver) if driver_uses_host_runtime(driver) => {
            crate::video::clear_real_error();
            (host_api().get_display_orientation)(displayIndex)
        }
        Ok(_) if displayIndex == 0 => SDL_DisplayOrientation_SDL_ORIENTATION_UNKNOWN,
        _ => SDL_DisplayOrientation_SDL_ORIENTATION_UNKNOWN,
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetNumDisplayModes(displayIndex: libc::c_int) -> libc::c_int {
    match require_video_driver() {
        Ok(driver) if driver_uses_host_runtime(driver) => {
            crate::video::clear_real_error();
            (host_api().get_num_display_modes)(displayIndex)
        }
        Ok(_) if displayIndex == 0 => 1,
        Ok(_) => crate::core::error::set_error_message("Display index out of range"),
        Err(()) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetDisplayMode(
    displayIndex: libc::c_int,
    modeIndex: libc::c_int,
    mode: *mut SDL_DisplayMode,
) -> libc::c_int {
    if mode.is_null() {
        return crate::core::error::invalid_param_error("mode");
    }
    match require_video_driver() {
        Ok(driver) if driver_uses_host_runtime(driver) => {
            crate::video::clear_real_error();
            (host_api().get_display_mode)(displayIndex, modeIndex, mode)
        }
        Ok(_) if displayIndex == 0 && modeIndex == 0 => {
            *mode = STUB_DISPLAY_MODE;
            0
        }
        Ok(_) => crate::core::error::set_error_message("Display mode index out of range"),
        Err(()) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetDesktopDisplayMode(
    displayIndex: libc::c_int,
    mode: *mut SDL_DisplayMode,
) -> libc::c_int {
    if mode.is_null() {
        return crate::core::error::invalid_param_error("mode");
    }
    match require_video_driver() {
        Ok(driver) if driver_uses_host_runtime(driver) => {
            crate::video::clear_real_error();
            (host_api().get_desktop_display_mode)(displayIndex, mode)
        }
        Ok(_) if displayIndex == 0 => {
            *mode = STUB_DISPLAY_MODE;
            0
        }
        Ok(_) => crate::core::error::set_error_message("Display index out of range"),
        Err(()) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetCurrentDisplayMode(
    displayIndex: libc::c_int,
    mode: *mut SDL_DisplayMode,
) -> libc::c_int {
    if mode.is_null() {
        return crate::core::error::invalid_param_error("mode");
    }
    match require_video_driver() {
        Ok(driver) if driver_uses_host_runtime(driver) => {
            crate::video::clear_real_error();
            (host_api().get_current_display_mode)(displayIndex, mode)
        }
        Ok(_) if displayIndex == 0 => {
            *mode = STUB_DISPLAY_MODE;
            0
        }
        Ok(_) => crate::core::error::set_error_message("Display index out of range"),
        Err(()) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetClosestDisplayMode(
    displayIndex: libc::c_int,
    mode: *const SDL_DisplayMode,
    closest: *mut SDL_DisplayMode,
) -> *mut SDL_DisplayMode {
    if mode.is_null() || closest.is_null() {
        let _ = crate::core::error::invalid_param_error("mode");
        return std::ptr::null_mut();
    }
    match require_video_driver() {
        Ok(driver) if driver_uses_host_runtime(driver) => {
            crate::video::clear_real_error();
            (host_api().get_closest_display_mode)(displayIndex, mode, closest)
        }
        Ok(_) if displayIndex == 0 => {
            *closest = *mode;
            closest
        }
        _ => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetPointDisplayIndex(point: *const SDL_Point) -> libc::c_int {
    if point.is_null() {
        return crate::core::error::invalid_param_error("point");
    }
    match require_video_driver() {
        Ok(driver) if driver_uses_host_runtime(driver) => {
            crate::video::clear_real_error();
            (host_api().get_point_display_index)(point)
        }
        Ok(_) => 0,
        Err(()) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetRectDisplayIndex(rect: *const SDL_Rect) -> libc::c_int {
    if rect.is_null() {
        return crate::core::error::invalid_param_error("rect");
    }
    match require_video_driver() {
        Ok(driver) if driver_uses_host_runtime(driver) => {
            crate::video::clear_real_error();
            (host_api().get_rect_display_index)(rect)
        }
        Ok(_) => 0,
        Err(()) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_IsScreenSaverEnabled() -> SDL_bool {
    match require_video_driver() {
        Ok(driver) if driver_uses_host_runtime(driver) => {
            crate::video::clear_real_error();
            (host_api().is_screen_saver_enabled)()
        }
        Ok(_) => {
            let state = lock_video_state();
            state.screen_saver_enabled as SDL_bool
        }
        Err(()) => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_EnableScreenSaver() {
    if let Some(driver) = active_driver() {
        if driver_uses_host_runtime(driver) {
            crate::video::clear_real_error();
            (host_api().enable_screen_saver)();
        } else {
            lock_video_state().screen_saver_enabled = true;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_DisableScreenSaver() {
    if let Some(driver) = active_driver() {
        if driver_uses_host_runtime(driver) {
            crate::video::clear_real_error();
            (host_api().disable_screen_saver)();
        } else {
            lock_video_state().screen_saver_enabled = false;
        }
    }
}
