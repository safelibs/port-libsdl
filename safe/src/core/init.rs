use std::sync::{Mutex, OnceLock};

use crate::abi::generated_types::{
    Uint32, SDL_INIT_AUDIO, SDL_INIT_EVENTS, SDL_INIT_EVERYTHING, SDL_INIT_GAMECONTROLLER,
    SDL_INIT_HAPTIC, SDL_INIT_JOYSTICK, SDL_INIT_SENSOR, SDL_INIT_TIMER, SDL_INIT_VIDEO,
};

struct InitState {
    main_ready: bool,
    in_main_quit: bool,
    subsystem_refcount: [u8; 32],
}

pub(crate) fn mark_main_ready() {
    lock_init_state().main_ready = true;
}

fn init_state() -> &'static Mutex<InitState> {
    static INIT_STATE: OnceLock<Mutex<InitState>> = OnceLock::new();
    INIT_STATE.get_or_init(|| {
        Mutex::new(InitState {
            main_ready: true,
            in_main_quit: false,
            subsystem_refcount: [0; 32],
        })
    })
}

fn lock_init_state() -> std::sync::MutexGuard<'static, InitState> {
    match init_state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn bit_index(flag: Uint32) -> Option<usize> {
    if flag.count_ones() == 1 {
        Some(flag.trailing_zeros() as usize)
    } else {
        None
    }
}

fn should_init(state: &InitState, subsystem: Uint32) -> bool {
    bit_index(subsystem)
        .map(|index| state.subsystem_refcount[index] == 0)
        .unwrap_or(false)
}

fn should_quit(state: &InitState, subsystem: Uint32) -> bool {
    let Some(index) = bit_index(subsystem) else {
        return false;
    };
    let count = state.subsystem_refcount[index];
    count != 0 && (count == 1 || state.in_main_quit)
}

fn incr_refcount(state: &mut InitState, subsystem: Uint32) {
    if let Some(index) = bit_index(subsystem) {
        state.subsystem_refcount[index] = state.subsystem_refcount[index].saturating_add(1);
    }
}

fn decr_refcount(state: &mut InitState, subsystem: Uint32) {
    if let Some(index) = bit_index(subsystem) {
        state.subsystem_refcount[index] = state.subsystem_refcount[index].saturating_sub(1);
    }
}

fn init_or_incr(state: &mut InitState, subsystem: Uint32) -> Result<(), ()> {
    if let Some(index) = bit_index(subsystem) {
        if state.subsystem_refcount[index] > 0 {
            state.subsystem_refcount[index] += 1;
            return Ok(());
        }
    }
    init_locked(state, subsystem)
}

fn rollback_to_snapshot(state: &mut InitState, snapshot: [u8; 32]) {
    loop {
        let mut rollback_flags = 0;
        for (index, count) in state.subsystem_refcount.iter().enumerate() {
            if *count > snapshot[index] {
                rollback_flags |= 1 << index;
            }
        }
        if rollback_flags == 0 {
            return;
        }
        quit_locked(state, rollback_flags);
    }
}

fn fail_init(state: &mut InitState, snapshot: [u8; 32]) -> Result<(), ()> {
    rollback_to_snapshot(state, snapshot);
    Err(())
}

fn init_locked(state: &mut InitState, flags: Uint32) -> Result<(), ()> {
    let snapshot = state.subsystem_refcount;
    let mut initialized = 0;

    if flags & SDL_INIT_EVENTS != 0 {
        if should_init(state, SDL_INIT_EVENTS) {
            if crate::events::queue::init_event_subsystem().is_err() {
                return fail_init(state, snapshot);
            }
        }
        incr_refcount(state, SDL_INIT_EVENTS);
        initialized |= SDL_INIT_EVENTS;
    }

    if flags & SDL_INIT_TIMER != 0 {
        if should_init(state, SDL_INIT_TIMER) {}
        incr_refcount(state, SDL_INIT_TIMER);
        initialized |= SDL_INIT_TIMER;
    }

    if flags & SDL_INIT_VIDEO != 0 {
        if should_init(state, SDL_INIT_VIDEO) {
            if init_or_incr(state, SDL_INIT_EVENTS).is_err() {
                return fail_init(state, snapshot);
            }
            if crate::video::display::init_video_subsystem().is_err() {
                return fail_init(state, snapshot);
            }
        }
        incr_refcount(state, SDL_INIT_VIDEO);
        initialized |= SDL_INIT_VIDEO;
    }

    if flags & SDL_INIT_AUDIO != 0 {
        if should_init(state, SDL_INIT_AUDIO) {
            if init_or_incr(state, SDL_INIT_EVENTS).is_err() {
                return fail_init(state, snapshot);
            }
            if crate::audio::device::init_audio_subsystem().is_err() {
                return fail_init(state, snapshot);
            }
        }
        incr_refcount(state, SDL_INIT_AUDIO);
        initialized |= SDL_INIT_AUDIO;
    }

    if flags & SDL_INIT_JOYSTICK != 0 {
        if should_init(state, SDL_INIT_JOYSTICK) {
            if init_or_incr(state, SDL_INIT_EVENTS).is_err() {
                return fail_init(state, snapshot);
            }
            if crate::input::init_input_subsystem(SDL_INIT_JOYSTICK).is_err() {
                return fail_init(state, snapshot);
            }
        }
        incr_refcount(state, SDL_INIT_JOYSTICK);
        initialized |= SDL_INIT_JOYSTICK;
    }

    if flags & SDL_INIT_GAMECONTROLLER != 0 {
        if should_init(state, SDL_INIT_GAMECONTROLLER) {
            if init_or_incr(state, SDL_INIT_JOYSTICK).is_err() {
                return fail_init(state, snapshot);
            }
            if crate::input::init_input_subsystem(SDL_INIT_GAMECONTROLLER).is_err() {
                return fail_init(state, snapshot);
            }
        }
        incr_refcount(state, SDL_INIT_GAMECONTROLLER);
        initialized |= SDL_INIT_GAMECONTROLLER;
    }

    if flags & SDL_INIT_HAPTIC != 0 {
        if should_init(state, SDL_INIT_HAPTIC) {
            if crate::input::init_input_subsystem(SDL_INIT_HAPTIC).is_err() {
                return fail_init(state, snapshot);
            }
        }
        incr_refcount(state, SDL_INIT_HAPTIC);
        initialized |= SDL_INIT_HAPTIC;
    }

    if flags & SDL_INIT_SENSOR != 0 {
        if should_init(state, SDL_INIT_SENSOR) {
            if crate::input::init_input_subsystem(SDL_INIT_SENSOR).is_err() {
                return fail_init(state, snapshot);
            }
        }
        incr_refcount(state, SDL_INIT_SENSOR);
        initialized |= SDL_INIT_SENSOR;
    }

    let _ = initialized;
    Ok(())
}

fn quit_locked(state: &mut InitState, flags: Uint32) {
    if flags & SDL_INIT_SENSOR != 0 {
        if should_quit(state, SDL_INIT_SENSOR) {
            crate::input::quit_input_subsystem(SDL_INIT_SENSOR);
        }
        decr_refcount(state, SDL_INIT_SENSOR);
    }

    if flags & SDL_INIT_GAMECONTROLLER != 0 {
        if should_quit(state, SDL_INIT_GAMECONTROLLER) {
            crate::input::quit_input_subsystem(SDL_INIT_GAMECONTROLLER);
            quit_locked(state, SDL_INIT_JOYSTICK);
        }
        decr_refcount(state, SDL_INIT_GAMECONTROLLER);
    }

    if flags & SDL_INIT_JOYSTICK != 0 {
        if should_quit(state, SDL_INIT_JOYSTICK) {
            crate::input::quit_input_subsystem(SDL_INIT_JOYSTICK);
            quit_locked(state, SDL_INIT_EVENTS);
        }
        decr_refcount(state, SDL_INIT_JOYSTICK);
    }

    if flags & SDL_INIT_HAPTIC != 0 {
        if should_quit(state, SDL_INIT_HAPTIC) {
            crate::input::quit_input_subsystem(SDL_INIT_HAPTIC);
        }
        decr_refcount(state, SDL_INIT_HAPTIC);
    }

    if flags & SDL_INIT_AUDIO != 0 {
        if should_quit(state, SDL_INIT_AUDIO) {
            crate::audio::device::quit_audio_subsystem();
            quit_locked(state, SDL_INIT_EVENTS);
        }
        decr_refcount(state, SDL_INIT_AUDIO);
    }

    if flags & SDL_INIT_VIDEO != 0 {
        if should_quit(state, SDL_INIT_VIDEO) {
            crate::video::display::quit_video_subsystem();
            quit_locked(state, SDL_INIT_EVENTS);
        }
        decr_refcount(state, SDL_INIT_VIDEO);
    }

    if flags & SDL_INIT_TIMER != 0 {
        if should_quit(state, SDL_INIT_TIMER) {
            crate::core::timer::timer_subsystem_quit();
        }
        decr_refcount(state, SDL_INIT_TIMER);
    }

    if flags & SDL_INIT_EVENTS != 0 {
        if should_quit(state, SDL_INIT_EVENTS) {
            crate::events::queue::quit_event_subsystem();
        }
        decr_refcount(state, SDL_INIT_EVENTS);
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_Init(flags: Uint32) -> libc::c_int {
    SDL_InitSubSystem(flags)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_InitSubSystem(flags: Uint32) -> libc::c_int {
    let state = lock_init_state();
    if !state.main_ready {
        return crate::core::error::set_error_message(
            "Application didn't initialize properly, did you include SDL_main.h in the file containing your main() function?",
        );
    }
    drop(state);

    crate::core::log::SDL_LogResetPriorities();
    crate::core::error::SDL_ClearError();

    let mut state = lock_init_state();
    match init_locked(&mut state, flags) {
        Ok(()) => 0,
        Err(()) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_QuitSubSystem(flags: Uint32) {
    let mut state = lock_init_state();
    quit_locked(&mut state, flags);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_WasInit(mut flags: Uint32) -> Uint32 {
    let state = lock_init_state();
    if flags.count_ones() == 1 {
        if let Some(index) = bit_index(flags) {
            return if state.subsystem_refcount[index] != 0 {
                flags
            } else {
                0
            };
        }
    }

    if flags == 0 {
        flags = SDL_INIT_EVERYTHING;
    }

    let mut initialized = 0;
    let mut bit = 0;
    while flags != 0 {
        if flags & 1 != 0 && state.subsystem_refcount[bit] > 0 {
            initialized |= 1 << bit;
        }
        flags >>= 1;
        bit += 1;
    }
    initialized
}

#[no_mangle]
pub unsafe extern "C" fn SDL_Quit() {
    {
        let mut state = lock_init_state();
        state.in_main_quit = true;
        quit_locked(&mut state, SDL_INIT_EVERYTHING);
        state.subsystem_refcount.fill(0);
        state.in_main_quit = false;
    }

    crate::core::timer::timer_subsystem_quit();
    crate::core::hints::SDL_ClearHints();
    crate::core::assert::assertions_quit();
    crate::core::log::log_quit();
    crate::core::thread::cleanup_current_thread_tls();
}
