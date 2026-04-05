#![allow(non_upper_case_globals)]
#![allow(clippy::all)]

#[path = "common/testutils.rs"]
mod testutils;

use std::ffi::CStr;
use std::ptr;

use safe_sdl::abi::generated_types::{
    SDL_GameControllerAxis_SDL_CONTROLLER_AXIS_MAX,
    SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_A,
    SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_MAX,
    SDL_JoystickType_SDL_JOYSTICK_TYPE_GAMECONTROLLER, SDL_VirtualJoystickDesc, SDL_bool_SDL_FALSE,
    SDL_bool_SDL_TRUE, Uint16, SDL_GUID, SDL_INIT_GAMECONTROLLER, SDL_PRESSED, SDL_RELEASED,
    SDL_VIRTUAL_JOYSTICK_DESC_VERSION,
};
use safe_sdl::input::guid::{
    SDL_GUIDFromString, SDL_GUIDToString, SDL_GetJoystickGUIDInfo, SDL_JoystickGetGUIDFromString,
    SDL_JoystickGetGUIDString,
};
use safe_sdl::input::joystick::{
    SDL_JoystickAttachVirtualEx, SDL_JoystickClose, SDL_JoystickDetachVirtual,
    SDL_JoystickGetButton, SDL_JoystickGetFirmwareVersion, SDL_JoystickGetGUID,
    SDL_JoystickGetProduct, SDL_JoystickGetProductVersion, SDL_JoystickGetSerial,
    SDL_JoystickGetType, SDL_JoystickGetVendor, SDL_JoystickIsVirtual, SDL_JoystickName,
    SDL_JoystickNumAxes, SDL_JoystickNumBalls, SDL_JoystickNumButtons, SDL_JoystickNumHats,
    SDL_JoystickOpen, SDL_JoystickSetVirtualButton, SDL_JoystickUpdate,
};

const TEST_USB_VENDOR_NVIDIA: Uint16 = 0x0955;
const TEST_USB_PRODUCT_NVIDIA_SHIELD_CONTROLLER_V104: Uint16 = 0x7214;

fn guid_from_halves(upper: u64, lower: u64) -> SDL_GUID {
    let mut bytes = [0u8; 16];
    for (block, value) in [upper, lower].into_iter().enumerate() {
        let offset = block * 8;
        for i in 0..8 {
            bytes[offset + i] = (value >> (56 - i * 8)) as u8;
        }
    }
    SDL_GUID { data: bytes }
}

fn c_string(ptr: *const libc::c_char) -> String {
    unsafe { testutils::string_from_c(ptr) }
}

#[test]
fn guid_string_roundtrip_and_bounds_match_upstream_cases() {
    let cases = [
        (
            "0000000000000000ffffffffffffffff",
            0x0000000000000000,
            0xffffffffffffffff,
        ),
        (
            "00112233445566778091a2b3c4d5e6f0",
            0x0011223344556677,
            0x8091a2b3c4d5e6f0,
        ),
        (
            "a0112233445566778091a2b3c4d5e6f0",
            0xa011223344556677,
            0x8091a2b3c4d5e6f0,
        ),
        (
            "a0112233445566778091a2b3c4d5e6f1",
            0xa011223344556677,
            0x8091a2b3c4d5e6f1,
        ),
        (
            "a0112233445566778191a2b3c4d5e6f0",
            0xa011223344556677,
            0x8191a2b3c4d5e6f0,
        ),
    ];

    for (text, upper, lower) in cases {
        let expected = guid_from_halves(upper, lower);
        let input = testutils::cstring(text);
        let guid = unsafe { SDL_GUIDFromString(input.as_ptr()) };
        assert_eq!(guid.data, expected.data, "roundtrip bytes for {text}");

        for size in 0..=36 {
            let fill = (size as u8).wrapping_add(0xa0);
            let mut buffer = [fill; 64];
            let offset = 4usize;
            let out = unsafe { buffer.as_mut_ptr().add(offset).cast::<libc::c_char>() };

            unsafe {
                SDL_GUIDToString(guid, out, size);
            }

            assert_eq!(
                &buffer[..offset],
                &[fill; 4],
                "prefix overwrite at size {size}"
            );

            let written = buffer[offset..]
                .iter()
                .position(|byte| *byte == fill)
                .unwrap_or(buffer.len() - offset);
            assert!(
                written <= size as usize,
                "wrote {written} bytes into size {size} for {text}"
            );

            if size >= 33 {
                let actual = unsafe { CStr::from_ptr(out) }
                    .to_string_lossy()
                    .into_owned();
                assert_eq!(actual, text, "full GUID string at size {size}");
            }
        }
    }
}

#[test]
fn virtual_joystick_attach_open_guid_info_and_button_updates_match_automation_case() {
    let _serial = testutils::serial_lock();
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_GAMECONTROLLER);

    let name = testutils::cstring("Virtual NVIDIA SHIELD Controller");
    let desc = SDL_VirtualJoystickDesc {
        version: SDL_VIRTUAL_JOYSTICK_DESC_VERSION as Uint16,
        type_: SDL_JoystickType_SDL_JOYSTICK_TYPE_GAMECONTROLLER as Uint16,
        naxes: SDL_GameControllerAxis_SDL_CONTROLLER_AXIS_MAX as Uint16,
        nbuttons: SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_MAX as Uint16,
        nhats: 0,
        vendor_id: TEST_USB_VENDOR_NVIDIA,
        product_id: TEST_USB_PRODUCT_NVIDIA_SHIELD_CONTROLLER_V104,
        padding: 0,
        button_mask: 0,
        axis_mask: 0,
        name: name.as_ptr(),
        userdata: ptr::null_mut(),
        Update: None,
        SetPlayerIndex: None,
        Rumble: None,
        RumbleTriggers: None,
        SetLED: None,
        SendEffect: None,
    };

    let device_index = unsafe { SDL_JoystickAttachVirtualEx(&desc) };
    assert!(device_index >= 0, "{}", testutils::current_error());
    assert_eq!(
        unsafe { SDL_JoystickIsVirtual(device_index) },
        SDL_bool_SDL_TRUE
    );

    let joystick = unsafe { SDL_JoystickOpen(device_index) };
    assert!(!joystick.is_null(), "{}", testutils::current_error());

    assert_eq!(
        c_string(unsafe { SDL_JoystickName(joystick) }),
        "Virtual NVIDIA SHIELD Controller"
    );
    assert_eq!(
        unsafe { SDL_JoystickGetVendor(joystick) },
        TEST_USB_VENDOR_NVIDIA
    );
    assert_eq!(
        unsafe { SDL_JoystickGetProduct(joystick) },
        TEST_USB_PRODUCT_NVIDIA_SHIELD_CONTROLLER_V104
    );
    assert_eq!(unsafe { SDL_JoystickGetProductVersion(joystick) }, 0);
    assert_eq!(unsafe { SDL_JoystickGetFirmwareVersion(joystick) }, 0);
    assert!(unsafe { SDL_JoystickGetSerial(joystick) }.is_null());
    assert_eq!(
        unsafe { SDL_JoystickGetType(joystick) },
        SDL_JoystickType_SDL_JOYSTICK_TYPE_GAMECONTROLLER
    );
    assert_eq!(
        unsafe { SDL_JoystickNumAxes(joystick) },
        SDL_GameControllerAxis_SDL_CONTROLLER_AXIS_MAX as i32
    );
    assert_eq!(unsafe { SDL_JoystickNumBalls(joystick) }, 0);
    assert_eq!(unsafe { SDL_JoystickNumHats(joystick) }, 0);
    assert_eq!(
        unsafe { SDL_JoystickNumButtons(joystick) },
        SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_MAX as i32
    );

    let guid = unsafe { SDL_JoystickGetGUID(joystick) };
    let mut guid_string = [0i8; 33];
    unsafe {
        SDL_JoystickGetGUIDString(guid, guid_string.as_mut_ptr(), guid_string.len() as i32);
    }
    let roundtrip = unsafe { SDL_JoystickGetGUIDFromString(guid_string.as_ptr()) };
    assert_eq!(roundtrip.data, guid.data);

    let mut vendor = 0u16;
    let mut product = 0u16;
    let mut version = 0u16;
    let mut crc = 0u16;
    unsafe {
        SDL_GetJoystickGUIDInfo(guid, &mut vendor, &mut product, &mut version, &mut crc);
    }
    assert_eq!(vendor, TEST_USB_VENDOR_NVIDIA);
    assert_eq!(product, TEST_USB_PRODUCT_NVIDIA_SHIELD_CONTROLLER_V104);
    assert_eq!(version, 0);
    assert_ne!(crc, 0);

    assert_eq!(
        unsafe {
            SDL_JoystickSetVirtualButton(
                joystick,
                SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_A as i32,
                SDL_PRESSED as u8,
            )
        },
        0
    );
    unsafe {
        SDL_JoystickUpdate();
    }
    assert_eq!(
        unsafe {
            SDL_JoystickGetButton(
                joystick,
                SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_A as i32,
            )
        },
        SDL_PRESSED as u8
    );

    assert_eq!(
        unsafe {
            SDL_JoystickSetVirtualButton(
                joystick,
                SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_A as i32,
                SDL_RELEASED as u8,
            )
        },
        0
    );
    unsafe {
        SDL_JoystickUpdate();
    }
    assert_eq!(
        unsafe {
            SDL_JoystickGetButton(
                joystick,
                SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_A as i32,
            )
        },
        SDL_RELEASED as u8
    );

    unsafe {
        SDL_JoystickClose(joystick);
    }
    assert_eq!(unsafe { SDL_JoystickDetachVirtual(device_index) }, 0);
    assert_eq!(
        unsafe { SDL_JoystickIsVirtual(device_index) },
        SDL_bool_SDL_FALSE
    );
}
