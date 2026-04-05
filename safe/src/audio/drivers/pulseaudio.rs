use crate::abi::generated_types::AUDIO_F32SYS;
use crate::audio::{AudioDriverDescriptor, DeviceTemplate};

const PLAYBACK: &[DeviceTemplate] = &[DeviceTemplate {
    name: "PulseAudio Default",
    name_cstr: b"PulseAudio Default\0",
    freq: 48_000,
    format: AUDIO_F32SYS as _,
    channels: 6,
    samples: 4096,
}];

const CAPTURE: &[DeviceTemplate] = &[DeviceTemplate {
    name: "PulseAudio Capture",
    name_cstr: b"PulseAudio Capture\0",
    freq: 44_100,
    format: AUDIO_F32SYS as _,
    channels: 1,
    samples: 4096,
}];

pub(crate) const DESCRIPTOR: AudioDriverDescriptor = AudioDriverDescriptor {
    name: "pulseaudio",
    name_cstr: b"pulseaudio\0",
    description: "PulseAudio",
    demand_only: false,
    playback_devices: PLAYBACK,
    capture_devices: CAPTURE,
};
