use std::collections::HashSet;
use std::ptr;
use std::sync::{Mutex, OnceLock};

use crate::abi::generated_types::{
    SDL_BlendMode, SDL_BlendMode_SDL_BLENDMODE_NONE, SDL_FPoint, SDL_FRect,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888, SDL_Point, SDL_Rect, SDL_Renderer,
    SDL_RendererFlags_SDL_RENDERER_SOFTWARE, SDL_RendererFlags_SDL_RENDERER_TARGETTEXTURE,
    SDL_RendererInfo, SDL_ScaleMode, SDL_ScaleMode_SDL_ScaleModeNearest, SDL_Surface, SDL_Texture,
    SDL_Vertex, SDL_Window, SDL_bool, Uint32, Uint8,
};
use crate::core::error::{invalid_param_error, set_error_message};

const LOCAL_RENDERER_TAG: usize = 0x1;
const LOCAL_TEXTURE_TAG: usize = 0x2;
const SOFTWARE_RENDERER_NAME: &[u8] = b"software\0";

struct LocalRenderer {
    window: *mut SDL_Window,
    default_target: *mut SDL_Surface,
    owns_default_target: bool,
    render_target: *mut SDL_Texture,
    draw_color: (Uint8, Uint8, Uint8, Uint8),
    draw_blend_mode: SDL_BlendMode,
    viewport: Option<SDL_Rect>,
    clip_rect: Option<SDL_Rect>,
    scale: (f32, f32),
    logical_size: Option<(libc::c_int, libc::c_int)>,
    integer_scale: SDL_bool,
}

unsafe impl Send for LocalRenderer {}

struct LocalTexture {
    owner_renderer: *mut SDL_Renderer,
    surface: *mut SDL_Surface,
    access: libc::c_int,
    scale_mode: SDL_ScaleMode,
    user_data: *mut libc::c_void,
}

unsafe impl Send for LocalTexture {}

fn local_texture_handles() -> &'static Mutex<HashSet<usize>> {
    static REGISTRY: OnceLock<Mutex<HashSet<usize>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashSet::new()))
}

fn register_local_texture(texture: *mut SDL_Texture) {
    local_texture_handles()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(texture as usize);
}

fn unregister_local_texture(texture: *mut SDL_Texture) {
    local_texture_handles()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(&(texture as usize));
}

fn active_local_textures() -> Vec<usize> {
    local_texture_handles()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .iter()
        .copied()
        .collect()
}

fn renderer_handle(renderer: *mut LocalRenderer) -> *mut SDL_Renderer {
    ((renderer as usize) | LOCAL_RENDERER_TAG) as *mut SDL_Renderer
}

unsafe fn renderer_ptr(renderer: *mut SDL_Renderer) -> *mut LocalRenderer {
    ((renderer as usize) & !LOCAL_RENDERER_TAG) as *mut LocalRenderer
}

fn texture_handle(texture: *mut LocalTexture) -> *mut SDL_Texture {
    ((texture as usize) | LOCAL_TEXTURE_TAG) as *mut SDL_Texture
}

unsafe fn texture_ptr(texture: *mut SDL_Texture) -> *mut LocalTexture {
    ((texture as usize) & !LOCAL_TEXTURE_TAG) as *mut LocalTexture
}

pub(crate) fn is_local_renderer(renderer: *mut SDL_Renderer) -> bool {
    !renderer.is_null() && ((renderer as usize) & LOCAL_RENDERER_TAG) != 0
}

pub(crate) fn is_local_texture(texture: *mut SDL_Texture) -> bool {
    !texture.is_null() && ((texture as usize) & LOCAL_TEXTURE_TAG) != 0
}

pub(crate) unsafe fn renderer_window(renderer: *mut SDL_Renderer) -> Option<*mut SDL_Window> {
    if !is_local_renderer(renderer) {
        return None;
    }
    Some((*renderer_ptr(renderer)).window)
}

pub(crate) unsafe fn renderer_name(renderer: *mut SDL_Renderer) -> Option<String> {
    if !is_local_renderer(renderer) {
        return None;
    }
    Some("software".to_string())
}

pub(crate) unsafe fn renderer_is_software(renderer: *mut SDL_Renderer) -> bool {
    is_local_renderer(renderer)
}

unsafe fn create_local_renderer(
    surface: *mut SDL_Surface,
    window: *mut SDL_Window,
    owns_default_target: bool,
) -> *mut SDL_Renderer {
    let renderer = Box::new(LocalRenderer {
        window,
        default_target: surface,
        owns_default_target,
        render_target: ptr::null_mut(),
        draw_color: (0, 0, 0, 255),
        draw_blend_mode: SDL_BlendMode_SDL_BLENDMODE_NONE,
        viewport: None,
        clip_rect: None,
        scale: (1.0, 1.0),
        logical_size: None,
        integer_scale: 0,
    });
    renderer_handle(Box::into_raw(renderer))
}

pub(crate) unsafe fn create_window_renderer(
    window: *mut SDL_Window,
    width: libc::c_int,
    height: libc::c_int,
) -> *mut SDL_Renderer {
    let surface = crate::video::surface::SDL_CreateRGBSurfaceWithFormat(
        0,
        width.max(1),
        height.max(1),
        32,
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888,
    );
    if surface.is_null() {
        return ptr::null_mut();
    }
    create_local_renderer(surface, window, true)
}

pub(crate) unsafe fn create_software_renderer(surface: *mut SDL_Surface) -> *mut SDL_Renderer {
    if surface.is_null() {
        let _ = invalid_param_error("surface");
        return ptr::null_mut();
    }
    if crate::video::surface::validate_surface_storage(surface).is_err() {
        let _ = set_error_message("Invalid surface");
        return ptr::null_mut();
    }
    create_local_renderer(surface, ptr::null_mut(), false)
}

unsafe fn current_target_surface(renderer: *mut SDL_Renderer) -> *mut SDL_Surface {
    let local = &mut *renderer_ptr(renderer);
    if local.render_target.is_null() {
        local.default_target
    } else {
        (*texture_ptr(local.render_target)).surface
    }
}

unsafe fn target_bounds(renderer: *mut SDL_Renderer) -> SDL_Rect {
    crate::video::surface::full_surface_rect(current_target_surface(renderer))
}

unsafe fn effective_viewport(renderer: *mut SDL_Renderer) -> SDL_Rect {
    let local = &*renderer_ptr(renderer);
    let bounds = target_bounds(renderer);
    match local.viewport {
        Some(viewport) => {
            crate::video::surface::intersect_rects(&bounds, &viewport).unwrap_or(SDL_Rect {
                x: bounds.x,
                y: bounds.y,
                w: 0,
                h: 0,
            })
        }
        None => bounds,
    }
}

fn scaled_dimension(value: libc::c_int, scale: f32) -> libc::c_int {
    if value == 0 {
        0
    } else {
        ((value as f32) * scale).round() as libc::c_int
    }
}

unsafe fn transformed_rect(renderer: *mut SDL_Renderer, rect: SDL_Rect) -> SDL_Rect {
    let local = &*renderer_ptr(renderer);
    let viewport = effective_viewport(renderer);
    SDL_Rect {
        x: viewport.x + ((rect.x as f32) * local.scale.0).round() as libc::c_int,
        y: viewport.y + ((rect.y as f32) * local.scale.1).round() as libc::c_int,
        w: scaled_dimension(rect.w, local.scale.0),
        h: scaled_dimension(rect.h, local.scale.1),
    }
}

unsafe fn current_draw_clip(renderer: *mut SDL_Renderer) -> Option<SDL_Rect> {
    let mut clip = effective_viewport(renderer);
    if clip.w <= 0 || clip.h <= 0 {
        return None;
    }

    if let Some(raw_clip) = (*renderer_ptr(renderer)).clip_rect {
        let transformed = transformed_rect(renderer, raw_clip);
        clip = crate::video::surface::intersect_rects(&clip, &transformed)?;
    }

    Some(clip)
}

unsafe fn texture_region(texture: *mut SDL_Texture) -> SDL_Rect {
    crate::video::surface::full_surface_rect((*texture_ptr(texture)).surface)
}

unsafe fn surface_clip_guard<T>(
    surface: *mut SDL_Surface,
    clip: Option<SDL_Rect>,
    f: impl FnOnce() -> T,
) -> T {
    let saved = (*surface).clip_rect;
    match clip {
        Some(rect) => {
            let _ = crate::video::surface::SDL_SetClipRect(surface, &rect);
        }
        None => {
            let _ = crate::video::surface::SDL_SetClipRect(
                surface,
                &SDL_Rect {
                    x: 0,
                    y: 0,
                    w: 0,
                    h: 0,
                },
            );
        }
    }
    let result = f();
    let _ = crate::video::surface::SDL_SetClipRect(surface, &saved);
    result
}

unsafe fn blended_draw_point(
    renderer: *mut SDL_Renderer,
    x: libc::c_int,
    y: libc::c_int,
    clip: &SDL_Rect,
) -> libc::c_int {
    if x < clip.x || y < clip.y || x >= clip.x + clip.w || y >= clip.y + clip.h {
        return 0;
    }

    let surface = current_target_surface(renderer);
    let descriptor = match crate::video::surface::validate_surface_storage(surface) {
        Ok(descriptor) => descriptor,
        Err(error) => return crate::video::surface::apply_math_error(error),
    };
    let pixel = match crate::video::surface::pixel_pointer(
        surface,
        descriptor.bytes_per_pixel,
        x,
        y,
        "render pixel overflow",
    ) {
        Ok(pixel) => pixel,
        Err(error) => return crate::video::surface::apply_math_error(error),
    };

    let local = &*renderer_ptr(renderer);
    let mut rgba = local.draw_color;
    if local.draw_blend_mode != SDL_BlendMode_SDL_BLENDMODE_NONE {
        let dst = crate::video::surface::read_rgba_pixel(
            (*surface).format,
            descriptor.bytes_per_pixel,
            pixel,
        );
        rgba = crate::video::surface::blend_pixel(rgba, dst, local.draw_blend_mode);
    }

    crate::video::surface::write_rgba_pixel(
        (*surface).format,
        descriptor.bytes_per_pixel,
        pixel,
        rgba,
    );
    0
}

unsafe fn render_fill_rect_internal(
    renderer: *mut SDL_Renderer,
    rect: SDL_Rect,
    clip: &SDL_Rect,
) -> libc::c_int {
    let Some(clipped) = crate::video::surface::intersect_rects(&rect, clip) else {
        return 0;
    };
    if clipped.w <= 0 || clipped.h <= 0 {
        return 0;
    }
    for row in 0..clipped.h {
        for column in 0..clipped.w {
            let result = blended_draw_point(renderer, clipped.x + column, clipped.y + row, clip);
            if result < 0 {
                return result;
            }
        }
    }
    0
}

unsafe fn render_draw_line_internal(
    renderer: *mut SDL_Renderer,
    mut x1: libc::c_int,
    mut y1: libc::c_int,
    x2: libc::c_int,
    y2: libc::c_int,
    clip: &SDL_Rect,
) -> libc::c_int {
    let dx = (x2 - x1).abs();
    let sx = if x1 < x2 { 1 } else { -1 };
    let dy = -(y2 - y1).abs();
    let sy = if y1 < y2 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        let result = blended_draw_point(renderer, x1, y1, clip);
        if result < 0 {
            return result;
        }
        if x1 == x2 && y1 == y2 {
            break;
        }
        let e2 = err * 2;
        if e2 >= dy {
            err += dy;
            x1 += sx;
        }
        if e2 <= dx {
            err += dx;
            y1 += sy;
        }
    }

    0
}

unsafe fn copy_surface_state(src: *mut SDL_Surface, dst: *mut SDL_Surface) {
    let mut r = 255;
    let mut g = 255;
    let mut b = 255;
    let mut alpha = 255;
    let mut blend_mode = SDL_BlendMode_SDL_BLENDMODE_NONE;
    let mut color_key = 0;

    let _ = crate::video::surface::SDL_GetSurfaceColorMod(src, &mut r, &mut g, &mut b);
    let _ = crate::video::surface::SDL_SetSurfaceColorMod(dst, r, g, b);
    let _ = crate::video::surface::SDL_GetSurfaceAlphaMod(src, &mut alpha);
    let _ = crate::video::surface::SDL_SetSurfaceAlphaMod(dst, alpha);
    let _ = crate::video::surface::SDL_GetSurfaceBlendMode(src, &mut blend_mode);
    let _ = crate::video::surface::SDL_SetSurfaceBlendMode(dst, blend_mode);
    if crate::video::surface::SDL_HasColorKey(src) != 0
        && crate::video::surface::SDL_GetColorKey(src, &mut color_key) == 0
    {
        let _ = crate::video::surface::SDL_SetColorKey(dst, 1, color_key);
    }
}

unsafe fn render_copy_common(
    renderer: *mut SDL_Renderer,
    texture: *mut SDL_Texture,
    srcrect: *const SDL_Rect,
    dstrect: SDL_Rect,
) -> libc::c_int {
    let texture_surface = (*texture_ptr(texture)).surface;
    let src_rect = if srcrect.is_null() {
        texture_region(texture)
    } else {
        *srcrect
    };
    let surface = current_target_surface(renderer);
    let clip = current_draw_clip(renderer);

    surface_clip_guard(surface, clip, || {
        if src_rect.w == dstrect.w && src_rect.h == dstrect.h {
            let mut dst = dstrect;
            crate::video::blit::SDL_UpperBlit(texture_surface, &src_rect, surface, &mut dst)
        } else {
            let scaled = crate::video::surface::SDL_CreateRGBSurfaceWithFormat(
                0,
                dstrect.w.max(1),
                dstrect.h.max(1),
                32,
                (*(*texture_surface).format).format,
            );
            if scaled.is_null() {
                return set_error_message("Couldn't scale texture");
            }
            let scaled_rect = crate::video::surface::full_surface_rect(scaled);
            let scale_result = crate::video::blit::SDL_UpperBlitScaled(
                texture_surface,
                &src_rect,
                scaled,
                &mut SDL_Rect {
                    x: scaled_rect.x,
                    y: scaled_rect.y,
                    w: scaled_rect.w,
                    h: scaled_rect.h,
                },
            );
            if scale_result == 0 {
                copy_surface_state(texture_surface, scaled);
            }
            let result = if scale_result == 0 {
                let mut dst = dstrect;
                crate::video::blit::SDL_UpperBlit(scaled, ptr::null(), surface, &mut dst)
            } else {
                scale_result
            };
            crate::video::surface::SDL_FreeSurface(scaled);
            result
        }
    })
}

unsafe fn texture_format(texture: *mut SDL_Texture) -> Uint32 {
    (*(*(*texture_ptr(texture)).surface).format).format
}

unsafe fn texture_size(texture: *mut SDL_Texture) -> (libc::c_int, libc::c_int) {
    let surface = (*texture_ptr(texture)).surface;
    ((*surface).w, (*surface).h)
}

unsafe fn build_renderer_info(info: *mut SDL_RendererInfo, renderer: *mut SDL_Renderer) {
    ptr::write_bytes(info, 0, 1);
    let surface = current_target_surface(renderer);
    (*info).name = SOFTWARE_RENDERER_NAME.as_ptr().cast();
    (*info).flags =
        SDL_RendererFlags_SDL_RENDERER_SOFTWARE | SDL_RendererFlags_SDL_RENDERER_TARGETTEXTURE;
    (*info).num_texture_formats = 1;
    (*info).texture_formats[0] = SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888;
    (*info).max_texture_width = (*surface).w.max(1);
    (*info).max_texture_height = (*surface).h.max(1);
}

unsafe fn float_rect_to_int(rect: *const SDL_FRect) -> Option<SDL_Rect> {
    if rect.is_null() {
        None
    } else {
        Some(SDL_Rect {
            x: (*rect).x.round() as libc::c_int,
            y: (*rect).y.round() as libc::c_int,
            w: (*rect).w.round() as libc::c_int,
            h: (*rect).h.round() as libc::c_int,
        })
    }
}

unsafe fn float_point_to_int(point: *const SDL_FPoint) -> Option<SDL_Point> {
    if point.is_null() {
        None
    } else {
        Some(SDL_Point {
            x: (*point).x.round() as libc::c_int,
            y: (*point).y.round() as libc::c_int,
        })
    }
}

pub(crate) unsafe fn get_num_render_drivers() -> libc::c_int {
    1
}

pub(crate) unsafe fn get_render_driver_info(
    index: libc::c_int,
    info: *mut SDL_RendererInfo,
) -> libc::c_int {
    if info.is_null() {
        return invalid_param_error("info");
    }
    if index != 0 {
        return set_error_message("Renderer index out of range");
    }
    ptr::write_bytes(info, 0, 1);
    (*info).name = SOFTWARE_RENDERER_NAME.as_ptr().cast();
    (*info).flags =
        SDL_RendererFlags_SDL_RENDERER_SOFTWARE | SDL_RendererFlags_SDL_RENDERER_TARGETTEXTURE;
    (*info).num_texture_formats = 1;
    (*info).texture_formats[0] = SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888;
    (*info).max_texture_width = i32::MAX;
    (*info).max_texture_height = i32::MAX;
    0
}

pub(crate) unsafe fn get_renderer_info(
    renderer: *mut SDL_Renderer,
    info: *mut SDL_RendererInfo,
) -> libc::c_int {
    if !is_local_renderer(renderer) {
        return invalid_param_error("renderer");
    }
    if info.is_null() {
        return invalid_param_error("info");
    }
    build_renderer_info(info, renderer);
    0
}

pub(crate) unsafe fn get_renderer_output_size(
    renderer: *mut SDL_Renderer,
    w: *mut libc::c_int,
    h: *mut libc::c_int,
) -> libc::c_int {
    if !is_local_renderer(renderer) {
        return invalid_param_error("renderer");
    }
    if w.is_null() || h.is_null() {
        return invalid_param_error("w");
    }
    let surface = current_target_surface(renderer);
    *w = (*surface).w;
    *h = (*surface).h;
    0
}

pub(crate) unsafe fn create_texture(
    renderer: *mut SDL_Renderer,
    format: Uint32,
    access: libc::c_int,
    w: libc::c_int,
    h: libc::c_int,
) -> *mut SDL_Texture {
    if !is_local_renderer(renderer) {
        let _ = invalid_param_error("renderer");
        return ptr::null_mut();
    }
    if w <= 0 {
        let _ = invalid_param_error("w");
        return ptr::null_mut();
    }
    if h <= 0 {
        let _ = invalid_param_error("h");
        return ptr::null_mut();
    }

    let surface = crate::video::surface::SDL_CreateRGBSurfaceWithFormat(0, w, h, 32, format);
    if surface.is_null() {
        return ptr::null_mut();
    }

    let texture = Box::new(LocalTexture {
        owner_renderer: renderer,
        surface,
        access,
        scale_mode: SDL_ScaleMode_SDL_ScaleModeNearest,
        user_data: ptr::null_mut(),
    });
    let handle = texture_handle(Box::into_raw(texture));
    register_local_texture(handle);
    handle
}

pub(crate) unsafe fn create_texture_from_surface(
    renderer: *mut SDL_Renderer,
    surface: *mut SDL_Surface,
) -> *mut SDL_Texture {
    if !is_local_renderer(renderer) {
        let _ = invalid_param_error("renderer");
        return ptr::null_mut();
    }
    if surface.is_null() {
        let _ = invalid_param_error("surface");
        return ptr::null_mut();
    }

    let duplicated = crate::video::surface::SDL_DuplicateSurface(surface);
    if duplicated.is_null() {
        return ptr::null_mut();
    }
    let texture = Box::new(LocalTexture {
        owner_renderer: renderer,
        surface: duplicated,
        access: 0,
        scale_mode: SDL_ScaleMode_SDL_ScaleModeNearest,
        user_data: ptr::null_mut(),
    });
    let handle = texture_handle(Box::into_raw(texture));
    register_local_texture(handle);
    handle
}

pub(crate) unsafe fn query_texture(
    texture: *mut SDL_Texture,
    format: *mut Uint32,
    access: *mut libc::c_int,
    w: *mut libc::c_int,
    h: *mut libc::c_int,
) -> libc::c_int {
    if !is_local_texture(texture) {
        return invalid_param_error("texture");
    }
    if !format.is_null() {
        *format = texture_format(texture);
    }
    if !access.is_null() {
        *access = (*texture_ptr(texture)).access;
    }
    let (width, height) = texture_size(texture);
    if !w.is_null() {
        *w = width;
    }
    if !h.is_null() {
        *h = height;
    }
    0
}

pub(crate) unsafe fn set_texture_color_mod(
    texture: *mut SDL_Texture,
    r: Uint8,
    g: Uint8,
    b: Uint8,
) -> libc::c_int {
    crate::video::surface::SDL_SetSurfaceColorMod((*texture_ptr(texture)).surface, r, g, b)
}

pub(crate) unsafe fn get_texture_color_mod(
    texture: *mut SDL_Texture,
    r: *mut Uint8,
    g: *mut Uint8,
    b: *mut Uint8,
) -> libc::c_int {
    crate::video::surface::SDL_GetSurfaceColorMod((*texture_ptr(texture)).surface, r, g, b)
}

pub(crate) unsafe fn set_texture_alpha_mod(texture: *mut SDL_Texture, alpha: Uint8) -> libc::c_int {
    crate::video::surface::SDL_SetSurfaceAlphaMod((*texture_ptr(texture)).surface, alpha)
}

pub(crate) unsafe fn get_texture_alpha_mod(
    texture: *mut SDL_Texture,
    alpha: *mut Uint8,
) -> libc::c_int {
    crate::video::surface::SDL_GetSurfaceAlphaMod((*texture_ptr(texture)).surface, alpha)
}

pub(crate) unsafe fn set_texture_blend_mode(
    texture: *mut SDL_Texture,
    blend_mode: SDL_BlendMode,
) -> libc::c_int {
    crate::video::surface::SDL_SetSurfaceBlendMode((*texture_ptr(texture)).surface, blend_mode)
}

pub(crate) unsafe fn get_texture_blend_mode(
    texture: *mut SDL_Texture,
    blend_mode: *mut SDL_BlendMode,
) -> libc::c_int {
    crate::video::surface::SDL_GetSurfaceBlendMode((*texture_ptr(texture)).surface, blend_mode)
}

pub(crate) unsafe fn set_texture_scale_mode(
    texture: *mut SDL_Texture,
    scale_mode: SDL_ScaleMode,
) -> libc::c_int {
    (*texture_ptr(texture)).scale_mode = scale_mode;
    0
}

pub(crate) unsafe fn get_texture_scale_mode(
    texture: *mut SDL_Texture,
    scale_mode: *mut SDL_ScaleMode,
) -> libc::c_int {
    if scale_mode.is_null() {
        return invalid_param_error("scaleMode");
    }
    *scale_mode = (*texture_ptr(texture)).scale_mode;
    0
}

pub(crate) unsafe fn set_texture_user_data(
    texture: *mut SDL_Texture,
    user_data: *mut libc::c_void,
) -> libc::c_int {
    (*texture_ptr(texture)).user_data = user_data;
    0
}

pub(crate) unsafe fn get_texture_user_data(texture: *mut SDL_Texture) -> *mut libc::c_void {
    (*texture_ptr(texture)).user_data
}

pub(crate) unsafe fn update_texture(
    texture: *mut SDL_Texture,
    rect: *const SDL_Rect,
    pixels: *const libc::c_void,
    pitch: libc::c_int,
) -> libc::c_int {
    if pixels.is_null() {
        return invalid_param_error("pixels");
    }
    if pitch <= 0 {
        return invalid_param_error("pitch");
    }

    let surface = (*texture_ptr(texture)).surface;
    let region = if rect.is_null() {
        crate::video::surface::full_surface_rect(surface)
    } else {
        *rect
    };
    if region.w <= 0 || region.h <= 0 {
        return 0;
    }

    let descriptor = match crate::video::surface::validate_surface_storage(surface) {
        Ok(descriptor) => descriptor,
        Err(error) => return crate::video::surface::apply_math_error(error),
    };
    let dst = match crate::video::surface::pixel_pointer(
        surface,
        descriptor.bytes_per_pixel,
        region.x,
        region.y,
        "texture update overflow",
    ) {
        Ok(ptr) => ptr,
        Err(error) => return crate::video::surface::apply_math_error(error),
    };

    crate::video::blit::SDL_ConvertPixels(
        region.w,
        region.h,
        texture_format(texture),
        pixels,
        pitch,
        texture_format(texture),
        dst.cast(),
        (*surface).pitch,
    )
}

pub(crate) unsafe fn update_yuv_texture(
    _texture: *mut SDL_Texture,
    _rect: *const SDL_Rect,
    _yplane: *const Uint8,
    _ypitch: libc::c_int,
    _uplane: *const Uint8,
    _upitch: libc::c_int,
    _vplane: *const Uint8,
    _vpitch: libc::c_int,
) -> libc::c_int {
    set_error_message("YUV texture updates require the host renderer")
}

pub(crate) unsafe fn update_nv_texture(
    _texture: *mut SDL_Texture,
    _rect: *const SDL_Rect,
    _yplane: *const Uint8,
    _ypitch: libc::c_int,
    _uvplane: *const Uint8,
    _uvpitch: libc::c_int,
) -> libc::c_int {
    set_error_message("NV texture updates require the host renderer")
}

pub(crate) unsafe fn lock_texture(
    texture: *mut SDL_Texture,
    rect: *const SDL_Rect,
    pixels: *mut *mut libc::c_void,
    pitch: *mut libc::c_int,
) -> libc::c_int {
    if pixels.is_null() || pitch.is_null() {
        return invalid_param_error("pixels");
    }
    let surface = (*texture_ptr(texture)).surface;
    let descriptor = match crate::video::surface::validate_surface_storage(surface) {
        Ok(descriptor) => descriptor,
        Err(error) => return crate::video::surface::apply_math_error(error),
    };
    let region = if rect.is_null() {
        crate::video::surface::full_surface_rect(surface)
    } else {
        *rect
    };
    let ptr = match crate::video::surface::pixel_pointer(
        surface,
        descriptor.bytes_per_pixel,
        region.x,
        region.y,
        "texture lock overflow",
    ) {
        Ok(ptr) => ptr,
        Err(error) => return crate::video::surface::apply_math_error(error),
    };
    *pixels = ptr.cast();
    *pitch = (*surface).pitch;
    0
}

pub(crate) unsafe fn lock_texture_to_surface(
    texture: *mut SDL_Texture,
    _rect: *const SDL_Rect,
    surface: *mut *mut SDL_Surface,
) -> libc::c_int {
    if surface.is_null() {
        return invalid_param_error("surface");
    }
    *surface = (*texture_ptr(texture)).surface;
    0
}

pub(crate) unsafe fn unlock_texture(_texture: *mut SDL_Texture) {}

pub(crate) unsafe fn render_target_supported(_renderer: *mut SDL_Renderer) -> SDL_bool {
    1
}

pub(crate) unsafe fn set_render_target(
    renderer: *mut SDL_Renderer,
    texture: *mut SDL_Texture,
) -> libc::c_int {
    if texture.is_null() {
        (*renderer_ptr(renderer)).render_target = ptr::null_mut();
        return 0;
    }
    if !is_local_texture(texture) {
        return set_error_message("Target texture must belong to the local renderer");
    }
    let target = &*texture_ptr(texture);
    if target.owner_renderer != renderer {
        return set_error_message("Texture belongs to a different renderer");
    }
    if target.access
        != crate::abi::generated_types::SDL_TextureAccess_SDL_TEXTUREACCESS_TARGET as libc::c_int
    {
        return set_error_message("Texture was not created with target access");
    }
    (*renderer_ptr(renderer)).render_target = texture;
    0
}

pub(crate) unsafe fn get_render_target(renderer: *mut SDL_Renderer) -> *mut SDL_Texture {
    (*renderer_ptr(renderer)).render_target
}

pub(crate) unsafe fn render_set_logical_size(
    renderer: *mut SDL_Renderer,
    w: libc::c_int,
    h: libc::c_int,
) -> libc::c_int {
    (*renderer_ptr(renderer)).logical_size = Some((w, h));
    0
}

pub(crate) unsafe fn render_get_logical_size(
    renderer: *mut SDL_Renderer,
    w: *mut libc::c_int,
    h: *mut libc::c_int,
) {
    let logical = (*renderer_ptr(renderer)).logical_size.unwrap_or((0, 0));
    if !w.is_null() {
        *w = logical.0;
    }
    if !h.is_null() {
        *h = logical.1;
    }
}

pub(crate) unsafe fn render_set_integer_scale(
    renderer: *mut SDL_Renderer,
    enable: SDL_bool,
) -> libc::c_int {
    (*renderer_ptr(renderer)).integer_scale = enable;
    0
}

pub(crate) unsafe fn render_get_integer_scale(renderer: *mut SDL_Renderer) -> SDL_bool {
    (*renderer_ptr(renderer)).integer_scale
}

pub(crate) unsafe fn render_set_viewport(
    renderer: *mut SDL_Renderer,
    rect: *const SDL_Rect,
) -> libc::c_int {
    (*renderer_ptr(renderer)).viewport = if rect.is_null() { None } else { Some(*rect) };
    0
}

pub(crate) unsafe fn render_get_viewport(renderer: *mut SDL_Renderer, rect: *mut SDL_Rect) {
    if rect.is_null() {
        return;
    }
    *rect = effective_viewport(renderer);
}

pub(crate) unsafe fn render_set_clip_rect(
    renderer: *mut SDL_Renderer,
    rect: *const SDL_Rect,
) -> libc::c_int {
    (*renderer_ptr(renderer)).clip_rect = if rect.is_null() { None } else { Some(*rect) };
    0
}

pub(crate) unsafe fn render_get_clip_rect(renderer: *mut SDL_Renderer, rect: *mut SDL_Rect) {
    if rect.is_null() {
        return;
    }
    *rect = (*renderer_ptr(renderer)).clip_rect.unwrap_or(SDL_Rect {
        x: 0,
        y: 0,
        w: 0,
        h: 0,
    });
}

pub(crate) unsafe fn render_is_clip_enabled(renderer: *mut SDL_Renderer) -> SDL_bool {
    (*renderer_ptr(renderer)).clip_rect.is_some() as SDL_bool
}

pub(crate) unsafe fn render_set_scale(
    renderer: *mut SDL_Renderer,
    scale_x: f32,
    scale_y: f32,
) -> libc::c_int {
    (*renderer_ptr(renderer)).scale = (scale_x, scale_y);
    0
}

pub(crate) unsafe fn render_get_scale(
    renderer: *mut SDL_Renderer,
    scale_x: *mut f32,
    scale_y: *mut f32,
) {
    let scale = (*renderer_ptr(renderer)).scale;
    if !scale_x.is_null() {
        *scale_x = scale.0;
    }
    if !scale_y.is_null() {
        *scale_y = scale.1;
    }
}

pub(crate) unsafe fn render_window_to_logical(
    renderer: *mut SDL_Renderer,
    window_x: libc::c_int,
    window_y: libc::c_int,
    logical_x: *mut f32,
    logical_y: *mut f32,
) -> libc::c_int {
    let viewport = effective_viewport(renderer);
    let scale = (*renderer_ptr(renderer)).scale;
    if !logical_x.is_null() {
        *logical_x = (window_x - viewport.x) as f32 / scale.0.max(f32::EPSILON);
    }
    if !logical_y.is_null() {
        *logical_y = (window_y - viewport.y) as f32 / scale.1.max(f32::EPSILON);
    }
    0
}

pub(crate) unsafe fn render_logical_to_window(
    renderer: *mut SDL_Renderer,
    logical_x: f32,
    logical_y: f32,
    window_x: *mut libc::c_int,
    window_y: *mut libc::c_int,
) -> libc::c_int {
    let viewport = effective_viewport(renderer);
    let scale = (*renderer_ptr(renderer)).scale;
    if !window_x.is_null() {
        *window_x = viewport.x + (logical_x * scale.0).round() as libc::c_int;
    }
    if !window_y.is_null() {
        *window_y = viewport.y + (logical_y * scale.1).round() as libc::c_int;
    }
    0
}

pub(crate) unsafe fn set_render_draw_color(
    renderer: *mut SDL_Renderer,
    r: Uint8,
    g: Uint8,
    b: Uint8,
    a: Uint8,
) -> libc::c_int {
    (*renderer_ptr(renderer)).draw_color = (r, g, b, a);
    0
}

pub(crate) unsafe fn get_render_draw_color(
    renderer: *mut SDL_Renderer,
    r: *mut Uint8,
    g: *mut Uint8,
    b: *mut Uint8,
    a: *mut Uint8,
) -> libc::c_int {
    if r.is_null() || g.is_null() || b.is_null() || a.is_null() {
        return invalid_param_error("r");
    }
    let color = (*renderer_ptr(renderer)).draw_color;
    *r = color.0;
    *g = color.1;
    *b = color.2;
    *a = color.3;
    0
}

pub(crate) unsafe fn set_render_draw_blend_mode(
    renderer: *mut SDL_Renderer,
    blend_mode: SDL_BlendMode,
) -> libc::c_int {
    (*renderer_ptr(renderer)).draw_blend_mode = blend_mode;
    0
}

pub(crate) unsafe fn get_render_draw_blend_mode(
    renderer: *mut SDL_Renderer,
    blend_mode: *mut SDL_BlendMode,
) -> libc::c_int {
    if blend_mode.is_null() {
        return invalid_param_error("blendMode");
    }
    *blend_mode = (*renderer_ptr(renderer)).draw_blend_mode;
    0
}

pub(crate) unsafe fn render_clear(renderer: *mut SDL_Renderer) -> libc::c_int {
    let surface = current_target_surface(renderer);
    let descriptor = match crate::video::surface::validate_surface_storage(surface) {
        Ok(descriptor) => descriptor,
        Err(error) => return crate::video::surface::apply_math_error(error),
    };
    let pixel = crate::video::pixels::SDL_MapRGBA(
        (*surface).format,
        (*renderer_ptr(renderer)).draw_color.0,
        (*renderer_ptr(renderer)).draw_color.1,
        (*renderer_ptr(renderer)).draw_color.2,
        (*renderer_ptr(renderer)).draw_color.3,
    );
    let bounds = target_bounds(renderer);
    for y in 0..bounds.h {
        let row = match crate::video::surface::pixel_pointer(
            surface,
            descriptor.bytes_per_pixel,
            bounds.x,
            bounds.y + y,
            "render clear overflow",
        ) {
            Ok(ptr) => ptr,
            Err(error) => return crate::video::surface::apply_math_error(error),
        };
        for x in 0..bounds.w {
            crate::video::surface::write_raw_pixel(
                row.add(x as usize * descriptor.bytes_per_pixel as usize),
                descriptor.bytes_per_pixel,
                pixel,
            );
        }
    }
    0
}

pub(crate) unsafe fn render_draw_point(
    renderer: *mut SDL_Renderer,
    x: libc::c_int,
    y: libc::c_int,
) -> libc::c_int {
    let Some(clip) = current_draw_clip(renderer) else {
        return 0;
    };
    let viewport = effective_viewport(renderer);
    let scale = (*renderer_ptr(renderer)).scale;
    blended_draw_point(
        renderer,
        viewport.x + ((x as f32) * scale.0).round() as libc::c_int,
        viewport.y + ((y as f32) * scale.1).round() as libc::c_int,
        &clip,
    )
}

pub(crate) unsafe fn render_draw_points(
    renderer: *mut SDL_Renderer,
    points: *const SDL_Point,
    count: libc::c_int,
) -> libc::c_int {
    if count < 0 {
        return invalid_param_error("count");
    }
    if count > 0 && points.is_null() {
        return invalid_param_error("points");
    }
    for index in 0..count as isize {
        let point = *points.offset(index);
        let result = render_draw_point(renderer, point.x, point.y);
        if result < 0 {
            return result;
        }
    }
    0
}

pub(crate) unsafe fn render_draw_line(
    renderer: *mut SDL_Renderer,
    x1: libc::c_int,
    y1: libc::c_int,
    x2: libc::c_int,
    y2: libc::c_int,
) -> libc::c_int {
    let Some(clip) = current_draw_clip(renderer) else {
        return 0;
    };
    let viewport = effective_viewport(renderer);
    let scale = (*renderer_ptr(renderer)).scale;
    render_draw_line_internal(
        renderer,
        viewport.x + ((x1 as f32) * scale.0).round() as libc::c_int,
        viewport.y + ((y1 as f32) * scale.1).round() as libc::c_int,
        viewport.x + ((x2 as f32) * scale.0).round() as libc::c_int,
        viewport.y + ((y2 as f32) * scale.1).round() as libc::c_int,
        &clip,
    )
}

pub(crate) unsafe fn render_draw_lines(
    renderer: *mut SDL_Renderer,
    points: *const SDL_Point,
    count: libc::c_int,
) -> libc::c_int {
    if count < 0 {
        return invalid_param_error("count");
    }
    if count > 1 && points.is_null() {
        return invalid_param_error("points");
    }
    for index in 0..count.saturating_sub(1) as isize {
        let start = *points.offset(index);
        let end = *points.offset(index + 1);
        let result = render_draw_line(renderer, start.x, start.y, end.x, end.y);
        if result < 0 {
            return result;
        }
    }
    0
}

pub(crate) unsafe fn render_draw_rect(
    renderer: *mut SDL_Renderer,
    rect: *const SDL_Rect,
) -> libc::c_int {
    let rect = if rect.is_null() {
        SDL_Rect {
            x: 0,
            y: 0,
            w: effective_viewport(renderer).w,
            h: effective_viewport(renderer).h,
        }
    } else {
        *rect
    };
    let result = render_draw_line(renderer, rect.x, rect.y, rect.x + rect.w - 1, rect.y);
    if result < 0 {
        return result;
    }
    let result = render_draw_line(renderer, rect.x, rect.y, rect.x, rect.y + rect.h - 1);
    if result < 0 {
        return result;
    }
    let result = render_draw_line(
        renderer,
        rect.x + rect.w - 1,
        rect.y,
        rect.x + rect.w - 1,
        rect.y + rect.h - 1,
    );
    if result < 0 {
        return result;
    }
    render_draw_line(
        renderer,
        rect.x,
        rect.y + rect.h - 1,
        rect.x + rect.w - 1,
        rect.y + rect.h - 1,
    )
}

pub(crate) unsafe fn render_draw_rects(
    renderer: *mut SDL_Renderer,
    rects: *const SDL_Rect,
    count: libc::c_int,
) -> libc::c_int {
    if count < 0 {
        return invalid_param_error("count");
    }
    if count > 0 && rects.is_null() {
        return invalid_param_error("rects");
    }
    for index in 0..count as isize {
        let result = render_draw_rect(renderer, rects.offset(index));
        if result < 0 {
            return result;
        }
    }
    0
}

pub(crate) unsafe fn render_fill_rect(
    renderer: *mut SDL_Renderer,
    rect: *const SDL_Rect,
) -> libc::c_int {
    let Some(clip) = current_draw_clip(renderer) else {
        return 0;
    };
    let viewport = effective_viewport(renderer);
    let requested = if rect.is_null() {
        SDL_Rect {
            x: 0,
            y: 0,
            w: viewport.w,
            h: viewport.h,
        }
    } else {
        *rect
    };
    render_fill_rect_internal(renderer, transformed_rect(renderer, requested), &clip)
}

pub(crate) unsafe fn render_fill_rects(
    renderer: *mut SDL_Renderer,
    rects: *const SDL_Rect,
    count: libc::c_int,
) -> libc::c_int {
    if count < 0 {
        return invalid_param_error("count");
    }
    if count > 0 && rects.is_null() {
        return invalid_param_error("rects");
    }
    for index in 0..count as isize {
        let result = render_fill_rect(renderer, rects.offset(index));
        if result < 0 {
            return result;
        }
    }
    0
}

pub(crate) unsafe fn render_copy(
    renderer: *mut SDL_Renderer,
    texture: *mut SDL_Texture,
    srcrect: *const SDL_Rect,
    dstrect: *const SDL_Rect,
) -> libc::c_int {
    if !is_local_texture(texture) {
        return set_error_message("Texture does not belong to the local renderer");
    }
    let dst = if dstrect.is_null() {
        let viewport = effective_viewport(renderer);
        SDL_Rect {
            x: 0,
            y: 0,
            w: viewport.w,
            h: viewport.h,
        }
    } else {
        *dstrect
    };
    render_copy_common(renderer, texture, srcrect, transformed_rect(renderer, dst))
}

pub(crate) unsafe fn render_copy_ex(
    renderer: *mut SDL_Renderer,
    texture: *mut SDL_Texture,
    srcrect: *const SDL_Rect,
    dstrect: *const SDL_Rect,
    _angle: f64,
    _center: *const SDL_Point,
    _flip: Uint32,
) -> libc::c_int {
    render_copy(renderer, texture, srcrect, dstrect)
}

pub(crate) unsafe fn render_draw_point_f(
    renderer: *mut SDL_Renderer,
    x: f32,
    y: f32,
) -> libc::c_int {
    render_draw_point(renderer, x.round() as libc::c_int, y.round() as libc::c_int)
}

pub(crate) unsafe fn render_draw_points_f(
    renderer: *mut SDL_Renderer,
    points: *const SDL_FPoint,
    count: libc::c_int,
) -> libc::c_int {
    if count < 0 {
        return invalid_param_error("count");
    }
    if count > 0 && points.is_null() {
        return invalid_param_error("points");
    }
    for index in 0..count as isize {
        let point = *points.offset(index);
        let result = render_draw_point_f(renderer, point.x, point.y);
        if result < 0 {
            return result;
        }
    }
    0
}

pub(crate) unsafe fn render_draw_line_f(
    renderer: *mut SDL_Renderer,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
) -> libc::c_int {
    render_draw_line(
        renderer,
        x1.round() as libc::c_int,
        y1.round() as libc::c_int,
        x2.round() as libc::c_int,
        y2.round() as libc::c_int,
    )
}

pub(crate) unsafe fn render_draw_lines_f(
    renderer: *mut SDL_Renderer,
    points: *const SDL_FPoint,
    count: libc::c_int,
) -> libc::c_int {
    if count < 0 {
        return invalid_param_error("count");
    }
    if count > 1 && points.is_null() {
        return invalid_param_error("points");
    }
    for index in 0..count.saturating_sub(1) as isize {
        let start = *points.offset(index);
        let end = *points.offset(index + 1);
        let result = render_draw_line_f(renderer, start.x, start.y, end.x, end.y);
        if result < 0 {
            return result;
        }
    }
    0
}

pub(crate) unsafe fn render_draw_rect_f(
    renderer: *mut SDL_Renderer,
    rect: *const SDL_FRect,
) -> libc::c_int {
    match float_rect_to_int(rect) {
        Some(rect) => render_draw_rect(renderer, &rect),
        None => render_draw_rect(renderer, ptr::null()),
    }
}

pub(crate) unsafe fn render_draw_rects_f(
    renderer: *mut SDL_Renderer,
    rects: *const SDL_FRect,
    count: libc::c_int,
) -> libc::c_int {
    if count < 0 {
        return invalid_param_error("count");
    }
    if count > 0 && rects.is_null() {
        return invalid_param_error("rects");
    }
    for index in 0..count as isize {
        let rect = float_rect_to_int(rects.offset(index));
        let result = match rect {
            Some(rect) => render_draw_rect(renderer, &rect),
            None => render_draw_rect(renderer, ptr::null()),
        };
        if result < 0 {
            return result;
        }
    }
    0
}

pub(crate) unsafe fn render_fill_rect_f(
    renderer: *mut SDL_Renderer,
    rect: *const SDL_FRect,
) -> libc::c_int {
    match float_rect_to_int(rect) {
        Some(rect) => render_fill_rect(renderer, &rect),
        None => render_fill_rect(renderer, ptr::null()),
    }
}

pub(crate) unsafe fn render_fill_rects_f(
    renderer: *mut SDL_Renderer,
    rects: *const SDL_FRect,
    count: libc::c_int,
) -> libc::c_int {
    if count < 0 {
        return invalid_param_error("count");
    }
    if count > 0 && rects.is_null() {
        return invalid_param_error("rects");
    }
    for index in 0..count as isize {
        let rect = float_rect_to_int(rects.offset(index));
        let result = match rect {
            Some(rect) => render_fill_rect(renderer, &rect),
            None => render_fill_rect(renderer, ptr::null()),
        };
        if result < 0 {
            return result;
        }
    }
    0
}

pub(crate) unsafe fn render_copy_f(
    renderer: *mut SDL_Renderer,
    texture: *mut SDL_Texture,
    srcrect: *const SDL_Rect,
    dstrect: *const SDL_FRect,
) -> libc::c_int {
    match float_rect_to_int(dstrect) {
        Some(rect) => render_copy(renderer, texture, srcrect, &rect),
        None => render_copy(renderer, texture, srcrect, ptr::null()),
    }
}

pub(crate) unsafe fn render_copy_ex_f(
    renderer: *mut SDL_Renderer,
    texture: *mut SDL_Texture,
    srcrect: *const SDL_Rect,
    dstrect: *const SDL_FRect,
    angle: f64,
    center: *const SDL_FPoint,
    flip: Uint32,
) -> libc::c_int {
    let int_center = float_point_to_int(center);
    match float_rect_to_int(dstrect) {
        Some(rect) => render_copy_ex(
            renderer,
            texture,
            srcrect,
            &rect,
            angle,
            int_center
                .as_ref()
                .map(|point| point as *const SDL_Point)
                .unwrap_or(ptr::null()),
            flip,
        ),
        None => render_copy_ex(
            renderer,
            texture,
            srcrect,
            ptr::null(),
            angle,
            ptr::null(),
            flip,
        ),
    }
}

pub(crate) unsafe fn render_geometry(
    renderer: *mut SDL_Renderer,
    texture: *mut SDL_Texture,
    vertices: *const SDL_Vertex,
    num_vertices: libc::c_int,
    _indices: *const libc::c_int,
    _num_indices: libc::c_int,
) -> libc::c_int {
    if vertices.is_null() || num_vertices < 3 {
        return invalid_param_error("vertices");
    }
    if !texture.is_null() {
        return render_copy(renderer, texture, ptr::null(), ptr::null());
    }

    let clip = match current_draw_clip(renderer) {
        Some(clip) => clip,
        None => return 0,
    };
    let a = (*vertices.offset(0)).position;
    let b = (*vertices.offset(1)).position;
    let c = (*vertices.offset(2)).position;
    let min_x = a.x.min(b.x).min(c.x).floor() as libc::c_int;
    let max_x = a.x.max(b.x).max(c.x).ceil() as libc::c_int;
    let min_y = a.y.min(b.y).min(c.y).floor() as libc::c_int;
    let max_y = a.y.max(b.y).max(c.y).ceil() as libc::c_int;
    let color = (*vertices).color;
    let saved = (*renderer_ptr(renderer)).draw_color;
    (*renderer_ptr(renderer)).draw_color = (color.r, color.g, color.b, color.a);

    let area = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    if area.abs() <= f32::EPSILON {
        (*renderer_ptr(renderer)).draw_color = saved;
        return 0;
    }

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let w0 = (b.x - a.x) * (py - a.y) - (b.y - a.y) * (px - a.x);
            let w1 = (c.x - b.x) * (py - b.y) - (c.y - b.y) * (px - b.x);
            let w2 = (a.x - c.x) * (py - c.y) - (a.y - c.y) * (px - c.x);
            let inside = if area > 0.0 {
                w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0
            } else {
                w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0
            };
            if inside {
                let result = render_draw_point(renderer, x, y);
                if result < 0 {
                    (*renderer_ptr(renderer)).draw_color = saved;
                    return result;
                }
            }
        }
    }

    (*renderer_ptr(renderer)).draw_color = saved;
    let _ = clip;
    0
}

pub(crate) unsafe fn render_geometry_raw(
    _renderer: *mut SDL_Renderer,
    _texture: *mut SDL_Texture,
    _xy: *const f32,
    _xy_stride: libc::c_int,
    _color: *const crate::abi::generated_types::SDL_Color,
    _color_stride: libc::c_int,
    _uv: *const f32,
    _uv_stride: libc::c_int,
    _num_vertices: libc::c_int,
    _indices: *const libc::c_void,
    _num_indices: libc::c_int,
    _size_indices: libc::c_int,
) -> libc::c_int {
    set_error_message("RenderGeometryRaw is unsupported for the local renderer")
}

pub(crate) unsafe fn render_read_pixels(
    renderer: *mut SDL_Renderer,
    rect: *const SDL_Rect,
    format: Uint32,
    pixels: *mut libc::c_void,
    pitch: libc::c_int,
) -> libc::c_int {
    if pixels.is_null() {
        return invalid_param_error("pixels");
    }
    if pitch <= 0 {
        return invalid_param_error("pitch");
    }
    let surface = current_target_surface(renderer);
    let descriptor = match crate::video::surface::validate_surface_storage(surface) {
        Ok(descriptor) => descriptor,
        Err(error) => return crate::video::surface::apply_math_error(error),
    };
    let region = if rect.is_null() {
        crate::video::surface::full_surface_rect(surface)
    } else {
        *rect
    };
    let src = match crate::video::surface::pixel_pointer(
        surface,
        descriptor.bytes_per_pixel,
        region.x,
        region.y,
        "read pixels overflow",
    ) {
        Ok(ptr) => ptr,
        Err(error) => return crate::video::surface::apply_math_error(error),
    };
    crate::video::blit::SDL_ConvertPixels(
        region.w,
        region.h,
        (*(*surface).format).format,
        src.cast(),
        (*surface).pitch,
        format,
        pixels,
        pitch,
    )
}

pub(crate) unsafe fn render_present(_renderer: *mut SDL_Renderer) {}

pub(crate) unsafe fn render_get_metal_layer(_renderer: *mut SDL_Renderer) -> *mut libc::c_void {
    ptr::null_mut()
}

pub(crate) unsafe fn render_get_metal_command_encoder(
    _renderer: *mut SDL_Renderer,
) -> *mut libc::c_void {
    ptr::null_mut()
}

pub(crate) unsafe fn destroy_owned_textures(renderer: *mut SDL_Renderer) {
    let handles = active_local_textures();
    for handle in handles {
        let texture = handle as *mut SDL_Texture;
        if !is_local_texture(texture) {
            continue;
        }
        if (*texture_ptr(texture)).owner_renderer == renderer {
            destroy_texture(texture);
        }
    }
}

pub(crate) unsafe fn destroy_texture(texture: *mut SDL_Texture) {
    if !is_local_texture(texture) {
        return;
    }
    unregister_local_texture(texture);
    let mut local = Box::from_raw(texture_ptr(texture));
    if is_local_renderer(local.owner_renderer)
        && (*renderer_ptr(local.owner_renderer)).render_target == texture
    {
        (*renderer_ptr(local.owner_renderer)).render_target = ptr::null_mut();
    }
    crate::video::surface::SDL_FreeSurface(local.surface);
    local.surface = ptr::null_mut();
}

pub(crate) unsafe fn destroy_renderer(renderer: *mut SDL_Renderer) {
    if !is_local_renderer(renderer) {
        return;
    }
    destroy_owned_textures(renderer);
    let mut local = Box::from_raw(renderer_ptr(renderer));
    if local.owns_default_target && !local.default_target.is_null() {
        crate::video::surface::SDL_FreeSurface(local.default_target);
        local.default_target = ptr::null_mut();
    }
}

pub(crate) unsafe fn render_flush(_renderer: *mut SDL_Renderer) -> libc::c_int {
    0
}

pub(crate) unsafe fn render_set_vsync(
    _renderer: *mut SDL_Renderer,
    _vsync: libc::c_int,
) -> libc::c_int {
    0
}

pub(crate) unsafe fn gl_bind_texture(
    _texture: *mut SDL_Texture,
    _texw: *mut f32,
    _texh: *mut f32,
) -> libc::c_int {
    set_error_message("OpenGL texture binding requires the host renderer")
}

pub(crate) unsafe fn gl_unbind_texture(_texture: *mut SDL_Texture) -> libc::c_int {
    set_error_message("OpenGL texture binding requires the host renderer")
}
