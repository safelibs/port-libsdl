use crate::abi::generated_types::{SDL_FPoint, SDL_FRect, SDL_Point, SDL_Rect, SDL_bool};
use crate::core::error::invalid_param_error;

const CODE_BOTTOM: i32 = 1;
const CODE_TOP: i32 = 2;
const CODE_LEFT: i32 = 4;
const CODE_RIGHT: i32 = 8;

#[inline]
fn invalid_rect_param(param: &str) -> SDL_bool {
    let _ = invalid_param_error(param);
    0
}

#[inline]
fn report_invalid_rect_param(param: &str) {
    let _ = invalid_param_error(param);
}

#[inline]
fn rect_empty(rect: &SDL_Rect) -> bool {
    rect.w <= 0 || rect.h <= 0
}

#[inline]
fn rect_empty_f(rect: &SDL_FRect) -> bool {
    rect.w <= 0.0 || rect.h <= 0.0
}

fn has_intersection_rect(a: &SDL_Rect, b: &SDL_Rect) -> SDL_bool {
    if rect_empty(a) || rect_empty(b) {
        return 0;
    }

    let mut amin = a.x as i64;
    let mut amax = amin + a.w as i64;
    let bmin = b.x as i64;
    let bmax = bmin + b.w as i64;
    if bmin > amin {
        amin = bmin;
    }
    if bmax < amax {
        amax = bmax;
    }
    if amax <= amin {
        return 0;
    }

    let mut amin = a.y as i64;
    let mut amax = amin + a.h as i64;
    let bmin = b.y as i64;
    let bmax = bmin + b.h as i64;
    if bmin > amin {
        amin = bmin;
    }
    if bmax < amax {
        amax = bmax;
    }
    if amax <= amin {
        return 0;
    }

    1
}

fn intersect_rect_rect(a: &SDL_Rect, b: &SDL_Rect, result: &mut SDL_Rect) -> SDL_bool {
    if rect_empty(a) || rect_empty(b) {
        result.w = 0;
        result.h = 0;
        return 0;
    }

    let mut amin = a.x as i64;
    let mut amax = amin + a.w as i64;
    let bmin = b.x as i64;
    let bmax = bmin + b.w as i64;
    if bmin > amin {
        amin = bmin;
    }
    result.x = amin as libc::c_int;
    if bmax < amax {
        amax = bmax;
    }
    result.w = (amax - amin) as libc::c_int;

    let mut amin = a.y as i64;
    let mut amax = amin + a.h as i64;
    let bmin = b.y as i64;
    let bmax = bmin + b.h as i64;
    if bmin > amin {
        amin = bmin;
    }
    result.y = amin as libc::c_int;
    if bmax < amax {
        amax = bmax;
    }
    result.h = (amax - amin) as libc::c_int;

    (!rect_empty(result)) as SDL_bool
}

fn union_rect_rect(a: &SDL_Rect, b: &SDL_Rect, result: &mut SDL_Rect) {
    if rect_empty(a) {
        if rect_empty(b) {
            *result = SDL_Rect {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
            };
        } else {
            *result = *b;
        }
        return;
    } else if rect_empty(b) {
        *result = *a;
        return;
    }

    let mut amin = a.x as i64;
    let mut amax = amin + a.w as i64;
    let bmin = b.x as i64;
    let bmax = bmin + b.w as i64;
    if bmin < amin {
        amin = bmin;
    }
    result.x = amin as libc::c_int;
    if bmax > amax {
        amax = bmax;
    }
    result.w = (amax - amin) as libc::c_int;

    let mut amin = a.y as i64;
    let mut amax = amin + a.h as i64;
    let bmin = b.y as i64;
    let bmax = bmin + b.h as i64;
    if bmin < amin {
        amin = bmin;
    }
    result.y = amin as libc::c_int;
    if bmax > amax {
        amax = bmax;
    }
    result.h = (amax - amin) as libc::c_int;
}

fn enclose_points_rect(
    points: *const SDL_Point,
    count: libc::c_int,
    clip: *const SDL_Rect,
    result: *mut SDL_Rect,
) -> SDL_bool {
    let points = unsafe { std::slice::from_raw_parts(points, count as usize) };

    let (minx, miny, maxx, maxy) = if clip.is_null() {
        if result.is_null() {
            return 1;
        }

        let mut minx = points[0].x as i64;
        let mut maxx = minx;
        let mut miny = points[0].y as i64;
        let mut maxy = miny;
        for point in &points[1..] {
            let x = point.x as i64;
            let y = point.y as i64;
            if x < minx {
                minx = x;
            } else if x > maxx {
                maxx = x;
            }
            if y < miny {
                miny = y;
            } else if y > maxy {
                maxy = y;
            }
        }
        (minx, miny, maxx, maxy)
    } else {
        let clip = unsafe { &*clip };
        if rect_empty(clip) {
            return 0;
        }
        let clip_minx = clip.x as i64;
        let clip_miny = clip.y as i64;
        let clip_maxx = clip.x as i64 + clip.w as i64 - 1;
        let clip_maxy = clip.y as i64 + clip.h as i64 - 1;
        let mut added = false;
        let mut minx = 0i64;
        let mut maxx = 0i64;
        let mut miny = 0i64;
        let mut maxy = 0i64;

        for point in points {
            let x = point.x as i64;
            let y = point.y as i64;
            if x < clip_minx || x > clip_maxx || y < clip_miny || y > clip_maxy {
                continue;
            }
            if !added {
                if result.is_null() {
                    return 1;
                }
                minx = x;
                maxx = x;
                miny = y;
                maxy = y;
                added = true;
                continue;
            }
            if x < minx {
                minx = x;
            } else if x > maxx {
                maxx = x;
            }
            if y < miny {
                miny = y;
            } else if y > maxy {
                maxy = y;
            }
        }

        if !added {
            return 0;
        }
        (minx, miny, maxx, maxy)
    };

    if !result.is_null() {
        unsafe {
            *result = SDL_Rect {
                x: minx as libc::c_int,
                y: miny as libc::c_int,
                w: (maxx - minx + 1) as libc::c_int,
                h: (maxy - miny + 1) as libc::c_int,
            };
        }
    }
    1
}

fn compute_out_code(rect: &SDL_Rect, x: libc::c_int, y: libc::c_int) -> i32 {
    let mut code = 0;
    let right = rect.x as i64 + rect.w as i64;
    let bottom = rect.y as i64 + rect.h as i64;
    if (y as i64) < rect.y as i64 {
        code |= CODE_TOP;
    } else if (y as i64) >= bottom {
        code |= CODE_BOTTOM;
    }
    if (x as i64) < rect.x as i64 {
        code |= CODE_LEFT;
    } else if (x as i64) >= right {
        code |= CODE_RIGHT;
    }
    code
}

fn intersect_rect_and_line_rect(
    rect: &SDL_Rect,
    x1: &mut libc::c_int,
    y1: &mut libc::c_int,
    x2: &mut libc::c_int,
    y2: &mut libc::c_int,
) -> SDL_bool {
    if rect_empty(rect) {
        return 0;
    }

    let mut lx1 = *x1;
    let mut ly1 = *y1;
    let mut lx2 = *x2;
    let mut ly2 = *y2;
    let rectx1 = rect.x;
    let recty1 = rect.y;
    let rectx2 = rect.x + rect.w - 1;
    let recty2 = rect.y + rect.h - 1;

    if lx1 >= rectx1
        && lx1 <= rectx2
        && lx2 >= rectx1
        && lx2 <= rectx2
        && ly1 >= recty1
        && ly1 <= recty2
        && ly2 >= recty1
        && ly2 <= recty2
    {
        return 1;
    }

    if (lx1 < rectx1 && lx2 < rectx1)
        || (lx1 > rectx2 && lx2 > rectx2)
        || (ly1 < recty1 && ly2 < recty1)
        || (ly1 > recty2 && ly2 > recty2)
    {
        return 0;
    }

    if ly1 == ly2 {
        if lx1 < rectx1 {
            *x1 = rectx1;
        } else if lx1 > rectx2 {
            *x1 = rectx2;
        }
        if lx2 < rectx1 {
            *x2 = rectx1;
        } else if lx2 > rectx2 {
            *x2 = rectx2;
        }
        return 1;
    }

    if lx1 == lx2 {
        if ly1 < recty1 {
            *y1 = recty1;
        } else if ly1 > recty2 {
            *y1 = recty2;
        }
        if ly2 < recty1 {
            *y2 = recty1;
        } else if ly2 > recty2 {
            *y2 = recty2;
        }
        return 1;
    }

    let mut outcode1 = compute_out_code(rect, lx1, ly1);
    let mut outcode2 = compute_out_code(rect, lx2, ly2);
    while outcode1 != 0 || outcode2 != 0 {
        if (outcode1 & outcode2) != 0 {
            return 0;
        }

        let (nx, ny) = if outcode1 != 0 {
            if (outcode1 & CODE_TOP) != 0 {
                let ny = recty1;
                let nx = lx1
                    + (((lx2 - lx1) as i64 * (ny - ly1) as i64) / (ly2 - ly1) as i64)
                        as libc::c_int;
                (nx, ny)
            } else if (outcode1 & CODE_BOTTOM) != 0 {
                let ny = recty2;
                let nx = lx1
                    + (((lx2 - lx1) as i64 * (ny - ly1) as i64) / (ly2 - ly1) as i64)
                        as libc::c_int;
                (nx, ny)
            } else if (outcode1 & CODE_LEFT) != 0 {
                let nx = rectx1;
                let ny = ly1
                    + (((ly2 - ly1) as i64 * (nx - lx1) as i64) / (lx2 - lx1) as i64)
                        as libc::c_int;
                (nx, ny)
            } else {
                let nx = rectx2;
                let ny = ly1
                    + (((ly2 - ly1) as i64 * (nx - lx1) as i64) / (lx2 - lx1) as i64)
                        as libc::c_int;
                (nx, ny)
            }
        } else if (outcode2 & CODE_TOP) != 0 {
            let ny = recty1;
            let nx = lx1
                + (((lx2 - lx1) as i64 * (ny - ly1) as i64) / (ly2 - ly1) as i64) as libc::c_int;
            (nx, ny)
        } else if (outcode2 & CODE_BOTTOM) != 0 {
            let ny = recty2;
            let nx = lx1
                + (((lx2 - lx1) as i64 * (ny - ly1) as i64) / (ly2 - ly1) as i64) as libc::c_int;
            (nx, ny)
        } else if (outcode2 & CODE_LEFT) != 0 {
            let nx = rectx1;
            let ny = ly1
                + (((ly2 - ly1) as i64 * (nx - lx1) as i64) / (lx2 - lx1) as i64) as libc::c_int;
            (nx, ny)
        } else {
            let nx = rectx2;
            let ny = ly1
                + (((ly2 - ly1) as i64 * (nx - lx1) as i64) / (lx2 - lx1) as i64) as libc::c_int;
            (nx, ny)
        };

        if outcode1 != 0 {
            lx1 = nx;
            ly1 = ny;
            outcode1 = compute_out_code(rect, lx1, ly1);
        } else {
            lx2 = nx;
            ly2 = ny;
            outcode2 = compute_out_code(rect, lx2, ly2);
        }
    }

    *x1 = lx1;
    *y1 = ly1;
    *x2 = lx2;
    *y2 = ly2;
    1
}

fn has_intersection_f_rect(a: &SDL_FRect, b: &SDL_FRect) -> SDL_bool {
    if rect_empty_f(a) || rect_empty_f(b) {
        return 0;
    }

    let mut amin = a.x as f64;
    let mut amax = amin + a.w as f64;
    let bmin = b.x as f64;
    let bmax = bmin + b.w as f64;
    if bmin > amin {
        amin = bmin;
    }
    if bmax < amax {
        amax = bmax;
    }
    if amax <= amin {
        return 0;
    }

    let mut amin = a.y as f64;
    let mut amax = amin + a.h as f64;
    let bmin = b.y as f64;
    let bmax = bmin + b.h as f64;
    if bmin > amin {
        amin = bmin;
    }
    if bmax < amax {
        amax = bmax;
    }
    if amax <= amin {
        return 0;
    }

    1
}

fn intersect_f_rect_rect(a: &SDL_FRect, b: &SDL_FRect, result: &mut SDL_FRect) -> SDL_bool {
    if rect_empty_f(a) || rect_empty_f(b) {
        result.w = 0.0;
        result.h = 0.0;
        return 0;
    }

    let mut amin = a.x;
    let mut amax = amin + a.w;
    let bmin = b.x;
    let bmax = bmin + b.w;
    if bmin > amin {
        amin = bmin;
    }
    result.x = amin;
    if bmax < amax {
        amax = bmax;
    }
    result.w = amax - amin;

    let mut amin = a.y;
    let mut amax = amin + a.h;
    let bmin = b.y;
    let bmax = bmin + b.h;
    if bmin > amin {
        amin = bmin;
    }
    result.y = amin;
    if bmax < amax {
        amax = bmax;
    }
    result.h = amax - amin;

    (!rect_empty_f(result)) as SDL_bool
}

fn union_f_rect_rect(a: &SDL_FRect, b: &SDL_FRect, result: &mut SDL_FRect) {
    if rect_empty_f(a) {
        if rect_empty_f(b) {
            *result = SDL_FRect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
            };
        } else {
            *result = *b;
        }
        return;
    } else if rect_empty_f(b) {
        *result = *a;
        return;
    }

    let mut amin = a.x;
    let mut amax = amin + a.w;
    let bmin = b.x;
    let bmax = bmin + b.w;
    if bmin < amin {
        amin = bmin;
    }
    result.x = amin;
    if bmax > amax {
        amax = bmax;
    }
    result.w = amax - amin;

    let mut amin = a.y;
    let mut amax = amin + a.h;
    let bmin = b.y;
    let bmax = bmin + b.h;
    if bmin < amin {
        amin = bmin;
    }
    result.y = amin;
    if bmax > amax {
        amax = bmax;
    }
    result.h = amax - amin;
}

fn enclose_points_f_rect(
    points: *const SDL_FPoint,
    count: libc::c_int,
    clip: *const SDL_FRect,
    result: *mut SDL_FRect,
) -> SDL_bool {
    let points = unsafe { std::slice::from_raw_parts(points, count as usize) };

    let (minx, miny, maxx, maxy) = if clip.is_null() {
        if result.is_null() {
            return 1;
        }

        let mut minx = points[0].x;
        let mut maxx = minx;
        let mut miny = points[0].y;
        let mut maxy = miny;
        for point in &points[1..] {
            let x = point.x;
            let y = point.y;
            if x < minx {
                minx = x;
            } else if x > maxx {
                maxx = x;
            }
            if y < miny {
                miny = y;
            } else if y > maxy {
                maxy = y;
            }
        }
        (minx, miny, maxx, maxy)
    } else {
        let clip = unsafe { &*clip };
        if rect_empty_f(clip) {
            return 0;
        }
        let clip_minx = clip.x;
        let clip_miny = clip.y;
        let clip_maxx = clip.x + clip.w - 1.0;
        let clip_maxy = clip.y + clip.h - 1.0;
        let mut added = false;
        let mut minx = 0.0f32;
        let mut maxx = 0.0f32;
        let mut miny = 0.0f32;
        let mut maxy = 0.0f32;

        for point in points {
            let x = point.x;
            let y = point.y;
            if x < clip_minx || x > clip_maxx || y < clip_miny || y > clip_maxy {
                continue;
            }
            if !added {
                if result.is_null() {
                    return 1;
                }
                minx = x;
                maxx = x;
                miny = y;
                maxy = y;
                added = true;
                continue;
            }
            if x < minx {
                minx = x;
            } else if x > maxx {
                maxx = x;
            }
            if y < miny {
                miny = y;
            } else if y > maxy {
                maxy = y;
            }
        }

        if !added {
            return 0;
        }
        (minx, miny, maxx, maxy)
    };

    if !result.is_null() {
        unsafe {
            *result = SDL_FRect {
                x: minx,
                y: miny,
                w: maxx - minx + 1.0,
                h: maxy - miny + 1.0,
            };
        }
    }
    1
}

fn compute_out_code_f(rect: &SDL_FRect, x: f32, y: f32) -> i32 {
    let mut code = 0;
    if y < rect.y {
        code |= CODE_TOP;
    } else if y >= rect.y + rect.h {
        code |= CODE_BOTTOM;
    }
    if x < rect.x {
        code |= CODE_LEFT;
    } else if x >= rect.x + rect.w {
        code |= CODE_RIGHT;
    }
    code
}

fn intersect_f_rect_and_line_rect(
    rect: &SDL_FRect,
    x1: &mut f32,
    y1: &mut f32,
    x2: &mut f32,
    y2: &mut f32,
) -> SDL_bool {
    if rect_empty_f(rect) {
        return 0;
    }

    let mut lx1 = *x1;
    let mut ly1 = *y1;
    let mut lx2 = *x2;
    let mut ly2 = *y2;
    let rectx1 = rect.x;
    let recty1 = rect.y;
    let rectx2 = rect.x + rect.w - 1.0;
    let recty2 = rect.y + rect.h - 1.0;

    if lx1 >= rectx1
        && lx1 <= rectx2
        && lx2 >= rectx1
        && lx2 <= rectx2
        && ly1 >= recty1
        && ly1 <= recty2
        && ly2 >= recty1
        && ly2 <= recty2
    {
        return 1;
    }

    if (lx1 < rectx1 && lx2 < rectx1)
        || (lx1 > rectx2 && lx2 > rectx2)
        || (ly1 < recty1 && ly2 < recty1)
        || (ly1 > recty2 && ly2 > recty2)
    {
        return 0;
    }

    if ly1 == ly2 {
        if lx1 < rectx1 {
            *x1 = rectx1;
        } else if lx1 > rectx2 {
            *x1 = rectx2;
        }
        if lx2 < rectx1 {
            *x2 = rectx1;
        } else if lx2 > rectx2 {
            *x2 = rectx2;
        }
        return 1;
    }

    if lx1 == lx2 {
        if ly1 < recty1 {
            *y1 = recty1;
        } else if ly1 > recty2 {
            *y1 = recty2;
        }
        if ly2 < recty1 {
            *y2 = recty1;
        } else if ly2 > recty2 {
            *y2 = recty2;
        }
        return 1;
    }

    let mut outcode1 = compute_out_code_f(rect, lx1, ly1);
    let mut outcode2 = compute_out_code_f(rect, lx2, ly2);
    while outcode1 != 0 || outcode2 != 0 {
        if (outcode1 & outcode2) != 0 {
            return 0;
        }

        let (nx, ny) = if outcode1 != 0 {
            if (outcode1 & CODE_TOP) != 0 {
                let ny = recty1;
                let nx = (lx1 as f64
                    + ((lx2 - lx1) as f64 * (ny - ly1) as f64) / (ly2 - ly1) as f64)
                    as f32;
                (nx, ny)
            } else if (outcode1 & CODE_BOTTOM) != 0 {
                let ny = recty2;
                let nx = (lx1 as f64
                    + ((lx2 - lx1) as f64 * (ny - ly1) as f64) / (ly2 - ly1) as f64)
                    as f32;
                (nx, ny)
            } else if (outcode1 & CODE_LEFT) != 0 {
                let nx = rectx1;
                let ny = (ly1 as f64
                    + ((ly2 - ly1) as f64 * (nx - lx1) as f64) / (lx2 - lx1) as f64)
                    as f32;
                (nx, ny)
            } else {
                let nx = rectx2;
                let ny = (ly1 as f64
                    + ((ly2 - ly1) as f64 * (nx - lx1) as f64) / (lx2 - lx1) as f64)
                    as f32;
                (nx, ny)
            }
        } else if (outcode2 & CODE_TOP) != 0 {
            let ny = recty1;
            let nx =
                (lx1 as f64 + ((lx2 - lx1) as f64 * (ny - ly1) as f64) / (ly2 - ly1) as f64) as f32;
            (nx, ny)
        } else if (outcode2 & CODE_BOTTOM) != 0 {
            let ny = recty2;
            let nx =
                (lx1 as f64 + ((lx2 - lx1) as f64 * (ny - ly1) as f64) / (ly2 - ly1) as f64) as f32;
            (nx, ny)
        } else if (outcode2 & CODE_LEFT) != 0 {
            let nx = rectx1;
            let ny =
                (ly1 as f64 + ((ly2 - ly1) as f64 * (nx - lx1) as f64) / (lx2 - lx1) as f64) as f32;
            (nx, ny)
        } else {
            let nx = rectx2;
            let ny =
                (ly1 as f64 + ((ly2 - ly1) as f64 * (nx - lx1) as f64) / (lx2 - lx1) as f64) as f32;
            (nx, ny)
        };

        if outcode1 != 0 {
            lx1 = nx;
            ly1 = ny;
            outcode1 = compute_out_code_f(rect, lx1, ly1);
        } else {
            lx2 = nx;
            ly2 = ny;
            outcode2 = compute_out_code_f(rect, lx2, ly2);
        }
    }

    *x1 = lx1;
    *y1 = ly1;
    *x2 = lx2;
    *y2 = ly2;
    1
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HasIntersection(A: *const SDL_Rect, B: *const SDL_Rect) -> SDL_bool {
    if A.is_null() {
        return invalid_rect_param("A");
    }
    if B.is_null() {
        return invalid_rect_param("B");
    }
    has_intersection_rect(&*A, &*B)
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
    intersect_rect_rect(&*A, &*B, &mut *result)
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
    union_rect_rect(&*A, &*B, &mut *result);
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
    enclose_points_rect(points, count, clip, result)
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
    intersect_rect_and_line_rect(&*rect, &mut *X1, &mut *Y1, &mut *X2, &mut *Y2)
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
    has_intersection_f_rect(&*A, &*B)
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
    intersect_f_rect_rect(&*A, &*B, &mut *result)
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
    union_f_rect_rect(&*A, &*B, &mut *result);
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
    enclose_points_f_rect(points, count, clip, result)
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
    intersect_f_rect_and_line_rect(&*rect, &mut *X1, &mut *Y1, &mut *X2, &mut *Y2)
}
