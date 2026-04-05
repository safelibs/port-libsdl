use crate::abi::generated_types::{
    SDL_AudioFormat, SDL_AudioSpec, AUDIO_F32LSB, AUDIO_F32MSB, AUDIO_S16LSB, AUDIO_S16MSB,
    AUDIO_S32LSB, AUDIO_S32MSB, AUDIO_S8, AUDIO_U16LSB, AUDIO_U16MSB, AUDIO_U8,
};

const AUDIO_F32LSB_FMT: SDL_AudioFormat = AUDIO_F32LSB as SDL_AudioFormat;
const AUDIO_F32MSB_FMT: SDL_AudioFormat = AUDIO_F32MSB as SDL_AudioFormat;
const AUDIO_S16LSB_FMT: SDL_AudioFormat = AUDIO_S16LSB as SDL_AudioFormat;
const AUDIO_S16MSB_FMT: SDL_AudioFormat = AUDIO_S16MSB as SDL_AudioFormat;
const AUDIO_S32LSB_FMT: SDL_AudioFormat = AUDIO_S32LSB as SDL_AudioFormat;
const AUDIO_S32MSB_FMT: SDL_AudioFormat = AUDIO_S32MSB as SDL_AudioFormat;
const AUDIO_S8_FMT: SDL_AudioFormat = AUDIO_S8 as SDL_AudioFormat;
const AUDIO_U16LSB_FMT: SDL_AudioFormat = AUDIO_U16LSB as SDL_AudioFormat;
const AUDIO_U16MSB_FMT: SDL_AudioFormat = AUDIO_U16MSB as SDL_AudioFormat;
const AUDIO_U8_FMT: SDL_AudioFormat = AUDIO_U8 as SDL_AudioFormat;

pub mod convert;
pub mod device;
pub mod resample;
pub mod stream;
pub mod wave;

pub mod drivers {
    pub mod alsa;
    pub mod disk;
    pub mod dsp;
    pub mod dummy;
    pub mod pipewire;
    pub mod pulseaudio;
    pub mod sndio;
}

#[derive(Clone, Copy)]
pub(crate) struct DeviceTemplate {
    pub name: &'static str,
    pub name_cstr: &'static [u8],
    pub freq: i32,
    pub format: SDL_AudioFormat,
    pub channels: u8,
    pub samples: u16,
}

#[derive(Clone, Copy)]
pub(crate) struct AudioDriverDescriptor {
    pub name: &'static str,
    pub name_cstr: &'static [u8],
    pub description: &'static str,
    pub demand_only: bool,
    pub playback_devices: &'static [DeviceTemplate],
    pub capture_devices: &'static [DeviceTemplate],
}

pub(crate) fn bytes_per_sample(format: SDL_AudioFormat) -> Option<usize> {
    let bits = (format & 0x00ff) as usize;
    match bits {
        8 | 16 | 32 => Some(bits / 8),
        _ => None,
    }
}

pub(crate) fn is_float_format(format: SDL_AudioFormat) -> bool {
    matches!(format, AUDIO_F32LSB_FMT | AUDIO_F32MSB_FMT)
}

pub(crate) fn is_signed_format(format: SDL_AudioFormat) -> bool {
    matches!(
        format,
        AUDIO_S8_FMT
            | AUDIO_S16LSB_FMT
            | AUDIO_S16MSB_FMT
            | AUDIO_S32LSB_FMT
            | AUDIO_S32MSB_FMT
            | AUDIO_F32LSB_FMT
            | AUDIO_F32MSB_FMT
    )
}

pub(crate) fn is_big_endian_format(format: SDL_AudioFormat) -> bool {
    matches!(
        format,
        AUDIO_S16MSB_FMT | AUDIO_U16MSB_FMT | AUDIO_S32MSB_FMT | AUDIO_F32MSB_FMT
    )
}

pub(crate) fn is_supported_format(format: SDL_AudioFormat) -> bool {
    matches!(
        format,
        AUDIO_S8_FMT
            | AUDIO_U8_FMT
            | AUDIO_S16LSB_FMT
            | AUDIO_S16MSB_FMT
            | AUDIO_U16LSB_FMT
            | AUDIO_U16MSB_FMT
            | AUDIO_S32LSB_FMT
            | AUDIO_S32MSB_FMT
            | AUDIO_F32LSB_FMT
            | AUDIO_F32MSB_FMT
    )
}

pub(crate) fn silence_value(format: SDL_AudioFormat) -> u8 {
    if matches!(format, AUDIO_U8_FMT) {
        0x80
    } else {
        0x00
    }
}

pub(crate) fn frame_size(format: SDL_AudioFormat, channels: u8) -> Option<usize> {
    bytes_per_sample(format)?.checked_mul(channels as usize)
}

pub(crate) fn normalize_audio_spec(spec: &mut SDL_AudioSpec) -> Result<(), &'static str> {
    if spec.freq <= 0 {
        return Err("desired->freq");
    }
    if spec.channels == 0 || spec.channels > 8 {
        return Err("desired->channels");
    }
    if !is_supported_format(spec.format) {
        return Err("desired->format");
    }

    if spec.samples == 0 {
        spec.samples = 4096;
    }

    let frame_size = frame_size(spec.format, spec.channels).ok_or("desired->format")?;
    let size = frame_size
        .checked_mul(spec.samples as usize)
        .ok_or("desired->samples")?;
    let size = u32::try_from(size).map_err(|_| "desired->samples")?;

    spec.silence = silence_value(spec.format);
    spec.padding = 0;
    spec.size = size;
    Ok(())
}
