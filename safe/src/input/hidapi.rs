use std::ffi::{CStr, CString};
use std::fs;
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;
use std::ptr;
use std::sync::{Mutex, OnceLock};

use crate::abi::generated_types::{wchar_t, SDL_bool, SDL_hid_device, SDL_hid_device_info, Uint32};
use crate::core::error::{invalid_param_error, set_error_message};

pub const SAFE_SDL_HINT_HIDAPI_DEVICE: &[u8; 23] = b"SAFE_SDL_HIDAPI_DEVICE\0";

const FIXTURE_MAGIC: &str = "SAFE_HIDAPI_FIXTURE_V1";

#[derive(Clone)]
struct HidFixture {
    path: String,
    vendor_id: u16,
    product_id: u16,
    release_number: u16,
    manufacturer: String,
    product: String,
    serial: String,
    usage_page: u16,
    usage: u16,
    interface_number: c_int,
    interface_class: c_int,
    interface_subclass: c_int,
    interface_protocol: c_int,
    input_report: Vec<u8>,
    feature_report: Vec<u8>,
}

#[repr(C)]
struct HidInfoNode {
    info: SDL_hid_device_info,
    path: CString,
    serial: Vec<wchar_t>,
    manufacturer: Vec<wchar_t>,
    product: Vec<wchar_t>,
}

#[repr(C)]
struct HidHandle {
    fixture: HidFixture,
    nonblocking: bool,
    input_report: Vec<u8>,
    feature_report: Vec<u8>,
    output_report: Vec<u8>,
}

#[derive(Default)]
struct HidDiscoveryState {
    last_spec: Option<String>,
    counter: Uint32,
}

fn discovery_state() -> &'static Mutex<HidDiscoveryState> {
    static STATE: OnceLock<Mutex<HidDiscoveryState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(HidDiscoveryState::default()))
}

fn clear_wide_buffer(buffer: *mut wchar_t, maxlen: usize) {
    if !buffer.is_null() && maxlen > 0 {
        unsafe {
            *buffer = 0;
        }
    }
}

fn write_wide_buffer(buffer: *mut wchar_t, maxlen: usize, value: &str) -> c_int {
    clear_wide_buffer(buffer, maxlen);
    if buffer.is_null() || maxlen == 0 {
        return 0;
    }
    let encoded = wide_string(value);
    let copy_len = encoded.len().min(maxlen);
    unsafe {
        ptr::copy_nonoverlapping(encoded.as_ptr(), buffer, copy_len);
        *buffer.add(copy_len.saturating_sub(1)) = 0;
    }
    0
}

fn wide_string(value: &str) -> Vec<wchar_t> {
    value
        .chars()
        .map(|ch| ch as wchar_t)
        .chain(std::iter::once(0))
        .collect()
}

fn wide_ptr_to_string(value: *const wchar_t) -> Option<String> {
    if value.is_null() {
        return None;
    }
    let mut len = 0usize;
    unsafe {
        while *value.add(len) != 0 {
            len += 1;
        }
        Some(
            (0..len)
                .filter_map(|index| char::from_u32(*value.add(index) as u32))
                .collect(),
        )
    }
}

fn current_fixture_spec() -> Option<String> {
    super::hint_value(SAFE_SDL_HINT_HIDAPI_DEVICE)
}

fn parse_fixture_paths() -> Vec<PathBuf> {
    current_fixture_spec()
        .map(|value| {
            value
                .split(':')
                .filter(|segment| !segment.is_empty())
                .map(PathBuf::from)
                .collect()
        })
        .unwrap_or_default()
}

fn parse_hex_u16(value: &str) -> Result<u16, c_int> {
    let trimmed = value.trim();
    let trimmed = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    u16::from_str_radix(trimmed, 16)
        .map_err(|_| set_error_message("Invalid HID fixture numeric field"))
}

fn parse_hex_bytes(value: &str) -> Result<Vec<u8>, c_int> {
    let hex = value.trim();
    if hex.is_empty() {
        return Ok(Vec::new());
    }
    if hex.len() % 2 != 0 {
        return Err(set_error_message("Invalid HID fixture byte string"));
    }
    (0..hex.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&hex[index..index + 2], 16)
                .map_err(|_| set_error_message("Invalid HID fixture byte string"))
        })
        .collect()
}

fn parse_fixture(path: PathBuf) -> Result<HidFixture, c_int> {
    let contents = fs::read_to_string(&path)
        .map_err(|_| set_error_message("Unable to read HID fixture file"))?;
    let mut lines = contents.lines();
    let Some(header) = lines.next() else {
        return Err(set_error_message("Empty HID fixture file"));
    };
    if header.trim() != FIXTURE_MAGIC {
        return Err(set_error_message(
            "HID fixture file is missing SAFE_HIDAPI_FIXTURE_V1 header",
        ));
    }

    let mut fixture = HidFixture {
        path: path.to_string_lossy().into_owned(),
        vendor_id: 0,
        product_id: 0,
        release_number: 0,
        manufacturer: String::new(),
        product: String::new(),
        serial: String::new(),
        usage_page: 0,
        usage: 0,
        interface_number: 0,
        interface_class: 0,
        interface_subclass: 0,
        interface_protocol: 0,
        input_report: Vec::new(),
        feature_report: Vec::new(),
    };

    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(set_error_message("Invalid HID fixture line"));
        };
        match key {
            "vendor" => fixture.vendor_id = parse_hex_u16(value)?,
            "product" => fixture.product_id = parse_hex_u16(value)?,
            "release" => fixture.release_number = parse_hex_u16(value)?,
            "manufacturer" => fixture.manufacturer = value.to_string(),
            "product_string" => fixture.product = value.to_string(),
            "serial" => fixture.serial = value.to_string(),
            "usage_page" => fixture.usage_page = parse_hex_u16(value)?,
            "usage" => fixture.usage = parse_hex_u16(value)?,
            "interface_number" => {
                fixture.interface_number = value
                    .parse()
                    .map_err(|_| set_error_message("Invalid HID interface number"))?
            }
            "interface_class" => {
                fixture.interface_class = value
                    .parse()
                    .map_err(|_| set_error_message("Invalid HID interface class"))?
            }
            "interface_subclass" => {
                fixture.interface_subclass = value
                    .parse()
                    .map_err(|_| set_error_message("Invalid HID interface subclass"))?
            }
            "interface_protocol" => {
                fixture.interface_protocol = value
                    .parse()
                    .map_err(|_| set_error_message("Invalid HID interface protocol"))?
            }
            "input" => fixture.input_report = parse_hex_bytes(value)?,
            "feature" => fixture.feature_report = parse_hex_bytes(value)?,
            _ => return Err(set_error_message("Unsupported HID fixture field")),
        }
    }

    Ok(fixture)
}

fn load_fixtures() -> Result<Vec<HidFixture>, c_int> {
    parse_fixture_paths()
        .into_iter()
        .map(parse_fixture)
        .collect()
}

fn handle_mut<'a>(dev: *mut SDL_hid_device) -> Option<&'a mut HidHandle> {
    if dev.is_null() {
        None
    } else {
        Some(unsafe { &mut *(dev as *mut HidHandle) })
    }
}

fn filtered_fixtures(vendor_id: u16, product_id: u16) -> Result<Vec<HidFixture>, c_int> {
    Ok(load_fixtures()?
        .into_iter()
        .filter(|fixture| {
            (vendor_id == 0 || fixture.vendor_id == vendor_id)
                && (product_id == 0 || fixture.product_id == product_id)
        })
        .collect())
}

fn open_fixture(fixture: HidFixture) -> *mut SDL_hid_device {
    Box::into_raw(Box::new(HidHandle {
        input_report: fixture.input_report.clone(),
        feature_report: fixture.feature_report.clone(),
        output_report: Vec::new(),
        fixture,
        nonblocking: false,
    })) as *mut SDL_hid_device
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_init() -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_exit() -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_device_change_count() -> Uint32 {
    let mut state = match discovery_state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    let current = current_fixture_spec().unwrap_or_default();
    if state.last_spec.as_deref() != Some(current.as_str()) {
        state.last_spec = Some(current);
        state.counter = state.counter.saturating_add(1).max(1);
    }
    state.counter
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_enumerate(
    vendor_id: libc::c_ushort,
    product_id: libc::c_ushort,
) -> *mut SDL_hid_device_info {
    let fixtures = match filtered_fixtures(vendor_id, product_id) {
        Ok(fixtures) => fixtures,
        Err(_) => return ptr::null_mut(),
    };
    let mut head: *mut SDL_hid_device_info = ptr::null_mut();
    let mut previous: *mut SDL_hid_device_info = ptr::null_mut();
    for fixture in fixtures {
        let path = CString::new(fixture.path.clone()).unwrap_or_default();
        let mut node = Box::new(HidInfoNode {
            info: SDL_hid_device_info {
                path: ptr::null_mut(),
                vendor_id: fixture.vendor_id,
                product_id: fixture.product_id,
                serial_number: ptr::null_mut(),
                release_number: fixture.release_number,
                manufacturer_string: ptr::null_mut(),
                product_string: ptr::null_mut(),
                usage_page: fixture.usage_page,
                usage: fixture.usage,
                interface_number: fixture.interface_number,
                interface_class: fixture.interface_class,
                interface_subclass: fixture.interface_subclass,
                interface_protocol: fixture.interface_protocol,
                next: ptr::null_mut(),
            },
            path,
            serial: wide_string(&fixture.serial),
            manufacturer: wide_string(&fixture.manufacturer),
            product: wide_string(&fixture.product),
        });
        node.info.path = node.path.as_ptr() as *mut c_char;
        node.info.serial_number = node.serial.as_mut_ptr();
        node.info.manufacturer_string = node.manufacturer.as_mut_ptr();
        node.info.product_string = node.product.as_mut_ptr();
        let node_ptr = Box::into_raw(node);
        let info_ptr = unsafe { &mut (*node_ptr).info as *mut SDL_hid_device_info };
        if previous.is_null() {
            head = info_ptr;
        } else {
            (*previous).next = info_ptr;
        }
        previous = info_ptr;
    }
    head
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_free_enumeration(mut devs: *mut SDL_hid_device_info) {
    while !devs.is_null() {
        let next = (*devs).next;
        drop(Box::from_raw(devs as *mut HidInfoNode));
        devs = next;
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_open(
    vendor_id: libc::c_ushort,
    product_id: libc::c_ushort,
    serial_number: *const wchar_t,
) -> *mut SDL_hid_device {
    let serial = wide_ptr_to_string(serial_number);
    let fixtures = match filtered_fixtures(vendor_id, product_id) {
        Ok(fixtures) => fixtures,
        Err(_) => return ptr::null_mut(),
    };
    fixtures
        .into_iter()
        .find(|fixture| {
            serial
                .as_ref()
                .map(|serial| serial == &fixture.serial)
                .unwrap_or(true)
        })
        .map(open_fixture)
        .unwrap_or_else(|| {
            let _ = set_error_message("HID device not found");
            ptr::null_mut()
        })
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_open_path(
    path: *const c_char,
    _exclusive: c_int,
) -> *mut SDL_hid_device {
    if path.is_null() {
        let _ = invalid_param_error("path");
        return ptr::null_mut();
    }
    let path = CStr::from_ptr(path).to_string_lossy().into_owned();
    let fixtures = match load_fixtures() {
        Ok(fixtures) => fixtures,
        Err(_) => return ptr::null_mut(),
    };
    fixtures
        .into_iter()
        .find(|fixture| fixture.path == path)
        .map(open_fixture)
        .unwrap_or_else(|| {
            let _ = set_error_message("HID device not found");
            ptr::null_mut()
        })
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_write(
    dev: *mut SDL_hid_device,
    data: *const u8,
    length: usize,
) -> c_int {
    let Some(handle) = handle_mut(dev) else {
        return invalid_param_error("dev");
    };
    if data.is_null() && length != 0 {
        return invalid_param_error("data");
    }
    handle.output_report = if data.is_null() || length == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(data, length).to_vec()
    };
    length as c_int
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_read_timeout(
    dev: *mut SDL_hid_device,
    data: *mut u8,
    length: usize,
    _milliseconds: c_int,
) -> c_int {
    let Some(handle) = handle_mut(dev) else {
        return invalid_param_error("dev");
    };
    if data.is_null() && length != 0 {
        return invalid_param_error("data");
    }
    if handle.input_report.is_empty() {
        return 0;
    }
    let copy_len = handle.input_report.len().min(length);
    if copy_len > 0 {
        ptr::copy_nonoverlapping(handle.input_report.as_ptr(), data, copy_len);
    }
    handle.input_report.clear();
    copy_len as c_int
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_read(
    dev: *mut SDL_hid_device,
    data: *mut u8,
    length: usize,
) -> c_int {
    let milliseconds = {
        let Some(handle) = handle_mut(dev) else {
            return invalid_param_error("dev");
        };
        if handle.nonblocking {
            0
        } else {
            -1
        }
    };
    SDL_hid_read_timeout(dev, data, length, milliseconds)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_set_nonblocking(
    dev: *mut SDL_hid_device,
    nonblock: c_int,
) -> c_int {
    let Some(handle) = handle_mut(dev) else {
        return invalid_param_error("dev");
    };
    handle.nonblocking = nonblock != 0;
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_send_feature_report(
    dev: *mut SDL_hid_device,
    data: *const u8,
    length: usize,
) -> c_int {
    let Some(handle) = handle_mut(dev) else {
        return invalid_param_error("dev");
    };
    if data.is_null() && length != 0 {
        return invalid_param_error("data");
    }
    handle.feature_report = if data.is_null() || length == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(data, length).to_vec()
    };
    length as c_int
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_get_feature_report(
    dev: *mut SDL_hid_device,
    data: *mut u8,
    length: usize,
) -> c_int {
    let Some(handle) = handle_mut(dev) else {
        return invalid_param_error("dev");
    };
    if data.is_null() && length != 0 {
        return invalid_param_error("data");
    }
    let copy_len = handle.feature_report.len().min(length);
    if copy_len > 0 {
        ptr::copy_nonoverlapping(handle.feature_report.as_ptr(), data, copy_len);
    }
    copy_len as c_int
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_close(dev: *mut SDL_hid_device) {
    if dev.is_null() {
        return;
    }
    drop(Box::from_raw(dev as *mut HidHandle));
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_get_manufacturer_string(
    dev: *mut SDL_hid_device,
    string: *mut wchar_t,
    maxlen: usize,
) -> c_int {
    let Some(handle) = handle_mut(dev) else {
        clear_wide_buffer(string, maxlen);
        return invalid_param_error("dev");
    };
    write_wide_buffer(string, maxlen, &handle.fixture.manufacturer)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_get_product_string(
    dev: *mut SDL_hid_device,
    string: *mut wchar_t,
    maxlen: usize,
) -> c_int {
    let Some(handle) = handle_mut(dev) else {
        clear_wide_buffer(string, maxlen);
        return invalid_param_error("dev");
    };
    write_wide_buffer(string, maxlen, &handle.fixture.product)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_get_serial_number_string(
    dev: *mut SDL_hid_device,
    string: *mut wchar_t,
    maxlen: usize,
) -> c_int {
    let Some(handle) = handle_mut(dev) else {
        clear_wide_buffer(string, maxlen);
        return invalid_param_error("dev");
    };
    write_wide_buffer(string, maxlen, &handle.fixture.serial)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_get_indexed_string(
    dev: *mut SDL_hid_device,
    string_index: c_int,
    string: *mut wchar_t,
    maxlen: usize,
) -> c_int {
    let Some(handle) = handle_mut(dev) else {
        clear_wide_buffer(string, maxlen);
        return invalid_param_error("dev");
    };
    let value = match string_index {
        0 => &handle.fixture.manufacturer,
        1 => &handle.fixture.product,
        2 => &handle.fixture.serial,
        _ => return set_error_message("Indexed HID string is not available"),
    };
    write_wide_buffer(string, maxlen, value)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_hid_ble_scan(_active: SDL_bool) {}
