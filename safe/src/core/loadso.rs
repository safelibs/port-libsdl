use std::ffi::CStr;

#[no_mangle]
pub unsafe extern "C" fn SDL_LoadObject(sofile: *const libc::c_char) -> *mut libc::c_void {
    if sofile.is_null() {
        let _ = crate::core::error::invalid_param_error("sofile");
        return std::ptr::null_mut();
    }
    let handle = libc::dlopen(sofile, libc::RTLD_NOW | libc::RTLD_LOCAL);
    if handle.is_null() {
        let detail = if libc::dlerror().is_null() {
            "dlopen() failed".to_string()
        } else {
            CStr::from_ptr(libc::dlerror())
                .to_string_lossy()
                .into_owned()
        };
        let _ = crate::core::error::set_error_message(&detail);
    }
    handle
}

#[no_mangle]
pub unsafe extern "C" fn SDL_LoadFunction(
    handle: *mut libc::c_void,
    name: *const libc::c_char,
) -> *mut libc::c_void {
    if handle.is_null() {
        let _ = crate::core::error::invalid_param_error("handle");
        return std::ptr::null_mut();
    }
    if name.is_null() {
        let _ = crate::core::error::invalid_param_error("name");
        return std::ptr::null_mut();
    }
    let symbol = libc::dlsym(handle, name);
    if symbol.is_null() {
        let detail = if libc::dlerror().is_null() {
            "dlsym() failed".to_string()
        } else {
            CStr::from_ptr(libc::dlerror())
                .to_string_lossy()
                .into_owned()
        };
        let _ = crate::core::error::set_error_message(&detail);
    }
    symbol
}

#[no_mangle]
pub unsafe extern "C" fn SDL_UnloadObject(handle: *mut libc::c_void) {
    if !handle.is_null() {
        libc::dlclose(handle);
    }
}
