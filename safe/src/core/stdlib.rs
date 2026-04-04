use crate::abi::generated_types::SDL_iconv_t;

type CompareCallback = Option<
    unsafe extern "C" fn(arg1: *const libc::c_void, arg2: *const libc::c_void) -> libc::c_int,
>;

crate::forward_sdl! {
    fn SDL_getenv(name: *const libc::c_char) -> *mut libc::c_char;
    fn SDL_setenv(
        name: *const libc::c_char,
        value: *const libc::c_char,
        overwrite: libc::c_int
    ) -> libc::c_int;
    fn SDL_qsort(base: *mut libc::c_void, nmemb: usize, size: usize, compare: CompareCallback);
    fn SDL_bsearch(
        key: *const libc::c_void,
        base: *const libc::c_void,
        nmemb: usize,
        size: usize,
        compare: CompareCallback
    ) -> *mut libc::c_void;
    fn SDL_abs(x: libc::c_int) -> libc::c_int;
    fn SDL_memset(dst: *mut libc::c_void, c: libc::c_int, len: usize) -> *mut libc::c_void;
    fn SDL_memcmp(s1: *const libc::c_void, s2: *const libc::c_void, len: usize) -> libc::c_int;
    fn SDL_strlen(str_: *const libc::c_char) -> usize;
    fn SDL_strlcpy(dst: *mut libc::c_char, src: *const libc::c_char, maxlen: usize) -> usize;
    fn SDL_strlcat(dst: *mut libc::c_char, src: *const libc::c_char, maxlen: usize) -> usize;
    fn SDL_strdup(str_: *const libc::c_char) -> *mut libc::c_char;
    fn SDL_strcmp(str1: *const libc::c_char, str2: *const libc::c_char) -> libc::c_int;
    fn SDL_atoi(str_: *const libc::c_char) -> libc::c_int;
    fn SDL_strtoul(
        str_: *const libc::c_char,
        endp: *mut *mut libc::c_char,
        base: libc::c_int
    ) -> libc::c_ulong;
    fn SDL_iconv_open(
        tocode: *const libc::c_char,
        fromcode: *const libc::c_char
    ) -> SDL_iconv_t;
    fn SDL_iconv_close(cd: SDL_iconv_t) -> libc::c_int;
    fn SDL_iconv(
        cd: SDL_iconv_t,
        inbuf: *mut *const libc::c_char,
        inbytesleft: *mut usize,
        outbuf: *mut *mut libc::c_char,
        outbytesleft: *mut usize
    ) -> usize;
    fn SDL_iconv_string(
        tocode: *const libc::c_char,
        fromcode: *const libc::c_char,
        inbuf: *const libc::c_char,
        inbytesleft: usize
    ) -> *mut libc::c_char;
}
