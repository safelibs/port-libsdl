use std::mem::{align_of, size_of};
use std::ptr;
use std::sync::OnceLock;

use crate::abi::generated_types::SDL_bool;
use crate::core::system::bool_to_sdl;

#[repr(C)]
struct SimdHeader {
    base: *mut libc::c_void,
    size: usize,
}

fn cache_line_size() -> libc::c_int {
    #[allow(clippy::useless_conversion)]
    let probed = unsafe { libc::sysconf(libc::_SC_LEVEL1_DCACHE_LINESIZE) };
    if probed > 0 {
        probed as libc::c_int
    } else {
        64
    }
}

fn simd_alignment() -> usize {
    static ALIGNMENT: OnceLock<usize> = OnceLock::new();
    *ALIGNMENT.get_or_init(|| {
        let mut alignment = size_of::<*const libc::c_void>().max(align_of::<SimdHeader>());
        if unsafe { SDL_HasSSE() } != 0 || unsafe { SDL_HasNEON() } != 0 {
            alignment = alignment.max(16);
        }
        if unsafe { SDL_HasAVX() } != 0 {
            alignment = alignment.max(32);
        }
        if unsafe { SDL_HasAVX512F() } != 0 {
            alignment = alignment.max(64);
        }
        alignment.next_power_of_two()
    })
}

unsafe fn simd_header(ptr: *mut libc::c_void) -> *mut SimdHeader {
    (ptr as *mut u8).sub(size_of::<SimdHeader>()) as *mut SimdHeader
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetCPUCount() -> libc::c_int {
    std::thread::available_parallelism()
        .map(|value| value.get() as libc::c_int)
        .unwrap_or(1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetCPUCacheLineSize() -> libc::c_int {
    cache_line_size()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HasRDTSC() -> SDL_bool {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        return bool_to_sdl(true);
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        return bool_to_sdl(false);
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HasAltiVec() -> SDL_bool {
    bool_to_sdl(false)
}
#[no_mangle]
pub unsafe extern "C" fn SDL_HasMMX() -> SDL_bool {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        return bool_to_sdl(std::arch::is_x86_feature_detected!("mmx"));
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        return bool_to_sdl(false);
    }
}
#[no_mangle]
pub unsafe extern "C" fn SDL_Has3DNow() -> SDL_bool {
    bool_to_sdl(false)
}
#[no_mangle]
pub unsafe extern "C" fn SDL_HasSSE() -> SDL_bool {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        return bool_to_sdl(std::arch::is_x86_feature_detected!("sse"));
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        return bool_to_sdl(false);
    }
}
#[no_mangle]
pub unsafe extern "C" fn SDL_HasSSE2() -> SDL_bool {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        return bool_to_sdl(std::arch::is_x86_feature_detected!("sse2"));
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        return bool_to_sdl(false);
    }
}
#[no_mangle]
pub unsafe extern "C" fn SDL_HasSSE3() -> SDL_bool {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        return bool_to_sdl(std::arch::is_x86_feature_detected!("sse3"));
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        return bool_to_sdl(false);
    }
}
#[no_mangle]
pub unsafe extern "C" fn SDL_HasSSE41() -> SDL_bool {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        return bool_to_sdl(std::arch::is_x86_feature_detected!("sse4.1"));
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        return bool_to_sdl(false);
    }
}
#[no_mangle]
pub unsafe extern "C" fn SDL_HasSSE42() -> SDL_bool {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        return bool_to_sdl(std::arch::is_x86_feature_detected!("sse4.2"));
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        return bool_to_sdl(false);
    }
}
#[no_mangle]
pub unsafe extern "C" fn SDL_HasAVX() -> SDL_bool {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        return bool_to_sdl(std::arch::is_x86_feature_detected!("avx"));
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        return bool_to_sdl(false);
    }
}
#[no_mangle]
pub unsafe extern "C" fn SDL_HasAVX2() -> SDL_bool {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        return bool_to_sdl(std::arch::is_x86_feature_detected!("avx2"));
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        return bool_to_sdl(false);
    }
}
#[no_mangle]
pub unsafe extern "C" fn SDL_HasAVX512F() -> SDL_bool {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        return bool_to_sdl(std::arch::is_x86_feature_detected!("avx512f"));
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        return bool_to_sdl(false);
    }
}
#[no_mangle]
pub unsafe extern "C" fn SDL_HasARMSIMD() -> SDL_bool {
    bool_to_sdl(false)
}
#[no_mangle]
pub unsafe extern "C" fn SDL_HasNEON() -> SDL_bool {
    #[cfg(target_arch = "aarch64")]
    {
        return bool_to_sdl(std::arch::is_aarch64_feature_detected!("neon"));
    }
    #[cfg(target_arch = "arm")]
    {
        return bool_to_sdl(std::arch::is_arm_feature_detected!("neon"));
    }
    #[cfg(not(any(target_arch = "aarch64", target_arch = "arm")))]
    {
        return bool_to_sdl(false);
    }
}
#[no_mangle]
pub unsafe extern "C" fn SDL_HasLSX() -> SDL_bool {
    bool_to_sdl(false)
}
#[no_mangle]
pub unsafe extern "C" fn SDL_HasLASX() -> SDL_bool {
    bool_to_sdl(false)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetSystemRAM() -> libc::c_int {
    let pages = libc::sysconf(libc::_SC_PHYS_PAGES);
    let page_size = libc::sysconf(libc::_SC_PAGESIZE);
    if pages <= 0 || page_size <= 0 {
        0
    } else {
        (((pages as i128) * (page_size as i128)) / (1024 * 1024)) as libc::c_int
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SIMDAlloc(len: usize) -> *mut libc::c_void {
    let alignment = simd_alignment();
    let total = len
        .saturating_add(alignment)
        .saturating_add(size_of::<SimdHeader>());
    let base = libc::malloc(total);
    if base.is_null() {
        let _ = crate::core::error::out_of_memory_error();
        return std::ptr::null_mut();
    }

    let start = (base as usize) + size_of::<SimdHeader>();
    let aligned = (start + alignment - 1) & !(alignment - 1);
    let header = (aligned - size_of::<SimdHeader>()) as *mut SimdHeader;
    (*header).base = base;
    (*header).size = len;
    aligned as *mut libc::c_void
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SIMDRealloc(mem: *mut libc::c_void, len: usize) -> *mut libc::c_void {
    if mem.is_null() {
        return SDL_SIMDAlloc(len);
    }
    let header = simd_header(mem);
    let old_size = (*header).size;
    let replacement = SDL_SIMDAlloc(len);
    if replacement.is_null() {
        return std::ptr::null_mut();
    }
    ptr::copy_nonoverlapping(
        mem.cast::<u8>(),
        replacement.cast::<u8>(),
        old_size.min(len),
    );
    SDL_SIMDFree(mem);
    replacement
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SIMDFree(ptr: *mut libc::c_void) {
    if ptr.is_null() {
        return;
    }
    let header = simd_header(ptr);
    libc::free((*header).base);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SIMDGetAlignment() -> usize {
    simd_alignment()
}
