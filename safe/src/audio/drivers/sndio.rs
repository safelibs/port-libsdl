use crate::abi::generated_types::AUDIO_F32SYS;
use crate::audio::{AudioDriverDescriptor, DeviceTemplate};

const PLAYBACK: &[DeviceTemplate] = &[DeviceTemplate {
    name: "sndio Default",
    name_cstr: b"sndio Default\0",
    freq: 48_000,
    format: AUDIO_F32SYS as _,
    channels: 2,
    samples: 4096,
}];

const CAPTURE: &[DeviceTemplate] = &[DeviceTemplate {
    name: "sndio Capture",
    name_cstr: b"sndio Capture\0",
    freq: 44_100,
    format: AUDIO_F32SYS as _,
    channels: 1,
    samples: 4096,
}];

pub(crate) const DESCRIPTOR: AudioDriverDescriptor = AudioDriverDescriptor {
    name: "sndio",
    name_cstr: b"sndio\0",
    description: "OpenBSD sndio",
    demand_only: false,
    playback_devices: PLAYBACK,
    capture_devices: CAPTURE,
};
