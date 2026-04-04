use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::abi::generated_types::{SDL_TimerCallback, SDL_TimerID, SDL_bool, Uint32, Uint64};
use crate::core::system::bool_to_sdl;

fn start_instant() -> &'static Instant {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now)
}

fn timers() -> &'static Mutex<HashMap<SDL_TimerID, Arc<AtomicBool>>> {
    static TIMERS: OnceLock<Mutex<HashMap<SDL_TimerID, Arc<AtomicBool>>>> = OnceLock::new();
    TIMERS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_timers() -> std::sync::MutexGuard<'static, HashMap<SDL_TimerID, Arc<AtomicBool>>> {
    match timers().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn next_timer_id() -> &'static AtomicI32 {
    static NEXT_ID: AtomicI32 = AtomicI32::new(1);
    &NEXT_ID
}

pub(crate) fn timer_subsystem_quit() {
    let timers = std::mem::take(&mut *lock_timers());
    for cancel in timers.into_values() {
        cancel.store(true, Ordering::Release);
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetTicks() -> Uint32 {
    SDL_GetTicks64() as Uint32
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetTicks64() -> Uint64 {
    start_instant().elapsed().as_millis() as Uint64
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetPerformanceCounter() -> Uint64 {
    start_instant().elapsed().as_nanos() as Uint64
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetPerformanceFrequency() -> Uint64 {
    1_000_000_000
}

#[no_mangle]
pub unsafe extern "C" fn SDL_Delay(ms: Uint32) {
    std::thread::sleep(Duration::from_millis(ms as u64));
}

#[no_mangle]
pub unsafe extern "C" fn SDL_AddTimer(
    interval: Uint32,
    callback: SDL_TimerCallback,
    param: *mut libc::c_void,
) -> SDL_TimerID {
    let Some(callback) = callback else {
        let _ = crate::core::error::invalid_param_error("callback");
        return 0;
    };
    let id = next_timer_id().fetch_add(1, Ordering::Relaxed);
    let cancel = Arc::new(AtomicBool::new(false));
    lock_timers().insert(id, cancel.clone());
    let param = param as usize;
    std::thread::spawn(move || {
        let mut next_interval = interval.max(1);
        loop {
            std::thread::sleep(Duration::from_millis(next_interval as u64));
            if cancel.load(Ordering::Acquire) {
                break;
            }
            let updated = unsafe { callback(next_interval, param as *mut libc::c_void) };
            if updated == 0 {
                break;
            }
            next_interval = updated.max(1);
        }
        lock_timers().remove(&id);
    });
    id
}

#[no_mangle]
pub unsafe extern "C" fn SDL_RemoveTimer(id: SDL_TimerID) -> SDL_bool {
    if let Some(cancel) = lock_timers().remove(&id) {
        cancel.store(true, Ordering::Release);
        bool_to_sdl(true)
    } else {
        bool_to_sdl(false)
    }
}
