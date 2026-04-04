use std::fs;
use std::path::Path;

use crate::abi::generated_types::{
    SDL_PowerState, SDL_PowerState_SDL_POWERSTATE_CHARGED, SDL_PowerState_SDL_POWERSTATE_CHARGING,
    SDL_PowerState_SDL_POWERSTATE_NO_BATTERY, SDL_PowerState_SDL_POWERSTATE_ON_BATTERY,
    SDL_PowerState_SDL_POWERSTATE_UNKNOWN,
};

fn read_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|text| text.trim().to_string())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetPowerInfo(
    seconds: *mut libc::c_int,
    percent: *mut libc::c_int,
) -> SDL_PowerState {
    let seconds_value = -1;
    let mut percent_value = -1;
    let mut state = SDL_PowerState_SDL_POWERSTATE_UNKNOWN;
    let mut found_battery = false;
    let mut found_any_supply = false;

    if let Ok(entries) = fs::read_dir("/sys/class/power_supply") {
        for entry in entries.flatten() {
            let path = entry.path();
            let supply_type = read_trimmed(&path.join("type"));
            match supply_type.as_deref() {
                Some("Battery") => {
                    found_battery = true;
                    found_any_supply = true;
                    if let Some(capacity) = read_trimmed(&path.join("capacity"))
                        .and_then(|value| value.parse::<libc::c_int>().ok())
                    {
                        percent_value = capacity.clamp(0, 100);
                    }
                    match read_trimmed(&path.join("status")).as_deref() {
                        Some("Charging") => state = SDL_PowerState_SDL_POWERSTATE_CHARGING,
                        Some("Full") => state = SDL_PowerState_SDL_POWERSTATE_CHARGED,
                        Some("Discharging") => state = SDL_PowerState_SDL_POWERSTATE_ON_BATTERY,
                        _ => {
                            if state == SDL_PowerState_SDL_POWERSTATE_UNKNOWN {
                                state = SDL_PowerState_SDL_POWERSTATE_ON_BATTERY;
                            }
                        }
                    }
                }
                Some("Mains") | Some("AC") => {
                    found_any_supply = true;
                    if state == SDL_PowerState_SDL_POWERSTATE_UNKNOWN {
                        state = SDL_PowerState_SDL_POWERSTATE_CHARGED;
                    }
                }
                Some(_) | None => {}
            }
        }
    }

    if !found_battery && found_any_supply {
        state = SDL_PowerState_SDL_POWERSTATE_NO_BATTERY;
    }

    if !seconds.is_null() {
        *seconds = seconds_value;
    }
    if !percent.is_null() {
        *percent = percent_value;
    }
    state
}
