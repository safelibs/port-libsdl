#![allow(non_upper_case_globals)]
#![allow(clippy::all)]

#[path = "common/testutils.rs"]
mod testutils;

use std::mem::MaybeUninit;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use safe_sdl::abi::generated_types::{SDL_AudioCVT, SDL_AudioSpec, AUDIO_S16SYS, SDL_INIT_AUDIO};
use safe_sdl::audio::convert::{SDL_BuildAudioCVT, SDL_ConvertAudio};
use safe_sdl::audio::device::{
    SDL_AudioQuit, SDL_CloseAudioDevice, SDL_DequeueAudio, SDL_GetAudioDeviceName,
    SDL_GetCurrentAudioDriver, SDL_GetDefaultAudioInfo, SDL_GetNumAudioDevices,
    SDL_GetQueuedAudioSize, SDL_OpenAudioDevice, SDL_PauseAudioDevice, SDL_QueueAudio,
};
use safe_sdl::audio::wave::{SDL_FreeWAV, SDL_LoadWAV_RW};
use safe_sdl::core::init::SDL_QuitSubSystem;
use safe_sdl::core::memory::SDL_free;
use safe_sdl::core::rwops::{
    SDL_RWFromConstMem, SDL_RWFromFile, SDL_RWclose, SDL_RWwrite, SDL_WriteLE16, SDL_WriteLE32,
};

static LOOP_CALLBACKS: AtomicUsize = AtomicUsize::new(0);
static MULTI_CALLBACK_A: AtomicUsize = AtomicUsize::new(0);
static MULTI_CALLBACK_B: AtomicUsize = AtomicUsize::new(0);
static SURROUND_ACTIVE_CHANNEL: AtomicUsize = AtomicUsize::new(0);
static SURROUND_CALLBACKS: AtomicUsize = AtomicUsize::new(0);

struct LoadedWave {
    spec: SDL_AudioSpec,
    data: *mut u8,
    len: u32,
}

impl LoadedWave {
    unsafe fn bytes(&self) -> &[u8] {
        std::slice::from_raw_parts(self.data, self.len as usize)
    }
}

impl Drop for LoadedWave {
    fn drop(&mut self) {
        unsafe {
            SDL_FreeWAV(self.data);
        }
    }
}

struct LoopingWave {
    data: Vec<u8>,
    position: usize,
}

struct OneShotWave {
    slot: usize,
    silence: u8,
    data: Vec<u8>,
    position: usize,
}

fn set_audio_driver(name: &str) -> testutils::ScopedEnvVar {
    let env = testutils::ScopedEnvVar::set("SDL_AUDIODRIVER", name);
    unsafe {
        SDL_AudioQuit();
        SDL_QuitSubSystem(SDL_INIT_AUDIO);
    }
    env
}

fn wait_until(timeout: Duration, predicate: impl Fn() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if predicate() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    predicate()
}

fn generated_pcm_wave_bytes() -> Vec<u8> {
    let sample_rate = 22_050u32;
    let channels = 1u16;
    let bits_per_sample = 16u16;
    let frames = sample_rate as usize / 4;
    let mut pcm = Vec::with_capacity(frames * 2);
    for index in 0..frames {
        let phase = (index as f32 * 440.0 * std::f32::consts::TAU) / sample_rate as f32;
        let sample = (phase.sin() * i16::MAX as f32 * 0.5) as i16;
        pcm.extend_from_slice(&sample.to_le_bytes());
    }

    let block_align = channels * (bits_per_sample / 8);
    let avg_bytes = sample_rate * block_align as u32;
    let mut bytes = Vec::with_capacity(44 + pcm.len());
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(pcm.len() as u32 + 36).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&avg_bytes.to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&bits_per_sample.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(pcm.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&pcm);
    bytes
}

unsafe fn load_generated_wave() -> LoadedWave {
    let bytes = generated_pcm_wave_bytes();
    let rw = SDL_RWFromConstMem(bytes.as_ptr().cast(), bytes.len() as i32);
    assert!(!rw.is_null(), "{}", testutils::current_error());

    let mut spec = MaybeUninit::<SDL_AudioSpec>::zeroed();
    let mut data = ptr::null_mut();
    let mut len = 0u32;
    assert!(
        !SDL_LoadWAV_RW(rw, 1, spec.as_mut_ptr(), &mut data, &mut len).is_null(),
        "{}",
        testutils::current_error()
    );
    LoadedWave {
        spec: spec.assume_init(),
        data,
        len,
    }
}

unsafe extern "C" fn looping_wave_callback(
    userdata: *mut libc::c_void,
    stream: *mut u8,
    len: libc::c_int,
) {
    let state = &mut *(userdata as *mut LoopingWave);
    let len = len.max(0) as usize;
    let out = std::slice::from_raw_parts_mut(stream, len);
    let mut written = 0usize;

    LOOP_CALLBACKS.fetch_add(1, Ordering::SeqCst);
    while written < out.len() && !state.data.is_empty() {
        let remaining = state.data.len() - state.position;
        let chunk = remaining.min(out.len() - written);
        out[written..written + chunk]
            .copy_from_slice(&state.data[state.position..state.position + chunk]);
        written += chunk;
        state.position += chunk;
        if state.position == state.data.len() {
            state.position = 0;
        }
    }
}

unsafe extern "C" fn one_shot_wave_callback(
    userdata: *mut libc::c_void,
    stream: *mut u8,
    len: libc::c_int,
) {
    let state = &mut *(userdata as *mut OneShotWave);
    let len = len.max(0) as usize;
    let out = std::slice::from_raw_parts_mut(stream, len);
    out.fill(state.silence);

    let remaining = state.data.len().saturating_sub(state.position);
    let chunk = remaining.min(out.len());
    if chunk > 0 {
        out[..chunk].copy_from_slice(&state.data[state.position..state.position + chunk]);
        state.position += chunk;
    }

    match state.slot {
        0 => MULTI_CALLBACK_A.fetch_add(1, Ordering::SeqCst),
        _ => MULTI_CALLBACK_B.fetch_add(1, Ordering::SeqCst),
    };
}

fn surround_channel_name(channel_index: usize, channel_count: usize) -> &'static str {
    match channel_index {
        0 => "Front Left",
        1 => "Front Right",
        2 => match channel_count {
            3 | 5 => "Low Frequency Effects",
            4 => "Back Left",
            _ => "Front Center",
        },
        3 => match channel_count {
            4 => "Back Right",
            5 => "Back Left",
            _ => "Low Frequency Effects",
        },
        4 => match channel_count {
            5 => "Back Right",
            6 => "Side Left",
            7 => "Back Center",
            8 => "Back Left",
            _ => "Unknown",
        },
        5 => match channel_count {
            6 => "Side Right",
            7 => "Side Left",
            8 => "Back Right",
            _ => "Unknown",
        },
        6 => match channel_count {
            7 => "Side Right",
            8 => "Side Left",
            _ => "Unknown",
        },
        7 => "Side Right",
        _ => "Unknown",
    }
}

unsafe extern "C" fn surround_callback(
    userdata: *mut libc::c_void,
    stream: *mut u8,
    len: libc::c_int,
) {
    let total_channels = userdata as usize;
    let active_channel = SURROUND_ACTIVE_CHANNEL.load(Ordering::SeqCst);
    let buffer = std::slice::from_raw_parts_mut(stream.cast::<i16>(), (len.max(0) as usize) / 2);
    buffer.fill(0);

    if active_channel < total_channels {
        for frame in buffer.chunks_exact_mut(total_channels) {
            frame[active_channel] = 0x4000;
        }
        if SURROUND_CALLBACKS.fetch_add(1, Ordering::SeqCst) % 2 == 1 {
            SURROUND_ACTIVE_CHANNEL.fetch_add(1, Ordering::SeqCst);
        }
    }
}

unsafe fn write_pcm_wave(
    path: &std::path::Path,
    sample_rate: i32,
    channels: u16,
    bits_per_sample: u16,
    pcm: &[u8],
) {
    let path_c = testutils::cstring(path.to_str().expect("utf-8 temp path"));
    let mode_c = testutils::cstring("wb");
    let rw = SDL_RWFromFile(path_c.as_ptr(), mode_c.as_ptr());
    assert!(!rw.is_null(), "{}", testutils::current_error());

    let block_align = channels * (bits_per_sample / 8);
    let avg_bytes = sample_rate as u32 * block_align as u32;

    assert_eq!(SDL_WriteLE32(rw, 0x46464952), 1);
    assert_eq!(SDL_WriteLE32(rw, pcm.len() as u32 + 36), 1);
    assert_eq!(SDL_WriteLE32(rw, 0x45564157), 1);
    assert_eq!(SDL_WriteLE32(rw, 0x20746D66), 1);
    assert_eq!(SDL_WriteLE32(rw, 16), 1);
    assert_eq!(SDL_WriteLE16(rw, 1), 1);
    assert_eq!(SDL_WriteLE16(rw, channels), 1);
    assert_eq!(SDL_WriteLE32(rw, sample_rate as u32), 1);
    assert_eq!(SDL_WriteLE32(rw, avg_bytes), 1);
    assert_eq!(SDL_WriteLE16(rw, block_align), 1);
    assert_eq!(SDL_WriteLE16(rw, bits_per_sample), 1);
    assert_eq!(SDL_WriteLE32(rw, 0x61746164), 1);
    assert_eq!(SDL_WriteLE32(rw, pcm.len() as u32), 1);
    assert_eq!(SDL_RWwrite(rw, pcm.as_ptr().cast(), pcm.len(), 1), 1);
    assert_eq!(SDL_RWclose(rw), 0);
}

#[test]
fn testaudioinfo_port_reports_inventory_and_default_specs() {
    let _serial = testutils::serial_lock();
    let _driver = set_audio_driver("dummy");
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_AUDIO);

    unsafe {
        assert_eq!(
            testutils::string_from_c(SDL_GetCurrentAudioDriver()),
            "dummy"
        );
        assert_eq!(SDL_GetNumAudioDevices(0), 1);
        assert_eq!(SDL_GetNumAudioDevices(1), 1);
        assert!(!SDL_GetAudioDeviceName(0, 0).is_null());
        assert!(!SDL_GetAudioDeviceName(0, 1).is_null());

        let mut name = ptr::null_mut();
        let mut spec = MaybeUninit::<SDL_AudioSpec>::zeroed();
        assert_eq!(
            SDL_GetDefaultAudioInfo(&mut name, spec.as_mut_ptr(), 0),
            0,
            "{}",
            testutils::current_error()
        );
        let spec = spec.assume_init();
        assert!(spec.freq > 0);
        assert!(spec.channels >= 1);
        assert!(!name.is_null());
        SDL_free(name.cast());
    }
}

#[test]
fn loopwave_and_loopwavequeue_ports_keep_callback_and_queue_playback_alive() {
    let _serial = testutils::serial_lock();
    let _driver = set_audio_driver("dummy");
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_AUDIO);

    unsafe {
        let wave = load_generated_wave();

        LOOP_CALLBACKS.store(0, Ordering::SeqCst);
        let callback_state = Box::new(LoopingWave {
            data: wave.bytes().to_vec(),
            position: 0,
        });
        let callback_ptr = Box::into_raw(callback_state);
        let callback_spec = SDL_AudioSpec {
            callback: Some(looping_wave_callback),
            userdata: callback_ptr.cast(),
            ..wave.spec
        };

        let playback = SDL_OpenAudioDevice(ptr::null(), 0, &callback_spec, ptr::null_mut(), 0);
        assert!(playback > 1, "{}", testutils::current_error());
        SDL_PauseAudioDevice(playback, 0);
        assert!(wait_until(Duration::from_secs(1), || LOOP_CALLBACKS
            .load(Ordering::SeqCst)
            > 0));
        SDL_CloseAudioDevice(playback);
        drop(Box::from_raw(callback_ptr));

        let queue_spec = SDL_AudioSpec {
            callback: None,
            userdata: ptr::null_mut(),
            ..wave.spec
        };
        let queued = SDL_OpenAudioDevice(ptr::null(), 0, &queue_spec, ptr::null_mut(), 0);
        assert!(queued > 1, "{}", testutils::current_error());
        assert_eq!(SDL_QueueAudio(queued, wave.data.cast(), wave.len), 0);
        assert!(SDL_GetQueuedAudioSize(queued) >= wave.len);
        SDL_PauseAudioDevice(queued, 0);
        assert!(wait_until(
            Duration::from_secs(1),
            || SDL_GetQueuedAudioSize(queued) < wave.len
        ));
        SDL_CloseAudioDevice(queued);
    }
}

#[test]
fn testaudiocapture_and_testaudiohotplug_ports_cover_capture_bridge_and_named_reopen() {
    let _serial = testutils::serial_lock();
    let _driver = set_audio_driver("dummy");
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_AUDIO);

    unsafe {
        let wanted = SDL_AudioSpec {
            freq: 44_100,
            format: AUDIO_S16SYS as _,
            channels: 1,
            silence: 0,
            samples: 1024,
            padding: 0,
            size: 0,
            callback: None,
            userdata: ptr::null_mut(),
        };

        let output_name = SDL_GetAudioDeviceName(0, 0);
        let capture_name = SDL_GetAudioDeviceName(0, 1);
        assert!(!output_name.is_null());
        assert!(!capture_name.is_null());

        let playback = SDL_OpenAudioDevice(output_name, 0, &wanted, ptr::null_mut(), 0);
        assert!(playback > 1, "{}", testutils::current_error());
        let capture = SDL_OpenAudioDevice(capture_name, 1, &wanted, ptr::null_mut(), 0);
        assert!(capture > 1, "{}", testutils::current_error());

        SDL_PauseAudioDevice(playback, 0);
        SDL_PauseAudioDevice(capture, 0);

        let mut moved = 0usize;
        for _ in 0..16 {
            let mut buffer = [0u8; 1024];
            let read = SDL_DequeueAudio(capture, buffer.as_mut_ptr().cast(), buffer.len() as u32);
            if read > 0 {
                moved += read as usize;
                assert_eq!(SDL_QueueAudio(playback, buffer.as_ptr().cast(), read), 0);
                if moved >= buffer.len() {
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(moved > 0);
        assert!(SDL_GetQueuedAudioSize(playback) > 0);

        SDL_CloseAudioDevice(capture);
        SDL_CloseAudioDevice(playback);

        let reopened = SDL_OpenAudioDevice(output_name, 0, &wanted, ptr::null_mut(), 0);
        assert!(reopened > 1, "{}", testutils::current_error());
        SDL_CloseAudioDevice(reopened);
    }
}

#[test]
fn testmultiaudio_port_replays_sound_across_sequential_and_parallel_device_opens() {
    let _serial = testutils::serial_lock();
    let _driver = set_audio_driver("dummy");
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_AUDIO);

    unsafe {
        let wave = load_generated_wave();
        let device_name = SDL_GetAudioDeviceName(0, 0);
        assert!(!device_name.is_null());

        MULTI_CALLBACK_A.store(0, Ordering::SeqCst);
        MULTI_CALLBACK_B.store(0, Ordering::SeqCst);

        let first_state = Box::new(OneShotWave {
            slot: 0,
            silence: wave.spec.silence,
            data: wave.bytes().to_vec(),
            position: 0,
        });
        let first_ptr = Box::into_raw(first_state);
        let desired = SDL_AudioSpec {
            callback: Some(one_shot_wave_callback),
            userdata: first_ptr.cast(),
            ..wave.spec
        };

        let sequential = SDL_OpenAudioDevice(device_name, 0, &desired, ptr::null_mut(), 0);
        assert!(sequential > 1, "{}", testutils::current_error());
        SDL_PauseAudioDevice(sequential, 0);
        assert!(wait_until(Duration::from_secs(1), || MULTI_CALLBACK_A
            .load(Ordering::SeqCst)
            > 0));
        SDL_CloseAudioDevice(sequential);
        drop(Box::from_raw(first_ptr));

        let parallel_state_a = Box::new(OneShotWave {
            slot: 0,
            silence: wave.spec.silence,
            data: wave.bytes().to_vec(),
            position: 0,
        });
        let parallel_state_b = Box::new(OneShotWave {
            slot: 1,
            silence: wave.spec.silence,
            data: wave.bytes().to_vec(),
            position: 0,
        });
        let parallel_ptr_a = Box::into_raw(parallel_state_a);
        let parallel_ptr_b = Box::into_raw(parallel_state_b);
        let desired_a = SDL_AudioSpec {
            callback: Some(one_shot_wave_callback),
            userdata: parallel_ptr_a.cast(),
            ..wave.spec
        };
        let desired_b = SDL_AudioSpec {
            callback: Some(one_shot_wave_callback),
            userdata: parallel_ptr_b.cast(),
            ..wave.spec
        };

        let dev_a = SDL_OpenAudioDevice(device_name, 0, &desired_a, ptr::null_mut(), 0);
        let dev_b = SDL_OpenAudioDevice(device_name, 0, &desired_b, ptr::null_mut(), 0);
        assert!(dev_a > 1, "{}", testutils::current_error());
        assert!(dev_b > 1, "{}", testutils::current_error());
        SDL_PauseAudioDevice(dev_a, 0);
        SDL_PauseAudioDevice(dev_b, 0);

        assert!(wait_until(Duration::from_secs(1), || {
            MULTI_CALLBACK_A.load(Ordering::SeqCst) > 1
                && MULTI_CALLBACK_B.load(Ordering::SeqCst) > 0
        }));

        SDL_CloseAudioDevice(dev_b);
        SDL_CloseAudioDevice(dev_a);
        drop(Box::from_raw(parallel_ptr_b));
        drop(Box::from_raw(parallel_ptr_a));
    }
}

#[test]
fn testresample_port_converts_wave_and_writes_reloadable_output() {
    let _serial = testutils::serial_lock();
    let _driver = set_audio_driver("dummy");
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_AUDIO);

    unsafe {
        let wave = load_generated_wave();
        let mut cvt = MaybeUninit::<SDL_AudioCVT>::zeroed().assume_init();
        assert_eq!(
            SDL_BuildAudioCVT(
                &mut cvt,
                wave.spec.format,
                wave.spec.channels,
                wave.spec.freq,
                AUDIO_S16SYS as _,
                1,
                16_000,
            ),
            1
        );

        let mut converted = vec![0u8; wave.len as usize * cvt.len_mult as usize];
        converted[..wave.len as usize].copy_from_slice(wave.bytes());
        cvt.buf = converted.as_mut_ptr();
        cvt.len = wave.len as i32;
        assert_eq!(
            SDL_ConvertAudio(&mut cvt),
            0,
            "{}",
            testutils::current_error()
        );

        let output_dir = tempfile::tempdir().expect("create tempdir");
        let output_path = output_dir.path().join("resampled.wav");
        write_pcm_wave(
            &output_path,
            16_000,
            1,
            16,
            &converted[..cvt.len_cvt as usize],
        );

        let output_path_c = testutils::cstring(output_path.to_str().expect("utf-8 temp path"));
        let mode_c = testutils::cstring("rb");
        let rw = SDL_RWFromFile(output_path_c.as_ptr(), mode_c.as_ptr());
        assert!(!rw.is_null(), "{}", testutils::current_error());

        let mut spec = MaybeUninit::<SDL_AudioSpec>::zeroed();
        let mut data = ptr::null_mut();
        let mut len = 0u32;
        assert!(
            !SDL_LoadWAV_RW(rw, 1, spec.as_mut_ptr(), &mut data, &mut len).is_null(),
            "{}",
            testutils::current_error()
        );
        let spec = spec.assume_init();
        assert_eq!(spec.freq, 16_000);
        assert_eq!(spec.channels, 1);
        assert_eq!(spec.format, AUDIO_S16SYS as _);
        assert_eq!(len as usize, cvt.len_cvt as usize);
        SDL_FreeWAV(data);
    }
}

#[test]
fn testsurround_port_cycles_named_channels_on_the_dummy_multichannel_device() {
    let _serial = testutils::serial_lock();
    let _driver = set_audio_driver("dummy");
    let _subsystem = testutils::SubsystemGuard::init(SDL_INIT_AUDIO);

    unsafe {
        let wanted = SDL_AudioSpec {
            freq: 48_000,
            format: AUDIO_S16SYS as _,
            channels: 6,
            silence: 0,
            samples: 1024,
            padding: 0,
            size: 0,
            callback: Some(surround_callback),
            userdata: 6usize as *mut libc::c_void,
        };

        let device =
            SDL_OpenAudioDevice(SDL_GetAudioDeviceName(0, 0), 0, &wanted, ptr::null_mut(), 0);
        assert!(device > 1, "{}", testutils::current_error());

        SURROUND_ACTIVE_CHANNEL.store(0, Ordering::SeqCst);
        SURROUND_CALLBACKS.store(0, Ordering::SeqCst);
        SDL_PauseAudioDevice(device, 0);

        assert_eq!(surround_channel_name(0, 6), "Front Left");
        assert_eq!(surround_channel_name(1, 6), "Front Right");
        assert_eq!(surround_channel_name(2, 6), "Front Center");
        assert_eq!(surround_channel_name(3, 6), "Low Frequency Effects");
        assert_eq!(surround_channel_name(4, 6), "Side Left");
        assert_eq!(surround_channel_name(5, 6), "Side Right");

        assert!(wait_until(Duration::from_secs(1), || {
            SURROUND_ACTIVE_CHANNEL.load(Ordering::SeqCst) >= 6
        }));
        SDL_CloseAudioDevice(device);
    }
}
