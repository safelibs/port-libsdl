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

fn ascii_isalpha(x: libc::c_int) -> bool {
    ascii_isupper(x) || ascii_islower(x)
}

fn ascii_isdigit(x: libc::c_int) -> bool {
    (b'0' as i32..=b'9' as i32).contains(&x)
}

fn ascii_isxdigit(x: libc::c_int) -> bool {
    ascii_isdigit(x)
        || (b'A' as i32..=b'F' as i32).contains(&x)
        || (b'a' as i32..=b'f' as i32).contains(&x)
}

fn ascii_isspace(x: libc::c_int) -> bool {
    x == b' ' as i32
        || x == b'\t' as i32
        || x == b'\r' as i32
        || x == b'\n' as i32
        || x == 0x0c
        || x == 0x0b
}

fn ascii_isupper(x: libc::c_int) -> bool {
    (b'A' as i32..=b'Z' as i32).contains(&x)
}

fn ascii_islower(x: libc::c_int) -> bool {
    (b'a' as i32..=b'z' as i32).contains(&x)
}

fn ascii_isprint(x: libc::c_int) -> bool {
    (0x20..0x7f).contains(&x)
}

fn ascii_toupper(x: libc::c_int) -> libc::c_int {
    if ascii_islower(x) {
        x - (b'a' as i32 - b'A' as i32)
    } else {
        x
    }
}

fn ascii_tolower(x: libc::c_int) -> libc::c_int {
    if ascii_isupper(x) {
        x + (b'a' as i32 - b'A' as i32)
    } else {
        x
    }
}

unsafe fn copy_wide_string(src: *const libc::wchar_t) -> *mut libc::wchar_t {
    if src.is_null() {
        return ptr::null_mut();
    }
    let len = SDL_wcslen(src).saturating_add(1);
    let bytes = len.saturating_mul(std::mem::size_of::<libc::wchar_t>());
    let dst = crate::core::memory::SDL_malloc(bytes) as *mut libc::wchar_t;
    if dst.is_null() {
        return ptr::null_mut();
    }
    ptr::copy_nonoverlapping(src, dst, len);
    dst
}

unsafe fn write_unsigned_radix(
    mut value: u64,
    string: *mut libc::c_char,
    radix: libc::c_int,
) -> *mut libc::c_char {
    const DIGITS: &[u8; 36] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";

    if string.is_null() {
        return ptr::null_mut();
    }
    if !(2..=36).contains(&radix) {
        *string = 0;
        return string;
    }

    let radix = radix as u64;
    let mut digits = [0u8; 65];
    let mut len = 0usize;
    if value == 0 {
        digits[len] = b'0';
        len += 1;
    } else {
        while value > 0 {
            digits[len] = DIGITS[(value % radix) as usize];
            value /= radix;
            len += 1;
        }
    }

    for index in 0..len {
        *string.add(index) = digits[len - 1 - index] as libc::c_char;
    }
    *string.add(len) = 0;
    string
}

unsafe fn write_signed_radix(
    value: i64,
    string: *mut libc::c_char,
    radix: libc::c_int,
) -> *mut libc::c_char {
    if string.is_null() {
        return ptr::null_mut();
    }
    if value < 0 {
        *string = b'-' as libc::c_char;
        write_unsigned_radix(value.unsigned_abs(), string.add(1), radix);
        string
    } else {
        write_unsigned_radix(value as u64, string, radix)
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
pub unsafe extern "C" fn SDL_memcpy(
    dst: *mut libc::c_void,
    src: *const libc::c_void,
    len: usize,
) -> *mut libc::c_void {
    libc::memcpy(dst, src, len)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_memmove(
    dst: *mut libc::c_void,
    src: *const libc::c_void,
    len: usize,
) -> *mut libc::c_void {
    libc::memmove(dst, src, len)
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
pub unsafe extern "C" fn SDL_strncmp(
    str1: *const libc::c_char,
    str2: *const libc::c_char,
    maxlen: usize,
) -> libc::c_int {
    match (str1.is_null(), str2.is_null()) {
        (true, true) => 0,
        (true, false) => -1,
        (false, true) => 1,
        (false, false) => libc::strncmp(str1, str2, maxlen),
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_strcasecmp(
    str1: *const libc::c_char,
    str2: *const libc::c_char,
) -> libc::c_int {
    match (str1.is_null(), str2.is_null()) {
        (true, true) => 0,
        (true, false) => -1,
        (false, true) => 1,
        (false, false) => libc::strcasecmp(str1, str2),
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_strncasecmp(
    str1: *const libc::c_char,
    str2: *const libc::c_char,
    maxlen: usize,
) -> libc::c_int {
    match (str1.is_null(), str2.is_null()) {
        (true, true) => 0,
        (true, false) => -1,
        (false, true) => 1,
        (false, false) => libc::strncasecmp(str1, str2, maxlen),
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_strchr(
    str_: *const libc::c_char,
    c: libc::c_int,
) -> *mut libc::c_char {
    if str_.is_null() {
        ptr::null_mut()
    } else {
        libc::strchr(str_, c)
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_strrchr(
    str_: *const libc::c_char,
    c: libc::c_int,
) -> *mut libc::c_char {
    if str_.is_null() {
        ptr::null_mut()
    } else {
        libc::strrchr(str_, c)
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_strstr(
    haystack: *const libc::c_char,
    needle: *const libc::c_char,
) -> *mut libc::c_char {
    if haystack.is_null() || needle.is_null() {
        return ptr::null_mut();
    }
    libc::strstr(haystack, needle)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_strcasestr(
    haystack: *const libc::c_char,
    needle: *const libc::c_char,
) -> *mut libc::c_char {
    if haystack.is_null() || needle.is_null() {
        return ptr::null_mut();
    }

    let length = SDL_strlen(needle);
    if length == 0 {
        return haystack.cast_mut();
    }

    let mut cursor = haystack;
    while *cursor != 0 {
        if SDL_strncasecmp(cursor, needle, length) == 0 {
            return cursor.cast_mut();
        }
        cursor = cursor.add(1);
    }
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_strrev(str_: *mut libc::c_char) -> *mut libc::c_char {
    if str_.is_null() {
        return ptr::null_mut();
    }
    let len = SDL_strlen(str_.cast());
    if len <= 1 {
        return str_;
    }
    let bytes = std::slice::from_raw_parts_mut(str_.cast::<u8>(), len);
    bytes.reverse();
    str_
}

#[no_mangle]
pub unsafe extern "C" fn SDL_strupr(str_: *mut libc::c_char) -> *mut libc::c_char {
    if str_.is_null() {
        return ptr::null_mut();
    }
    let mut cursor = str_;
    while *cursor != 0 {
        *cursor = ascii_toupper((*cursor as u8) as i32) as libc::c_char;
        cursor = cursor.add(1);
    }
    str_
}

#[no_mangle]
pub unsafe extern "C" fn SDL_strlwr(str_: *mut libc::c_char) -> *mut libc::c_char {
    if str_.is_null() {
        return ptr::null_mut();
    }
    let mut cursor = str_;
    while *cursor != 0 {
        *cursor = ascii_tolower((*cursor as u8) as i32) as libc::c_char;
        cursor = cursor.add(1);
    }
    str_
}

#[no_mangle]
pub unsafe extern "C" fn SDL_itoa(
    value: libc::c_int,
    string: *mut libc::c_char,
    radix: libc::c_int,
) -> *mut libc::c_char {
    SDL_ltoa(value as libc::c_long, string, radix)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_uitoa(
    value: libc::c_uint,
    string: *mut libc::c_char,
    radix: libc::c_int,
) -> *mut libc::c_char {
    SDL_ultoa(value as libc::c_ulong, string, radix)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ltoa(
    value: libc::c_long,
    string: *mut libc::c_char,
    radix: libc::c_int,
) -> *mut libc::c_char {
    write_signed_radix(value as i64, string, radix)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ultoa(
    value: libc::c_ulong,
    string: *mut libc::c_char,
    radix: libc::c_int,
) -> *mut libc::c_char {
    write_unsigned_radix(value as u64, string, radix)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_lltoa(
    value: i64,
    string: *mut libc::c_char,
    radix: libc::c_int,
) -> *mut libc::c_char {
    write_signed_radix(value, string, radix)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ulltoa(
    value: u64,
    string: *mut libc::c_char,
    radix: libc::c_int,
) -> *mut libc::c_char {
    write_unsigned_radix(value, string, radix)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_wcslen(wstr: *const libc::wchar_t) -> usize {
    if wstr.is_null() {
        return 0;
    }
    let mut len = 0usize;
    let mut cursor = wstr;
    while *cursor != 0 {
        len += 1;
        cursor = cursor.add(1);
    }
    len
}

#[no_mangle]
pub unsafe extern "C" fn SDL_wcslcpy(
    dst: *mut libc::wchar_t,
    src: *const libc::wchar_t,
    maxlen: usize,
) -> usize {
    let src_len = SDL_wcslen(src);
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
pub unsafe extern "C" fn SDL_wcslcat(
    dst: *mut libc::wchar_t,
    src: *const libc::wchar_t,
    maxlen: usize,
) -> usize {
    if dst.is_null() {
        return SDL_wcslen(src);
    }
    let mut dst_len = 0usize;
    while dst_len < maxlen && *dst.add(dst_len) != 0 {
        dst_len += 1;
    }
    let src_len = SDL_wcslen(src);
    if dst_len == maxlen {
        return maxlen + src_len;
    }
    SDL_wcslcpy(dst.add(dst_len), src, maxlen - dst_len);
    dst_len + src_len
}

#[no_mangle]
pub unsafe extern "C" fn SDL_wcsdup(wstr: *const libc::wchar_t) -> *mut libc::wchar_t {
    copy_wide_string(wstr)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_wcsstr(
    haystack: *const libc::wchar_t,
    needle: *const libc::wchar_t,
) -> *mut libc::wchar_t {
    if haystack.is_null() || needle.is_null() {
        return ptr::null_mut();
    }
    let length = SDL_wcslen(needle);
    if length == 0 {
        return haystack.cast_mut();
    }
    let mut cursor = haystack;
    while *cursor != 0 {
        if SDL_wcsncmp(cursor, needle, length) == 0 {
            return cursor.cast_mut();
        }
        cursor = cursor.add(1);
    }
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_wcscmp(
    str1: *const libc::wchar_t,
    str2: *const libc::wchar_t,
) -> libc::c_int {
    match (str1.is_null(), str2.is_null()) {
        (true, true) => 0,
        (true, false) => -1,
        (false, true) => 1,
        (false, false) => {
            let mut a = str1;
            let mut b = str2;
            while *a != 0 && *b != 0 && *a == *b {
                a = a.add(1);
                b = b.add(1);
            }
            (*a as i64 - *b as i64) as libc::c_int
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_wcsncmp(
    str1: *const libc::wchar_t,
    str2: *const libc::wchar_t,
    maxlen: usize,
) -> libc::c_int {
    match (str1.is_null(), str2.is_null()) {
        (true, true) => 0,
        (true, false) => -1,
        (false, true) => 1,
        (false, false) => {
            let mut a = str1;
            let mut b = str2;
            let mut remaining = maxlen;
            while remaining > 0 && *a != 0 && *b != 0 && *a == *b {
                a = a.add(1);
                b = b.add(1);
                remaining -= 1;
            }
            if remaining == 0 {
                0
            } else {
                (*a as i64 - *b as i64) as libc::c_int
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_wcscasecmp(
    str1: *const libc::wchar_t,
    str2: *const libc::wchar_t,
) -> libc::c_int {
    SDL_wcsncasecmp(str1, str2, usize::MAX)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_wcsncasecmp(
    str1: *const libc::wchar_t,
    str2: *const libc::wchar_t,
    maxlen: usize,
) -> libc::c_int {
    match (str1.is_null(), str2.is_null()) {
        (true, true) => 0,
        (true, false) => -1,
        (false, true) => 1,
        (false, false) => {
            let mut a = str1;
            let mut b = str2;
            let mut remaining = maxlen;
            while remaining > 0 {
                let left = *a as u32;
                let right = *b as u32;
                let folded_left = if left < 0x80 {
                    ascii_toupper(left as i32) as u32
                } else {
                    left
                };
                let folded_right = if right < 0x80 {
                    ascii_toupper(right as i32) as u32
                } else {
                    right
                };
                if folded_left != folded_right || left == 0 {
                    return (folded_left as i64 - folded_right as i64) as libc::c_int;
                }
                a = a.add(1);
                b = b.add(1);
                remaining -= 1;
            }
            0
        }
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
pub unsafe extern "C" fn SDL_atof(str_: *const libc::c_char) -> libc::c_double {
    if str_.is_null() {
        0.0
    } else {
        libc::atof(str_)
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
pub unsafe extern "C" fn SDL_strtod(
    str_: *const libc::c_char,
    endp: *mut *mut libc::c_char,
) -> libc::c_double {
    if str_.is_null() {
        if !endp.is_null() {
            *endp = ptr::null_mut();
        }
        return 0.0;
    }
    libc::strtod(str_, endp)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_strtol(
    str_: *const libc::c_char,
    endp: *mut *mut libc::c_char,
    base: libc::c_int,
) -> libc::c_long {
    if str_.is_null() {
        if !endp.is_null() {
            *endp = ptr::null_mut();
        }
        return 0;
    }
    libc::strtol(str_, endp, base)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_strtoll(
    str_: *const libc::c_char,
    endp: *mut *mut libc::c_char,
    base: libc::c_int,
) -> libc::c_longlong {
    if str_.is_null() {
        if !endp.is_null() {
            *endp = ptr::null_mut();
        }
        return 0;
    }
    libc::strtoll(str_, endp, base)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_strtoull(
    str_: *const libc::c_char,
    endp: *mut *mut libc::c_char,
    base: libc::c_int,
) -> libc::c_ulonglong {
    if str_.is_null() {
        if !endp.is_null() {
            *endp = ptr::null_mut();
        }
        return 0;
    }
    libc::strtoull(str_, endp, base)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_isalpha(x: libc::c_int) -> libc::c_int {
    ascii_isalpha(x) as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn SDL_isalnum(x: libc::c_int) -> libc::c_int {
    (ascii_isalpha(x) || ascii_isdigit(x)) as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn SDL_isdigit(x: libc::c_int) -> libc::c_int {
    ascii_isdigit(x) as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn SDL_isxdigit(x: libc::c_int) -> libc::c_int {
    ascii_isxdigit(x) as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ispunct(x: libc::c_int) -> libc::c_int {
    (SDL_isgraph(x) != 0 && SDL_isalnum(x) == 0) as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn SDL_isspace(x: libc::c_int) -> libc::c_int {
    ascii_isspace(x) as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn SDL_isupper(x: libc::c_int) -> libc::c_int {
    ascii_isupper(x) as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn SDL_islower(x: libc::c_int) -> libc::c_int {
    ascii_islower(x) as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn SDL_isprint(x: libc::c_int) -> libc::c_int {
    ascii_isprint(x) as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn SDL_isgraph(x: libc::c_int) -> libc::c_int {
    (ascii_isprint(x) && x != b' ' as i32) as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn SDL_iscntrl(x: libc::c_int) -> libc::c_int {
    ((0..=0x1f).contains(&x) || x == 0x7f) as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn SDL_toupper(x: libc::c_int) -> libc::c_int {
    ascii_toupper(x)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_tolower(x: libc::c_int) -> libc::c_int {
    ascii_tolower(x)
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
