macro_rules! export_unary_f64 {
    ($name:ident, $method:ident) => {
        #[no_mangle]
        pub unsafe extern "C" fn $name(x: f64) -> f64 {
            x.$method()
        }
    };
}

macro_rules! export_unary_f32 {
    ($name:ident, $method:ident) => {
        #[no_mangle]
        pub unsafe extern "C" fn $name(x: f32) -> f32 {
            x.$method()
        }
    };
}

#[no_mangle]
pub unsafe extern "C" fn SDL_acos(x: f64) -> f64 {
    x.acos()
}
#[no_mangle]
pub unsafe extern "C" fn SDL_acosf(x: f32) -> f32 {
    x.acos()
}
#[no_mangle]
pub unsafe extern "C" fn SDL_asin(x: f64) -> f64 {
    x.asin()
}
#[no_mangle]
pub unsafe extern "C" fn SDL_asinf(x: f32) -> f32 {
    x.asin()
}
#[no_mangle]
pub unsafe extern "C" fn SDL_atan(x: f64) -> f64 {
    x.atan()
}
#[no_mangle]
pub unsafe extern "C" fn SDL_atanf(x: f32) -> f32 {
    x.atan()
}
#[no_mangle]
pub unsafe extern "C" fn SDL_atan2(x: f64, y: f64) -> f64 {
    x.atan2(y)
}
#[no_mangle]
pub unsafe extern "C" fn SDL_atan2f(x: f32, y: f32) -> f32 {
    x.atan2(y)
}
export_unary_f64!(SDL_ceil, ceil);
export_unary_f32!(SDL_ceilf, ceil);
#[no_mangle]
pub unsafe extern "C" fn SDL_copysign(x: f64, y: f64) -> f64 {
    x.copysign(y)
}
#[no_mangle]
pub unsafe extern "C" fn SDL_copysignf(x: f32, y: f32) -> f32 {
    x.copysign(y)
}
#[no_mangle]
pub unsafe extern "C" fn SDL_cos(x: f64) -> f64 {
    x.cos()
}
#[no_mangle]
pub unsafe extern "C" fn SDL_cosf(x: f32) -> f32 {
    x.cos()
}
#[no_mangle]
pub unsafe extern "C" fn SDL_exp(x: f64) -> f64 {
    x.exp()
}
#[no_mangle]
pub unsafe extern "C" fn SDL_expf(x: f32) -> f32 {
    x.exp()
}
#[no_mangle]
pub unsafe extern "C" fn SDL_fabs(x: f64) -> f64 {
    x.abs()
}
#[no_mangle]
pub unsafe extern "C" fn SDL_fabsf(x: f32) -> f32 {
    x.abs()
}
export_unary_f64!(SDL_floor, floor);
export_unary_f32!(SDL_floorf, floor);
export_unary_f64!(SDL_trunc, trunc);
export_unary_f32!(SDL_truncf, trunc);
#[no_mangle]
pub unsafe extern "C" fn SDL_fmod(x: f64, y: f64) -> f64 {
    x % y
}
#[no_mangle]
pub unsafe extern "C" fn SDL_fmodf(x: f32, y: f32) -> f32 {
    x % y
}
#[no_mangle]
pub unsafe extern "C" fn SDL_log(x: f64) -> f64 {
    x.ln()
}
#[no_mangle]
pub unsafe extern "C" fn SDL_logf(x: f32) -> f32 {
    x.ln()
}
#[no_mangle]
pub unsafe extern "C" fn SDL_log10(x: f64) -> f64 {
    x.log10()
}
#[no_mangle]
pub unsafe extern "C" fn SDL_log10f(x: f32) -> f32 {
    x.log10()
}
#[no_mangle]
pub unsafe extern "C" fn SDL_lround(x: f64) -> libc::c_long {
    x.round() as libc::c_long
}
#[no_mangle]
pub unsafe extern "C" fn SDL_lroundf(x: f32) -> libc::c_long {
    x.round() as libc::c_long
}
#[no_mangle]
pub unsafe extern "C" fn SDL_pow(x: f64, y: f64) -> f64 {
    x.powf(y)
}
#[no_mangle]
pub unsafe extern "C" fn SDL_powf(x: f32, y: f32) -> f32 {
    x.powf(y)
}
#[no_mangle]
pub unsafe extern "C" fn SDL_round(x: f64) -> f64 {
    x.round()
}
#[no_mangle]
pub unsafe extern "C" fn SDL_roundf(x: f32) -> f32 {
    x.round()
}
#[no_mangle]
pub unsafe extern "C" fn SDL_scalbn(x: f64, n: libc::c_int) -> f64 {
    x * 2.0f64.powi(n)
}
#[no_mangle]
pub unsafe extern "C" fn SDL_scalbnf(x: f32, n: libc::c_int) -> f32 {
    x * 2.0f32.powi(n)
}
#[no_mangle]
pub unsafe extern "C" fn SDL_sin(x: f64) -> f64 {
    x.sin()
}
#[no_mangle]
pub unsafe extern "C" fn SDL_sinf(x: f32) -> f32 {
    x.sin()
}
#[no_mangle]
pub unsafe extern "C" fn SDL_sqrt(x: f64) -> f64 {
    x.sqrt()
}
#[no_mangle]
pub unsafe extern "C" fn SDL_sqrtf(x: f32) -> f32 {
    x.sqrt()
}
#[no_mangle]
pub unsafe extern "C" fn SDL_tan(x: f64) -> f64 {
    x.tan()
}
#[no_mangle]
pub unsafe extern "C" fn SDL_tanf(x: f32) -> f32 {
    x.tan()
}
