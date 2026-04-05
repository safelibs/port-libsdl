use std::collections::HashMap;
use std::ffi::CStr;
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::abi::generated_types::{
    SDL_AudioCallback, SDL_AudioDeviceID, SDL_AudioFormat, SDL_AudioSpec, SDL_AudioStatus,
    SDL_AudioStatus_SDL_AUDIO_PAUSED, SDL_AudioStatus_SDL_AUDIO_PLAYING,
    SDL_AudioStatus_SDL_AUDIO_STOPPED, SDL_HINT_AUDIODRIVER,
};
use crate::audio::drivers::{alsa, disk, dsp, dummy, pipewire, pulseaudio, sndio};
use crate::audio::{normalize_audio_spec, AudioDriverDescriptor, DeviceTemplate};
use crate::core::hints::SDL_GetHint;

const DRIVER_REGISTRY: &[AudioDriverDescriptor] = &[
    pulseaudio::DESCRIPTOR,
    alsa::DESCRIPTOR,
    sndio::DESCRIPTOR,
    pipewire::DESCRIPTOR,
    dsp::DESCRIPTOR,
    disk::DESCRIPTOR,
    dummy::DESCRIPTOR,
];

#[derive(Default)]
struct ByteQueue {
    data: Vec<u8>,
    start: usize,
}

impl ByteQueue {
    fn len(&self) -> usize {
        self.data.len().saturating_sub(self.start)
    }

    fn clear(&mut self) {
        self.data.clear();
        self.start = 0;
    }

    fn compact(&mut self) {
        if self.start == 0 {
            return;
        }
        if self.start >= self.data.len() {
            self.clear();
        } else if self.start >= self.data.len() / 2 {
            self.data.drain(..self.start);
            self.start = 0;
        }
    }

    fn push(&mut self, bytes: &[u8]) {
        self.compact();
        self.data.extend_from_slice(bytes);
    }

    fn read_into(&mut self, dst: &mut [u8]) -> usize {
        let count = dst.len().min(self.len());
        if count > 0 {
            dst[..count].copy_from_slice(&self.data[self.start..self.start + count]);
            self.start += count;
            self.compact();
        }
        count
    }

    fn discard(&mut self, len: usize) -> usize {
        let count = len.min(self.len());
        self.start += count;
        self.compact();
        count
    }
}

#[derive(Clone, Copy)]
struct DeviceSpec {
    freq: i32,
    format: SDL_AudioFormat,
    channels: u8,
    silence: u8,
    samples: u16,
    size: u32,
    callback: SDL_AudioCallback,
    userdata: usize,
}

impl From<SDL_AudioSpec> for DeviceSpec {
    fn from(value: SDL_AudioSpec) -> Self {
        Self {
            freq: value.freq,
            format: value.format,
            channels: value.channels,
            silence: value.silence,
            samples: value.samples,
            size: value.size,
            callback: value.callback,
            userdata: value.userdata as usize,
        }
    }
}

impl DeviceSpec {
    fn as_sdl(self) -> SDL_AudioSpec {
        SDL_AudioSpec {
            freq: self.freq,
            format: self.format,
            channels: self.channels,
            silence: self.silence,
            samples: self.samples,
            padding: 0,
            size: self.size,
            callback: self.callback,
            userdata: self.userdata as *mut libc::c_void,
        }
    }
}

struct DeviceInner {
    iscapture: bool,
    queue_mode: bool,
    spec: DeviceSpec,
    status: SDL_AudioStatus,
    lock_count: u32,
    callback_active: bool,
    closing: bool,
    queue: ByteQueue,
}

struct DeviceControl {
    inner: Mutex<DeviceInner>,
    condvar: Condvar,
}

struct DeviceRecord {
    control: Arc<DeviceControl>,
    thread: Option<JoinHandle<()>>,
}

struct AudioState {
    current_driver_index: Option<usize>,
    devices: HashMap<SDL_AudioDeviceID, DeviceRecord>,
    next_device_id: SDL_AudioDeviceID,
    legacy_device: Option<SDL_AudioDeviceID>,
}

impl Default for AudioState {
    fn default() -> Self {
        Self {
            current_driver_index: None,
            devices: HashMap::new(),
            next_device_id: 2,
            legacy_device: None,
        }
    }
}

fn audio_state() -> &'static Mutex<AudioState> {
    static STATE: OnceLock<Mutex<AudioState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(AudioState::default()))
}

fn lock_audio_state() -> std::sync::MutexGuard<'static, AudioState> {
    match audio_state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn lock_device(control: &DeviceControl) -> std::sync::MutexGuard<'_, DeviceInner> {
    match control.inner.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn canonical_driver_name(name: &str) -> &str {
    match name {
        "pulse" => "pulseaudio",
        _ => name,
    }
}

fn driver_by_name(name: &str) -> Option<usize> {
    let name = canonical_driver_name(name);
    DRIVER_REGISTRY
        .iter()
        .position(|driver| driver.name == name)
}

fn hint_driver_name() -> Option<String> {
    unsafe {
        let hint = SDL_GetHint(SDL_HINT_AUDIODRIVER.as_ptr().cast());
        if hint.is_null() {
            return None;
        }
        CStr::from_ptr(hint).to_str().ok().map(str::to_string)
    }
}

fn resolve_driver_candidate_list(names: &str) -> Option<usize> {
    names
        .split(',')
        .map(str::trim)
        .filter(|candidate| !candidate.is_empty())
        .find_map(driver_by_name)
}

fn resolve_driver_index(requested: Option<&str>) -> Result<usize, ()> {
    if let Some(requested) = requested.filter(|value| !value.is_empty()) {
        if let Some(index) = resolve_driver_candidate_list(requested) {
            return Ok(index);
        }
        let _ = crate::core::error::set_error_message(&format!(
            "Audio target '{}' not available",
            requested
        ));
        return Err(());
    }

    if let Some(hint) = hint_driver_name().filter(|value| !value.is_empty()) {
        if let Some(index) = resolve_driver_candidate_list(&hint) {
            return Ok(index);
        }
        let _ = crate::core::error::set_error_message(&format!(
            "Audio target '{}' not available",
            hint
        ));
        return Err(());
    }

    DRIVER_REGISTRY
        .iter()
        .position(|driver| !driver.demand_only)
        .ok_or_else(|| {
            let _ = crate::core::error::set_error_message("No usable audio driver");
        })
}

fn templates_for_driver(driver_index: usize, iscapture: bool) -> &'static [DeviceTemplate] {
    let driver = &DRIVER_REGISTRY[driver_index];
    if iscapture {
        driver.capture_devices
    } else {
        driver.playback_devices
    }
}

fn with_current_driver<T>(callback: impl FnOnce(usize) -> T) -> Result<T, ()> {
    let state = lock_audio_state();
    state.current_driver_index.map(callback).ok_or_else(|| {
        let _ = crate::core::error::set_error_message("Audio subsystem is not initialized");
    })
}

fn locate_device_template(
    driver_index: usize,
    iscapture: bool,
    requested: Option<&CStr>,
) -> Result<&'static DeviceTemplate, ()> {
    let templates = templates_for_driver(driver_index, iscapture);
    if templates.is_empty() {
        let _ = crate::core::error::set_error_message("No such audio device");
        return Err(());
    }

    if let Some(requested) = requested {
        let requested = requested.to_bytes();
        templates
            .iter()
            .find(|template| template.name.as_bytes() == requested)
            .ok_or_else(|| {
                let _ = crate::core::error::set_error_message("No such audio device");
            })
    } else {
        Ok(&templates[0])
    }
}

fn thread_period(spec: DeviceSpec) -> Duration {
    let millis = ((spec.samples as u64) * 1000 / spec.freq.max(1) as u64).max(5);
    Duration::from_millis(millis)
}

fn generate_capture_bytes(spec: DeviceSpec, len: usize) -> Vec<u8> {
    vec![spec.silence; len]
}

fn device_thread(control: Arc<DeviceControl>) {
    loop {
        let (spec, queue_mode, status) = {
            let mut inner = lock_device(&control);
            while !inner.closing
                && (inner.status != SDL_AudioStatus_SDL_AUDIO_PLAYING || inner.lock_count != 0)
            {
                inner = match control.condvar.wait(inner) {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
            }
            if inner.closing {
                return;
            }
            (inner.spec, inner.queue_mode, inner.status)
        };

        if status != SDL_AudioStatus_SDL_AUDIO_PLAYING {
            continue;
        }

        let cycle_bytes = spec.size as usize;
        if queue_mode {
            let mut inner = lock_device(&control);
            if inner.closing
                || inner.status != SDL_AudioStatus_SDL_AUDIO_PLAYING
                || inner.lock_count != 0
            {
                continue;
            }
            if inner.iscapture {
                let max_queue = cycle_bytes.saturating_mul(8).max(cycle_bytes);
                if inner.queue.len() < max_queue {
                    inner.queue.push(&generate_capture_bytes(spec, cycle_bytes));
                }
            } else {
                inner.queue.discard(cycle_bytes);
            }
            drop(inner);
        } else if let Some(callback) = spec.callback {
            let mut buffer = vec![spec.silence; cycle_bytes];
            {
                let mut inner = lock_device(&control);
                if inner.closing {
                    return;
                }
                inner.callback_active = true;
            }
            unsafe {
                callback(
                    spec.userdata as *mut libc::c_void,
                    buffer.as_mut_ptr(),
                    cycle_bytes as i32,
                );
            }
            let mut inner = lock_device(&control);
            inner.callback_active = false;
            control.condvar.notify_all();
        }

        let timeout = thread_period(spec);
        let inner = lock_device(&control);
        if inner.closing {
            return;
        }
        let _ = control.condvar.wait_timeout(inner, timeout);
    }
}

fn shutdown_record(mut record: DeviceRecord) {
    {
        let mut inner = lock_device(&record.control);
        inner.closing = true;
        record.control.condvar.notify_all();
    }
    if let Some(handle) = record.thread.take() {
        let _ = handle.join();
    }
}

fn close_all_devices() {
    let records = {
        let mut state = lock_audio_state();
        state.legacy_device = None;
        std::mem::take(&mut state.devices)
            .into_values()
            .collect::<Vec<_>>()
    };
    for record in records {
        shutdown_record(record);
    }
}

fn ensure_audio_driver() -> Result<usize, ()> {
    {
        let state = lock_audio_state();
        if let Some(index) = state.current_driver_index {
            return Ok(index);
        }
    }
    init_audio_internal(None)?;
    with_current_driver(|index| index)
}

fn init_audio_internal(requested: Option<&str>) -> Result<(), ()> {
    let driver_index = resolve_driver_index(requested)?;
    let existing = {
        let state = lock_audio_state();
        state.current_driver_index
    };
    if existing == Some(driver_index) {
        return Ok(());
    }
    if existing.is_some() {
        close_all_devices();
    }

    let mut state = lock_audio_state();
    state.current_driver_index = Some(driver_index);
    Ok(())
}

pub(crate) fn init_audio_subsystem() -> Result<(), ()> {
    init_audio_internal(None)
}

pub(crate) fn quit_audio_subsystem() {
    close_all_devices();
    let mut state = lock_audio_state();
    state.current_driver_index = None;
}

fn open_device_internal(
    requested_device: Option<&CStr>,
    iscapture: bool,
    desired: SDL_AudioSpec,
    legacy: bool,
) -> Result<(SDL_AudioDeviceID, SDL_AudioSpec), ()> {
    let driver_index = ensure_audio_driver()?;
    let _ = locate_device_template(driver_index, iscapture, requested_device)?;

    let mut obtained = desired;
    if let Err(param) = normalize_audio_spec(&mut obtained) {
        let _ = crate::core::error::set_error_message(&format!("Parameter '{param}' is invalid"));
        return Err(());
    }

    let mut state = lock_audio_state();
    if legacy {
        if state.legacy_device.is_some() {
            let _ = crate::core::error::set_error_message("Legacy audio device already open");
            return Err(());
        }
    }

    let id = if legacy {
        1
    } else {
        let id = state.next_device_id;
        state.next_device_id = state.next_device_id.saturating_add(1).max(2);
        id
    };

    let control = Arc::new(DeviceControl {
        inner: Mutex::new(DeviceInner {
            iscapture,
            queue_mode: obtained.callback.is_none(),
            spec: obtained.into(),
            status: SDL_AudioStatus_SDL_AUDIO_PAUSED,
            lock_count: 0,
            callback_active: false,
            closing: false,
            queue: ByteQueue::default(),
        }),
        condvar: Condvar::new(),
    });

    let thread_control = Arc::clone(&control);
    let thread = thread::spawn(move || device_thread(thread_control));
    state.devices.insert(
        id,
        DeviceRecord {
            control,
            thread: Some(thread),
        },
    );
    if legacy {
        state.legacy_device = Some(id);
    }
    Ok((id, obtained))
}

fn close_device_by_id(id: SDL_AudioDeviceID) {
    let record = {
        let mut state = lock_audio_state();
        if state.legacy_device == Some(id) {
            state.legacy_device = None;
        }
        state.devices.remove(&id)
    };
    if let Some(record) = record {
        shutdown_record(record);
    }
}

fn device_control(id: SDL_AudioDeviceID) -> Option<Arc<DeviceControl>> {
    let state = lock_audio_state();
    state
        .devices
        .get(&id)
        .map(|record| Arc::clone(&record.control))
}

pub(crate) fn legacy_output_format() -> Option<SDL_AudioFormat> {
    let legacy_id = {
        let state = lock_audio_state();
        state.legacy_device
    }?;
    let control = device_control(legacy_id)?;
    let format = lock_device(&control).spec.format;
    Some(format)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetNumAudioDrivers() -> libc::c_int {
    DRIVER_REGISTRY.len() as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetAudioDriver(index: libc::c_int) -> *const libc::c_char {
    if index < 0 {
        return std::ptr::null();
    }
    DRIVER_REGISTRY
        .get(index as usize)
        .map(|driver| driver.name_cstr.as_ptr().cast())
        .unwrap_or(std::ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_AudioInit(driver_name: *const libc::c_char) -> libc::c_int {
    let requested = if driver_name.is_null() {
        None
    } else {
        CStr::from_ptr(driver_name).to_str().ok()
    };
    match init_audio_internal(requested) {
        Ok(()) => 0,
        Err(()) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_AudioQuit() {
    quit_audio_subsystem();
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetCurrentAudioDriver() -> *const libc::c_char {
    let state = lock_audio_state();
    state
        .current_driver_index
        .and_then(|index| DRIVER_REGISTRY.get(index))
        .map(|driver| driver.name_cstr.as_ptr().cast())
        .unwrap_or(std::ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetNumAudioDevices(iscapture: libc::c_int) -> libc::c_int {
    match with_current_driver(|index| templates_for_driver(index, iscapture != 0).len()) {
        Ok(count) => count as libc::c_int,
        Err(()) => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetAudioDeviceName(
    index: libc::c_int,
    iscapture: libc::c_int,
) -> *const libc::c_char {
    if index < 0 {
        return std::ptr::null();
    }
    match with_current_driver(|driver_index| {
        templates_for_driver(driver_index, iscapture != 0)
            .get(index as usize)
            .map(|device| device.name_cstr.as_ptr().cast())
    }) {
        Ok(Some(name)) => name,
        _ => std::ptr::null(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetAudioDeviceSpec(
    index: libc::c_int,
    iscapture: libc::c_int,
    spec: *mut SDL_AudioSpec,
) -> libc::c_int {
    if spec.is_null() {
        return crate::core::error::invalid_param_error("spec");
    }
    if index < 0 {
        return crate::core::error::set_error_message("No such audio device");
    }

    let template = match with_current_driver(|driver_index| {
        templates_for_driver(driver_index, iscapture != 0)
            .get(index as usize)
            .copied()
    }) {
        Ok(Some(template)) => template,
        _ => return crate::core::error::set_error_message("No such audio device"),
    };

    let mut audio_spec = SDL_AudioSpec {
        freq: template.freq,
        format: template.format,
        channels: template.channels,
        silence: 0,
        samples: template.samples,
        padding: 0,
        size: 0,
        callback: None,
        userdata: std::ptr::null_mut(),
    };
    if let Err(param) = normalize_audio_spec(&mut audio_spec) {
        return crate::core::error::set_error_message(&format!("Parameter '{param}' is invalid"));
    }
    *spec = audio_spec;
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetDefaultAudioInfo(
    name: *mut *mut libc::c_char,
    spec: *mut SDL_AudioSpec,
    iscapture: libc::c_int,
) -> libc::c_int {
    let template = match with_current_driver(|driver_index| {
        templates_for_driver(driver_index, iscapture != 0)
            .first()
            .copied()
    }) {
        Ok(Some(template)) => template,
        _ => return crate::core::error::set_error_message("No default audio device"),
    };

    if !name.is_null() {
        *name = crate::core::memory::alloc_c_string(template.name);
        if (*name).is_null() {
            return crate::core::error::out_of_memory_error();
        }
    }

    if !spec.is_null() {
        let mut device_spec = SDL_AudioSpec {
            freq: template.freq,
            format: template.format,
            channels: template.channels,
            silence: 0,
            samples: template.samples,
            padding: 0,
            size: 0,
            callback: None,
            userdata: std::ptr::null_mut(),
        };
        if normalize_audio_spec(&mut device_spec).is_err() {
            return crate::core::error::set_error_message(
                "Default audio device specification is invalid",
            );
        }
        *spec = device_spec;
    }

    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_OpenAudio(
    desired: *mut SDL_AudioSpec,
    obtained: *mut SDL_AudioSpec,
) -> libc::c_int {
    if desired.is_null() {
        return crate::core::error::invalid_param_error("desired");
    }
    let requested = *desired;
    match open_device_internal(None, false, requested, true) {
        Ok((_id, actual)) => {
            if obtained.is_null() {
                *desired = actual;
            } else {
                *obtained = actual;
            }
            0
        }
        Err(()) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_OpenAudioDevice(
    device: *const libc::c_char,
    iscapture: libc::c_int,
    desired: *const SDL_AudioSpec,
    obtained: *mut SDL_AudioSpec,
    _allowed_changes: libc::c_int,
) -> SDL_AudioDeviceID {
    if desired.is_null() {
        let _ = crate::core::error::invalid_param_error("desired");
        return 0;
    }
    let requested_device = (!device.is_null()).then(|| CStr::from_ptr(device));
    match open_device_internal(requested_device, iscapture != 0, *desired, false) {
        Ok((id, actual)) => {
            if !obtained.is_null() {
                *obtained = actual;
            }
            id
        }
        Err(()) => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetAudioStatus() -> SDL_AudioStatus {
    let legacy = {
        let state = lock_audio_state();
        state.legacy_device
    };
    legacy.map_or(SDL_AudioStatus_SDL_AUDIO_STOPPED, |id| unsafe {
        SDL_GetAudioDeviceStatus(id)
    })
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetAudioDeviceStatus(dev: SDL_AudioDeviceID) -> SDL_AudioStatus {
    let Some(control) = device_control(dev) else {
        return SDL_AudioStatus_SDL_AUDIO_STOPPED;
    };
    let status = lock_device(&control).status;
    status
}

#[no_mangle]
pub unsafe extern "C" fn SDL_PauseAudio(pause_on: libc::c_int) {
    let legacy = {
        let state = lock_audio_state();
        state.legacy_device
    };
    if let Some(id) = legacy {
        SDL_PauseAudioDevice(id, pause_on);
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_PauseAudioDevice(dev: SDL_AudioDeviceID, pause_on: libc::c_int) {
    if let Some(control) = device_control(dev) {
        let mut inner = lock_device(&control);
        inner.status = if pause_on == 0 {
            SDL_AudioStatus_SDL_AUDIO_PLAYING
        } else {
            SDL_AudioStatus_SDL_AUDIO_PAUSED
        };
        control.condvar.notify_all();
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_LockAudio() {
    let legacy = {
        let state = lock_audio_state();
        state.legacy_device
    };
    if let Some(id) = legacy {
        SDL_LockAudioDevice(id);
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_LockAudioDevice(dev: SDL_AudioDeviceID) {
    if let Some(control) = device_control(dev) {
        let mut inner = lock_device(&control);
        inner.lock_count = inner.lock_count.saturating_add(1);
        while inner.callback_active {
            inner = match control.condvar.wait(inner) {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_UnlockAudio() {
    let legacy = {
        let state = lock_audio_state();
        state.legacy_device
    };
    if let Some(id) = legacy {
        SDL_UnlockAudioDevice(id);
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_UnlockAudioDevice(dev: SDL_AudioDeviceID) {
    if let Some(control) = device_control(dev) {
        let mut inner = lock_device(&control);
        inner.lock_count = inner.lock_count.saturating_sub(1);
        if inner.lock_count == 0 {
            control.condvar.notify_all();
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CloseAudio() {
    let legacy = {
        let state = lock_audio_state();
        state.legacy_device
    };
    if let Some(id) = legacy {
        close_device_by_id(id);
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CloseAudioDevice(dev: SDL_AudioDeviceID) {
    close_device_by_id(dev);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_QueueAudio(
    dev: SDL_AudioDeviceID,
    data: *const libc::c_void,
    len: u32,
) -> libc::c_int {
    let Some(control) = device_control(dev) else {
        return crate::core::error::set_error_message("Invalid audio device ID");
    };
    if len > 0 && data.is_null() {
        return crate::core::error::invalid_param_error("data");
    }

    let mut inner = lock_device(&control);
    if inner.iscapture {
        return crate::core::error::set_error_message("Cannot queue audio on a capture device");
    }
    if !inner.queue_mode {
        return crate::core::error::set_error_message("Audio device uses a callback");
    }
    if len > 0 {
        inner
            .queue
            .push(std::slice::from_raw_parts(data.cast(), len as usize));
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_DequeueAudio(
    dev: SDL_AudioDeviceID,
    data: *mut libc::c_void,
    len: u32,
) -> u32 {
    let Some(control) = device_control(dev) else {
        let _ = crate::core::error::set_error_message("Invalid audio device ID");
        return 0;
    };
    if len > 0 && data.is_null() {
        let _ = crate::core::error::invalid_param_error("data");
        return 0;
    }

    let mut inner = lock_device(&control);
    if !inner.iscapture || !inner.queue_mode {
        return 0;
    }
    let dst = std::slice::from_raw_parts_mut(data.cast::<u8>(), len as usize);
    inner.queue.read_into(dst) as u32
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetQueuedAudioSize(dev: SDL_AudioDeviceID) -> u32 {
    let Some(control) = device_control(dev) else {
        return 0;
    };
    let inner = lock_device(&control);
    if !inner.queue_mode {
        return 0;
    }
    inner.queue.len().min(u32::MAX as usize) as u32
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ClearQueuedAudio(dev: SDL_AudioDeviceID) {
    if let Some(control) = device_control(dev) {
        let mut inner = lock_device(&control);
        if inner.queue_mode {
            inner.queue.clear();
        }
    }
}
