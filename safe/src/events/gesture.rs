use std::sync::OnceLock;

use crate::abi::generated_types::{SDL_GestureID, SDL_RWops, SDL_TouchID};

struct GestureApi {
    load_dollar_templates: unsafe extern "C" fn(SDL_TouchID, *mut SDL_RWops) -> libc::c_int,
    record_gesture: unsafe extern "C" fn(SDL_TouchID) -> libc::c_int,
    save_all_dollar_templates: unsafe extern "C" fn(*mut SDL_RWops) -> libc::c_int,
    save_dollar_template: unsafe extern "C" fn(SDL_GestureID, *mut SDL_RWops) -> libc::c_int,
}

fn api() -> &'static GestureApi {
    static API: OnceLock<GestureApi> = OnceLock::new();
    API.get_or_init(|| GestureApi {
        load_dollar_templates: crate::video::load_symbol(b"SDL_LoadDollarTemplates\0"),
        record_gesture: crate::video::load_symbol(b"SDL_RecordGesture\0"),
        save_all_dollar_templates: crate::video::load_symbol(b"SDL_SaveAllDollarTemplates\0"),
        save_dollar_template: crate::video::load_symbol(b"SDL_SaveDollarTemplate\0"),
    })
}

#[no_mangle]
pub unsafe extern "C" fn SDL_LoadDollarTemplates(
    touchId: SDL_TouchID,
    src: *mut SDL_RWops,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().load_dollar_templates)(touchId, src)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_RecordGesture(touchId: SDL_TouchID) -> libc::c_int {
    crate::video::clear_real_error();
    (api().record_gesture)(touchId)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SaveAllDollarTemplates(dst: *mut SDL_RWops) -> libc::c_int {
    crate::video::clear_real_error();
    (api().save_all_dollar_templates)(dst)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SaveDollarTemplate(
    gestureId: SDL_GestureID,
    dst: *mut SDL_RWops,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().save_dollar_template)(gestureId, dst)
}
