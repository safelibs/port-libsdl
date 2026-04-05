use crate::abi::generated_types::{SDL_MessageBoxData, SDL_Window, Uint32};

#[no_mangle]
pub unsafe extern "C" fn SDL_ShowMessageBox(
    _messageboxdata: *const SDL_MessageBoxData,
    buttonid: *mut libc::c_int,
) -> libc::c_int {
    if !buttonid.is_null() {
        *buttonid = 0;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ShowSimpleMessageBox(
    _flags: Uint32,
    _title: *const libc::c_char,
    _message: *const libc::c_char,
    _window: *mut SDL_Window,
) -> libc::c_int {
    0
}
