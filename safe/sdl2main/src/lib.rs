/// Exported placeholder used to keep the `sdl2main` archive non-empty and linkable.
///
/// # Safety
/// This symbol uses the C ABI and may be invoked by foreign code. Callers must
/// uphold the ABI contract for `extern "C"` functions.
#[no_mangle]
pub unsafe extern "C" fn SDL_main_stub_symbol() -> libc::c_int {
    0
}
