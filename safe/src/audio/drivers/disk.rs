use crate::abi::generated_types::AUDIO_F32SYS;
use crate::audio::{AudioDriverDescriptor, DeviceTemplate};

const PLAYBACK: &[DeviceTemplate] = &[DeviceTemplate {
    name: "Disk Writer",
    name_cstr: b"Disk Writer\0",
    freq: 48_000,
    format: AUDIO_F32SYS as _,
    channels: 2,
    samples: 4096,
}];

pub(crate) const DESCRIPTOR: AudioDriverDescriptor = AudioDriverDescriptor {
    name: "disk",
    name_cstr: b"disk\0",
    description: "direct-to-disk audio",
    demand_only: true,
    playback_devices: PLAYBACK,
    capture_devices: &[],
};
