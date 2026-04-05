#![allow(non_upper_case_globals)]
#![allow(clippy::all)]

#[path = "common/testutils.rs"]
mod testutils;

use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use safe_sdl::abi::generated_types::{
    SDL_AudioCVT, SDL_AudioSpec, AUDIO_F32SYS, AUDIO_S16SYS, AUDIO_U8, SDL_INIT_AUDIO,
};
use safe_sdl::audio::device::{
    SDL_AudioInit, SDL_AudioQuit, SDL_CloseAudio, SDL_CloseAudioDevice, SDL_DequeueAudio,
    SDL_GetAudioDeviceName, SDL_GetAudioDeviceSpec, SDL_GetAudioDeviceStatus, SDL_GetAudioDriver,
    SDL_GetAudioStatus, SDL_GetCurrentAudioDriver, SDL_GetDefaultAudioInfo, SDL_GetNumAudioDevices,
    SDL_GetNumAudioDrivers, SDL_GetQueuedAudioSize, SDL_LockAudioDevice, SDL_OpenAudio,
    SDL_OpenAudioDevice, SDL_PauseAudio, SDL_PauseAudioDevice, SDL_QueueAudio,
    SDL_UnlockAudioDevice,
};
use safe_sdl::audio::stream::{
    SDL_AudioStreamAvailable, SDL_AudioStreamClear, SDL_AudioStreamFlush, SDL_AudioStreamGet,
    SDL_AudioStreamPut, SDL_FreeAudioStream, SDL_NewAudioStream,
};
use safe_sdl::core::init::SDL_QuitSubSystem;
use safe_sdl::core::memory::SDL_free;

static CALLBACK_COUNT: AtomicUsize = AtomicUsize::new(0);
static CALLBACK_BYTES: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn counting_callback(
    _userdata: *mut libc::c_void,
    _stream: *mut u8,
    len: libc::c_int,
) {
    CALLBACK_COUNT.fetch_add(1, Ordering::SeqCst);
    CALLBACK_BYTES.fetch_add(len.max(0) as usize, Ordering::SeqCst);
}

fn cstr_string(ptr: *const libc::c_char) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() }
    }
}

fn audio_format(value: u32) -> u16 {
    value as u16
}

fn make_spec(freq: i32, format: u16, channels: u8, samples: u16) -> SDL_AudioSpec {
    SDL_AudioSpec {
        freq,
        format,
        channels,
        silence: 0,
        samples,
        padding: 0,
        size: 0,
        callback: Some(counting_callback),
        userdata: ptr::null_mut(),
    }
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

fn snr_db(reference: &[f32], output: &[f32]) -> f64 {
    let mut signal = 0.0f64;
    let mut noise = 0.0f64;
    for (&target, &observed) in reference.iter().zip(output.iter()) {
        let delta = observed as f64 - target as f64;
        signal += (target as f64) * (target as f64);
        noise += delta * delta;
    }
    10.0 * (signal / noise.max(1.0e-20)).log10()
}

fn sine_wave_sample(index: usize, rate: i32, freq: i32, phase: f64) -> f32 {
    ((((index as i64 * freq as i64) % rate as i64) as f64 / rate as f64) * std::f64::consts::TAU
        + phase)
        .sin() as f32
}

fn decode_f32_le(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

#[test]
fn audio_driver_inventory_current_driver_and_demand_only_selection_follow_contract() {
    let _serial = testutils::serial_lock();

    unsafe {
        SDL_AudioQuit();
        let _no_hint = testutils::ScopedEnvVar::set("SDL_AUDIODRIVER", "");

        let expected = [
            "pulseaudio",
            "alsa",
            "sndio",
            "pipewire",
            "dsp",
            "disk",
            "dummy",
        ];
        assert_eq!(SDL_GetNumAudioDrivers(), expected.len() as i32);
        for (index, expected_name) in expected.iter().enumerate() {
            assert_eq!(
                cstr_string(SDL_GetAudioDriver(index as i32)),
                *expected_name
            );
        }

        assert_eq!(
            SDL_AudioInit(ptr::null()),
            0,
            "{}",
            testutils::current_error()
        );
        assert_eq!(cstr_string(SDL_GetCurrentAudioDriver()), "pulseaudio");
        SDL_AudioQuit();

        let _dummy = set_audio_driver("dummy");
        assert_eq!(
            SDL_AudioInit(ptr::null()),
            0,
            "{}",
            testutils::current_error()
        );
        assert_eq!(cstr_string(SDL_GetCurrentAudioDriver()), "dummy");
        SDL_AudioQuit();

        let _fallback = set_audio_driver("bogus,dummy");
        assert_eq!(
            SDL_AudioInit(ptr::null()),
            0,
            "{}",
            testutils::current_error()
        );
        assert_eq!(cstr_string(SDL_GetCurrentAudioDriver()), "dummy");
        SDL_AudioQuit();

        let _invalid_hint = set_audio_driver("bogus");
        assert_eq!(SDL_AudioInit(ptr::null()), -1);
        assert_eq!(
            testutils::current_error(),
            "Audio target 'bogus' not available"
        );
        assert!(SDL_GetCurrentAudioDriver().is_null());

        assert_eq!(
            SDL_AudioInit(testutils::cstring("pulse").as_ptr()),
            0,
            "{}",
            testutils::current_error()
        );
        assert_eq!(cstr_string(SDL_GetCurrentAudioDriver()), "pulseaudio");
        SDL_AudioQuit();

        assert_eq!(SDL_AudioInit(testutils::cstring("disk").as_ptr()), 0);
        assert_eq!(cstr_string(SDL_GetCurrentAudioDriver()), "disk");
        SDL_AudioQuit();
    }
}

#[test]
fn audio_enumeration_default_info_and_device_open_close_cover_playback_and_capture() {
    let _serial = testutils::serial_lock();
    let _driver = set_audio_driver("dummy");

    unsafe {
        assert_eq!(
            SDL_AudioInit(ptr::null()),
            0,
            "{}",
            testutils::current_error()
        );
        assert_eq!(cstr_string(SDL_GetCurrentAudioDriver()), "dummy");

        assert_eq!(SDL_GetNumAudioDevices(0), 1);
        assert_eq!(SDL_GetNumAudioDevices(1), 1);
        assert_eq!(SDL_GetNumAudioDevices(7), 1);

        let playback_name = SDL_GetAudioDeviceName(0, 0);
        let capture_name = SDL_GetAudioDeviceName(0, 1);
        assert!(!playback_name.is_null());
        assert!(!capture_name.is_null());
        assert!(SDL_GetAudioDeviceName(-1, 0).is_null());
        assert!(SDL_GetAudioDeviceName(4, 0).is_null());

        let mut playback_spec = MaybeUninit::<SDL_AudioSpec>::zeroed();
        assert_eq!(
            SDL_GetAudioDeviceSpec(0, 0, playback_spec.as_mut_ptr()),
            0,
            "{}",
            testutils::current_error()
        );
        let playback_spec = playback_spec.assume_init();
        assert!(playback_spec.freq > 0);
        assert!(playback_spec.channels >= 2);

        let mut capture_spec = MaybeUninit::<SDL_AudioSpec>::zeroed();
        assert_eq!(
            SDL_GetAudioDeviceSpec(0, 1, capture_spec.as_mut_ptr()),
            0,
            "{}",
            testutils::current_error()
        );

        let mut default_name = ptr::null_mut();
        let mut default_spec = MaybeUninit::<SDL_AudioSpec>::zeroed();
        assert_eq!(
            SDL_GetDefaultAudioInfo(&mut default_name, default_spec.as_mut_ptr(), 0),
            0,
            "{}",
            testutils::current_error()
        );
        assert!(!default_name.is_null());
        assert!(!cstr_string(default_name).is_empty());
        SDL_free(default_name.cast());

        let queue_spec = SDL_AudioSpec {
            callback: None,
            ..make_spec(44_100, audio_format(AUDIO_F32SYS), 1, 2048)
        };
        let playback = SDL_OpenAudioDevice(playback_name, 0, &queue_spec, ptr::null_mut(), 0);
        assert!(playback > 1, "{}", testutils::current_error());
        assert_ne!(SDL_GetAudioDeviceStatus(playback), 0);

        let capture = SDL_OpenAudioDevice(capture_name, 1, &queue_spec, ptr::null_mut(), 0);
        assert!(capture > 1, "{}", testutils::current_error());

        SDL_LockAudioDevice(playback);
        SDL_UnlockAudioDevice(playback);

        SDL_CloseAudioDevice(capture);
        SDL_CloseAudioDevice(playback);
        SDL_AudioQuit();
    }
}

#[test]
fn audio_pause_unpause_queue_and_legacy_open_close_cover_callback_and_push_paths() {
    let _serial = testutils::serial_lock();
    let _driver = set_audio_driver("dummy");

    unsafe {
        CALLBACK_COUNT.store(0, Ordering::SeqCst);
        CALLBACK_BYTES.store(0, Ordering::SeqCst);

        assert_eq!(
            SDL_AudioInit(ptr::null()),
            0,
            "{}",
            testutils::current_error()
        );
        let mut desired = make_spec(22_050, audio_format(AUDIO_S16SYS), 2, 2048);
        assert_eq!(SDL_OpenAudio(&mut desired, ptr::null_mut()), 0);
        assert_eq!(SDL_OpenAudio(&mut desired, ptr::null_mut()), -1);
        assert_eq!(
            SDL_GetAudioStatus(),
            safe_sdl::abi::generated_types::SDL_AudioStatus_SDL_AUDIO_PAUSED
        );

        SDL_PauseAudio(0);
        assert!(wait_until(Duration::from_secs(1), || CALLBACK_COUNT
            .load(Ordering::SeqCst)
            > 0));
        assert!(CALLBACK_BYTES.load(Ordering::SeqCst) > 0);

        SDL_PauseAudio(1);
        let observed = CALLBACK_COUNT.load(Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(250));
        assert_eq!(CALLBACK_COUNT.load(Ordering::SeqCst), observed);
        SDL_CloseAudio();
        SDL_AudioQuit();

        assert_eq!(SDL_AudioInit(ptr::null()), 0);
        let device_name = SDL_GetAudioDeviceName(0, 0);
        let queue_spec = SDL_AudioSpec {
            callback: None,
            ..make_spec(44_100, audio_format(AUDIO_U8), 1, 1024)
        };
        let playback = SDL_OpenAudioDevice(device_name, 0, &queue_spec, ptr::null_mut(), 0);
        assert!(playback > 1);
        let queued = vec![0x80u8; 4096];
        assert_eq!(
            SDL_QueueAudio(playback, queued.as_ptr().cast(), queued.len() as u32),
            0
        );
        assert!(SDL_GetQueuedAudioSize(playback) >= queued.len() as u32);
        SDL_PauseAudioDevice(playback, 0);
        assert!(wait_until(
            Duration::from_secs(1),
            || SDL_GetQueuedAudioSize(playback) < queued.len() as u32
        ));
        SDL_CloseAudioDevice(playback);

        let capture = SDL_OpenAudioDevice(
            SDL_GetAudioDeviceName(0, 1),
            1,
            &queue_spec,
            ptr::null_mut(),
            0,
        );
        assert!(capture > 1);
        SDL_PauseAudioDevice(capture, 0);
        assert!(wait_until(
            Duration::from_secs(1),
            || SDL_GetQueuedAudioSize(capture) > 0
        ));
        let mut buffer = vec![0u8; 512];
        let read = SDL_DequeueAudio(capture, buffer.as_mut_ptr().cast(), buffer.len() as u32);
        assert!(read > 0);
        SDL_CloseAudioDevice(capture);
        SDL_AudioQuit();
    }
}

#[test]
fn audio_build_audio_cvt_convert_and_audio_stream_cover_format_conversion() {
    let _serial = testutils::serial_lock();

    unsafe {
        assert_eq!(
            safe_sdl::audio::convert::SDL_BuildAudioCVT(
                ptr::null_mut(),
                audio_format(AUDIO_U8),
                1,
                22_050,
                audio_format(AUDIO_U8),
                1,
                22_050,
            ),
            -1
        );

        let mut same = MaybeUninit::<SDL_AudioCVT>::zeroed().assume_init();
        assert_eq!(
            safe_sdl::audio::convert::SDL_BuildAudioCVT(
                &mut same,
                audio_format(AUDIO_U8),
                1,
                22_050,
                audio_format(AUDIO_U8),
                1,
                22_050,
            ),
            0
        );

        let mut cvt = MaybeUninit::<SDL_AudioCVT>::zeroed().assume_init();
        assert_eq!(
            safe_sdl::audio::convert::SDL_BuildAudioCVT(
                &mut cvt,
                audio_format(AUDIO_U8),
                1,
                22_050,
                audio_format(AUDIO_S16SYS),
                2,
                44_100,
            ),
            1
        );
        assert!(cvt.len_mult > 1);

        let source = vec![0u8, 32, 64, 96, 128, 160, 192, 224];
        let mut converted = vec![0u8; source.len() * cvt.len_mult as usize];
        converted[..source.len()].copy_from_slice(&source);
        cvt.buf = converted.as_mut_ptr();
        cvt.len = source.len() as i32;
        assert_eq!(safe_sdl::audio::convert::SDL_ConvertAudio(&mut cvt), 0);
        assert!(cvt.len_cvt as usize > source.len());

        let stream = SDL_NewAudioStream(
            audio_format(AUDIO_U8),
            1,
            22_050,
            audio_format(AUDIO_S16SYS),
            2,
            44_100,
        );
        assert!(!stream.is_null(), "{}", testutils::current_error());
        assert_eq!(
            SDL_AudioStreamPut(stream, source.as_ptr(), source.len() as i32),
            0
        );
        assert!(SDL_AudioStreamAvailable(stream) > 0);
        assert_eq!(SDL_AudioStreamFlush(stream), 0);
        let mut output = vec![0u8; SDL_AudioStreamAvailable(stream) as usize];
        let got = SDL_AudioStreamGet(stream, output.as_mut_ptr(), output.len() as i32);
        assert!(got > 0);
        SDL_AudioStreamClear(stream);
        assert_eq!(SDL_AudioStreamAvailable(stream), 0);
        SDL_FreeAudioStream(stream);
    }
}

#[test]
fn audio_resample_loss_port_keeps_reasonable_signal_quality() {
    let _serial = testutils::serial_lock();

    unsafe {
        let scenarios = [
            (3, 440, 0.0, 44_100, 48_000, 35.0, 0.08),
            (
                3,
                1_000,
                std::f64::consts::FRAC_PI_2,
                20_000,
                10_000,
                18.0,
                0.20,
            ),
        ];

        for (seconds, freq, phase, src_rate, dst_rate, min_snr, max_error) in scenarios {
            let frames_in = seconds * src_rate as usize;
            let frames_out = seconds * dst_rate as usize;
            let samples = (0..frames_in)
                .flat_map(|index| sine_wave_sample(index, src_rate, freq, phase).to_le_bytes())
                .collect::<Vec<_>>();

            let mut cvt = MaybeUninit::<SDL_AudioCVT>::zeroed().assume_init();
            assert_eq!(
                safe_sdl::audio::convert::SDL_BuildAudioCVT(
                    &mut cvt,
                    audio_format(AUDIO_F32SYS),
                    1,
                    src_rate,
                    audio_format(AUDIO_F32SYS),
                    1,
                    dst_rate,
                ),
                1
            );

            let mut buffer = vec![0u8; samples.len() * cvt.len_mult as usize];
            buffer[..samples.len()].copy_from_slice(&samples);
            cvt.buf = buffer.as_mut_ptr();
            cvt.len = samples.len() as i32;
            assert_eq!(safe_sdl::audio::convert::SDL_ConvertAudio(&mut cvt), 0);
            assert_eq!(cvt.len_cvt as usize, frames_out * 4);

            let output = decode_f32_le(&buffer[..cvt.len_cvt as usize]);
            let reference = (0..frames_out)
                .map(|index| sine_wave_sample(index, dst_rate, freq, phase))
                .collect::<Vec<_>>();

            let observed_snr = snr_db(&reference, &output);
            let observed_max_error = reference
                .iter()
                .zip(output.iter())
                .map(|(&expected, &actual)| (expected - actual).abs())
                .fold(0.0f32, f32::max) as f64;

            assert!(observed_snr >= min_snr, "snr {observed_snr} < {min_snr}");
            assert!(
                observed_max_error <= max_error,
                "max_error {observed_max_error} > {max_error}"
            );
        }
    }
}
