use crate::abi::generated_types::AUDIO_F32SYS;
use crate::audio::{AudioDriverDescriptor, DeviceTemplate};

const PLAYBACK: &[DeviceTemplate] = &[DeviceTemplate {
    name: "PipeWire Default",
    name_cstr: b"PipeWire Default\0",
    freq: 48_000,
    format: AUDIO_F32SYS as _,
    channels: 6,
    samples: 4096,
}];

const CAPTURE: &[DeviceTemplate] = &[DeviceTemplate {
    name: "PipeWire Capture",
    name_cstr: b"PipeWire Capture\0",
    freq: 44_100,
    format: AUDIO_F32SYS as _,
    channels: 1,
    samples: 4096,
}];

pub(crate) const DESCRIPTOR: AudioDriverDescriptor = AudioDriverDescriptor {
    name: "pipewire",
    name_cstr: b"pipewire\0",
    description: "Pipewire",
    demand_only: false,
    playback_devices: PLAYBACK,
    capture_devices: CAPTURE,
};
