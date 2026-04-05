use std::sync::OnceLock;

use crate::abi::generated_types::{SDL_MessageBoxData, SDL_Window, Uint32};

struct MessageBoxApi {
    show_message_box:
        unsafe extern "C" fn(*const SDL_MessageBoxData, *mut libc::c_int) -> libc::c_int,
    show_simple_message_box: unsafe extern "C" fn(
        Uint32,
        *const libc::c_char,
        *const libc::c_char,
        *mut SDL_Window,
    ) -> libc::c_int,
}

fn api() -> &'static MessageBoxApi {
    static API: OnceLock<MessageBoxApi> = OnceLock::new();
    API.get_or_init(|| MessageBoxApi {
        show_message_box: crate::video::load_symbol(b"SDL_ShowMessageBox\0"),
        show_simple_message_box: crate::video::load_symbol(b"SDL_ShowSimpleMessageBox\0"),
    })
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ShowMessageBox(
    messageboxdata: *const SDL_MessageBoxData,
    buttonid: *mut libc::c_int,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().show_message_box)(messageboxdata, buttonid)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ShowSimpleMessageBox(
    flags: Uint32,
    title: *const libc::c_char,
    message: *const libc::c_char,
    window: *mut SDL_Window,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().show_simple_message_box)(flags, title, message, window)
}
