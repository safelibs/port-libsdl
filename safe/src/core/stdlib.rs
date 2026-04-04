use std::ffi::CStr;
use std::ptr;

use crate::abi::generated_types::SDL_iconv_t;

type CompareCallback = Option<
    unsafe extern "C" fn(arg1: *const libc::c_void, arg2: *const libc::c_void) -> libc::c_int,
>;

unsafe extern "C" {
    fn iconv_open(tocode: *const libc::c_char, fromcode: *const libc::c_char) -> *mut libc::c_void;
    fn iconv(
        cd: *mut libc::c_void,
        inbuf: *mut *mut libc::c_char,
        inbytesleft: *mut usize,
        outbuf: *mut *mut libc::c_char,
        outbytesleft: *mut usize,
    ) -> usize;
    fn iconv_close(cd: *mut libc::c_void) -> libc::c_int;
}

fn copy_c_string(src: *const libc::c_char) -> *mut libc::c_char {
    if src.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let bytes = CStr::from_ptr(src).to_bytes_with_nul();
        crate::core::memory::alloc_bytes(bytes)
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_getenv(name: *const libc::c_char) -> *mut libc::c_char {
    if name.is_null() {
        return std::ptr::null_mut();
    }
    libc::getenv(name)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_setenv(
    name: *const libc::c_char,
    value: *const libc::c_char,
    overwrite: libc::c_int,
) -> libc::c_int {
    if name.is_null() || value.is_null() {
        return crate::core::error::invalid_param_error(if name.is_null() {
            "name"
        } else {
            "value"
        });
    }
    libc::setenv(name, value, overwrite)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_qsort(
    base: *mut libc::c_void,
    nmemb: usize,
    size: usize,
    compare: CompareCallback,
) {
    if base.is_null() || nmemb == 0 || size == 0 {
        return;
    }
    libc::qsort(base, nmemb, size, compare);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_bsearch(
    key: *const libc::c_void,
    base: *const libc::c_void,
    nmemb: usize,
    size: usize,
    compare: CompareCallback,
) -> *mut libc::c_void {
    if key.is_null() || base.is_null() || nmemb == 0 || size == 0 {
        return std::ptr::null_mut();
    }
    libc::bsearch(key, base, nmemb, size, compare)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_abs(x: libc::c_int) -> libc::c_int {
    x.abs()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_memset(
    dst: *mut libc::c_void,
    c: libc::c_int,
    len: usize,
) -> *mut libc::c_void {
    libc::memset(dst, c, len)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_memcmp(
    s1: *const libc::c_void,
    s2: *const libc::c_void,
    len: usize,
) -> libc::c_int {
    libc::memcmp(s1, s2, len)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_strlen(str_: *const libc::c_char) -> usize {
    if str_.is_null() {
        0
    } else {
        libc::strlen(str_)
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_strlcpy(
    dst: *mut libc::c_char,
    src: *const libc::c_char,
    maxlen: usize,
) -> usize {
    let src_len = SDL_strlen(src);
    if maxlen == 0 || dst.is_null() {
        return src_len;
    }
    let copy_len = src_len.min(maxlen.saturating_sub(1));
    if copy_len > 0 && !src.is_null() {
        ptr::copy_nonoverlapping(src, dst, copy_len);
    }
    *dst.add(copy_len) = 0;
    src_len
}

#[no_mangle]
pub unsafe extern "C" fn SDL_strlcat(
    dst: *mut libc::c_char,
    src: *const libc::c_char,
    maxlen: usize,
) -> usize {
    if dst.is_null() {
        return SDL_strlen(src);
    }
    let dst_len = libc::strnlen(dst, maxlen);
    let src_len = SDL_strlen(src);
    if dst_len == maxlen {
        return maxlen + src_len;
    }
    SDL_strlcpy(dst.add(dst_len), src, maxlen - dst_len);
    dst_len + src_len
}

#[no_mangle]
pub unsafe extern "C" fn SDL_strdup(str_: *const libc::c_char) -> *mut libc::c_char {
    copy_c_string(str_)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_strcmp(
    str1: *const libc::c_char,
    str2: *const libc::c_char,
) -> libc::c_int {
    match (str1.is_null(), str2.is_null()) {
        (true, true) => 0,
        (true, false) => -1,
        (false, true) => 1,
        (false, false) => libc::strcmp(str1, str2),
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_atoi(str_: *const libc::c_char) -> libc::c_int {
    if str_.is_null() {
        0
    } else {
        libc::atoi(str_)
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_strtoul(
    str_: *const libc::c_char,
    endp: *mut *mut libc::c_char,
    base: libc::c_int,
) -> libc::c_ulong {
    if str_.is_null() {
        if !endp.is_null() {
            *endp = std::ptr::null_mut();
        }
        return 0;
    }
    libc::strtoul(str_, endp, base)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_iconv_open(
    tocode: *const libc::c_char,
    fromcode: *const libc::c_char,
) -> SDL_iconv_t {
    if tocode.is_null() || fromcode.is_null() {
        let _ = crate::core::error::invalid_param_error(if tocode.is_null() {
            "tocode"
        } else {
            "fromcode"
        });
        return std::ptr::null_mut();
    }
    let cd = iconv_open(tocode, fromcode);
    if cd as isize == -1 {
        let _ = crate::core::error::set_error_message(&crate::core::system::last_os_error_message(
            "iconv_open() failed",
        ));
        std::ptr::null_mut()
    } else {
        cd.cast()
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_iconv_close(cd: SDL_iconv_t) -> libc::c_int {
    if cd.is_null() {
        return crate::core::error::invalid_param_error("cd");
    }
    iconv_close(cd.cast())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_iconv(
    cd: SDL_iconv_t,
    inbuf: *mut *const libc::c_char,
    inbytesleft: *mut usize,
    outbuf: *mut *mut libc::c_char,
    outbytesleft: *mut usize,
) -> usize {
    if cd.is_null() {
        let _ = crate::core::error::invalid_param_error("cd");
        return usize::MAX;
    }
    iconv(
        cd.cast(),
        inbuf.cast::<*mut libc::c_char>(),
        inbytesleft,
        outbuf,
        outbytesleft,
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDL_iconv_string(
    tocode: *const libc::c_char,
    fromcode: *const libc::c_char,
    inbuf: *const libc::c_char,
    inbytesleft: usize,
) -> *mut libc::c_char {
    if tocode.is_null() || fromcode.is_null() || inbuf.is_null() {
        let _ = crate::core::error::invalid_param_error(if tocode.is_null() {
            "tocode"
        } else if fromcode.is_null() {
            "fromcode"
        } else {
            "inbuf"
        });
        return std::ptr::null_mut();
    }

    let cd = SDL_iconv_open(tocode, fromcode);
    if cd.is_null() {
        return std::ptr::null_mut();
    }

    let mut capacity = inbytesleft.saturating_mul(4).saturating_add(32).max(32);
    let mut output = crate::core::memory::SDL_malloc(capacity) as *mut libc::c_char;
    if output.is_null() {
        let _ = SDL_iconv_close(cd);
        let _ = crate::core::error::out_of_memory_error();
        return std::ptr::null_mut();
    }

    let mut out_ptr = output;
    let mut out_left = capacity;
    let mut input = inbuf;
    let mut input_left = inbytesleft;

    loop {
        let rc = SDL_iconv(cd, &mut input, &mut input_left, &mut out_ptr, &mut out_left);
        if rc != usize::MAX {
            break;
        }

        if std::io::Error::last_os_error().raw_os_error() != Some(libc::E2BIG) {
            let _ = crate::core::error::set_error_message(
                &crate::core::system::last_os_error_message("iconv() failed"),
            );
            crate::core::memory::SDL_free(output.cast());
            let _ = SDL_iconv_close(cd);
            return std::ptr::null_mut();
        }

        let used = out_ptr.offset_from(output) as usize;
        capacity = capacity.saturating_mul(2).max(used + 32);
        let grown = crate::core::memory::SDL_realloc(output.cast(), capacity) as *mut libc::c_char;
        if grown.is_null() {
            crate::core::memory::SDL_free(output.cast());
            let _ = SDL_iconv_close(cd);
            let _ = crate::core::error::out_of_memory_error();
            return std::ptr::null_mut();
        }
        output = grown;
        out_ptr = output.add(used);
        out_left = capacity - used;
    }

    if out_left == 0 {
        let used = out_ptr.offset_from(output) as usize;
        let grown =
            crate::core::memory::SDL_realloc(output.cast(), capacity + 1) as *mut libc::c_char;
        if grown.is_null() {
            crate::core::memory::SDL_free(output.cast());
            let _ = SDL_iconv_close(cd);
            let _ = crate::core::error::out_of_memory_error();
            return std::ptr::null_mut();
        }
        output = grown;
        out_ptr = output.add(used);
    }
    *out_ptr = 0;
    let _ = SDL_iconv_close(cd);
    output
}
