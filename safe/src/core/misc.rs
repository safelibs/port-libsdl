use std::ffi::CStr;
use std::process::Command;

#[no_mangle]
pub unsafe extern "C" fn SDL_OpenURL(url: *const libc::c_char) -> libc::c_int {
    if url.is_null() {
        return crate::core::error::invalid_param_error("url");
    }
    let url = CStr::from_ptr(url).to_string_lossy().into_owned();
    match Command::new("xdg-open")
        .arg(&url)
        .env_remove("LD_PRELOAD")
        .status()
    {
        Ok(status) if status.success() => 0,
        Ok(status) => crate::core::error::set_error_message(&format!(
            "xdg-open reported error or failed to launch: {}",
            status.code().unwrap_or(-1)
        )),
        Err(error) => crate::core::error::set_error_message(&format!("xdg-open failed: {error}")),
    }
}
