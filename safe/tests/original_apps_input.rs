#![allow(non_upper_case_globals)]
#![allow(clippy::all)]

#[path = "common/testutils.rs"]
mod testutils;

use std::ffi::CStr;
use std::os::raw::{c_int, c_void};
use std::ptr;
use std::sync::atomic::{AtomicI32, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use safe_sdl::abi::generated_types::{
    wchar_t, SDL_GameControllerAxis_SDL_CONTROLLER_AXIS_LEFTX,
    SDL_GameControllerAxis_SDL_CONTROLLER_AXIS_MAX,
    SDL_GameControllerBindType_SDL_CONTROLLER_BINDTYPE_AXIS,
    SDL_GameControllerBindType_SDL_CONTROLLER_BINDTYPE_BUTTON,
    SDL_GameControllerBindType_SDL_CONTROLLER_BINDTYPE_HAT,
    SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_A,
    SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_DPAD_LEFT,
    SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_DPAD_RIGHT,
    SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_DPAD_UP,
    SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_MAX,
    SDL_GameControllerType_SDL_CONTROLLER_TYPE_PS4,
    SDL_GameControllerType_SDL_CONTROLLER_TYPE_VIRTUAL, SDL_HapticEffect, SDL_HapticLeftRight,
    SDL_JoystickType_SDL_JOYSTICK_TYPE_GAMECONTROLLER, SDL_SensorType_SDL_SENSOR_ACCEL,
    SDL_SensorType_SDL_SENSOR_GYRO, SDL_VirtualJoystickDesc, SDL_bool_SDL_FALSE, SDL_bool_SDL_TRUE,
    Uint16, SDL_HAPTIC_LEFTRIGHT, SDL_HAT_RIGHTUP, SDL_HINT_JOYSTICK_DEVICE,
    SDL_INIT_GAMECONTROLLER, SDL_INIT_HAPTIC, SDL_INIT_JOYSTICK, SDL_INIT_SENSOR, SDL_PRESSED,
    SDL_VIRTUAL_JOYSTICK_DESC_VERSION,
};
use safe_sdl::core::memory::SDL_free;
use safe_sdl::core::rwops::SDL_RWFromConstMem;
use safe_sdl::input::gamecontroller::{
    SDL_GameControllerAddMappingsFromRW, SDL_GameControllerClose,
    SDL_GameControllerFromPlayerIndex, SDL_GameControllerGetAxis,
    SDL_GameControllerGetAxisFromString, SDL_GameControllerGetBindForAxis,
    SDL_GameControllerGetBindForButton, SDL_GameControllerGetButton,
    SDL_GameControllerGetButtonFromString, SDL_GameControllerGetNumTouchpadFingers,
    SDL_GameControllerGetNumTouchpads, SDL_GameControllerGetSensorData,
    SDL_GameControllerGetSensorDataRate, SDL_GameControllerGetSensorDataWithTimestamp,
    SDL_GameControllerGetTouchpadFinger, SDL_GameControllerGetType, SDL_GameControllerHasAxis,
    SDL_GameControllerHasButton, SDL_GameControllerHasLED, SDL_GameControllerHasRumble,
    SDL_GameControllerHasRumbleTriggers, SDL_GameControllerHasSensor,
    SDL_GameControllerIsSensorEnabled, SDL_GameControllerMappingForDeviceIndex,
    SDL_GameControllerMappingForGUID, SDL_GameControllerMappingForIndex,
    SDL_GameControllerNameForIndex, SDL_GameControllerNumMappings, SDL_GameControllerOpen,
    SDL_GameControllerPathForIndex, SDL_GameControllerRumble, SDL_GameControllerRumbleTriggers,
    SDL_GameControllerSendEffect, SDL_GameControllerSetLED, SDL_GameControllerSetPlayerIndex,
    SDL_GameControllerSetSensorEnabled, SDL_GameControllerTypeForIndex, SDL_GameControllerUpdate,
    SDL_IsGameController,
};
use safe_sdl::input::haptic::{
    SDL_HapticClose, SDL_HapticGetEffectStatus, SDL_HapticIndex, SDL_HapticName,
    SDL_HapticNewEffect, SDL_HapticNumAxes, SDL_HapticNumEffects, SDL_HapticNumEffectsPlaying,
    SDL_HapticOpen, SDL_HapticOpenFromJoystick, SDL_HapticOpenFromMouse, SDL_HapticPause,
    SDL_HapticQuery, SDL_HapticRumbleInit, SDL_HapticRumblePlay, SDL_HapticRumbleStop,
    SDL_HapticRumbleSupported, SDL_HapticRunEffect, SDL_HapticSetAutocenter, SDL_HapticSetGain,
    SDL_HapticStopAll, SDL_HapticStopEffect, SDL_HapticUnpause, SDL_JoystickIsHaptic,
    SDL_MouseIsHaptic, SDL_NumHaptics,
};
use safe_sdl::input::hidapi::{
    SDL_hid_ble_scan, SDL_hid_close, SDL_hid_device_change_count, SDL_hid_enumerate, SDL_hid_exit,
    SDL_hid_free_enumeration, SDL_hid_get_feature_report, SDL_hid_get_manufacturer_string,
    SDL_hid_get_product_string, SDL_hid_get_serial_number_string, SDL_hid_init, SDL_hid_open,
    SDL_hid_open_path, SDL_hid_read, SDL_hid_send_feature_report, SDL_hid_set_nonblocking,
    SDL_hid_write, SAFE_SDL_HINT_HIDAPI_DEVICE,
};
use safe_sdl::input::joystick::{
    SDL_JoystickAttachVirtualEx, SDL_JoystickClose, SDL_JoystickDetachVirtual,
    SDL_JoystickFromInstanceID, SDL_JoystickFromPlayerIndex, SDL_JoystickGetAxis,
    SDL_JoystickGetButton, SDL_JoystickGetDeviceGUID, SDL_JoystickGetDeviceProduct,
    SDL_JoystickGetDeviceProductVersion, SDL_JoystickGetDeviceVendor, SDL_JoystickGetHat,
    SDL_JoystickHasLED, SDL_JoystickHasRumble, SDL_JoystickHasRumbleTriggers,
    SDL_JoystickInstanceID, SDL_JoystickNameForIndex, SDL_JoystickNumButtons, SDL_JoystickOpen,
    SDL_JoystickPathForIndex, SDL_JoystickRumble, SDL_JoystickRumbleTriggers,
    SDL_JoystickSendEffect, SDL_JoystickSetLED, SDL_JoystickSetVirtualAxis,
    SDL_JoystickSetVirtualButton, SDL_JoystickSetVirtualHat, SDL_NumJoysticks,
};
use safe_sdl::input::sensor::{
    SDL_LockSensors, SDL_NumSensors, SDL_SensorClose, SDL_SensorFromInstanceID, SDL_SensorGetData,
    SDL_SensorGetDataWithTimestamp, SDL_SensorGetDeviceInstanceID, SDL_SensorGetDeviceName,
    SDL_SensorGetDeviceNonPortableType, SDL_SensorGetDeviceType, SDL_SensorGetInstanceID,
    SDL_SensorGetName, SDL_SensorGetNonPortableType, SDL_SensorGetType, SDL_SensorOpen,
    SDL_SensorUpdate, SDL_UnlockSensors,
};

const TEST_USB_VENDOR_SONY: Uint16 = 0x054c;
const TEST_USB_PRODUCT_DUALSHOCK4: Uint16 = 0x09cc;

static UPDATE_CALLS: AtomicUsize = AtomicUsize::new(0);
static LAST_PLAYER_INDEX: AtomicI32 = AtomicI32::new(-1);
static LAST_RUMBLE_LOW: AtomicU32 = AtomicU32::new(0);
static LAST_RUMBLE_HIGH: AtomicU32 = AtomicU32::new(0);
static LAST_TRIGGER_LEFT: AtomicU32 = AtomicU32::new(0);
static LAST_TRIGGER_RIGHT: AtomicU32 = AtomicU32::new(0);
static LAST_LED_PACKED: AtomicU32 = AtomicU32::new(0);

fn effect_bytes() -> &'static Mutex<Vec<u8>> {
    static EFFECT_BYTES: OnceLock<Mutex<Vec<u8>>> = OnceLock::new();
    EFFECT_BYTES.get_or_init(|| Mutex::new(Vec::new()))
}

unsafe extern "C" fn update_callback(_userdata: *mut c_void) {
    UPDATE_CALLS.fetch_add(1, Ordering::SeqCst);
}

unsafe extern "C" fn set_player_index_callback(_userdata: *mut c_void, player_index: c_int) {
    LAST_PLAYER_INDEX.store(player_index, Ordering::SeqCst);
}

unsafe extern "C" fn rumble_callback(_userdata: *mut c_void, low: Uint16, high: Uint16) -> c_int {
    LAST_RUMBLE_LOW.store(low as u32, Ordering::SeqCst);
    LAST_RUMBLE_HIGH.store(high as u32, Ordering::SeqCst);
    0
}

unsafe extern "C" fn rumble_triggers_callback(
    _userdata: *mut c_void,
    left: Uint16,
    right: Uint16,
) -> c_int {
    LAST_TRIGGER_LEFT.store(left as u32, Ordering::SeqCst);
    LAST_TRIGGER_RIGHT.store(right as u32, Ordering::SeqCst);
    0
}

unsafe extern "C" fn set_led_callback(
    _userdata: *mut c_void,
    red: u8,
    green: u8,
    blue: u8,
) -> c_int {
    LAST_LED_PACKED.store(
        ((red as u32) << 16) | ((green as u32) << 8) | blue as u32,
        Ordering::SeqCst,
    );
    0
}

unsafe extern "C" fn send_effect_callback(
    _userdata: *mut c_void,
    data: *const c_void,
    size: c_int,
) -> c_int {
    let bytes = if data.is_null() || size <= 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(data.cast::<u8>(), size as usize).to_vec()
    };
    let mut guard = effect_bytes().lock().unwrap();
    *guard = bytes;
    0
}

fn c_string(ptr: *const libc::c_char) -> String {
    unsafe { testutils::string_from_c(ptr) }
}

fn free_owned_string(ptr: *mut libc::c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let value = c_string(ptr);
    unsafe {
        SDL_free(ptr.cast());
    }
    value
}

fn guid_string(guid: safe_sdl::abi::generated_types::SDL_JoystickGUID) -> String {
    let mut buffer = [0i8; 33];
    unsafe {
        safe_sdl::input::guid::SDL_JoystickGetGUIDString(
            guid,
            buffer.as_mut_ptr(),
            buffer.len() as c_int,
        );
    }
    unsafe { CStr::from_ptr(buffer.as_ptr()) }
        .to_string_lossy()
        .into_owned()
}

fn evdev_mapping_for(device_index: c_int) -> String {
    let guid = unsafe { SDL_JoystickGetDeviceGUID(device_index) };
    let name = c_string(unsafe { SDL_JoystickNameForIndex(device_index) });
    format!(
        "{},{},a:b0,b:b1,back:b8,dpdown:h0.4,dpleft:h0.8,dpright:h0.2,dpup:h0.1,guide:b10,leftshoulder:b4,leftstick:b11,lefttrigger:a2,leftx:a0,lefty:a1,rightshoulder:b5,rightstick:b12,righttrigger:a5,rightx:a3,righty:a4,start:b9,x:b2,y:b3,platform:Linux,type:ps4",
        guid_string(guid),
        name
    )
}

fn reset_virtual_telemetry() {
    UPDATE_CALLS.store(0, Ordering::SeqCst);
    LAST_PLAYER_INDEX.store(-1, Ordering::SeqCst);
    LAST_RUMBLE_LOW.store(0, Ordering::SeqCst);
    LAST_RUMBLE_HIGH.store(0, Ordering::SeqCst);
    LAST_TRIGGER_LEFT.store(0, Ordering::SeqCst);
    LAST_TRIGGER_RIGHT.store(0, Ordering::SeqCst);
    LAST_LED_PACKED.store(0, Ordering::SeqCst);
    effect_bytes().lock().unwrap().clear();
}

fn virtual_desc(
    name: &std::ffi::CString,
    vendor: Uint16,
    product: Uint16,
    with_callbacks: bool,
) -> SDL_VirtualJoystickDesc {
    SDL_VirtualJoystickDesc {
        version: SDL_VIRTUAL_JOYSTICK_DESC_VERSION as Uint16,
        type_: SDL_JoystickType_SDL_JOYSTICK_TYPE_GAMECONTROLLER as Uint16,
        naxes: SDL_GameControllerAxis_SDL_CONTROLLER_AXIS_MAX as Uint16,
        nbuttons: SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_MAX as Uint16,
        nhats: 1,
        vendor_id: vendor,
        product_id: product,
        padding: 0,
        button_mask: 0,
        axis_mask: 0,
        name: name.as_ptr(),
        userdata: ptr::null_mut(),
        Update: with_callbacks.then_some(update_callback),
        SetPlayerIndex: with_callbacks.then_some(set_player_index_callback),
        Rumble: with_callbacks.then_some(rumble_callback),
        RumbleTriggers: with_callbacks.then_some(rumble_triggers_callback),
        SetLED: with_callbacks.then_some(set_led_callback),
        SendEffect: with_callbacks.then_some(send_effect_callback),
    }
}

#[test]
fn controllermap_gamecontroller_and_testevdev_ports_cover_mapping_and_fixture_behavior() {
    let _serial = testutils::serial_lock();
    let dir = tempfile::tempdir().expect("tempdir");
    let fake_path = dir.path().join("joystick-event0");
    testutils::write_default_evdev_gamepad_fixture(&fake_path);
    let _hint =
        testutils::HintGuard::set(SDL_HINT_JOYSTICK_DEVICE, &fake_path.display().to_string());
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_GAMECONTROLLER);

    unsafe {
        assert_eq!(SDL_NumJoysticks(), 1);
    }

    let device_index = 0;
    assert_eq!(
        c_string(unsafe { SDL_JoystickPathForIndex(device_index) }),
        fake_path.display().to_string()
    );
    assert_eq!(
        c_string(unsafe { SDL_JoystickNameForIndex(device_index) }),
        "SDL Fake evdev Gamepad"
    );
    assert_eq!(
        unsafe { SDL_JoystickGetDeviceVendor(device_index) },
        TEST_USB_VENDOR_SONY
    );
    assert_eq!(
        unsafe { SDL_JoystickGetDeviceProduct(device_index) },
        TEST_USB_PRODUCT_DUALSHOCK4
    );
    assert_eq!(
        unsafe { SDL_JoystickGetDeviceProductVersion(device_index) },
        0x0001
    );

    let mapping_db = format!(
        "# ignored comment\n{}\n{}\n",
        evdev_mapping_for(device_index).replace("platform:Linux", "platform:Windows"),
        evdev_mapping_for(device_index)
    );
    let rw = unsafe { SDL_RWFromConstMem(mapping_db.as_ptr().cast(), mapping_db.len() as c_int) };
    assert!(!rw.is_null(), "{}", testutils::current_error());
    assert_eq!(
        unsafe { SDL_GameControllerAddMappingsFromRW(rw, 1) },
        1,
        "{}",
        testutils::current_error()
    );
    assert_eq!(unsafe { SDL_GameControllerNumMappings() }, 1);

    let guid = unsafe { SDL_JoystickGetDeviceGUID(device_index) };
    let mapping_for_guid = free_owned_string(unsafe { SDL_GameControllerMappingForGUID(guid) });
    let mapping_for_index = free_owned_string(unsafe { SDL_GameControllerMappingForIndex(0) });
    let mapping_for_device =
        free_owned_string(unsafe { SDL_GameControllerMappingForDeviceIndex(device_index) });
    assert!(mapping_for_guid.contains("platform:Linux"));
    assert!(mapping_for_guid.contains("type:ps4"));
    assert_eq!(mapping_for_guid, mapping_for_index);
    assert_eq!(mapping_for_guid, mapping_for_device);

    let axis_name = testutils::cstring("leftx");
    let button_name = testutils::cstring("a");
    assert_eq!(
        unsafe { SDL_GameControllerGetAxisFromString(axis_name.as_ptr()) },
        SDL_GameControllerAxis_SDL_CONTROLLER_AXIS_LEFTX
    );
    assert_eq!(
        unsafe { SDL_GameControllerGetButtonFromString(button_name.as_ptr()) },
        SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_A
    );
    assert_eq!(
        c_string(unsafe { SDL_GameControllerNameForIndex(device_index) }),
        "SDL Fake evdev Gamepad"
    );
    assert_eq!(
        c_string(unsafe { SDL_GameControllerPathForIndex(device_index) }),
        fake_path.display().to_string()
    );
    assert_eq!(
        unsafe { SDL_IsGameController(device_index) },
        SDL_bool_SDL_TRUE
    );
    assert_eq!(
        unsafe { SDL_GameControllerTypeForIndex(device_index) },
        SDL_GameControllerType_SDL_CONTROLLER_TYPE_PS4
    );

    let controller = unsafe { SDL_GameControllerOpen(device_index) };
    assert!(!controller.is_null(), "{}", testutils::current_error());
    unsafe {
        SDL_GameControllerUpdate();
    }

    let axis_bind = unsafe {
        SDL_GameControllerGetBindForAxis(
            controller,
            SDL_GameControllerAxis_SDL_CONTROLLER_AXIS_LEFTX,
        )
    };
    assert_eq!(
        axis_bind.bindType,
        SDL_GameControllerBindType_SDL_CONTROLLER_BINDTYPE_AXIS
    );
    assert_eq!(unsafe { axis_bind.value.axis }, 0);
    let button_bind = unsafe {
        SDL_GameControllerGetBindForButton(
            controller,
            SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_A,
        )
    };
    assert_eq!(
        button_bind.bindType,
        SDL_GameControllerBindType_SDL_CONTROLLER_BINDTYPE_BUTTON
    );
    assert_eq!(unsafe { button_bind.value.button }, 0);
    let dpad_bind = unsafe {
        SDL_GameControllerGetBindForButton(
            controller,
            SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_DPAD_RIGHT,
        )
    };
    assert_eq!(
        dpad_bind.bindType,
        SDL_GameControllerBindType_SDL_CONTROLLER_BINDTYPE_HAT
    );
    assert_eq!(unsafe { dpad_bind.value.hat.hat }, 0);
    assert_eq!(unsafe { dpad_bind.value.hat.hat_mask }, 0x02);

    assert_eq!(
        unsafe {
            SDL_GameControllerHasAxis(controller, SDL_GameControllerAxis_SDL_CONTROLLER_AXIS_LEFTX)
        },
        SDL_bool_SDL_TRUE
    );
    assert_eq!(
        unsafe {
            SDL_GameControllerHasButton(
                controller,
                SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_A,
            )
        },
        SDL_bool_SDL_TRUE
    );
    assert_eq!(
        unsafe { SDL_GameControllerGetType(controller) },
        SDL_GameControllerType_SDL_CONTROLLER_TYPE_PS4
    );
    assert_eq!(
        unsafe {
            SDL_GameControllerGetButton(
                controller,
                SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_A,
            )
        },
        SDL_PRESSED as u8
    );
    assert_eq!(
        unsafe {
            SDL_GameControllerGetButton(
                controller,
                SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_DPAD_UP,
            )
        },
        SDL_PRESSED as u8
    );
    assert_eq!(
        unsafe {
            SDL_GameControllerGetButton(
                controller,
                SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_DPAD_RIGHT,
            )
        },
        SDL_PRESSED as u8
    );
    assert_eq!(
        unsafe {
            SDL_GameControllerGetButton(
                controller,
                SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_DPAD_LEFT,
            )
        },
        0
    );
    assert!(
        unsafe {
            SDL_GameControllerGetAxis(controller, SDL_GameControllerAxis_SDL_CONTROLLER_AXIS_LEFTX)
        } >= 12000
    );

    unsafe {
        SDL_GameControllerClose(controller);
    }
}

#[test]
fn virtual_joystick_hotplug_rumble_and_player_index_cover_joystick_hotplug_and_rumble_ports() {
    let _serial = testutils::serial_lock();
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_GAMECONTROLLER);
    reset_virtual_telemetry();

    let name_one = testutils::cstring("Virtual One");
    let name_two = testutils::cstring("Virtual Two");
    let index_one =
        unsafe { SDL_JoystickAttachVirtualEx(&virtual_desc(&name_one, 0x1234, 0x5678, true)) };
    let index_two =
        unsafe { SDL_JoystickAttachVirtualEx(&virtual_desc(&name_two, 0xabcd, 0xef01, false)) };
    assert_eq!(index_one, 0);
    assert_eq!(index_two, 1);
    assert_eq!(unsafe { SDL_NumJoysticks() }, 2);
    assert_eq!(
        c_string(unsafe { SDL_JoystickNameForIndex(0) }),
        "Virtual One"
    );
    assert_eq!(
        c_string(unsafe { SDL_JoystickNameForIndex(1) }),
        "Virtual Two"
    );

    let controller = unsafe { SDL_GameControllerOpen(index_one) };
    assert!(!controller.is_null(), "{}", testutils::current_error());
    assert_eq!(
        unsafe { SDL_GameControllerGetType(controller) },
        SDL_GameControllerType_SDL_CONTROLLER_TYPE_VIRTUAL
    );
    assert_eq!(
        unsafe { SDL_GameControllerHasLED(controller) },
        SDL_bool_SDL_TRUE
    );
    assert_eq!(
        unsafe { SDL_GameControllerHasRumble(controller) },
        SDL_bool_SDL_TRUE
    );
    assert_eq!(
        unsafe { SDL_GameControllerHasRumbleTriggers(controller) },
        SDL_bool_SDL_TRUE
    );

    let joystick_one =
        unsafe { safe_sdl::input::gamecontroller::SDL_GameControllerGetJoystick(controller) };
    let joystick_two = unsafe { SDL_JoystickOpen(index_two) };
    assert!(!joystick_one.is_null());
    assert!(!joystick_two.is_null(), "{}", testutils::current_error());
    assert_eq!(
        unsafe { SDL_JoystickNumButtons(joystick_one) },
        SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_MAX as i32
    );
    assert_eq!(
        unsafe { SDL_JoystickHasLED(joystick_one) },
        SDL_bool_SDL_TRUE
    );
    assert_eq!(
        unsafe { SDL_JoystickHasRumble(joystick_one) },
        SDL_bool_SDL_TRUE
    );
    assert_eq!(
        unsafe { SDL_JoystickHasRumbleTriggers(joystick_one) },
        SDL_bool_SDL_TRUE
    );

    let instance_one = unsafe { SDL_JoystickInstanceID(joystick_one) };
    assert_eq!(
        unsafe { SDL_JoystickFromInstanceID(instance_one) },
        joystick_one
    );
    assert!(unsafe { SDL_JoystickFromPlayerIndex(7) }.is_null());
    assert!(unsafe { SDL_GameControllerFromPlayerIndex(7) }.is_null());

    unsafe {
        SDL_GameControllerSetPlayerIndex(controller, 7);
    }
    assert_eq!(LAST_PLAYER_INDEX.load(Ordering::SeqCst), 7);
    assert_eq!(unsafe { SDL_JoystickFromPlayerIndex(7) }, joystick_one);
    assert_eq!(unsafe { SDL_GameControllerFromPlayerIndex(7) }, controller);

    assert_eq!(
        unsafe {
            SDL_JoystickSetVirtualButton(
                joystick_one,
                SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_A as i32,
                SDL_PRESSED as u8,
            )
        },
        0
    );
    assert_eq!(
        unsafe { SDL_JoystickSetVirtualAxis(joystick_one, 0, 22_222) },
        0
    );
    assert_eq!(
        unsafe { SDL_JoystickSetVirtualAxis(joystick_one, 1, -11_111) },
        0
    );
    assert_eq!(
        unsafe { SDL_JoystickSetVirtualAxis(joystick_one, 2, 16_384) },
        0
    );
    assert_eq!(
        unsafe { SDL_JoystickSetVirtualAxis(joystick_one, 3, 8_192) },
        0
    );
    assert_eq!(
        unsafe { SDL_JoystickSetVirtualAxis(joystick_one, 4, -12_288) },
        0
    );
    assert_eq!(
        unsafe { SDL_JoystickSetVirtualAxis(joystick_one, 5, 4_096) },
        0
    );
    assert_eq!(
        unsafe { SDL_JoystickSetVirtualHat(joystick_one, 0, SDL_HAT_RIGHTUP as u8) },
        0
    );
    unsafe {
        SDL_GameControllerUpdate();
    }
    assert!(UPDATE_CALLS.load(Ordering::SeqCst) > 0);
    assert_eq!(
        unsafe {
            SDL_JoystickGetButton(
                joystick_one,
                SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_A as i32,
            )
        },
        SDL_PRESSED as u8
    );
    assert_eq!(unsafe { SDL_JoystickGetAxis(joystick_one, 0) }, 22_222);
    assert_eq!(
        unsafe { SDL_JoystickGetHat(joystick_one, 0) },
        SDL_HAT_RIGHTUP as u8
    );
    assert_eq!(
        unsafe {
            SDL_GameControllerGetButton(
                controller,
                SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_A,
            )
        },
        SDL_PRESSED as u8
    );
    assert_eq!(
        unsafe {
            SDL_GameControllerGetButton(
                controller,
                SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_DPAD_UP,
            )
        },
        SDL_PRESSED as u8
    );
    assert_eq!(
        unsafe {
            SDL_GameControllerGetButton(
                controller,
                SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_DPAD_RIGHT,
            )
        },
        SDL_PRESSED as u8
    );
    assert_eq!(
        unsafe {
            SDL_GameControllerGetAxis(controller, SDL_GameControllerAxis_SDL_CONTROLLER_AXIS_LEFTX)
        },
        22_222
    );

    let mut finger_state = 99u8;
    let mut x = 1.0f32;
    let mut y = 1.0f32;
    let mut pressure = 1.0f32;
    assert_eq!(unsafe { SDL_GameControllerGetNumTouchpads(controller) }, 1);
    assert_eq!(
        unsafe { SDL_GameControllerGetNumTouchpadFingers(controller, 0) },
        1
    );
    assert_eq!(
        unsafe {
            SDL_GameControllerGetTouchpadFinger(
                controller,
                0,
                0,
                &mut finger_state,
                &mut x,
                &mut y,
                &mut pressure,
            )
        },
        0
    );
    assert_eq!(finger_state, SDL_PRESSED as u8);
    assert!((x - ((22_222.0 + 32_768.0) / 65_535.0)).abs() < 0.01);
    assert!((y - ((-11_111.0 + 32_768.0) / 65_535.0)).abs() < 0.01);
    assert!(pressure > 0.70 && pressure <= 1.0);

    assert_eq!(
        unsafe { SDL_GameControllerHasSensor(controller, SDL_SensorType_SDL_SENSOR_ACCEL) },
        SDL_bool_SDL_TRUE
    );
    assert_eq!(
        unsafe { SDL_GameControllerHasSensor(controller, SDL_SensorType_SDL_SENSOR_GYRO) },
        SDL_bool_SDL_TRUE
    );
    assert_eq!(
        unsafe { SDL_GameControllerIsSensorEnabled(controller, SDL_SensorType_SDL_SENSOR_ACCEL) },
        SDL_bool_SDL_TRUE
    );
    assert!(
        unsafe { SDL_GameControllerGetSensorDataRate(controller, SDL_SensorType_SDL_SENSOR_ACCEL) }
            > 0.0
    );
    let mut accel = [0.0f32; 3];
    let mut accel_timestamp = 0u64;
    assert_eq!(
        unsafe {
            SDL_GameControllerGetSensorDataWithTimestamp(
                controller,
                SDL_SensorType_SDL_SENSOR_ACCEL,
                &mut accel_timestamp,
                accel.as_mut_ptr(),
                accel.len() as c_int,
            )
        },
        0
    );
    assert!(accel_timestamp > 0);
    assert!(accel[0] > 0.0);
    assert!(accel[1] < 0.0);
    assert!(accel[2] > 0.0);
    assert_eq!(
        unsafe {
            SDL_GameControllerSetSensorEnabled(
                controller,
                SDL_SensorType_SDL_SENSOR_ACCEL,
                SDL_bool_SDL_FALSE,
            )
        },
        0
    );
    assert_eq!(
        unsafe { SDL_GameControllerIsSensorEnabled(controller, SDL_SensorType_SDL_SENSOR_ACCEL) },
        SDL_bool_SDL_FALSE
    );
    assert!(
        unsafe {
            SDL_GameControllerGetSensorData(
                controller,
                SDL_SensorType_SDL_SENSOR_ACCEL,
                accel.as_mut_ptr(),
                accel.len() as c_int,
            )
        } < 0
    );
    assert_eq!(
        unsafe {
            SDL_GameControllerSetSensorEnabled(
                controller,
                SDL_SensorType_SDL_SENSOR_ACCEL,
                SDL_bool_SDL_TRUE,
            )
        },
        0
    );
    assert_eq!(
        unsafe {
            SDL_GameControllerGetSensorData(
                controller,
                SDL_SensorType_SDL_SENSOR_ACCEL,
                accel.as_mut_ptr(),
                accel.len() as c_int,
            )
        },
        0
    );

    assert_eq!(
        unsafe { SDL_JoystickRumble(joystick_one, 0x1234, 0x5678, 250) },
        0
    );
    assert_eq!(
        unsafe { SDL_JoystickRumbleTriggers(joystick_one, 0x1111, 0x2222, 250) },
        0
    );
    assert_eq!(unsafe { SDL_JoystickSetLED(joystick_one, 1, 2, 3) }, 0);
    assert_eq!(
        unsafe { SDL_JoystickSendEffect(joystick_one, b"hi\0".as_ptr().cast(), 2) },
        0
    );
    assert_eq!(LAST_RUMBLE_LOW.load(Ordering::SeqCst), 0x1234);
    assert_eq!(LAST_RUMBLE_HIGH.load(Ordering::SeqCst), 0x5678);
    assert_eq!(LAST_TRIGGER_LEFT.load(Ordering::SeqCst), 0x1111);
    assert_eq!(LAST_TRIGGER_RIGHT.load(Ordering::SeqCst), 0x2222);
    assert_eq!(LAST_LED_PACKED.load(Ordering::SeqCst), 0x010203);
    assert_eq!(&*effect_bytes().lock().unwrap(), b"hi");

    assert_eq!(
        unsafe { SDL_GameControllerRumble(controller, 0x3333, 0x4444, 100) },
        0
    );
    assert_eq!(
        unsafe { SDL_GameControllerRumbleTriggers(controller, 0x5555, 0x6666, 100) },
        0
    );
    assert_eq!(unsafe { SDL_GameControllerSetLED(controller, 9, 8, 7) }, 0);
    assert_eq!(
        unsafe { SDL_GameControllerSendEffect(controller, b"abcd".as_ptr().cast(), 4) },
        0
    );
    assert_eq!(LAST_RUMBLE_LOW.load(Ordering::SeqCst), 0x3333);
    assert_eq!(LAST_RUMBLE_HIGH.load(Ordering::SeqCst), 0x4444);
    assert_eq!(LAST_TRIGGER_LEFT.load(Ordering::SeqCst), 0x5555);
    assert_eq!(LAST_TRIGGER_RIGHT.load(Ordering::SeqCst), 0x6666);
    assert_eq!(LAST_LED_PACKED.load(Ordering::SeqCst), 0x090807);
    assert_eq!(&*effect_bytes().lock().unwrap(), b"abcd");

    unsafe {
        SDL_GameControllerClose(controller);
        SDL_JoystickClose(joystick_two);
    }
    assert_eq!(unsafe { SDL_JoystickDetachVirtual(index_one) }, 0);
    assert_eq!(unsafe { SDL_NumJoysticks() }, 1);
    assert_eq!(
        c_string(unsafe { SDL_JoystickNameForIndex(0) }),
        "Virtual Two"
    );
    assert_eq!(unsafe { SDL_JoystickDetachVirtual(0) }, 0);
    assert_eq!(unsafe { SDL_NumJoysticks() }, 0);
}

#[test]
fn haptic_sensor_and_hidapi_ports_cover_virtual_and_fixture_backed_paths() {
    let _serial = testutils::serial_lock();
    let dir = tempfile::tempdir().expect("tempdir");
    let hid_fixture = dir.path().join("fixture-hid.txt");
    testutils::write_default_hidapi_fixture(&hid_fixture);
    let _hid_hint = testutils::HintGuard::set(
        SAFE_SDL_HINT_HIDAPI_DEVICE,
        &hid_fixture.display().to_string(),
    );
    let _subsystem = testutils::SubsystemGuard::init(
        SDL_INIT_JOYSTICK | SDL_INIT_GAMECONTROLLER | SDL_INIT_HAPTIC | SDL_INIT_SENSOR,
    );
    reset_virtual_telemetry();

    assert_eq!(unsafe { SDL_MouseIsHaptic() }, 0);
    assert!(unsafe { SDL_HapticOpenFromMouse() }.is_null());

    let name = testutils::cstring("Virtual Haptics");
    let device_index = unsafe { SDL_JoystickAttachVirtualEx(&virtual_desc(&name, 0, 0, true)) };
    let joystick = unsafe { SDL_JoystickOpen(device_index) };
    assert!(!joystick.is_null(), "{}", testutils::current_error());
    let controller = unsafe { SDL_GameControllerOpen(device_index) };
    assert!(!controller.is_null(), "{}", testutils::current_error());

    assert_eq!(
        unsafe { SDL_JoystickSetVirtualButton(joystick, 0, SDL_PRESSED as u8) },
        0
    );
    assert_eq!(
        unsafe { SDL_JoystickSetVirtualAxis(joystick, 0, 20_000) },
        0
    );
    assert_eq!(
        unsafe { SDL_JoystickSetVirtualAxis(joystick, 1, -10_000) },
        0
    );
    assert_eq!(
        unsafe { SDL_JoystickSetVirtualAxis(joystick, 2, 15_000) },
        0
    );
    assert_eq!(unsafe { SDL_JoystickSetVirtualAxis(joystick, 3, 9_000) }, 0);
    assert_eq!(
        unsafe { SDL_JoystickSetVirtualAxis(joystick, 4, -7_000) },
        0
    );
    assert_eq!(unsafe { SDL_JoystickSetVirtualAxis(joystick, 5, 5_000) }, 0);
    unsafe {
        SDL_SensorUpdate();
    }

    assert_eq!(unsafe { SDL_NumHaptics() }, 1);
    assert_eq!(c_string(unsafe { SDL_HapticName(0) }), "Virtual Haptics");
    assert_eq!(unsafe { SDL_JoystickIsHaptic(joystick) }, 1);
    let haptic = unsafe { SDL_HapticOpen(0) };
    assert!(!haptic.is_null(), "{}", testutils::current_error());
    let haptic_from_joystick = unsafe { SDL_HapticOpenFromJoystick(joystick) };
    assert!(
        !haptic_from_joystick.is_null(),
        "{}",
        testutils::current_error()
    );
    assert_eq!(unsafe { SDL_HapticIndex(haptic) }, 0);
    assert!(unsafe { SDL_HapticNumEffects(haptic) } >= 16);
    assert_eq!(unsafe { SDL_HapticNumAxes(haptic) }, 2);
    assert_eq!(unsafe { SDL_HapticNumEffectsPlaying(haptic) }, 0);
    assert_ne!(unsafe { SDL_HapticQuery(haptic) } & SDL_HAPTIC_LEFTRIGHT, 0);
    assert_eq!(unsafe { SDL_HapticSetGain(haptic, 50) }, 0);
    assert_eq!(unsafe { SDL_HapticSetAutocenter(haptic, 25) }, 0);
    assert_eq!(unsafe { SDL_HapticRumbleSupported(haptic) }, 1);
    assert_eq!(unsafe { SDL_HapticRumbleInit(haptic) }, 0);
    assert_eq!(unsafe { SDL_HapticRumblePlay(haptic, 0.5, 250) }, 0);
    assert_eq!(LAST_RUMBLE_LOW.load(Ordering::SeqCst), 32_767);
    assert_eq!(LAST_RUMBLE_HIGH.load(Ordering::SeqCst), 32_767);
    assert_eq!(unsafe { SDL_HapticRumbleStop(haptic) }, 0);
    assert_eq!(LAST_RUMBLE_LOW.load(Ordering::SeqCst), 0);
    assert_eq!(LAST_RUMBLE_HIGH.load(Ordering::SeqCst), 0);

    let mut effect = SDL_HapticEffect {
        leftright: SDL_HapticLeftRight {
            type_: SDL_HAPTIC_LEFTRIGHT as Uint16,
            length: 500,
            large_magnitude: 0x4000,
            small_magnitude: 0x2000,
        },
    };
    let effect_id = unsafe { SDL_HapticNewEffect(haptic, &mut effect) };
    assert!(effect_id >= 0, "{}", testutils::current_error());
    assert_eq!(unsafe { SDL_HapticRunEffect(haptic, effect_id, 1) }, 0);
    assert_eq!(LAST_RUMBLE_LOW.load(Ordering::SeqCst), 0x2000);
    assert_eq!(LAST_RUMBLE_HIGH.load(Ordering::SeqCst), 0x1000);
    assert_eq!(unsafe { SDL_HapticGetEffectStatus(haptic, effect_id) }, 1);
    assert_eq!(unsafe { SDL_HapticPause(haptic) }, 0);
    assert_eq!(unsafe { SDL_HapticUnpause(haptic) }, 0);
    assert_eq!(unsafe { SDL_HapticStopEffect(haptic, effect_id) }, 0);
    assert_eq!(unsafe { SDL_HapticGetEffectStatus(haptic, effect_id) }, 0);
    assert_eq!(unsafe { SDL_HapticStopAll(haptic) }, 0);
    assert_eq!(unsafe { SDL_HapticNumEffectsPlaying(haptic) }, 0);

    unsafe {
        SDL_LockSensors();
        SDL_UnlockSensors();
    }
    assert!(unsafe { SDL_NumSensors() } >= 2);
    assert!(c_string(unsafe { SDL_SensorGetDeviceName(0) }).contains("accelerometer"));
    assert_eq!(
        unsafe { SDL_SensorGetDeviceType(0) },
        SDL_SensorType_SDL_SENSOR_ACCEL
    );
    assert_eq!(
        unsafe { SDL_SensorGetDeviceNonPortableType(0) },
        SDL_SensorType_SDL_SENSOR_ACCEL
    );
    let sensor_instance_id = unsafe { SDL_SensorGetDeviceInstanceID(0) };
    assert!(sensor_instance_id >= 0);
    let sensor = unsafe { SDL_SensorOpen(0) };
    assert!(!sensor.is_null(), "{}", testutils::current_error());
    assert_eq!(
        unsafe { SDL_SensorFromInstanceID(sensor_instance_id) },
        sensor
    );
    assert_eq!(
        c_string(unsafe { SDL_SensorGetName(sensor) }),
        c_string(unsafe { SDL_SensorGetDeviceName(0) })
    );
    assert_eq!(
        unsafe { SDL_SensorGetType(sensor) },
        SDL_SensorType_SDL_SENSOR_ACCEL
    );
    assert_eq!(
        unsafe { SDL_SensorGetNonPortableType(sensor) },
        SDL_SensorType_SDL_SENSOR_ACCEL
    );
    assert_eq!(
        unsafe { SDL_SensorGetInstanceID(sensor) },
        sensor_instance_id
    );
    let mut sensor_values = [0.0f32; 3];
    let mut timestamp = 0u64;
    assert_eq!(
        unsafe {
            SDL_SensorGetDataWithTimestamp(
                sensor,
                &mut timestamp,
                sensor_values.as_mut_ptr(),
                sensor_values.len() as c_int,
            )
        },
        0
    );
    assert!(timestamp > 0);
    assert!(sensor_values[0] > 0.0);
    assert!(sensor_values[1] < 0.0);
    assert!(sensor_values[2] > 0.0);
    assert_eq!(
        unsafe {
            SDL_SensorGetData(
                sensor,
                sensor_values.as_mut_ptr(),
                sensor_values.len() as c_int,
            )
        },
        0
    );
    assert_eq!(
        unsafe { SDL_GameControllerHasSensor(controller, SDL_SensorType_SDL_SENSOR_ACCEL) },
        SDL_bool_SDL_TRUE
    );
    assert_eq!(
        unsafe { SDL_GameControllerHasSensor(controller, SDL_SensorType_SDL_SENSOR_GYRO) },
        SDL_bool_SDL_TRUE
    );
    assert!(
        unsafe { SDL_GameControllerGetSensorDataRate(controller, SDL_SensorType_SDL_SENSOR_ACCEL) }
            > 0.0
    );
    assert_eq!(
        unsafe { SDL_GameControllerIsSensorEnabled(controller, SDL_SensorType_SDL_SENSOR_ACCEL) },
        SDL_bool_SDL_TRUE
    );
    assert_eq!(
        unsafe {
            SDL_GameControllerSetSensorEnabled(
                controller,
                SDL_SensorType_SDL_SENSOR_ACCEL,
                SDL_bool_SDL_FALSE,
            )
        },
        0
    );
    assert!(
        unsafe {
            SDL_GameControllerGetSensorData(
                controller,
                SDL_SensorType_SDL_SENSOR_ACCEL,
                sensor_values.as_mut_ptr(),
                sensor_values.len() as c_int,
            )
        } < 0
    );
    assert_eq!(
        unsafe {
            SDL_GameControllerSetSensorEnabled(
                controller,
                SDL_SensorType_SDL_SENSOR_ACCEL,
                SDL_bool_SDL_TRUE,
            )
        },
        0
    );
    assert_eq!(
        unsafe {
            SDL_GameControllerGetSensorDataWithTimestamp(
                controller,
                SDL_SensorType_SDL_SENSOR_GYRO,
                &mut timestamp,
                sensor_values.as_mut_ptr(),
                sensor_values.len() as c_int,
            )
        },
        0
    );
    assert!(timestamp > 0);

    assert_eq!(unsafe { SDL_hid_init() }, 0);
    assert!(unsafe { SDL_hid_device_change_count() } > 0);
    let enumeration = unsafe { SDL_hid_enumerate(0x1234, 0x5678) };
    assert!(!enumeration.is_null(), "{}", testutils::current_error());
    assert_eq!(unsafe { (*enumeration).vendor_id }, 0x1234);
    assert_eq!(unsafe { (*enumeration).product_id }, 0x5678);
    assert_eq!(
        c_string(unsafe { (*enumeration).path }),
        hid_fixture.display().to_string()
    );
    let hid = unsafe { SDL_hid_open(0x1234, 0x5678, ptr::null()) };
    assert!(!hid.is_null(), "{}", testutils::current_error());
    let hid_path = testutils::cstring(&hid_fixture.display().to_string());
    let hid_by_path = unsafe { SDL_hid_open_path(hid_path.as_ptr(), 0) };
    assert!(!hid_by_path.is_null(), "{}", testutils::current_error());
    let output = [0x00u8, 0x10, 0x20];
    assert_eq!(
        unsafe { SDL_hid_write(hid, output.as_ptr(), output.len()) },
        output.len() as c_int
    );
    let mut read_buffer = [0u8; 8];
    assert_eq!(
        unsafe { SDL_hid_read(hid, read_buffer.as_mut_ptr(), read_buffer.len()) },
        4
    );
    assert_eq!(&read_buffer[..4], &[0x00, 0x01, 0x02, 0x03]);
    assert_eq!(unsafe { SDL_hid_set_nonblocking(hid, 1) }, 0);
    let feature = [0x33u8, 0xaa, 0xbb];
    assert_eq!(
        unsafe { SDL_hid_send_feature_report(hid, feature.as_ptr(), feature.len()) },
        feature.len() as c_int
    );
    let mut feature_buffer = [0u8; 8];
    assert_eq!(
        unsafe {
            SDL_hid_get_feature_report(hid, feature_buffer.as_mut_ptr(), feature_buffer.len())
        },
        feature.len() as c_int
    );
    assert_eq!(&feature_buffer[..feature.len()], &feature);
    let mut wide_buffer = [0 as wchar_t; 32];
    assert_eq!(
        unsafe {
            SDL_hid_get_manufacturer_string(hid, wide_buffer.as_mut_ptr(), wide_buffer.len())
        },
        0
    );
    assert_eq!(testutils::wide_string_from_buffer(&wide_buffer), "Safe SDL");
    assert_eq!(
        unsafe { SDL_hid_get_product_string(hid, wide_buffer.as_mut_ptr(), wide_buffer.len()) },
        0
    );
    assert_eq!(
        testutils::wide_string_from_buffer(&wide_buffer),
        "Fixture HID Device"
    );
    assert_eq!(
        unsafe {
            SDL_hid_get_serial_number_string(hid, wide_buffer.as_mut_ptr(), wide_buffer.len())
        },
        0
    );
    assert_eq!(testutils::wide_string_from_buffer(&wide_buffer), "SAFE123");
    unsafe {
        SDL_hid_ble_scan(SDL_bool_SDL_FALSE);
    }
    unsafe {
        SDL_hid_close(hid_by_path);
        SDL_hid_close(hid);
        SDL_hid_free_enumeration(enumeration);
    }
    assert_eq!(unsafe { SDL_hid_exit() }, 0);

    unsafe {
        SDL_SensorClose(sensor);
        SDL_HapticClose(haptic_from_joystick);
        SDL_HapticClose(haptic);
        SDL_GameControllerClose(controller);
        SDL_JoystickClose(joystick);
    }
    assert_eq!(unsafe { SDL_JoystickDetachVirtual(device_index) }, 0);
}
