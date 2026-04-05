use std::os::raw::c_uint;
use std::ptr;

use crate::testsupport::SDLTest_RandomContext;

#[no_mangle]
pub unsafe extern "C" fn SDLTest_RandomInit(
    rndContext: *mut SDLTest_RandomContext,
    xi: c_uint,
    ci: c_uint,
) {
    if rndContext.is_null() {
        return;
    }
    (*rndContext).a = 1_655_692_410;
    (*rndContext).x = if xi == 0 { 30_903 } else { xi };
    (*rndContext).c = ci;
    (*rndContext).ah = (*rndContext).a >> 16;
    (*rndContext).al = (*rndContext).a & 0xffff;
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_RandomInitTime(rndContext: *mut SDLTest_RandomContext) {
    if rndContext.is_null() {
        return;
    }
    libc::srand(libc::time(ptr::null_mut()) as c_uint);
    let a = libc::rand() as c_uint;
    libc::srand(crate::abi::generated_types::SDL_GetPerformanceCounter() as c_uint);
    let b = libc::rand() as c_uint;
    SDLTest_RandomInit(rndContext, a, b);
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_Random(rndContext: *mut SDLTest_RandomContext) -> c_uint {
    if rndContext.is_null() {
        return c_uint::MAX;
    }
    let xh = (*rndContext).x >> 16;
    let xl = (*rndContext).x & 0xffff;
    (*rndContext).x = (*rndContext)
        .x
        .wrapping_mul((*rndContext).a)
        .wrapping_add((*rndContext).c);
    (*rndContext).c = xh
        .wrapping_mul((*rndContext).ah)
        .wrapping_add((xh.wrapping_mul((*rndContext).al)) >> 16)
        .wrapping_add((xl.wrapping_mul((*rndContext).ah)) >> 16);
    if xl.wrapping_mul((*rndContext).al) >= (!(*rndContext).c).wrapping_add(1) {
        (*rndContext).c = (*rndContext).c.wrapping_add(1);
    }
    (*rndContext).x
}
