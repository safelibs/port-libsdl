use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::abi::generated_types::{
    SDL_Event, SDL_EventFilter, SDL_EventType_SDL_QUIT, SDL_bool, SDL_eventaction,
    SDL_eventaction_SDL_PEEKEVENT, Uint32, SDL_INIT_EVENTS,
};

struct QueueApi {
    add_event_watch: unsafe extern "C" fn(SDL_EventFilter, *mut libc::c_void),
    del_event_watch: unsafe extern "C" fn(SDL_EventFilter, *mut libc::c_void),
    event_state: unsafe extern "C" fn(Uint32, libc::c_int) -> u8,
    filter_events: unsafe extern "C" fn(SDL_EventFilter, *mut libc::c_void),
    flush_event: unsafe extern "C" fn(Uint32),
    flush_events: unsafe extern "C" fn(Uint32, Uint32),
    get_event_filter:
        unsafe extern "C" fn(*mut SDL_EventFilter, *mut *mut libc::c_void) -> SDL_bool,
    has_event: unsafe extern "C" fn(Uint32) -> SDL_bool,
    has_events: unsafe extern "C" fn(Uint32, Uint32) -> SDL_bool,
    peep_events: unsafe extern "C" fn(
        *mut SDL_Event,
        libc::c_int,
        SDL_eventaction,
        Uint32,
        Uint32,
    ) -> libc::c_int,
    poll_event: unsafe extern "C" fn(*mut SDL_Event) -> libc::c_int,
    pump_events: unsafe extern "C" fn(),
    push_event: unsafe extern "C" fn(*mut SDL_Event) -> libc::c_int,
    init_subsystem: unsafe extern "C" fn(Uint32) -> libc::c_int,
    quit_subsystem: unsafe extern "C" fn(Uint32),
    register_events: unsafe extern "C" fn(libc::c_int) -> Uint32,
    set_event_filter: unsafe extern "C" fn(SDL_EventFilter, *mut libc::c_void),
    wait_event: unsafe extern "C" fn(*mut SDL_Event) -> libc::c_int,
    wait_event_timeout: unsafe extern "C" fn(*mut SDL_Event, libc::c_int) -> libc::c_int,
}

fn api() -> &'static QueueApi {
    static API: OnceLock<QueueApi> = OnceLock::new();
    API.get_or_init(|| QueueApi {
        add_event_watch: crate::video::load_symbol(b"SDL_AddEventWatch\0"),
        del_event_watch: crate::video::load_symbol(b"SDL_DelEventWatch\0"),
        event_state: crate::video::load_symbol(b"SDL_EventState\0"),
        filter_events: crate::video::load_symbol(b"SDL_FilterEvents\0"),
        flush_event: crate::video::load_symbol(b"SDL_FlushEvent\0"),
        flush_events: crate::video::load_symbol(b"SDL_FlushEvents\0"),
        get_event_filter: crate::video::load_symbol(b"SDL_GetEventFilter\0"),
        has_event: crate::video::load_symbol(b"SDL_HasEvent\0"),
        has_events: crate::video::load_symbol(b"SDL_HasEvents\0"),
        peep_events: crate::video::load_symbol(b"SDL_PeepEvents\0"),
        poll_event: crate::video::load_symbol(b"SDL_PollEvent\0"),
        pump_events: crate::video::load_symbol(b"SDL_PumpEvents\0"),
        push_event: crate::video::load_symbol(b"SDL_PushEvent\0"),
        init_subsystem: crate::video::load_symbol(b"SDL_InitSubSystem\0"),
        quit_subsystem: crate::video::load_symbol(b"SDL_QuitSubSystem\0"),
        register_events: crate::video::load_symbol(b"SDL_RegisterEvents\0"),
        set_event_filter: crate::video::load_symbol(b"SDL_SetEventFilter\0"),
        wait_event: crate::video::load_symbol(b"SDL_WaitEvent\0"),
        wait_event_timeout: crate::video::load_symbol(b"SDL_WaitEventTimeout\0"),
    })
}

pub(crate) fn init_event_subsystem() -> Result<(), ()> {
    let _ = SDL_INIT_EVENTS;
    Ok(())
}

pub(crate) fn quit_event_subsystem() {
    if !host_event_active().load(Ordering::Acquire) {
        return;
    }

    let _guard = lock_host_event_transition();
    if host_event_active().load(Ordering::Acquire) {
        crate::video::clear_real_error();
        unsafe {
            (api().quit_subsystem)(SDL_INIT_EVENTS);
        }
        host_event_active().store(false, Ordering::Release);
    }
}

fn host_event_active() -> &'static AtomicBool {
    static ACTIVE: AtomicBool = AtomicBool::new(false);
    &ACTIVE
}

fn host_event_transition() -> &'static Mutex<()> {
    static TRANSITION: OnceLock<Mutex<()>> = OnceLock::new();
    TRANSITION.get_or_init(|| Mutex::new(()))
}

fn lock_host_event_transition() -> std::sync::MutexGuard<'static, ()> {
    match host_event_transition().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

pub(crate) fn ensure_real_event_subsystem() -> bool {
    if host_event_active().load(Ordering::Acquire) {
        return false;
    }

    let _guard = lock_host_event_transition();
    if host_event_active().load(Ordering::Acquire) {
        return false;
    }
    crate::video::clear_real_error();
    let rc = unsafe { (api().init_subsystem)(SDL_INIT_EVENTS) };
    if rc == 0 {
        host_event_active().store(true, Ordering::Release);
        true
    } else {
        false
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_AddEventWatch(filter: SDL_EventFilter, userdata: *mut libc::c_void) {
    ensure_real_event_subsystem();
    crate::video::clear_real_error();
    (api().add_event_watch)(filter, userdata);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_DelEventWatch(filter: SDL_EventFilter, userdata: *mut libc::c_void) {
    ensure_real_event_subsystem();
    crate::video::clear_real_error();
    (api().del_event_watch)(filter, userdata);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_EventState(type_: Uint32, state: libc::c_int) -> u8 {
    ensure_real_event_subsystem();
    crate::video::clear_real_error();
    (api().event_state)(type_, state)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_FilterEvents(filter: SDL_EventFilter, userdata: *mut libc::c_void) {
    ensure_real_event_subsystem();
    crate::video::clear_real_error();
    (api().filter_events)(filter, userdata);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_FlushEvent(type_: Uint32) {
    ensure_real_event_subsystem();
    crate::video::clear_real_error();
    (api().flush_event)(type_);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_FlushEvents(minType: Uint32, maxType: Uint32) {
    ensure_real_event_subsystem();
    crate::video::clear_real_error();
    (api().flush_events)(minType, maxType);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetEventFilter(
    filter: *mut SDL_EventFilter,
    userdata: *mut *mut libc::c_void,
) -> SDL_bool {
    ensure_real_event_subsystem();
    crate::video::clear_real_error();
    (api().get_event_filter)(filter, userdata)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HasEvent(type_: Uint32) -> SDL_bool {
    ensure_real_event_subsystem();
    crate::video::clear_real_error();
    (api().has_event)(type_)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HasEvents(minType: Uint32, maxType: Uint32) -> SDL_bool {
    ensure_real_event_subsystem();
    crate::video::clear_real_error();
    (api().has_events)(minType, maxType)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_PeepEvents(
    events: *mut SDL_Event,
    numevents: libc::c_int,
    action: SDL_eventaction,
    minType: Uint32,
    maxType: Uint32,
) -> libc::c_int {
    ensure_real_event_subsystem();
    crate::video::clear_real_error();
    (api().peep_events)(events, numevents, action, minType, maxType)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_PollEvent(event: *mut SDL_Event) -> libc::c_int {
    ensure_real_event_subsystem();
    crate::video::clear_real_error();
    (api().poll_event)(event)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_PumpEvents() {
    ensure_real_event_subsystem();
    crate::video::clear_real_error();
    (api().pump_events)();
}

#[no_mangle]
pub unsafe extern "C" fn SDL_PushEvent(event: *mut SDL_Event) -> libc::c_int {
    ensure_real_event_subsystem();
    crate::video::clear_real_error();
    (api().push_event)(event)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_RegisterEvents(numevents: libc::c_int) -> Uint32 {
    ensure_real_event_subsystem();
    crate::video::clear_real_error();
    (api().register_events)(numevents)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetEventFilter(filter: SDL_EventFilter, userdata: *mut libc::c_void) {
    ensure_real_event_subsystem();
    crate::video::clear_real_error();
    (api().set_event_filter)(filter, userdata);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_WaitEvent(event: *mut SDL_Event) -> libc::c_int {
    ensure_real_event_subsystem();
    crate::video::clear_real_error();
    (api().wait_event)(event)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_WaitEventTimeout(
    event: *mut SDL_Event,
    timeout: libc::c_int,
) -> libc::c_int {
    ensure_real_event_subsystem();
    crate::video::clear_real_error();
    (api().wait_event_timeout)(event, timeout)
}

pub unsafe extern "C" fn SDL_QuitRequested() -> SDL_bool {
    ensure_real_event_subsystem();
    crate::video::clear_real_error();
    (api().pump_events)();
    ((api().peep_events)(
        std::ptr::null_mut(),
        0,
        SDL_eventaction_SDL_PEEKEVENT,
        SDL_EventType_SDL_QUIT as Uint32,
        SDL_EventType_SDL_QUIT as Uint32,
    ) > 0) as SDL_bool
}
