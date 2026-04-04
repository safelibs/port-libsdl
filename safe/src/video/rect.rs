use crate::abi::generated_types::{SDL_FPoint, SDL_FRect, SDL_Point, SDL_Rect, SDL_bool};
use crate::video::surface::real_sdl;

#[no_mangle]
pub unsafe extern "C" fn SDL_HasIntersection(A: *const SDL_Rect, B: *const SDL_Rect) -> SDL_bool {
    (real_sdl().has_intersection)(A, B)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_IntersectRect(
    A: *const SDL_Rect,
    B: *const SDL_Rect,
    result: *mut SDL_Rect,
) -> SDL_bool {
    (real_sdl().intersect_rect)(A, B, result)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_UnionRect(
    A: *const SDL_Rect,
    B: *const SDL_Rect,
    result: *mut SDL_Rect,
) {
    (real_sdl().union_rect)(A, B, result);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_EnclosePoints(
    points: *const SDL_Point,
    count: libc::c_int,
    clip: *const SDL_Rect,
    result: *mut SDL_Rect,
) -> SDL_bool {
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
    (real_sdl().intersect_rect_and_line)(rect, X1, Y1, X2, Y2)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HasIntersectionF(
    A: *const SDL_FRect,
    B: *const SDL_FRect,
) -> SDL_bool {
    (real_sdl().has_intersection_f)(A, B)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_IntersectFRect(
    A: *const SDL_FRect,
    B: *const SDL_FRect,
    result: *mut SDL_FRect,
) -> SDL_bool {
    (real_sdl().intersect_f_rect)(A, B, result)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_UnionFRect(
    A: *const SDL_FRect,
    B: *const SDL_FRect,
    result: *mut SDL_FRect,
) {
    (real_sdl().union_f_rect)(A, B, result);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_EncloseFPoints(
    points: *const SDL_FPoint,
    count: libc::c_int,
    clip: *const SDL_FRect,
    result: *mut SDL_FRect,
) -> SDL_bool {
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
    (real_sdl().intersect_f_rect_and_line)(rect, X1, Y1, X2, Y2)
}
