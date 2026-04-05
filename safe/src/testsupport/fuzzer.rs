use std::mem;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::abi::generated_types::{
    SDL_bool, SDL_bool_SDL_TRUE, Sint16, Sint32, Sint64, Sint8, Uint16, Uint32, Uint64, Uint8,
};
use crate::testsupport::{alloc_c_string, SDLTest_RandomContext};

fn rnd_context() -> &'static Mutex<SDLTest_RandomContext> {
    static CONTEXT: OnceLock<Mutex<SDLTest_RandomContext>> = OnceLock::new();
    CONTEXT.get_or_init(|| Mutex::new(unsafe { mem::zeroed() }))
}

fn lock_context() -> std::sync::MutexGuard<'static, SDLTest_RandomContext> {
    match rnd_context().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

static FUZZER_INVOCATIONS: AtomicI32 = AtomicI32::new(0);

#[no_mangle]
pub unsafe extern "C" fn SDLTest_FuzzerInit(execKey: Uint64) {
    let a = ((execKey >> 32) & 0xffff_ffff) as Uint32;
    let b = (execKey & 0xffff_ffff) as Uint32;
    let mut ctx = lock_context();
    *ctx = mem::zeroed();
    crate::testsupport::random::SDLTest_RandomInit(&mut *ctx, a, b);
    FUZZER_INVOCATIONS.store(0, Ordering::Relaxed);
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_GetFuzzerInvocationCount() -> libc::c_int {
    FUZZER_INVOCATIONS.load(Ordering::Relaxed)
}

fn bump() {
    FUZZER_INVOCATIONS.fetch_add(1, Ordering::Relaxed);
}

unsafe fn random_raw() -> Uint32 {
    let mut ctx = lock_context();
    crate::testsupport::random::SDLTest_Random(&mut *ctx)
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_RandomUint8() -> Uint8 {
    bump();
    (random_raw() & 0xff) as Uint8
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_RandomSint8() -> Sint8 {
    bump();
    (random_raw() & 0xff) as Sint8
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_RandomUint16() -> Uint16 {
    bump();
    (random_raw() & 0xffff) as Uint16
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_RandomSint16() -> Sint16 {
    bump();
    (random_raw() & 0xffff) as Sint16
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_RandomSint32() -> Sint32 {
    bump();
    random_raw() as Sint32
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_RandomUint32() -> Uint32 {
    bump();
    random_raw()
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_RandomUint64() -> Uint64 {
    bump();
    let low = random_raw() as Uint64;
    let high = random_raw() as Uint64;
    (high << 32) | low
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_RandomSint64() -> Sint64 {
    bump();
    SDLTest_RandomUint64() as Sint64
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_RandomIntegerInRange(min: Sint32, max: Sint32) -> Sint32 {
    let (min64, max64) = if min > max {
        (max as i64, min as i64)
    } else {
        (min as i64, max as i64)
    };
    if min64 == max64 {
        return min64 as Sint32;
    }
    let number = SDLTest_RandomUint32() as i64;
    ((number % ((max64 + 1) - min64)) + min64) as Sint32
}

unsafe fn unsigned_boundary(
    max_value: Uint64,
    boundary1: Uint64,
    boundary2: Uint64,
    valid: SDL_bool,
) -> Uint64 {
    let (b1, b2) = if boundary1 > boundary2 {
        (boundary2, boundary1)
    } else {
        (boundary1, boundary2)
    };
    let mut choices = [0u64; 4];
    let mut count = 0usize;
    if valid == SDL_bool_SDL_TRUE {
        if b1 == b2 {
            return b1;
        }
        let delta = b2 - b1;
        if delta < 4 {
            for offset in 0..=delta {
                choices[count] = b1 + offset;
                count += 1;
            }
        } else {
            choices[0] = b1;
            choices[1] = b1 + 1;
            choices[2] = b2 - 1;
            choices[3] = b2;
            count = 4;
        }
    } else {
        if b1 > 0 {
            choices[count] = b1 - 1;
            count += 1;
        }
        if b2 < max_value {
            choices[count] = b2 + 1;
            count += 1;
        }
    }
    if count == 0 {
        crate::testsupport::unsupported_error();
        return 0;
    }
    choices[(SDLTest_RandomUint8() as usize) % count]
}

unsafe fn signed_boundary(
    min_value: Sint64,
    max_value: Sint64,
    boundary1: Sint64,
    boundary2: Sint64,
    valid: SDL_bool,
) -> Sint64 {
    let (b1, b2) = if boundary1 > boundary2 {
        (boundary2, boundary1)
    } else {
        (boundary1, boundary2)
    };
    let mut choices = [0i64; 4];
    let mut count = 0usize;
    if valid == SDL_bool_SDL_TRUE {
        if b1 == b2 {
            return b1;
        }
        let delta = b2 - b1;
        if delta < 4 {
            for offset in 0..=delta {
                choices[count] = b1 + offset;
                count += 1;
            }
        } else {
            choices[0] = b1;
            choices[1] = b1 + 1;
            choices[2] = b2 - 1;
            choices[3] = b2;
            count = 4;
        }
    } else {
        if b1 > min_value {
            choices[count] = b1 - 1;
            count += 1;
        }
        if b2 < max_value {
            choices[count] = b2 + 1;
            count += 1;
        }
    }
    if count == 0 {
        crate::testsupport::unsupported_error();
        return min_value;
    }
    choices[(SDLTest_RandomUint8() as usize) % count]
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_RandomUint8BoundaryValue(
    boundary1: Uint8,
    boundary2: Uint8,
    validDomain: SDL_bool,
) -> Uint8 {
    unsigned_boundary(
        u8::MAX as Uint64,
        boundary1 as Uint64,
        boundary2 as Uint64,
        validDomain,
    ) as Uint8
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_RandomUint16BoundaryValue(
    boundary1: Uint16,
    boundary2: Uint16,
    validDomain: SDL_bool,
) -> Uint16 {
    unsigned_boundary(
        u16::MAX as Uint64,
        boundary1 as Uint64,
        boundary2 as Uint64,
        validDomain,
    ) as Uint16
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_RandomUint32BoundaryValue(
    boundary1: Uint32,
    boundary2: Uint32,
    validDomain: SDL_bool,
) -> Uint32 {
    unsigned_boundary(
        u32::MAX as Uint64,
        boundary1 as Uint64,
        boundary2 as Uint64,
        validDomain,
    ) as Uint32
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_RandomUint64BoundaryValue(
    boundary1: Uint64,
    boundary2: Uint64,
    validDomain: SDL_bool,
) -> Uint64 {
    unsigned_boundary(u64::MAX, boundary1, boundary2, validDomain)
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_RandomSint8BoundaryValue(
    boundary1: Sint8,
    boundary2: Sint8,
    validDomain: SDL_bool,
) -> Sint8 {
    signed_boundary(
        i8::MIN as Sint64,
        i8::MAX as Sint64,
        boundary1 as Sint64,
        boundary2 as Sint64,
        validDomain,
    ) as Sint8
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_RandomSint16BoundaryValue(
    boundary1: Sint16,
    boundary2: Sint16,
    validDomain: SDL_bool,
) -> Sint16 {
    signed_boundary(
        i16::MIN as Sint64,
        i16::MAX as Sint64,
        boundary1 as Sint64,
        boundary2 as Sint64,
        validDomain,
    ) as Sint16
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_RandomSint32BoundaryValue(
    boundary1: Sint32,
    boundary2: Sint32,
    validDomain: SDL_bool,
) -> Sint32 {
    signed_boundary(
        i32::MIN as Sint64,
        i32::MAX as Sint64,
        boundary1 as Sint64,
        boundary2 as Sint64,
        validDomain,
    ) as Sint32
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_RandomSint64BoundaryValue(
    boundary1: Sint64,
    boundary2: Sint64,
    validDomain: SDL_bool,
) -> Sint64 {
    signed_boundary(i64::MIN, i64::MAX, boundary1, boundary2, validDomain)
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_RandomUnitFloat() -> f32 {
    SDLTest_RandomUint32() as f32 / u32::MAX as f32
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_RandomFloat() -> f32 {
    (SDLTest_RandomUnitDouble() * 2.0 * f32::MAX as f64 - f32::MAX as f64) as f32
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_RandomUnitDouble() -> f64 {
    ((SDLTest_RandomUint64() >> 11) as f64) * (1.0 / 9_007_199_254_740_992.0)
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_RandomDouble() -> f64 {
    let mut r = 0.0f64;
    let mut s = 1.0f64;
    {
        let mut ctx = lock_context();
        loop {
            s /= u32::MAX as f64 + 1.0;
            r += crate::testsupport::random::SDLTest_Random(&mut *ctx) as f64 * s;
            if s <= f64::EPSILON {
                break;
            }
        }
    }
    bump();
    r
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_RandomAsciiString() -> *mut libc::c_char {
    SDLTest_RandomAsciiStringWithMaximumLength(255)
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_RandomAsciiStringWithMaximumLength(
    maxLength: libc::c_int,
) -> *mut libc::c_char {
    if maxLength < 1 {
        crate::testsupport::invalid_param_error("maxLength");
        return std::ptr::null_mut();
    }
    let mut size = (SDLTest_RandomUint32() % ((maxLength + 1) as Uint32)) as libc::c_int;
    if size == 0 {
        size = 1;
    }
    SDLTest_RandomAsciiStringOfSize(size)
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_RandomAsciiStringOfSize(size: libc::c_int) -> *mut libc::c_char {
    if size < 1 {
        crate::testsupport::invalid_param_error("size");
        return std::ptr::null_mut();
    }
    let mut bytes = Vec::with_capacity(size as usize);
    for _ in 0..size {
        bytes.push(SDLTest_RandomIntegerInRange(32, 126) as u8);
    }
    bump();
    alloc_c_string(std::str::from_utf8(&bytes).unwrap_or(""))
}
