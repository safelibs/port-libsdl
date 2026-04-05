use std::os::raw::c_int;
use std::ptr;

use crate::abi::generated_types::{
    SDL_Sensor, SDL_SensorID, SDL_SensorType, SDL_SensorType_SDL_SENSOR_INVALID, Uint64,
};
use crate::core::error::{invalid_param_error, set_error_message};

use super::{device_by_instance, lock_input_state, sensor_instance_id, SensorHandle};

#[derive(Clone, Copy)]
struct SensorDescriptor {
    joystick_instance_id: crate::abi::generated_types::SDL_JoystickID,
    sensor_index: usize,
    instance_id: SDL_SensorID,
}

fn update_devices_for_sensors() {
    let mut state = lock_input_state();
    if !state.sensor_initialized {
        return;
    }
    super::refresh_hint_devices(&mut state);
    for device in &mut state.devices {
        if let Some(evdev) = device.evdev.as_mut() {
            let _ = super::linux::evdev::poll_device(evdev, &mut device.state);
        }
        if let Some(callbacks) = device.callbacks {
            if let Some(callback) = callbacks.update {
                unsafe {
                    callback(callbacks.userdata);
                }
            }
        }
        device.state.apply_pending();
        super::refresh_device_features(device);
    }
}

fn descriptors_from_state(state: &super::InputState) -> Vec<SensorDescriptor> {
    state
        .devices
        .iter()
        .flat_map(|device| {
            device
                .sensors
                .iter()
                .enumerate()
                .map(|(sensor_index, _)| SensorDescriptor {
                    joystick_instance_id: device.instance_id,
                    sensor_index,
                    instance_id: sensor_instance_id(device.instance_id, sensor_index),
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn descriptor_for_device_index(
    state: &super::InputState,
    device_index: c_int,
) -> Option<SensorDescriptor> {
    usize::try_from(device_index)
        .ok()
        .and_then(|index| descriptors_from_state(state).get(index).copied())
}

fn descriptor_for_instance_id(
    state: &super::InputState,
    instance_id: SDL_SensorID,
) -> Option<SensorDescriptor> {
    descriptors_from_state(state)
        .into_iter()
        .find(|descriptor| descriptor.instance_id == instance_id)
}

fn descriptor_for_handle(sensor: *mut SDL_Sensor) -> Option<SensorDescriptor> {
    if sensor.is_null() {
        None
    } else {
        let handle = unsafe { &*(sensor as *mut SensorHandle) };
        Some(SensorDescriptor {
            joystick_instance_id: handle.joystick_instance_id,
            sensor_index: handle.sensor_index,
            instance_id: handle.instance_id,
        })
    }
}

fn sensor_name_ptr(state: &super::InputState, descriptor: SensorDescriptor) -> *const libc::c_char {
    device_by_instance(state, descriptor.joystick_instance_id)
        .and_then(|device| device.sensors.get(descriptor.sensor_index))
        .map(|sensor| sensor.name.as_ptr())
        .unwrap_or(ptr::null())
}

fn sensor_type_value(state: &super::InputState, descriptor: SensorDescriptor) -> SDL_SensorType {
    device_by_instance(state, descriptor.joystick_instance_id)
        .and_then(|device| device.sensors.get(descriptor.sensor_index))
        .map(|sensor| sensor.type_)
        .unwrap_or(SDL_SensorType_SDL_SENSOR_INVALID)
}

fn sensor_non_portable_type(state: &super::InputState, descriptor: SensorDescriptor) -> c_int {
    device_by_instance(state, descriptor.joystick_instance_id)
        .and_then(|device| device.sensors.get(descriptor.sensor_index))
        .map(|sensor| sensor.non_portable_type)
        .unwrap_or(-1)
}

fn write_sensor_data(values: &[f32], data: *mut f32, num_values: c_int) {
    if data.is_null() || num_values <= 0 {
        return;
    }
    let requested = num_values as usize;
    unsafe {
        for index in 0..requested {
            *data.add(index) = values.get(index).copied().unwrap_or(0.0);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_LockSensors() {}

#[no_mangle]
pub unsafe extern "C" fn SDL_UnlockSensors() {}

#[no_mangle]
pub unsafe extern "C" fn SDL_NumSensors() -> c_int {
    let mut state = lock_input_state();
    if !state.sensor_initialized {
        return 0;
    }
    super::refresh_hint_devices(&mut state);
    for device in &mut state.devices {
        super::refresh_device_features(device);
    }
    descriptors_from_state(&state).len() as c_int
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SensorGetDeviceName(device_index: c_int) -> *const libc::c_char {
    let mut state = lock_input_state();
    if !state.sensor_initialized {
        return ptr::null();
    }
    super::refresh_hint_devices(&mut state);
    let Some(descriptor) = descriptor_for_device_index(&state, device_index) else {
        return ptr::null();
    };
    sensor_name_ptr(&state, descriptor)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SensorGetDeviceType(device_index: c_int) -> SDL_SensorType {
    let mut state = lock_input_state();
    if !state.sensor_initialized {
        return SDL_SensorType_SDL_SENSOR_INVALID;
    }
    super::refresh_hint_devices(&mut state);
    descriptor_for_device_index(&state, device_index)
        .map(|descriptor| sensor_type_value(&state, descriptor))
        .unwrap_or(SDL_SensorType_SDL_SENSOR_INVALID)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SensorGetDeviceNonPortableType(device_index: c_int) -> c_int {
    let mut state = lock_input_state();
    if !state.sensor_initialized {
        return -1;
    }
    super::refresh_hint_devices(&mut state);
    descriptor_for_device_index(&state, device_index)
        .map(|descriptor| sensor_non_portable_type(&state, descriptor))
        .unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SensorGetDeviceInstanceID(device_index: c_int) -> SDL_SensorID {
    let mut state = lock_input_state();
    if !state.sensor_initialized {
        return -1;
    }
    super::refresh_hint_devices(&mut state);
    descriptor_for_device_index(&state, device_index)
        .map(|descriptor| descriptor.instance_id)
        .unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SensorOpen(device_index: c_int) -> *mut SDL_Sensor {
    let mut state = lock_input_state();
    if !state.sensor_initialized {
        let _ = set_error_message("Sensor subsystem isn't initialized");
        return ptr::null_mut();
    }
    super::refresh_hint_devices(&mut state);
    let Some(descriptor) = descriptor_for_device_index(&state, device_index) else {
        let _ = set_error_message("Invalid sensor device index");
        return ptr::null_mut();
    };
    let handle = Box::into_raw(Box::new(SensorHandle {
        instance_id: descriptor.instance_id,
        joystick_instance_id: descriptor.joystick_instance_id,
        sensor_index: descriptor.sensor_index,
    }));
    state
        .open_sensors
        .push((handle as usize, descriptor.instance_id));
    handle.cast()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SensorFromInstanceID(instance_id: SDL_SensorID) -> *mut SDL_Sensor {
    let state = lock_input_state();
    state
        .open_sensors
        .iter()
        .find(|(_, open_id)| *open_id == instance_id)
        .map(|(ptr, _)| *ptr as *mut SDL_Sensor)
        .unwrap_or(ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SensorGetName(sensor: *mut SDL_Sensor) -> *const libc::c_char {
    let Some(descriptor) = descriptor_for_handle(sensor) else {
        let _ = invalid_param_error("sensor");
        return ptr::null();
    };
    let state = lock_input_state();
    sensor_name_ptr(&state, descriptor)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SensorGetType(sensor: *mut SDL_Sensor) -> SDL_SensorType {
    let Some(descriptor) = descriptor_for_handle(sensor) else {
        let _ = invalid_param_error("sensor");
        return SDL_SensorType_SDL_SENSOR_INVALID;
    };
    let state = lock_input_state();
    sensor_type_value(&state, descriptor)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SensorGetNonPortableType(sensor: *mut SDL_Sensor) -> c_int {
    let Some(descriptor) = descriptor_for_handle(sensor) else {
        let _ = invalid_param_error("sensor");
        return -1;
    };
    let state = lock_input_state();
    sensor_non_portable_type(&state, descriptor)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SensorGetInstanceID(sensor: *mut SDL_Sensor) -> SDL_SensorID {
    descriptor_for_handle(sensor)
        .map(|descriptor| descriptor.instance_id)
        .unwrap_or_else(|| {
            let _ = invalid_param_error("sensor");
            -1
        })
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SensorGetData(
    sensor: *mut SDL_Sensor,
    data: *mut f32,
    num_values: c_int,
) -> c_int {
    SDL_SensorGetDataWithTimestamp(sensor, ptr::null_mut(), data, num_values)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SensorGetDataWithTimestamp(
    sensor: *mut SDL_Sensor,
    timestamp: *mut Uint64,
    data: *mut f32,
    num_values: c_int,
) -> c_int {
    if !timestamp.is_null() {
        *timestamp = 0;
    }
    write_sensor_data(&[], data, num_values);
    let Some(descriptor) = descriptor_for_handle(sensor) else {
        return invalid_param_error("sensor");
    };
    let state = lock_input_state();
    let Some(device) = device_by_instance(&state, descriptor.joystick_instance_id) else {
        return set_error_message("Sensor is no longer attached");
    };
    let Some(sensor_state) = device.sensors.get(descriptor.sensor_index) else {
        return set_error_message("Sensor is no longer available");
    };
    if !sensor_state.enabled {
        return set_error_message("Sensor is disabled");
    }
    if !timestamp.is_null() {
        *timestamp = sensor_state.timestamp_us;
    }
    write_sensor_data(&sensor_state.values, data, num_values);
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SensorClose(sensor: *mut SDL_Sensor) {
    if sensor.is_null() {
        return;
    }
    let mut state = lock_input_state();
    super::close_sensor_handle(&mut state, sensor.cast::<SensorHandle>());
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SensorUpdate() {
    update_devices_for_sensors();
}
