use std::ffi::CStr;
use std::path::PathBuf;

#[no_mangle]
pub unsafe extern "C" fn SDL_GetBasePath() -> *mut libc::c_char {
    let path = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));

    let mut text = path.to_string_lossy().into_owned();
    if !text.ends_with('/') {
        text.push('/');
    }
    crate::core::memory::alloc_c_string(&text)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetPrefPath(
    org: *const libc::c_char,
    app: *const libc::c_char,
) -> *mut libc::c_char {
    if app.is_null() {
        let _ = crate::core::error::invalid_param_error("app");
        return std::ptr::null_mut();
    }

    let app = CStr::from_ptr(app).to_string_lossy();
    let org = if org.is_null() {
        String::new()
    } else {
        CStr::from_ptr(org).to_string_lossy().into_owned()
    };

    let base = if let Ok(path) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(path)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".local/share")
    } else {
        let _ = crate::core::error::set_error_message(
            "neither XDG_DATA_HOME nor HOME environment is set",
        );
        return std::ptr::null_mut();
    };

    let full = if org.is_empty() {
        base.join(app.as_ref())
    } else {
        base.join(org).join(app.as_ref())
    };

    if let Err(error) = std::fs::create_dir_all(&full) {
        let _ = crate::core::error::set_error_message(&format!(
            "Couldn't create directory '{}': {}",
            full.display(),
            error
        ));
        return std::ptr::null_mut();
    }

    let mut text = full.to_string_lossy().into_owned();
    if !text.ends_with('/') {
        text.push('/');
    }
    crate::core::memory::alloc_c_string(&text)
}
