use std::os::raw::c_int;
use std::ptr;

use crate::abi::generated_types::{
    SDL_Sensor, SDL_SensorID, SDL_SensorType, SDL_SensorType_SDL_SENSOR_INVALID, Uint64,
};
use crate::core::error::{invalid_param_error, set_error_message};

fn unavailable() -> c_int {
    set_error_message("Sensors are not available in this runtime")
}

#[no_mangle]
pub unsafe extern "C" fn SDL_LockSensors() {}

#[no_mangle]
pub unsafe extern "C" fn SDL_UnlockSensors() {}

#[no_mangle]
pub unsafe extern "C" fn SDL_NumSensors() -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SensorGetDeviceName(_device_index: c_int) -> *const libc::c_char {
    ptr::null()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SensorGetDeviceType(_device_index: c_int) -> SDL_SensorType {
    SDL_SensorType_SDL_SENSOR_INVALID
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SensorGetDeviceNonPortableType(_device_index: c_int) -> c_int {
    -1
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SensorGetDeviceInstanceID(_device_index: c_int) -> SDL_SensorID {
    -1
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SensorOpen(_device_index: c_int) -> *mut SDL_Sensor {
    let _ = unavailable();
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SensorFromInstanceID(_instance_id: SDL_SensorID) -> *mut SDL_Sensor {
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SensorGetName(sensor: *mut SDL_Sensor) -> *const libc::c_char {
    if sensor.is_null() {
        let _ = invalid_param_error("sensor");
    }
    ptr::null()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SensorGetType(sensor: *mut SDL_Sensor) -> SDL_SensorType {
    if sensor.is_null() {
        let _ = invalid_param_error("sensor");
    }
    SDL_SensorType_SDL_SENSOR_INVALID
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SensorGetNonPortableType(sensor: *mut SDL_Sensor) -> c_int {
    if sensor.is_null() {
        let _ = invalid_param_error("sensor");
    }
    -1
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SensorGetInstanceID(sensor: *mut SDL_Sensor) -> SDL_SensorID {
    if sensor.is_null() {
        let _ = invalid_param_error("sensor");
    }
    -1
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SensorGetData(
    sensor: *mut SDL_Sensor,
    data: *mut f32,
    num_values: c_int,
) -> c_int {
    if sensor.is_null() {
        return invalid_param_error("sensor");
    }
    if !data.is_null() && num_values > 0 {
        ptr::write_bytes(data, 0, num_values as usize);
    }
    unavailable()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SensorGetDataWithTimestamp(
    sensor: *mut SDL_Sensor,
    timestamp: *mut Uint64,
    data: *mut f32,
    num_values: c_int,
) -> c_int {
    if sensor.is_null() {
        return invalid_param_error("sensor");
    }
    if !timestamp.is_null() {
        *timestamp = 0;
    }
    if !data.is_null() && num_values > 0 {
        ptr::write_bytes(data, 0, num_values as usize);
    }
    unavailable()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SensorClose(_sensor: *mut SDL_Sensor) {}

#[no_mangle]
pub unsafe extern "C" fn SDL_SensorUpdate() {}
