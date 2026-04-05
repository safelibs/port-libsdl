use crate::abi::generated_types::AUDIO_F32SYS;
use crate::audio::{AudioDriverDescriptor, DeviceTemplate};

const PLAYBACK: &[DeviceTemplate] = &[DeviceTemplate {
    name: "OSS /dev/dsp",
    name_cstr: b"OSS /dev/dsp\0",
    freq: 48_000,
    format: AUDIO_F32SYS as _,
    channels: 2,
    samples: 4096,
}];

const CAPTURE: &[DeviceTemplate] = &[DeviceTemplate {
    name: "OSS Capture",
    name_cstr: b"OSS Capture\0",
    freq: 44_100,
    format: AUDIO_F32SYS as _,
    channels: 1,
    samples: 4096,
}];

pub(crate) const DESCRIPTOR: AudioDriverDescriptor = AudioDriverDescriptor {
    name: "dsp",
    name_cstr: b"dsp\0",
    description: "OSS /dev/dsp standard audio",
    demand_only: false,
    playback_devices: PLAYBACK,
    capture_devices: CAPTURE,
};
