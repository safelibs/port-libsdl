use std::collections::{HashMap, VecDeque};
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::abi::generated_types::{
    SDL_Event, SDL_EventFilter, SDL_EventType_SDL_LASTEVENT, SDL_EventType_SDL_QUIT,
    SDL_EventType_SDL_USEREVENT, SDL_bool, SDL_eventaction, SDL_eventaction_SDL_ADDEVENT,
    SDL_eventaction_SDL_GETEVENT, SDL_eventaction_SDL_PEEKEVENT, Uint32, SDL_DISABLE, SDL_ENABLE,
    SDL_INIT_EVENTS, SDL_QUERY,
};

#[derive(Clone, Copy)]
struct CallbackRecord {
    callback: SDL_EventFilter,
    callback_addr: usize,
    userdata: usize,
}

struct EventQueueState {
    active: bool,
    queue: VecDeque<SDL_Event>,
    watchers: Vec<CallbackRecord>,
    filter: Option<CallbackRecord>,
    event_states: HashMap<Uint32, u8>,
    next_user_event: Uint32,
}

unsafe impl Send for EventQueueState {}

impl Default for EventQueueState {
    fn default() -> Self {
        Self {
            active: false,
            queue: VecDeque::new(),
            watchers: Vec::new(),
            filter: None,
            event_states: HashMap::new(),
            next_user_event: SDL_EventType_SDL_USEREVENT as Uint32,
        }
    }
}

struct EventRuntime {
    state: Mutex<EventQueueState>,
    condvar: Condvar,
}

fn runtime() -> &'static EventRuntime {
    static RUNTIME: OnceLock<EventRuntime> = OnceLock::new();
    RUNTIME.get_or_init(|| EventRuntime {
        state: Mutex::new(EventQueueState::default()),
        condvar: Condvar::new(),
    })
}

fn lock_event_state() -> std::sync::MutexGuard<'static, EventQueueState> {
    match runtime().state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn ensure_active(state: &EventQueueState) -> Result<(), ()> {
    if state.active {
        Ok(())
    } else {
        let _ = crate::core::error::set_error_message("Events subsystem has not been initialized");
        Err(())
    }
}

fn current_processing_state(state: &EventQueueState, type_: Uint32) -> u8 {
    *state
        .event_states
        .get(&type_)
        .unwrap_or(&(SDL_ENABLE as u8))
}

fn event_enabled(state: &EventQueueState, type_: Uint32) -> bool {
    current_processing_state(state, type_) != SDL_DISABLE as u8
}

fn call_filter(record: CallbackRecord, event: &mut SDL_Event) -> bool {
    record
        .callback
        .map(|callback| unsafe { callback(record.userdata as *mut libc::c_void, event) != 0 })
        .unwrap_or(true)
}

fn callback_addr(callback: SDL_EventFilter) -> usize {
    callback.map(|callback| callback as usize).unwrap_or(0)
}

fn notify_watchers(state: &EventQueueState, event: &mut SDL_Event) {
    for watcher in state.watchers.iter().copied() {
        if let Some(callback) = watcher.callback {
            unsafe {
                callback(watcher.userdata as *mut libc::c_void, event);
            }
        }
    }
}

fn queue_contains_type(state: &EventQueueState, minType: Uint32, maxType: Uint32) -> bool {
    state
        .queue
        .iter()
        .any(|event| unsafe { event.type_ >= minType && event.type_ <= maxType })
}

fn queue_count_type(state: &EventQueueState, minType: Uint32, maxType: Uint32) -> libc::c_int {
    state
        .queue
        .iter()
        .filter(|event| unsafe { event.type_ >= minType && event.type_ <= maxType })
        .count() as libc::c_int
}

fn push_copied_event(
    state: &mut EventQueueState,
    mut event: SDL_Event,
    apply_global_filter: bool,
) -> libc::c_int {
    let event_type = unsafe { event.type_ };
    if !event_enabled(state, event_type) {
        return 0;
    }
    if apply_global_filter {
        if let Some(filter) = state.filter {
            if !call_filter(filter, &mut event) {
                return 0;
            }
        }
        notify_watchers(state, &mut event);
    }
    state.queue.push_back(event);
    runtime().condvar.notify_all();
    1
}

fn pop_matching_event(
    state: &mut EventQueueState,
    minType: Uint32,
    maxType: Uint32,
) -> Option<SDL_Event> {
    let position = state
        .queue
        .iter()
        .position(|event| unsafe { event.type_ >= minType && event.type_ <= maxType })?;
    state.queue.remove(position)
}

fn peek_matching_event(
    state: &EventQueueState,
    minType: Uint32,
    maxType: Uint32,
) -> Option<SDL_Event> {
    state
        .queue
        .iter()
        .copied()
        .find(|event| unsafe { event.type_ >= minType && event.type_ <= maxType })
}

pub(crate) fn init_event_subsystem() -> Result<(), ()> {
    let _ = SDL_INIT_EVENTS;
    let mut state = lock_event_state();
    if !state.active {
        *state = EventQueueState::default();
        state.active = true;
    }
    Ok(())
}

pub(crate) fn quit_event_subsystem() {
    *lock_event_state() = EventQueueState::default();
    runtime().condvar.notify_all();
}

#[no_mangle]
pub unsafe extern "C" fn SDL_AddEventWatch(filter: SDL_EventFilter, userdata: *mut libc::c_void) {
    let mut state = lock_event_state();
    if ensure_active(&state).is_err() {
        return;
    }
    state.watchers.push(CallbackRecord {
        callback: filter,
        callback_addr: callback_addr(filter),
        userdata: userdata as usize,
    });
}

#[no_mangle]
pub unsafe extern "C" fn SDL_DelEventWatch(filter: SDL_EventFilter, userdata: *mut libc::c_void) {
    let mut state = lock_event_state();
    if ensure_active(&state).is_err() {
        return;
    }
    let userdata = userdata as usize;
    let filter_addr = callback_addr(filter);
    state
        .watchers
        .retain(|watcher| watcher.callback_addr != filter_addr || watcher.userdata != userdata);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_EventState(type_: Uint32, state_value: libc::c_int) -> u8 {
    let mut state = lock_event_state();
    if ensure_active(&state).is_err() {
        return SDL_DISABLE as u8;
    }

    let previous = current_processing_state(&state, type_);
    if state_value == SDL_QUERY {
        return previous;
    }

    let next = if state_value == SDL_DISABLE as libc::c_int {
        SDL_DISABLE as u8
    } else {
        SDL_ENABLE as u8
    };
    state.event_states.insert(type_, next);
    if next == SDL_DISABLE as u8 {
        state.queue.retain(|event| unsafe { event.type_ != type_ });
    }
    previous
}

#[no_mangle]
pub unsafe extern "C" fn SDL_FilterEvents(filter: SDL_EventFilter, userdata: *mut libc::c_void) {
    let mut state = lock_event_state();
    if ensure_active(&state).is_err() {
        return;
    }
    let Some(callback) = filter else {
        return;
    };
    let userdata = userdata as usize;
    state
        .queue
        .retain_mut(|event| callback(userdata as *mut libc::c_void, event) != 0);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_FlushEvent(type_: Uint32) {
    SDL_FlushEvents(type_, type_);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_FlushEvents(minType: Uint32, maxType: Uint32) {
    let mut state = lock_event_state();
    if ensure_active(&state).is_err() {
        return;
    }
    state
        .queue
        .retain(|event| unsafe { event.type_ < minType || event.type_ > maxType });
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetEventFilter(
    filter: *mut SDL_EventFilter,
    userdata: *mut *mut libc::c_void,
) -> SDL_bool {
    let state = lock_event_state();
    if ensure_active(&state).is_err() {
        return 0;
    }
    if let Some(record) = state.filter {
        if !filter.is_null() {
            *filter = record.callback;
        }
        if !userdata.is_null() {
            *userdata = record.userdata as *mut libc::c_void;
        }
        1
    } else {
        if !filter.is_null() {
            *filter = None;
        }
        if !userdata.is_null() {
            *userdata = std::ptr::null_mut();
        }
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HasEvent(type_: Uint32) -> SDL_bool {
    SDL_HasEvents(type_, type_)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HasEvents(minType: Uint32, maxType: Uint32) -> SDL_bool {
    let state = lock_event_state();
    if ensure_active(&state).is_err() {
        return 0;
    }
    queue_contains_type(&state, minType, maxType) as SDL_bool
}

#[no_mangle]
pub unsafe extern "C" fn SDL_PeepEvents(
    events: *mut SDL_Event,
    numevents: libc::c_int,
    action: SDL_eventaction,
    minType: Uint32,
    maxType: Uint32,
) -> libc::c_int {
    let mut state = lock_event_state();
    if ensure_active(&state).is_err() {
        return -1;
    }

    match action {
        SDL_eventaction_SDL_ADDEVENT => {
            if events.is_null() || numevents < 0 {
                return crate::core::error::invalid_param_error("events");
            }
            let mut inserted = 0;
            for index in 0..numevents as usize {
                inserted += push_copied_event(&mut state, *events.add(index), false);
            }
            inserted
        }
        SDL_eventaction_SDL_PEEKEVENT | SDL_eventaction_SDL_GETEVENT => {
            if numevents <= 0 || events.is_null() {
                return queue_count_type(&state, minType, maxType);
            }

            let mut copied = 0;
            if action == SDL_eventaction_SDL_PEEKEVENT {
                for event in state
                    .queue
                    .iter()
                    .copied()
                    .filter(|event| unsafe { event.type_ >= minType && event.type_ <= maxType })
                    .take(numevents as usize)
                {
                    *events.add(copied as usize) = event;
                    copied += 1;
                }
                return copied;
            }

            let mut removed_indices = Vec::new();
            for (index, event) in state.queue.iter().copied().enumerate() {
                if unsafe { event.type_ < minType || event.type_ > maxType } {
                    continue;
                }
                *events.add(copied as usize) = event;
                removed_indices.push(index);
                copied += 1;
                if copied == numevents {
                    break;
                }
            }
            for index in removed_indices.into_iter().rev() {
                let _ = state.queue.remove(index);
            }
            copied
        }
        _ => crate::core::error::set_error_message("Unsupported event queue action"),
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_PollEvent(event: *mut SDL_Event) -> libc::c_int {
    let mut state = lock_event_state();
    if ensure_active(&state).is_err() {
        return 0;
    }
    match pop_matching_event(&mut state, 0, SDL_EventType_SDL_LASTEVENT as Uint32) {
        Some(queued) => {
            if !event.is_null() {
                *event = queued;
            }
            1
        }
        None => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_PumpEvents() {}

#[no_mangle]
pub unsafe extern "C" fn SDL_PushEvent(event: *mut SDL_Event) -> libc::c_int {
    if event.is_null() {
        return crate::core::error::invalid_param_error("event");
    }
    let mut state = lock_event_state();
    if ensure_active(&state).is_err() {
        return -1;
    }
    push_copied_event(&mut state, *event, true)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_RegisterEvents(numevents: libc::c_int) -> Uint32 {
    if numevents <= 0 {
        let _ = crate::core::error::set_error_message("Number of events must be positive");
        return u32::MAX;
    }

    let mut state = lock_event_state();
    if ensure_active(&state).is_err() {
        return u32::MAX;
    }

    let end = state.next_user_event.saturating_add(numevents as Uint32);
    if end > SDL_EventType_SDL_LASTEVENT as Uint32 {
        let _ = crate::core::error::set_error_message("No more user events available");
        return u32::MAX;
    }

    let base = state.next_user_event;
    state.next_user_event = end;
    base
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetEventFilter(filter: SDL_EventFilter, userdata: *mut libc::c_void) {
    let mut state = lock_event_state();
    if ensure_active(&state).is_err() {
        return;
    }
    state.filter = filter.map(|callback| CallbackRecord {
        callback: Some(callback),
        callback_addr: callback as usize,
        userdata: userdata as usize,
    });
}

#[no_mangle]
pub unsafe extern "C" fn SDL_WaitEvent(event: *mut SDL_Event) -> libc::c_int {
    SDL_WaitEventTimeout(event, -1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_WaitEventTimeout(
    event: *mut SDL_Event,
    timeout: libc::c_int,
) -> libc::c_int {
    let deadline = if timeout < 0 {
        None
    } else {
        Some(Instant::now() + Duration::from_millis(timeout as u64))
    };

    let mut state = lock_event_state();
    if ensure_active(&state).is_err() {
        return 0;
    }

    loop {
        if let Some(queued) =
            pop_matching_event(&mut state, 0, SDL_EventType_SDL_LASTEVENT as Uint32)
        {
            if !event.is_null() {
                *event = queued;
            }
            return 1;
        }

        match deadline {
            Some(deadline) => {
                let now = Instant::now();
                if now >= deadline {
                    return 0;
                }
                let timeout = deadline.saturating_duration_since(now);
                state = match runtime().condvar.wait_timeout(state, timeout) {
                    Ok((guard, _)) => guard,
                    Err(poisoned) => poisoned.into_inner().0,
                };
            }
            None => {
                state = match runtime().condvar.wait(state) {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
            }
        }
    }
}

pub unsafe extern "C" fn SDL_QuitRequested() -> SDL_bool {
    let state = lock_event_state();
    if !state.active {
        return 0;
    }
    queue_contains_type(
        &state,
        SDL_EventType_SDL_QUIT as Uint32,
        SDL_EventType_SDL_QUIT as Uint32,
    ) as SDL_bool
}
