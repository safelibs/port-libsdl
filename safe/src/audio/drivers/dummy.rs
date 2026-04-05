use crate::abi::generated_types::AUDIO_F32SYS;
use crate::audio::{AudioDriverDescriptor, DeviceTemplate};

const PLAYBACK: &[DeviceTemplate] = &[DeviceTemplate {
    name: "Dummy Playback",
    name_cstr: b"Dummy Playback\0",
    freq: 48_000,
    format: AUDIO_F32SYS as _,
    channels: 6,
    samples: 4096,
}];

const CAPTURE: &[DeviceTemplate] = &[DeviceTemplate {
    name: "Dummy Capture",
    name_cstr: b"Dummy Capture\0",
    freq: 44_100,
    format: AUDIO_F32SYS as _,
    channels: 1,
    samples: 4096,
}];

pub(crate) const DESCRIPTOR: AudioDriverDescriptor = AudioDriverDescriptor {
    name: "dummy",
    name_cstr: b"dummy\0",
    description: "SDL dummy audio driver",
    demand_only: true,
    playback_devices: PLAYBACK,
    capture_devices: CAPTURE,
};
