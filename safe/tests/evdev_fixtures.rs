#![allow(clippy::all)]

#[path = "common/testutils.rs"]
mod testutils;

use tempfile::tempdir;

use safe_sdl::abi::generated_types::{
    SDL_HAT_RIGHTUP, SDL_HINT_JOYSTICK_DEVICE, SDL_INIT_JOYSTICK, SDL_PRESSED,
};
use safe_sdl::input::joystick::{
    SDL_JoystickClose, SDL_JoystickGetAxis, SDL_JoystickGetButton, SDL_JoystickGetDeviceProduct,
    SDL_JoystickGetDeviceVendor, SDL_JoystickGetHat, SDL_JoystickNameForIndex, SDL_JoystickOpen,
    SDL_JoystickPathForIndex, SDL_JoystickUpdate, SDL_NumJoysticks,
};
use safe_sdl::input::linux::evdev::parse_device_hint;
use safe_sdl::input::linux::udev::discover_device_nodes;

fn c_string(ptr: *const libc::c_char) -> String {
    unsafe { testutils::string_from_c(ptr) }
}

#[test]
fn parse_device_hint_preserves_order_and_ignores_empty_segments() {
    let paths = parse_device_hint(":/tmp/js2::/tmp/js10:/tmp/js1:");
    let rendered = paths
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(rendered, vec!["/tmp/js2", "/tmp/js10", "/tmp/js1"]);
}

#[test]
fn discover_device_nodes_sorts_by_prefix_then_numeric_suffix() {
    let dir = tempdir().expect("tempdir");
    for name in ["event10", "event2", "event1", "js3", "js11", "js2"] {
        std::fs::write(dir.path().join(name), b"fixture").expect("fixture file");
    }

    let entries = discover_device_nodes(dir.path()).expect("discover device nodes");
    let names = entries
        .iter()
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec!["event1", "event2", "event10", "js2", "js3", "js11"]
    );
}

#[test]
#[ignore = "run via xtask run-evdev-fixture-tests"]
fn hinted_evdev_devices_appear_in_hint_order_with_probed_fixture_metadata() {
    let _serial = testutils::serial_lock();
    let dir = tempdir().expect("tempdir");
    let event2 = dir.path().join("event2");
    let event1 = dir.path().join("event1");
    testutils::write_default_evdev_gamepad_fixture(&event2);
    testutils::write_default_evdev_gamepad_fixture(&event1);
    let _hint = testutils::HintGuard::set(
        SDL_HINT_JOYSTICK_DEVICE,
        &format!("{}:{}", event2.display(), event1.display()),
    );
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_JOYSTICK);

    unsafe {
        assert_eq!(SDL_NumJoysticks(), 2);
        assert_eq!(
            c_string(SDL_JoystickPathForIndex(0)),
            event2.display().to_string()
        );
        assert_eq!(
            c_string(SDL_JoystickPathForIndex(1)),
            event1.display().to_string()
        );
        assert_eq!(
            c_string(SDL_JoystickNameForIndex(0)),
            "SDL Fake evdev Gamepad"
        );
        assert_eq!(SDL_JoystickGetDeviceVendor(0), 0x054c);
        assert_eq!(SDL_JoystickGetDeviceProduct(0), 0x09cc);

        let joystick = SDL_JoystickOpen(0);
        assert!(!joystick.is_null());
        SDL_JoystickUpdate();
        assert!(SDL_JoystickGetAxis(joystick, 0) >= 12000);
        assert_eq!(SDL_JoystickGetButton(joystick, 0), SDL_PRESSED as u8);
        assert_eq!(SDL_JoystickGetHat(joystick, 0), SDL_HAT_RIGHTUP as u8);
        SDL_JoystickClose(joystick);
    }
}

#[test]
#[ignore = "run via xtask run-evdev-fixture-tests"]
fn hinted_device_directory_expands_through_linux_discovery_order() {
    let _serial = testutils::serial_lock();
    let dir = tempdir().expect("tempdir");
    let event10 = dir.path().join("event10");
    let event2 = dir.path().join("event2");
    let event1 = dir.path().join("event1");
    testutils::write_default_evdev_gamepad_fixture(&event10);
    testutils::write_default_evdev_gamepad_fixture(&event2);
    testutils::write_default_evdev_gamepad_fixture(&event1);
    std::fs::write(dir.path().join("notes.txt"), b"ignore").expect("noise file");

    let _hint =
        testutils::HintGuard::set(SDL_HINT_JOYSTICK_DEVICE, &dir.path().display().to_string());
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_JOYSTICK);

    unsafe {
        assert_eq!(SDL_NumJoysticks(), 3);
        assert_eq!(
            c_string(SDL_JoystickPathForIndex(0)),
            event1.display().to_string()
        );
        assert_eq!(
            c_string(SDL_JoystickPathForIndex(1)),
            event2.display().to_string()
        );
        assert_eq!(
            c_string(SDL_JoystickPathForIndex(2)),
            event10.display().to_string()
        );
    }
}
