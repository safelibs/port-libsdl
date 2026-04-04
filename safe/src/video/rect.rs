use crate::abi::generated_types::{SDL_FPoint, SDL_FRect, SDL_Point, SDL_Rect, SDL_bool};
use crate::core::error::invalid_param_error;
use crate::video::surface::real_sdl;

#[inline]
fn invalid_rect_param(param: &str) -> SDL_bool {
    let _ = invalid_param_error(param);
    0
}

#[inline]
fn report_invalid_rect_param(param: &str) {
    let _ = invalid_param_error(param);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HasIntersection(A: *const SDL_Rect, B: *const SDL_Rect) -> SDL_bool {
    if A.is_null() {
        return invalid_rect_param("A");
    }
    if B.is_null() {
        return invalid_rect_param("B");
    }
    (real_sdl().has_intersection)(A, B)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_IntersectRect(
    A: *const SDL_Rect,
    B: *const SDL_Rect,
    result: *mut SDL_Rect,
) -> SDL_bool {
    if A.is_null() {
        return invalid_rect_param("A");
    }
    if B.is_null() {
        return invalid_rect_param("B");
    }
    if result.is_null() {
        return invalid_rect_param("result");
    }
    (real_sdl().intersect_rect)(A, B, result)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_UnionRect(
    A: *const SDL_Rect,
    B: *const SDL_Rect,
    result: *mut SDL_Rect,
) {
    if A.is_null() {
        report_invalid_rect_param("A");
        return;
    }
    if B.is_null() {
        report_invalid_rect_param("B");
        return;
    }
    if result.is_null() {
        report_invalid_rect_param("result");
        return;
    }
    (real_sdl().union_rect)(A, B, result);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_EnclosePoints(
    points: *const SDL_Point,
    count: libc::c_int,
    clip: *const SDL_Rect,
    result: *mut SDL_Rect,
) -> SDL_bool {
    if points.is_null() {
        return invalid_rect_param("points");
    }
    if count < 1 {
        return invalid_rect_param("count");
    }
    (real_sdl().enclose_points)(points, count, clip, result)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_IntersectRectAndLine(
    rect: *const SDL_Rect,
    X1: *mut libc::c_int,
    Y1: *mut libc::c_int,
    X2: *mut libc::c_int,
    Y2: *mut libc::c_int,
) -> SDL_bool {
    if rect.is_null() {
        return invalid_rect_param("rect");
    }
    if X1.is_null() {
        return invalid_rect_param("X1");
    }
    if Y1.is_null() {
        return invalid_rect_param("Y1");
    }
    if X2.is_null() {
        return invalid_rect_param("X2");
    }
    if Y2.is_null() {
        return invalid_rect_param("Y2");
    }
    (real_sdl().intersect_rect_and_line)(rect, X1, Y1, X2, Y2)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HasIntersectionF(
    A: *const SDL_FRect,
    B: *const SDL_FRect,
) -> SDL_bool {
    if A.is_null() {
        return invalid_rect_param("A");
    }
    if B.is_null() {
        return invalid_rect_param("B");
    }
    (real_sdl().has_intersection_f)(A, B)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_IntersectFRect(
    A: *const SDL_FRect,
    B: *const SDL_FRect,
    result: *mut SDL_FRect,
) -> SDL_bool {
    if A.is_null() {
        return invalid_rect_param("A");
    }
    if B.is_null() {
        return invalid_rect_param("B");
    }
    if result.is_null() {
        return invalid_rect_param("result");
    }
    (real_sdl().intersect_f_rect)(A, B, result)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_UnionFRect(
    A: *const SDL_FRect,
    B: *const SDL_FRect,
    result: *mut SDL_FRect,
) {
    if A.is_null() {
        report_invalid_rect_param("A");
        return;
    }
    if B.is_null() {
        report_invalid_rect_param("B");
        return;
    }
    if result.is_null() {
        report_invalid_rect_param("result");
        return;
    }
    (real_sdl().union_f_rect)(A, B, result);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_EncloseFPoints(
    points: *const SDL_FPoint,
    count: libc::c_int,
    clip: *const SDL_FRect,
    result: *mut SDL_FRect,
) -> SDL_bool {
    if points.is_null() {
        return invalid_rect_param("points");
    }
    if count < 1 {
        return invalid_rect_param("count");
    }
    (real_sdl().enclose_f_points)(points, count, clip, result)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_IntersectFRectAndLine(
    rect: *const SDL_FRect,
    X1: *mut f32,
    Y1: *mut f32,
    X2: *mut f32,
    Y2: *mut f32,
) -> SDL_bool {
    if rect.is_null() {
        return invalid_rect_param("rect");
    }
    if X1.is_null() {
        return invalid_rect_param("X1");
    }
    if Y1.is_null() {
        return invalid_rect_param("Y1");
    }
    if X2.is_null() {
        return invalid_rect_param("X2");
    }
    if Y2.is_null() {
        return invalid_rect_param("Y2");
    }
    (real_sdl().intersect_f_rect_and_line)(rect, X1, Y1, X2, Y2)
}
