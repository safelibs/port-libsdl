#![allow(dead_code)]
#![allow(clippy::all)]

use std::ffi::{CStr, CString};
use std::path::Path;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::thread;
use std::time::Duration;

use safe_sdl::abi::generated_types::{wchar_t, Uint32};
use safe_sdl::core::error::SDL_GetError;
use safe_sdl::core::hints::{SDL_ResetHint, SDL_SetHint};
use safe_sdl::core::init::{SDL_InitSubSystem, SDL_QuitSubSystem};
use safe_sdl::video::display::{SDL_VideoInit, SDL_VideoQuit};

static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub fn serial_lock() -> MutexGuard<'static, ()> {
    match TEST_LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("safe crate lives under repo root")
        .to_path_buf()
}

pub fn resource_path(file: &str) -> PathBuf {
    repo_root().join("original/test").join(file)
}

pub fn get_nearby_filename(file: &str) -> PathBuf {
    let path = resource_path(file);
    if path.exists() {
        path
    } else {
        repo_root().join(file)
    }
}

pub fn get_resource_filename(user_specified: Option<&str>, default_name: &str) -> PathBuf {
    user_specified
        .map(PathBuf::from)
        .unwrap_or_else(|| get_nearby_filename(default_name))
}

pub fn load_utf8_fixture() -> Vec<u8> {
    std::fs::read(resource_path("utf8.txt")).expect("read utf8.txt fixture")
}

pub fn cstring(value: &str) -> CString {
    CString::new(value).expect("CString value")
}

pub unsafe fn string_from_c(ptr: *const libc::c_char) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

pub fn current_error() -> String {
    unsafe { string_from_c(SDL_GetError()) }
}

pub fn c_ptr(bytes: &[u8]) -> *const libc::c_char {
    bytes.as_ptr().cast()
}

pub fn write_default_evdev_gamepad_fixture(path: &Path) {
    let fixture = "\
SDL_EVDEV_FIXTURE_V1
name=SDL Fake evdev Gamepad
bustype=0x03
vendor=0x054c
product=0x09cc
version=0x0001
key=0x130:1
key=0x131:0
key=0x133:0
key=0x134:0
key=0x136:0
key=0x137:0
key=0x138:0
key=0x139:0
key=0x13a:0
key=0x13b:0
key=0x13c:0
key=0x13d:0
key=0x13e:0
abs=0x00,-32768,32767,0,0,0,16384
abs=0x01,-32768,32767,0,0,0,0
abs=0x02,0,32767,0,0,0,0
abs=0x03,-32768,32767,0,0,0,0
abs=0x04,-32768,32767,0,0,0,0
abs=0x05,0,32767,0,0,0,0
abs=0x10,-1,1,0,0,0,1
abs=0x11,-1,1,0,0,0,-1
";
    std::fs::write(path, fixture).expect("write evdev fixture");
}

pub fn write_default_hidapi_fixture(path: &Path) {
    let fixture = "\
SAFE_HIDAPI_FIXTURE_V1
vendor=0x1234
product=0x5678
release=0x0001
manufacturer=Safe SDL
product_string=Fixture HID Device
serial=SAFE123
usage_page=0x0001
usage=0x0005
interface_number=2
interface_class=3
interface_subclass=0
interface_protocol=0
input=00010203
feature=10aabbcc
";
    std::fs::write(path, fixture).expect("write hidapi fixture");
}

pub fn wide_string_from_buffer(buffer: &[wchar_t]) -> String {
    let end = buffer
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(buffer.len());
    buffer[..end]
        .iter()
        .filter_map(|value| char::from_u32(*value as u32))
        .collect()
}

pub struct ScopedEnvVar {
    key: String,
    previous: Option<String>,
}

impl ScopedEnvVar {
    pub fn set(key: &str, value: &str) -> Self {
        let previous = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self {
            key: key.to_string(),
            previous,
        }
    }
}

impl Drop for ScopedEnvVar {
    fn drop(&mut self) {
        if let Some(value) = &self.previous {
            std::env::set_var(&self.key, value);
        } else {
            std::env::remove_var(&self.key);
        }
    }
}

pub struct HintGuard {
    name: &'static [u8],
}

impl HintGuard {
    pub fn set(name: &'static [u8], value: &str) -> Self {
        let value = cstring(value);
        unsafe {
            SDL_SetHint(name.as_ptr().cast(), value.as_ptr());
        }
        Self { name }
    }
}

impl Drop for HintGuard {
    fn drop(&mut self) {
        unsafe {
            SDL_ResetHint(self.name.as_ptr().cast());
        }
    }
}

pub struct VideoDriverGuard;

impl VideoDriverGuard {
    pub fn init(driver_name: &str) -> Result<Self, String> {
        let driver_name = cstring(driver_name);
        let rc = unsafe { SDL_VideoInit(driver_name.as_ptr()) };
        if rc == 0 {
            Ok(Self)
        } else {
            Err(current_error())
        }
    }
}

impl Drop for VideoDriverGuard {
    fn drop(&mut self) {
        unsafe { SDL_VideoQuit() };
    }
}

pub struct XvfbGuard {
    child: Child,
}

impl Drop for XvfbGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub struct X11DisplayGuard {
    _xvfb: Option<XvfbGuard>,
    _display: ScopedEnvVar,
    _driver: ScopedEnvVar,
}

fn spawn_xvfb() -> Option<(XvfbGuard, String)> {
    for display in 91..100 {
        let display_name = format!(":{display}");
        let child = Command::new("Xvfb")
            .arg(&display_name)
            .arg("-screen")
            .arg("0")
            .arg("1024x768x24")
            .arg("-nolisten")
            .arg("tcp")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        let Ok(child) = child else {
            return None;
        };
        thread::sleep(Duration::from_millis(500));
        return Some((XvfbGuard { child }, display_name));
    }
    None
}

pub fn acquire_x11_display() -> Option<X11DisplayGuard> {
    let existing_display = std::env::var("DISPLAY").ok();
    let xvfb = if existing_display.is_some() {
        None
    } else {
        spawn_xvfb()
    };
    let display_name = existing_display.or_else(|| xvfb.as_ref().map(|(_, name)| name.clone()))?;
    Some(X11DisplayGuard {
        _xvfb: xvfb.map(|(guard, _)| guard),
        _display: ScopedEnvVar::set("DISPLAY", &display_name),
        _driver: ScopedEnvVar::set("SDL_VIDEODRIVER", "x11"),
    })
}

pub struct SubsystemGuard {
    flags: Uint32,
}

impl SubsystemGuard {
    pub fn init(flags: Uint32) -> Self {
        let rc = unsafe { SDL_InitSubSystem(flags) };
        assert_eq!(
            rc,
            0,
            "SDL_InitSubSystem({flags:#x}) failed: {}",
            current_error()
        );
        Self { flags }
    }
}

impl Drop for SubsystemGuard {
    fn drop(&mut self) {
        unsafe { SDL_QuitSubSystem(self.flags) };
    }
}
