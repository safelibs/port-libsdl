#![allow(clippy::all)]

#[path = "common/testutils.rs"]
mod testutils;

use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::ptr;
use std::sync::atomic::{AtomicU32, Ordering};

use safe_sdl::abi::generated_types::{
    SDL_Event, SDL_EventType_SDL_FIRSTEVENT, SDL_EventType_SDL_KEYDOWN,
    SDL_EventType_SDL_LASTEVENT, SDL_EventType_SDL_MOUSEBUTTONDOWN, SDL_KeyCode_SDLK_a,
    SDL_KeyboardEvent, SDL_Keysym, SDL_MouseButtonEvent, SDL_Scancode_SDL_SCANCODE_A,
    SDL_UserEvent, SDL_eventaction_SDL_ADDEVENT, SDL_eventaction_SDL_PEEKEVENT, SDL_DISABLE,
    SDL_INIT_EVENTS, SDL_INIT_TIMER, SDL_INIT_VIDEO, SDL_PRESSED,
};
use safe_sdl::core::init::{SDL_Init, SDL_Quit};
use safe_sdl::core::timer::{SDL_AddTimer, SDL_Delay, SDL_GetTicks};
use safe_sdl::events::queue::{
    SDL_AddEventWatch, SDL_DelEventWatch, SDL_EventState, SDL_FlushEvents, SDL_HasEvent,
    SDL_HasEvents, SDL_PeepEvents, SDL_PollEvent, SDL_PushEvent, SDL_RegisterEvents,
    SDL_SetEventFilter, SDL_WaitEventTimeout,
};

struct SdlGuard;

impl SdlGuard {
    fn init_validator_flags() -> Self {
        unsafe {
            assert_eq!(
                SDL_Init(SDL_INIT_VIDEO | SDL_INIT_TIMER | SDL_INIT_EVENTS),
                0,
                "{}",
                testutils::current_error()
            );
        }
        Self
    }
}

impl Drop for SdlGuard {
    fn drop(&mut self) {
        unsafe { SDL_Quit() };
    }
}

#[repr(C)]
struct ReentrantCallbackState {
    filter_calls: AtomicU32,
    watch_calls: AtomicU32,
    errors: AtomicU32,
}

unsafe extern "C" fn query_queue_filter(
    userdata: *mut c_void,
    _event: *mut SDL_Event,
) -> libc::c_int {
    let state = &*(userdata as *const ReentrantCallbackState);
    let queued = SDL_PeepEvents(
        ptr::null_mut(),
        0,
        SDL_eventaction_SDL_PEEKEVENT,
        SDL_EventType_SDL_FIRSTEVENT,
        SDL_EventType_SDL_LASTEVENT,
    );
    if queued < 0 {
        state.errors.fetch_add(1, Ordering::SeqCst);
    }
    state.filter_calls.fetch_add(1, Ordering::SeqCst);
    1
}

unsafe extern "C" fn query_queue_watch(
    userdata: *mut c_void,
    _event: *mut SDL_Event,
) -> libc::c_int {
    let state = &*(userdata as *const ReentrantCallbackState);
    let _ = SDL_HasEvents(SDL_EventType_SDL_FIRSTEVENT, SDL_EventType_SDL_LASTEVENT);
    state.watch_calls.fetch_add(1, Ordering::SeqCst);
    1
}

#[repr(C)]
struct TimerEventContext {
    event_type: u32,
    pushed: AtomicU32,
}

unsafe extern "C" fn post_timer_event(_interval: u32, userdata: *mut c_void) -> u32 {
    let context = &*(userdata as *const TimerEventContext);
    let mut event = SDL_Event {
        user: SDL_UserEvent {
            type_: context.event_type,
            timestamp: SDL_GetTicks(),
            windowID: 0,
            code: 99,
            data1: ptr::null_mut(),
            data2: ptr::null_mut(),
        },
    };
    if SDL_PushEvent(&mut event) == 1 {
        context.pushed.store(1, Ordering::SeqCst);
    }
    0
}

#[test]
fn push_poll_peep_blocked_key_and_mouse_events_match_validator_cases() {
    let _serial = testutils::serial_lock();
    let _video = testutils::ScopedEnvVar::set("SDL_VIDEODRIVER", "dummy");
    let _sdl = SdlGuard::init_validator_flags();

    unsafe {
        SDL_FlushEvents(SDL_EventType_SDL_FIRSTEVENT, SDL_EventType_SDL_LASTEVENT);

        let event_type = SDL_RegisterEvents(2);
        assert_ne!(event_type, u32::MAX, "{}", testutils::current_error());
        let blocked_type = event_type + 1;

        let mut custom = SDL_Event {
            user: SDL_UserEvent {
                type_: event_type,
                timestamp: 0,
                windowID: 0,
                code: 7,
                data1: ptr::null_mut(),
                data2: ptr::null_mut(),
            },
        };
        assert_eq!(
            SDL_PushEvent(&mut custom),
            1,
            "{}",
            testutils::current_error()
        );
        assert_ne!(SDL_HasEvent(event_type), 0);

        let mut peeked = MaybeUninit::<SDL_Event>::zeroed();
        assert_eq!(
            SDL_PeepEvents(
                peeked.as_mut_ptr(),
                1,
                SDL_eventaction_SDL_PEEKEVENT,
                event_type,
                event_type,
            ),
            1
        );
        let peeked = peeked.assume_init();
        assert_eq!(peeked.user.type_, event_type);
        assert_eq!(peeked.user.code, 7);

        let mut polled_custom = MaybeUninit::<SDL_Event>::zeroed();
        assert_eq!(SDL_PollEvent(polled_custom.as_mut_ptr()), 1);
        let polled_custom = polled_custom.assume_init();
        assert_eq!(polled_custom.user.type_, event_type);
        assert_eq!(polled_custom.user.code, 7);

        assert_eq!(SDL_EventState(blocked_type, SDL_DISABLE as i32), 1);
        let mut blocked = SDL_Event {
            user: SDL_UserEvent {
                type_: blocked_type,
                timestamp: 0,
                windowID: 0,
                code: 11,
                data1: ptr::null_mut(),
                data2: ptr::null_mut(),
            },
        };
        assert_eq!(SDL_PushEvent(&mut blocked), 0);
        assert_eq!(SDL_HasEvent(blocked_type), 0);

        let mut key = SDL_Event {
            key: SDL_KeyboardEvent {
                type_: SDL_EventType_SDL_KEYDOWN,
                timestamp: 0,
                windowID: 0,
                state: SDL_PRESSED as u8,
                repeat: 0,
                padding2: 0,
                padding3: 0,
                keysym: SDL_Keysym {
                    scancode: SDL_Scancode_SDL_SCANCODE_A,
                    sym: SDL_KeyCode_SDLK_a as i32,
                    mod_: 0,
                    unused: 0,
                },
            },
        };
        assert_eq!(
            SDL_PeepEvents(
                &mut key,
                1,
                SDL_eventaction_SDL_ADDEVENT,
                SDL_EventType_SDL_FIRSTEVENT,
                SDL_EventType_SDL_LASTEVENT,
            ),
            1
        );
        let mut polled_key = MaybeUninit::<SDL_Event>::zeroed();
        assert_eq!(SDL_PollEvent(polled_key.as_mut_ptr()), 1);
        let polled_key = polled_key.assume_init();
        assert_eq!(polled_key.key.type_, SDL_EventType_SDL_KEYDOWN);
        assert_eq!(polled_key.key.keysym.sym, SDL_KeyCode_SDLK_a as i32);

        let mut mouse = SDL_Event {
            button: SDL_MouseButtonEvent {
                type_: SDL_EventType_SDL_MOUSEBUTTONDOWN,
                timestamp: 0,
                windowID: 0,
                which: 0,
                button: 1,
                state: SDL_PRESSED as u8,
                clicks: 1,
                padding1: 0,
                x: 2,
                y: 3,
            },
        };
        assert_eq!(SDL_PushEvent(&mut mouse), 1);
        let mut polled_mouse = MaybeUninit::<SDL_Event>::zeroed();
        assert_eq!(SDL_PollEvent(polled_mouse.as_mut_ptr()), 1);
        let polled_mouse = polled_mouse.assume_init();
        assert_eq!(polled_mouse.button.type_, SDL_EventType_SDL_MOUSEBUTTONDOWN);
        assert_eq!((polled_mouse.button.x, polled_mouse.button.y), (2, 3));
    }
}

#[test]
fn event_filter_and_watch_callbacks_can_query_queue_without_deadlock() {
    let _serial = testutils::serial_lock();
    let _video = testutils::ScopedEnvVar::set("SDL_VIDEODRIVER", "dummy");
    let _sdl = SdlGuard::init_validator_flags();
    let state = ReentrantCallbackState {
        filter_calls: AtomicU32::new(0),
        watch_calls: AtomicU32::new(0),
        errors: AtomicU32::new(0),
    };

    unsafe {
        SDL_FlushEvents(SDL_EventType_SDL_FIRSTEVENT, SDL_EventType_SDL_LASTEVENT);
        SDL_SetEventFilter(
            Some(query_queue_filter),
            (&state as *const ReentrantCallbackState).cast_mut().cast(),
        );
        SDL_AddEventWatch(
            Some(query_queue_watch),
            (&state as *const ReentrantCallbackState).cast_mut().cast(),
        );

        let event_type = SDL_RegisterEvents(1);
        assert_ne!(event_type, u32::MAX, "{}", testutils::current_error());
        let mut event = SDL_Event {
            user: SDL_UserEvent {
                type_: event_type,
                timestamp: 0,
                windowID: 0,
                code: 17,
                data1: ptr::null_mut(),
                data2: ptr::null_mut(),
            },
        };
        assert_eq!(
            SDL_PushEvent(&mut event),
            1,
            "{}",
            testutils::current_error()
        );

        SDL_DelEventWatch(
            Some(query_queue_watch),
            (&state as *const ReentrantCallbackState).cast_mut().cast(),
        );
        SDL_SetEventFilter(None, ptr::null_mut());

        assert_eq!(state.errors.load(Ordering::SeqCst), 0);
        assert_eq!(state.filter_calls.load(Ordering::SeqCst), 1);
        assert_eq!(state.watch_calls.load(Ordering::SeqCst), 1);

        let mut out = MaybeUninit::<SDL_Event>::zeroed();
        assert_eq!(SDL_PollEvent(out.as_mut_ptr()), 1);
        let out = out.assume_init();
        assert_eq!(out.user.type_, event_type);
        assert_eq!(out.user.code, 17);
    }
}

#[test]
fn wait_timeout_ticks_delay_and_timer_event_match_validator_cases() {
    let _serial = testutils::serial_lock();
    let _video = testutils::ScopedEnvVar::set("SDL_VIDEODRIVER", "dummy");
    let _sdl = SdlGuard::init_validator_flags();

    unsafe {
        SDL_FlushEvents(SDL_EventType_SDL_FIRSTEVENT, SDL_EventType_SDL_LASTEVENT);

        let start = SDL_GetTicks();
        SDL_Delay(20);
        assert!(SDL_GetTicks().wrapping_sub(start) >= 5);

        let before_wait = SDL_GetTicks();
        let mut out = MaybeUninit::<SDL_Event>::zeroed();
        assert_eq!(SDL_WaitEventTimeout(out.as_mut_ptr(), 10), 0);
        assert!(SDL_GetTicks().wrapping_sub(before_wait) >= 5);

        let event_type = SDL_RegisterEvents(1);
        assert_ne!(event_type, u32::MAX, "{}", testutils::current_error());
        let context = TimerEventContext {
            event_type,
            pushed: AtomicU32::new(0),
        };
        let timer_id = SDL_AddTimer(
            10,
            Some(post_timer_event),
            (&context as *const TimerEventContext).cast_mut().cast(),
        );
        assert_ne!(timer_id, 0, "{}", testutils::current_error());

        assert_eq!(SDL_WaitEventTimeout(out.as_mut_ptr(), 1_000), 1);
        let out = out.assume_init();
        assert_eq!(out.user.type_, event_type);
        assert_eq!(out.user.code, 99);
        assert_eq!(context.pushed.load(Ordering::SeqCst), 1);
    }
}
