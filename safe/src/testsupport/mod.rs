use std::collections::BTreeMap;
use std::ffi::CStr;
use std::mem::{self, MaybeUninit};
use std::os::raw::{c_char, c_int, c_uint, c_ulong, c_void};
use std::ptr;
use std::sync::{Mutex, OnceLock};

use crate::abi::generated_types::{
    self as sdl, SDL_AudioSpec, SDL_Rect, SDL_Renderer, SDL_Texture, SDL_Window, SDL_bool, Uint32,
};

pub mod assert;
pub mod common;
pub mod compare;
pub mod crc32;
pub mod font;
pub mod fuzzer;
pub mod harness;
pub mod images;
pub mod log;
pub mod md5;
pub mod memory;
pub mod random;

pub const SDLTEST_MAX_LOGMESSAGE_LENGTH: usize = 3584;
pub const FONT_CHARACTER_SIZE: i32 = 8;
pub const FONT_LINE_HEIGHT: i32 = FONT_CHARACTER_SIZE + 2;

pub const VERBOSE_VIDEO: Uint32 = 0x0000_0001;
pub const VERBOSE_MODES: Uint32 = 0x0000_0002;
pub const VERBOSE_RENDER: Uint32 = 0x0000_0004;
pub const VERBOSE_EVENT: Uint32 = 0x0000_0008;
pub const VERBOSE_AUDIO: Uint32 = 0x0000_0010;
pub const VERBOSE_MOTION: Uint32 = 0x0000_0020;

pub const TEST_ENABLED: c_int = 1;
pub const TEST_DISABLED: c_int = 0;

pub const TEST_ABORTED: c_int = -1;
pub const TEST_STARTED: c_int = 0;
pub const TEST_COMPLETED: c_int = 1;
pub const TEST_SKIPPED: c_int = 2;

pub const TEST_RESULT_PASSED: c_int = 0;
pub const TEST_RESULT_FAILED: c_int = 1;
pub const TEST_RESULT_NO_ASSERT: c_int = 2;
pub const TEST_RESULT_SKIPPED: c_int = 3;
pub const TEST_RESULT_SETUP_FAILURE: c_int = 4;

pub const ASSERT_FAIL: c_int = 0;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SDLTest_RandomContext {
    pub a: c_uint,
    pub x: c_uint,
    pub c: c_uint,
    pub ah: c_uint,
    pub al: c_uint,
}

#[repr(C)]
pub struct SDLTest_Crc32Context {
    pub crc32_table: [c_uint; 256],
}

#[repr(C)]
pub struct SDLTest_Md5Context {
    pub i: [c_ulong; 2],
    pub buf: [c_ulong; 4],
    pub in_: [u8; 64],
    pub digest: [u8; 16],
}

#[repr(C)]
pub struct SDLTest_TextWindow {
    pub rect: SDL_Rect,
    pub current: c_int,
    pub numlines: c_int,
    pub lines: *mut *mut c_char,
}

pub type SDLTest_TestCaseSetUpFp = Option<unsafe extern "C" fn(arg: *mut c_void)>;
pub type SDLTest_TestCaseFp = Option<unsafe extern "C" fn(arg: *mut c_void) -> c_int>;
pub type SDLTest_TestCaseTearDownFp = Option<unsafe extern "C" fn(arg: *mut c_void)>;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SDLTest_TestCaseReference {
    pub testCase: SDLTest_TestCaseFp,
    pub name: *const c_char,
    pub description: *const c_char,
    pub enabled: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SDLTest_TestSuiteReference {
    pub name: *const c_char,
    pub testSetUp: SDLTest_TestCaseSetUpFp,
    pub testCases: *const *const SDLTest_TestCaseReference,
    pub testTearDown: SDLTest_TestCaseTearDownFp,
}

#[repr(C)]
pub struct SDLTest_CommonState {
    pub argv: *mut *mut c_char,
    pub flags: Uint32,
    pub verbose: Uint32,
    pub videodriver: *const c_char,
    pub display: c_int,
    pub window_title: *const c_char,
    pub window_icon: *const c_char,
    pub window_flags: Uint32,
    pub flash_on_focus_loss: SDL_bool,
    pub window_x: c_int,
    pub window_y: c_int,
    pub window_w: c_int,
    pub window_h: c_int,
    pub window_minW: c_int,
    pub window_minH: c_int,
    pub window_maxW: c_int,
    pub window_maxH: c_int,
    pub logical_w: c_int,
    pub logical_h: c_int,
    pub scale: f32,
    pub depth: c_int,
    pub refresh_rate: c_int,
    pub num_windows: c_int,
    pub windows: *mut *mut SDL_Window,
    pub renderdriver: *const c_char,
    pub render_flags: Uint32,
    pub skip_renderer: SDL_bool,
    pub renderers: *mut *mut SDL_Renderer,
    pub targets: *mut *mut SDL_Texture,
    pub audiodriver: *const c_char,
    pub audiospec: SDL_AudioSpec,
    pub gl_red_size: c_int,
    pub gl_green_size: c_int,
    pub gl_blue_size: c_int,
    pub gl_alpha_size: c_int,
    pub gl_buffer_size: c_int,
    pub gl_depth_size: c_int,
    pub gl_stencil_size: c_int,
    pub gl_double_buffer: c_int,
    pub gl_accum_red_size: c_int,
    pub gl_accum_green_size: c_int,
    pub gl_accum_blue_size: c_int,
    pub gl_accum_alpha_size: c_int,
    pub gl_stereo: c_int,
    pub gl_multisamplebuffers: c_int,
    pub gl_multisamplesamples: c_int,
    pub gl_retained_backing: c_int,
    pub gl_accelerated: c_int,
    pub gl_major_version: c_int,
    pub gl_minor_version: c_int,
    pub gl_debug: c_int,
    pub gl_profile_mask: c_int,
    pub confine: SDL_Rect,
}

pub fn usage_cache() -> &'static Mutex<BTreeMap<usize, Vec<u8>>> {
    static CACHE: OnceLock<Mutex<BTreeMap<usize, Vec<u8>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(BTreeMap::new()))
}

pub fn lock_usage_cache() -> std::sync::MutexGuard<'static, BTreeMap<usize, Vec<u8>>> {
    match usage_cache().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

pub unsafe fn c_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    CStr::from_ptr(ptr).to_string_lossy().into_owned()
}

pub unsafe fn optional_c_string(ptr: *const c_char) -> Option<String> {
    (!ptr.is_null()).then(|| c_string(ptr))
}

pub unsafe fn c_array_len(mut values: *mut *mut c_char) -> usize {
    let mut len = 0usize;
    while !values.is_null() && !(*values).is_null() {
        len += 1;
        values = values.add(1);
    }
    len
}

pub unsafe fn argv_at(argv: *mut *mut c_char, index: usize) -> *mut c_char {
    if argv.is_null() {
        return ptr::null_mut();
    }
    *argv.add(index)
}

pub fn windowpos_undefined() -> c_int {
    sdl::SDL_WINDOWPOS_UNDEFINED_MASK as c_int
}

pub fn windowpos_centered() -> c_int {
    sdl::SDL_WINDOWPOS_CENTERED_MASK as c_int
}

pub unsafe fn alloc_c_string(value: &str) -> *mut c_char {
    let bytes = value.as_bytes();
    let ptr = sdl::SDL_malloc(bytes.len() + 1).cast::<u8>();
    if ptr.is_null() {
        return ptr.cast();
    }
    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
    *ptr.add(bytes.len()) = 0;
    ptr.cast()
}

pub unsafe fn copy_bytes_to_surface(
    surface: *mut sdl::SDL_Surface,
    pixels: &[u8],
    pitch: usize,
    row_len: usize,
    height: usize,
) {
    if surface.is_null() {
        return;
    }
    let dst_pitch = (*surface).pitch as usize;
    let mut src = pixels.as_ptr();
    let mut dst = (*surface).pixels.cast::<u8>();
    for _ in 0..height {
        ptr::copy_nonoverlapping(src, dst, row_len);
        src = src.add(pitch);
        dst = dst.add(dst_pitch);
    }
}

pub unsafe fn with_c_buffer<T>(value: &str, f: impl FnOnce(*const c_char) -> T) -> T {
    let mut bytes = value.as_bytes().to_vec();
    bytes.push(0);
    f(bytes.as_ptr().cast())
}

pub fn parse_int(value: *const c_char) -> Option<c_int> {
    unsafe { c_string(value).trim().parse::<c_int>().ok() }
}

pub fn zeroed_audio_spec() -> SDL_AudioSpec {
    unsafe { mem::zeroed() }
}

pub fn maybe_uninit_zeroed<T>() -> MaybeUninit<T> {
    MaybeUninit::zeroed()
}

pub unsafe fn invalid_param_error(param: &str) -> c_int {
    with_c_buffer(param, |param_ptr| {
        sdl::SDL_SetError(b"Parameter '%s' is invalid\0".as_ptr().cast(), param_ptr)
    })
}

pub unsafe fn unsupported_error() -> c_int {
    sdl::SDL_Error(sdl::SDL_errorcode_SDL_UNSUPPORTED)
}
