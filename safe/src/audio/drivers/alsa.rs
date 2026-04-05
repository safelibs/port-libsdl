use crate::abi::generated_types::AUDIO_F32SYS;
use crate::audio::{AudioDriverDescriptor, DeviceTemplate};

const PLAYBACK: &[DeviceTemplate] = &[DeviceTemplate {
    name: "ALSA Default",
    name_cstr: b"ALSA Default\0",
    freq: 48_000,
    format: AUDIO_F32SYS as _,
    channels: 6,
    samples: 4096,
}];

const CAPTURE: &[DeviceTemplate] = &[DeviceTemplate {
    name: "ALSA Capture",
    name_cstr: b"ALSA Capture\0",
    freq: 44_100,
    format: AUDIO_F32SYS as _,
    channels: 1,
    samples: 4096,
}];

pub(crate) const DESCRIPTOR: AudioDriverDescriptor = AudioDriverDescriptor {
    name: "alsa",
    name_cstr: b"alsa\0",
    description: "ALSA PCM audio",
    demand_only: false,
    playback_devices: PLAYBACK,
    capture_devices: CAPTURE,
};
