use std::ffi::CStr;

const DEFAULT_EGL_PATHS: [&[u8]; 3] = [
    b"libEGL.so.1\0",
    b"libEGL.so\0",
    b"/usr/lib/x86_64-linux-gnu/libEGL.so.1\0",
];

pub struct EglLibrary {
    handle: *mut libc::c_void,
    egl_get_proc_address: unsafe extern "C" fn(*const libc::c_char) -> *mut libc::c_void,
}

unsafe impl Send for EglLibrary {}
unsafe impl Sync for EglLibrary {}

impl EglLibrary {
    pub fn load(path: Option<&CStr>) -> Result<Self, String> {
        let mut candidates = Vec::new();
        if let Some(path) = path {
            candidates.push(path.to_bytes_with_nul().to_vec());
        } else {
            candidates.extend(DEFAULT_EGL_PATHS.iter().map(|entry| entry.to_vec()));
        }

        for candidate in candidates {
            let handle = unsafe {
                libc::dlopen(candidate.as_ptr().cast(), libc::RTLD_LOCAL | libc::RTLD_NOW)
            };
            if handle.is_null() {
                continue;
            }

            let symbol = unsafe { libc::dlsym(handle, b"eglGetProcAddress\0".as_ptr().cast()) };
            if symbol.is_null() {
                unsafe {
                    libc::dlclose(handle);
                }
                continue;
            }

            let egl_get_proc_address = unsafe { std::mem::transmute_copy(&symbol) };
            return Ok(Self {
                handle,
                egl_get_proc_address,
            });
        }

        Err("unable to load an EGL runtime with eglGetProcAddress".to_string())
    }

    pub fn get_proc_address(&self, proc_name: &CStr) -> *mut libc::c_void {
        unsafe { (self.egl_get_proc_address)(proc_name.as_ptr()) }
    }
}

impl Drop for EglLibrary {
    fn drop(&mut self) {
        unsafe {
            libc::dlclose(self.handle);
        }
    }
}
