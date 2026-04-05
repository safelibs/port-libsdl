use std::collections::BTreeMap;
use std::ffi::CString;
use std::fs;
use std::io::{self, ErrorKind};
use std::mem::size_of;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr;

use crate::abi::generated_types::{
    SDL_JoystickType, SDL_JoystickType_SDL_JOYSTICK_TYPE_GAMECONTROLLER,
    SDL_JoystickType_SDL_JOYSTICK_TYPE_UNKNOWN, Sint16, Uint8, SDL_HAT_CENTERED, SDL_HAT_DOWN,
    SDL_HAT_LEFT, SDL_HAT_LEFTDOWN, SDL_HAT_LEFTUP, SDL_HAT_RIGHT, SDL_HAT_RIGHTDOWN,
    SDL_HAT_RIGHTUP, SDL_HAT_UP, SDL_PRESSED, SDL_RELEASED,
};
use crate::input::DeviceState;

const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;

const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
const ABS_Z: u16 = 0x02;
const ABS_RX: u16 = 0x03;
const ABS_RY: u16 = 0x04;
const ABS_RZ: u16 = 0x05;
const ABS_HAT0X: u16 = 0x10;
const ABS_HAT0Y: u16 = 0x11;

const BTN_A: u16 = 0x130;
const BTN_B: u16 = 0x131;
const BTN_X: u16 = 0x133;
const BTN_Y: u16 = 0x134;
const BTN_TL: u16 = 0x136;
const BTN_TR: u16 = 0x137;
const BTN_TL2: u16 = 0x138;
const BTN_TR2: u16 = 0x139;
const BTN_SELECT: u16 = 0x13a;
const BTN_START: u16 = 0x13b;
const BTN_MODE: u16 = 0x13c;
const BTN_THUMBL: u16 = 0x13d;
const BTN_THUMBR: u16 = 0x13e;

const EV_MAX: usize = 0x1f;
const KEY_MAX: usize = 0x2ff;
const ABS_MAX: usize = 0x3f;

const FIXTURE_MAGIC: &str = "SDL_EVDEV_FIXTURE_V1";

const BITS_PER_LONG: usize = size_of::<libc::c_ulong>() * 8;
const fn nbits(x: usize) -> usize {
    ((x - 1) / BITS_PER_LONG) + 1
}

const AXIS_CODES: [u16; 6] = [ABS_X, ABS_Y, ABS_Z, ABS_RX, ABS_RY, ABS_RZ];
const BUTTON_CODES: [u16; 13] = [
    BTN_A, BTN_B, BTN_X, BTN_Y, BTN_TL, BTN_TR, BTN_TL2, BTN_TR2, BTN_SELECT, BTN_START, BTN_MODE,
    BTN_THUMBL, BTN_THUMBR,
];

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxInputId {
    bustype: u16,
    vendor: u16,
    product: u16,
    version: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxInputAbsInfo {
    value: i32,
    minimum: i32,
    maximum: i32,
    fuzz: i32,
    flat: i32,
    resolution: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct LinuxInputEvent {
    _time: libc::timeval,
    type_: u16,
    code: u16,
    value: i32,
}

struct HatState {
    x_code: u16,
    y_code: u16,
    x_value: i32,
    y_value: i32,
}

pub(crate) struct RealDevice {
    fd: OwnedFd,
    axis_codes: Vec<u16>,
    button_codes: Vec<u16>,
    hat: Option<HatState>,
}

pub(crate) enum EvdevSource {
    Real(RealDevice),
    Fixture,
}

pub(crate) struct ProbedDevice {
    pub name: String,
    pub bus: u16,
    pub vendor: u16,
    pub product: u16,
    pub version: u16,
    pub joystick_type: SDL_JoystickType,
    pub state: DeviceState,
    pub source: EvdevSource,
}

struct DeviceLayout {
    axis_codes: Vec<u16>,
    button_codes: Vec<u16>,
    hat: Option<HatState>,
}

#[derive(Default)]
struct FixtureDescription {
    name: Option<String>,
    bustype: Option<u16>,
    vendor: Option<u16>,
    product: Option<u16>,
    version: Option<u16>,
    buttons: BTreeMap<u16, Uint8>,
    abs: BTreeMap<u16, LinuxInputAbsInfo>,
}

pub fn parse_device_hint(value: &str) -> Vec<PathBuf> {
    value
        .split(':')
        .filter(|segment| !segment.is_empty())
        .map(PathBuf::from)
        .collect()
}

pub fn expand_device_hint_paths(paths: Vec<PathBuf>) -> io::Result<Vec<PathBuf>> {
    let mut expanded = Vec::new();
    for path in paths {
        if path.is_dir() {
            expanded.extend(crate::input::linux::udev::discover_device_nodes(&path)?);
        } else {
            expanded.push(path);
        }
    }
    Ok(expanded)
}

pub(crate) fn probe_device(path: &Path) -> io::Result<ProbedDevice> {
    match probe_real_device(path) {
        Ok(device) => Ok(device),
        Err(error)
            if matches!(
                error.raw_os_error(),
                Some(libc::ENOTTY | libc::EINVAL | libc::ENOSYS)
            ) =>
        {
            probe_fixture_file(path)
        }
        Err(error) => Err(error),
    }
}

pub(crate) fn poll_device(source: &mut EvdevSource, state: &mut DeviceState) -> io::Result<()> {
    match source {
        EvdevSource::Real(device) => poll_real_device(device, state),
        EvdevSource::Fixture => Ok(()),
    }
}

fn probe_real_device(path: &Path) -> io::Result<ProbedDevice> {
    let c_path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "path contains interior NUL"))?;
    let fd = unsafe {
        libc::open(
            c_path.as_ptr(),
            libc::O_RDONLY | libc::O_NONBLOCK | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    let fd = unsafe { OwnedFd::from_raw_fd(fd) };
    let raw_fd = fd.as_raw_fd();

    let mut input_id = LinuxInputId::default();
    ioctl_struct(raw_fd, eviocgid(), &mut input_id)?;

    let mut name = vec![0u8; 128];
    ioctl_bytes(raw_fd, eviocgname(name.len()), &mut name)?;
    let nul = name
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(name.len());
    let name = String::from_utf8_lossy(&name[..nul]).into_owned();

    let mut ev_bits = vec![0 as libc::c_ulong; nbits(EV_MAX + 1)];
    ioctl_bits(
        raw_fd,
        eviocgbit(0, ev_bits.len() * size_of::<libc::c_ulong>()),
        &mut ev_bits,
    )?;
    let mut key_bits = vec![0 as libc::c_ulong; nbits(KEY_MAX + 1)];
    ioctl_bits(
        raw_fd,
        eviocgbit(EV_KEY as usize, key_bits.len() * size_of::<libc::c_ulong>()),
        &mut key_bits,
    )?;
    let mut abs_bits = vec![0 as libc::c_ulong; nbits(ABS_MAX + 1)];
    ioctl_bits(
        raw_fd,
        eviocgbit(EV_ABS as usize, abs_bits.len() * size_of::<libc::c_ulong>()),
        &mut abs_bits,
    )?;
    let mut key_state = vec![0 as libc::c_ulong; nbits(KEY_MAX + 1)];
    ioctl_bits(
        raw_fd,
        eviocgkey(key_state.len() * size_of::<libc::c_ulong>()),
        &mut key_state,
    )?;

    if !test_bit(&ev_bits, EV_KEY as usize) && !test_bit(&ev_bits, EV_ABS as usize) {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "device exposes neither EV_KEY nor EV_ABS",
        ));
    }

    let mut fixture = FixtureDescription {
        name: Some(name),
        bustype: Some(input_id.bustype),
        vendor: Some(input_id.vendor),
        product: Some(input_id.product),
        version: Some(input_id.version),
        ..FixtureDescription::default()
    };

    for code in BUTTON_CODES {
        if test_bit(&key_bits, code as usize) {
            fixture.buttons.insert(
                code,
                if test_bit(&key_state, code as usize) {
                    SDL_PRESSED as Uint8
                } else {
                    SDL_RELEASED as Uint8
                },
            );
        }
    }

    for code in AXIS_CODES.into_iter().chain([ABS_HAT0X, ABS_HAT0Y]) {
        if test_bit(&abs_bits, code as usize) {
            let mut absinfo = LinuxInputAbsInfo::default();
            ioctl_struct(raw_fd, eviocgabs(code as usize), &mut absinfo)?;
            fixture.abs.insert(code, absinfo);
        }
    }

    let (state, layout) = build_state(&fixture);
    Ok(ProbedDevice {
        name: fixture.name.unwrap_or_default(),
        bus: fixture.bustype.unwrap_or_default(),
        vendor: fixture.vendor.unwrap_or_default(),
        product: fixture.product.unwrap_or_default(),
        version: fixture.version.unwrap_or_default(),
        joystick_type: classify_device(
            layout.button_codes.len(),
            layout.axis_codes.len(),
            layout.hat.is_some(),
        ),
        state,
        source: EvdevSource::Real(RealDevice {
            fd,
            axis_codes: layout.axis_codes,
            button_codes: layout.button_codes,
            hat: layout.hat,
        }),
    })
}

fn probe_fixture_file(path: &Path) -> io::Result<ProbedDevice> {
    let contents = fs::read_to_string(path)?;
    let fixture = if contents.trim().is_empty() && supports_builtin_fixture(path) {
        builtin_testevdev_fixture()
    } else {
        parse_fixture_description(&contents)?
    };
    let (state, layout) = build_state(&fixture);
    Ok(ProbedDevice {
        name: fixture
            .name
            .unwrap_or_else(|| "SDL Fixture evdev Gamepad".to_string()),
        bus: fixture.bustype.unwrap_or(0x03),
        vendor: fixture.vendor.unwrap_or_default(),
        product: fixture.product.unwrap_or_default(),
        version: fixture.version.unwrap_or_default(),
        joystick_type: classify_device(
            layout.button_codes.len(),
            layout.axis_codes.len(),
            layout.hat.is_some(),
        ),
        state,
        source: EvdevSource::Fixture,
    })
}

fn supports_builtin_fixture(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.contains("event") || name.starts_with("js"))
        .unwrap_or(false)
}

fn builtin_testevdev_fixture() -> FixtureDescription {
    let mut fixture = FixtureDescription {
        name: Some("SDL Fake evdev Gamepad".to_string()),
        bustype: Some(0x03),
        vendor: Some(0x054c),
        product: Some(0x09cc),
        version: Some(0x0001),
        ..FixtureDescription::default()
    };

    for code in BUTTON_CODES {
        fixture.buttons.insert(
            code,
            if code == BTN_A {
                SDL_PRESSED as Uint8
            } else {
                SDL_RELEASED as Uint8
            },
        );
    }

    fixture.abs.insert(
        ABS_X,
        LinuxInputAbsInfo {
            minimum: -32768,
            maximum: 32767,
            value: 16384,
            ..LinuxInputAbsInfo::default()
        },
    );
    fixture.abs.insert(
        ABS_Y,
        LinuxInputAbsInfo {
            minimum: -32768,
            maximum: 32767,
            ..LinuxInputAbsInfo::default()
        },
    );
    fixture.abs.insert(
        ABS_Z,
        LinuxInputAbsInfo {
            maximum: 32767,
            ..LinuxInputAbsInfo::default()
        },
    );
    fixture.abs.insert(
        ABS_RX,
        LinuxInputAbsInfo {
            minimum: -32768,
            maximum: 32767,
            ..LinuxInputAbsInfo::default()
        },
    );
    fixture.abs.insert(
        ABS_RY,
        LinuxInputAbsInfo {
            minimum: -32768,
            maximum: 32767,
            ..LinuxInputAbsInfo::default()
        },
    );
    fixture.abs.insert(
        ABS_RZ,
        LinuxInputAbsInfo {
            maximum: 32767,
            ..LinuxInputAbsInfo::default()
        },
    );
    fixture.abs.insert(
        ABS_HAT0X,
        LinuxInputAbsInfo {
            minimum: -1,
            maximum: 1,
            value: 1,
            ..LinuxInputAbsInfo::default()
        },
    );
    fixture.abs.insert(
        ABS_HAT0Y,
        LinuxInputAbsInfo {
            minimum: -1,
            maximum: 1,
            value: -1,
            ..LinuxInputAbsInfo::default()
        },
    );
    fixture
}

fn parse_fixture_description(contents: &str) -> io::Result<FixtureDescription> {
    let mut lines = contents.lines();
    let Some(header) = lines.next() else {
        return Err(io::Error::new(ErrorKind::InvalidData, "empty fixture file"));
    };
    if header.trim() != FIXTURE_MAGIC {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "fixture file is missing SDL_EVDEV_FIXTURE_V1 header",
        ));
    }

    let mut fixture = FixtureDescription::default();
    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                format!("fixture line missing '=': {line}"),
            ));
        };
        match key {
            "name" => fixture.name = Some(value.to_string()),
            "bustype" => fixture.bustype = Some(parse_u16(value)?),
            "vendor" => fixture.vendor = Some(parse_u16(value)?),
            "product" => fixture.product = Some(parse_u16(value)?),
            "version" => fixture.version = Some(parse_u16(value)?),
            "key" => {
                let Some((code, pressed)) = value.split_once(':') else {
                    return Err(io::Error::new(
                        ErrorKind::InvalidData,
                        format!("fixture key entry missing ':' in {value}"),
                    ));
                };
                fixture.buttons.insert(
                    parse_u16(code)?,
                    if parse_i32(pressed)? != 0 {
                        SDL_PRESSED as Uint8
                    } else {
                        SDL_RELEASED as Uint8
                    },
                );
            }
            "abs" => {
                let parts = value.split(',').collect::<Vec<_>>();
                if parts.len() != 7 {
                    return Err(io::Error::new(
                        ErrorKind::InvalidData,
                        format!("fixture abs entry needs 7 fields: {value}"),
                    ));
                }
                fixture.abs.insert(
                    parse_u16(parts[0])?,
                    LinuxInputAbsInfo {
                        minimum: parse_i32(parts[1])?,
                        maximum: parse_i32(parts[2])?,
                        fuzz: parse_i32(parts[3])?,
                        flat: parse_i32(parts[4])?,
                        resolution: parse_i32(parts[5])?,
                        value: parse_i32(parts[6])?,
                    },
                );
            }
            other => {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    format!("unsupported fixture key {other}"),
                ));
            }
        }
    }

    Ok(fixture)
}

fn build_state(fixture: &FixtureDescription) -> (DeviceState, DeviceLayout) {
    let axis_codes = AXIS_CODES
        .into_iter()
        .filter(|code| fixture.abs.contains_key(code))
        .collect::<Vec<_>>();
    let button_codes = BUTTON_CODES
        .into_iter()
        .filter(|code| fixture.buttons.contains_key(code))
        .collect::<Vec<_>>();
    let hat_codes = if fixture.abs.contains_key(&ABS_HAT0X) || fixture.abs.contains_key(&ABS_HAT0Y)
    {
        Some(HatState {
            x_code: ABS_HAT0X,
            y_code: ABS_HAT0Y,
            x_value: fixture
                .abs
                .get(&ABS_HAT0X)
                .map(|info| info.value)
                .unwrap_or_default(),
            y_value: fixture
                .abs
                .get(&ABS_HAT0Y)
                .map(|info| info.value)
                .unwrap_or_default(),
        })
    } else {
        None
    };

    let mut state = DeviceState::new(
        axis_codes.len(),
        button_codes.len(),
        usize::from(hat_codes.is_some()),
    );
    for (index, code) in axis_codes.iter().enumerate() {
        if let Some(info) = fixture.abs.get(code) {
            state.pending_axes[index] = clamp_i16(info.value);
        }
    }
    for (index, code) in button_codes.iter().enumerate() {
        state.pending_buttons[index] = fixture
            .buttons
            .get(code)
            .copied()
            .unwrap_or(SDL_RELEASED as Uint8);
    }
    if let Some(hat) = hat_codes.as_ref() {
        state.pending_hats[0] = hat_from_xy(hat.x_value, hat.y_value);
    }
    state.apply_pending();
    (
        state,
        DeviceLayout {
            axis_codes,
            button_codes,
            hat: hat_codes,
        },
    )
}

fn classify_device(buttons: usize, axes: usize, has_hat: bool) -> SDL_JoystickType {
    if buttons >= BUTTON_CODES.len() && axes >= AXIS_CODES.len() && has_hat {
        SDL_JoystickType_SDL_JOYSTICK_TYPE_GAMECONTROLLER
    } else {
        SDL_JoystickType_SDL_JOYSTICK_TYPE_UNKNOWN
    }
}

fn poll_real_device(device: &mut RealDevice, state: &mut DeviceState) -> io::Result<()> {
    let event_size = size_of::<LinuxInputEvent>();
    let mut bytes = [0u8; 32 * size_of::<LinuxInputEvent>()];

    loop {
        let read_len = unsafe {
            libc::read(
                device.fd.as_raw_fd(),
                bytes.as_mut_ptr().cast(),
                bytes.len(),
            )
        };
        if read_len < 0 {
            let error = io::Error::last_os_error();
            if matches!(error.raw_os_error(), Some(libc::EAGAIN | libc::EINTR)) {
                return Ok(());
            }
            return Err(error);
        }
        if read_len == 0 {
            return Ok(());
        }

        let read_len = read_len as usize;
        for chunk in bytes[..read_len].chunks_exact(event_size) {
            let event = unsafe { ptr::read_unaligned(chunk.as_ptr().cast::<LinuxInputEvent>()) };
            apply_event(device, state, event);
        }

        if read_len < bytes.len() {
            return Ok(());
        }
    }
}

fn apply_event(device: &mut RealDevice, state: &mut DeviceState, event: LinuxInputEvent) {
    match event.type_ {
        EV_KEY => {
            if let Some(index) = device
                .button_codes
                .iter()
                .position(|code| *code == event.code)
            {
                state.pending_buttons[index] = if event.value != 0 {
                    SDL_PRESSED as Uint8
                } else {
                    SDL_RELEASED as Uint8
                };
            }
        }
        EV_ABS => {
            if let Some(index) = device
                .axis_codes
                .iter()
                .position(|code| *code == event.code)
            {
                state.pending_axes[index] = clamp_i16(event.value);
                return;
            }
            if let Some(hat_state) = device.hat.as_mut() {
                if event.code == hat_state.x_code || event.code == hat_state.y_code {
                    if event.code == hat_state.x_code {
                        hat_state.x_value = event.value;
                    } else {
                        hat_state.y_value = event.value;
                    }
                    if let Some(hat) = state.pending_hats.get_mut(0) {
                        *hat = hat_from_xy(hat_state.x_value, hat_state.y_value);
                    }
                }
            }
        }
        _ => {}
    }
}

fn clamp_i16(value: i32) -> Sint16 {
    value.clamp(Sint16::MIN as i32, Sint16::MAX as i32) as Sint16
}

fn hat_from_xy(x: i32, y: i32) -> Uint8 {
    match (x.signum(), y.signum()) {
        (0, -1) => SDL_HAT_UP as Uint8,
        (0, 1) => SDL_HAT_DOWN as Uint8,
        (-1, 0) => SDL_HAT_LEFT as Uint8,
        (1, 0) => SDL_HAT_RIGHT as Uint8,
        (1, -1) => SDL_HAT_RIGHTUP as Uint8,
        (1, 1) => SDL_HAT_RIGHTDOWN as Uint8,
        (-1, -1) => SDL_HAT_LEFTUP as Uint8,
        (-1, 1) => SDL_HAT_LEFTDOWN as Uint8,
        _ => SDL_HAT_CENTERED as Uint8,
    }
}

fn parse_u16(value: &str) -> io::Result<u16> {
    let value = value.trim();
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u16::from_str_radix(hex, 16)
    } else {
        value.parse::<u16>()
    }
    .map_err(|_| io::Error::new(ErrorKind::InvalidData, format!("invalid u16 value {value}")))
}

fn parse_i32(value: &str) -> io::Result<i32> {
    let value = value.trim();
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        i32::from_str_radix(hex, 16)
    } else {
        value.parse::<i32>()
    }
    .map_err(|_| io::Error::new(ErrorKind::InvalidData, format!("invalid i32 value {value}")))
}

fn test_bit(bits: &[libc::c_ulong], bit: usize) -> bool {
    bits.get(bit / BITS_PER_LONG)
        .map(|value| (value & ((1 as libc::c_ulong) << (bit % BITS_PER_LONG))) != 0)
        .unwrap_or(false)
}

fn ioctl_struct<T>(fd: libc::c_int, request: libc::c_ulong, value: &mut T) -> io::Result<()> {
    let rc = unsafe { libc::ioctl(fd, request, value as *mut T) };
    if rc < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn ioctl_bytes(fd: libc::c_int, request: libc::c_ulong, value: &mut [u8]) -> io::Result<()> {
    let rc = unsafe { libc::ioctl(fd, request, value.as_mut_ptr()) };
    if rc < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn ioctl_bits(
    fd: libc::c_int,
    request: libc::c_ulong,
    value: &mut [libc::c_ulong],
) -> io::Result<()> {
    let rc = unsafe { libc::ioctl(fd, request, value.as_mut_ptr()) };
    if rc < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

const IOC_NRBITS: usize = 8;
const IOC_TYPEBITS: usize = 8;
const IOC_SIZEBITS: usize = 14;

const IOC_NRSHIFT: usize = 0;
const IOC_TYPESHIFT: usize = IOC_NRSHIFT + IOC_NRBITS;
const IOC_SIZESHIFT: usize = IOC_TYPESHIFT + IOC_TYPEBITS;
const IOC_DIRSHIFT: usize = IOC_SIZESHIFT + IOC_SIZEBITS;

const IOC_READ: usize = 2;

const fn ioc(dir: usize, type_: usize, nr: usize, size: usize) -> libc::c_ulong {
    ((dir << IOC_DIRSHIFT)
        | (type_ << IOC_TYPESHIFT)
        | (nr << IOC_NRSHIFT)
        | (size << IOC_SIZESHIFT)) as libc::c_ulong
}

const fn ior(type_: usize, nr: usize, size: usize) -> libc::c_ulong {
    ioc(IOC_READ, type_, nr, size)
}

const fn eviocgid() -> libc::c_ulong {
    ior('E' as usize, 0x02, size_of::<LinuxInputId>())
}

const fn eviocgname(len: usize) -> libc::c_ulong {
    ioc(IOC_READ, 'E' as usize, 0x06, len)
}

const fn eviocgbit(ev: usize, len: usize) -> libc::c_ulong {
    ioc(IOC_READ, 'E' as usize, 0x20 + ev, len)
}

const fn eviocgkey(len: usize) -> libc::c_ulong {
    ioc(IOC_READ, 'E' as usize, 0x18, len)
}

const fn eviocgabs(abs: usize) -> libc::c_ulong {
    ior('E' as usize, 0x40 + abs, size_of::<LinuxInputAbsInfo>())
}
