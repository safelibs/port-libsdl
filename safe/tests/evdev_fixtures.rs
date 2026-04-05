#[path = "common/testutils.rs"]
mod testutils;

use tempfile::tempdir;

use safe_sdl::abi::generated_types::{SDL_HINT_JOYSTICK_DEVICE, SDL_INIT_JOYSTICK};
use safe_sdl::input::joystick::{
    SDL_JoystickGetDeviceProduct, SDL_JoystickGetDeviceVendor, SDL_JoystickNameForIndex,
    SDL_JoystickPathForIndex, SDL_NumJoysticks,
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
    assert_eq!(names, vec!["event1", "event2", "event10", "js2", "js3", "js11"]);
}

#[test]
fn hinted_evdev_devices_appear_in_hint_order_with_synthetic_metadata() {
    let _serial = testutils::serial_lock();
    let _hint = testutils::HintGuard::set(
        SDL_HINT_JOYSTICK_DEVICE,
        "/tmp/fixture-event2:/tmp/fixture-event1",
    );
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_JOYSTICK);

    unsafe {
        assert_eq!(SDL_NumJoysticks(), 2);
        assert_eq!(
            c_string(SDL_JoystickPathForIndex(0)),
            "/tmp/fixture-event2"
        );
        assert_eq!(
            c_string(SDL_JoystickPathForIndex(1)),
            "/tmp/fixture-event1"
        );
        assert_eq!(
            c_string(SDL_JoystickNameForIndex(0)),
            "SDL Fake evdev Gamepad"
        );
        assert_eq!(SDL_JoystickGetDeviceVendor(0), 0x054c);
        assert_eq!(SDL_JoystickGetDeviceProduct(0), 0x09cc);
    }
}
