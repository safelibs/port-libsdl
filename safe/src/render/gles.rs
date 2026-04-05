use std::sync::{Mutex, OnceLock};

use crate::abi::generated_types::{
    SDL_GLContext, SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGRA32,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGRX32, SDL_PixelFormatEnum_SDL_PIXELFORMAT_EXTERNAL_OES,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_IYUV, SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV12,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV21, SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBA32,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBX32, SDL_PixelFormatEnum_SDL_PIXELFORMAT_YV12,
    SDL_Renderer, SDL_ScaleMode, SDL_ScaleMode_SDL_ScaleModeNearest,
    SDL_TextureAccess_SDL_TEXTUREACCESS_STATIC, SDL_TextureAccess_SDL_TEXTUREACCESS_STREAMING,
    SDL_Window, Uint32,
};

const GL_NO_ERROR: u32 = 0;
const GL_TEXTURE_2D: u32 = 0x0DE1;
const GL_TEXTURE_EXTERNAL_OES: u32 = 0x8D65;
const GL_TEXTURE0: u32 = 0x84C0;
const GL_TEXTURE1: u32 = GL_TEXTURE0 + 1;
const GL_TEXTURE2: u32 = GL_TEXTURE0 + 2;
const GL_TEXTURE_MIN_FILTER: u32 = 0x2801;
const GL_TEXTURE_MAG_FILTER: u32 = 0x2800;
const GL_TEXTURE_WRAP_S: u32 = 0x2802;
const GL_TEXTURE_WRAP_T: u32 = 0x2803;
const GL_NEAREST: i32 = 0x2600;
const GL_LINEAR: i32 = 0x2601;
const GL_CLAMP_TO_EDGE: i32 = 0x812F;
const GL_UNSIGNED_BYTE: u32 = 0x1401;
const GL_RGBA: u32 = 0x1908;
const GL_LUMINANCE: u32 = 0x1909;
const GL_LUMINANCE_ALPHA: u32 = 0x190A;

#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TextureLifecycleCounters {
    pub created_textures: u32,
    pub destroyed_textures: u32,
    pub allocated_pixel_buffers: u32,
    pub freed_pixel_buffers: u32,
}

#[derive(Debug, Default)]
struct TrackerState {
    fail_after: Option<usize>,
    current_step: usize,
    counters: TextureLifecycleCounters,
}

fn tracker() -> &'static Mutex<TrackerState> {
    static TRACKER: OnceLock<Mutex<TrackerState>> = OnceLock::new();
    TRACKER.get_or_init(|| Mutex::new(TrackerState::default()))
}

fn reset_tracker_steps() {
    let mut state = tracker()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    state.current_step = 0;
}

fn failure_injection_enabled() -> bool {
    tracker()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .fail_after
        .is_some()
}

fn inject_failure_if_requested(label: &str) -> Result<(), String> {
    let mut state = tracker()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    state.current_step += 1;
    if state.fail_after == Some(state.current_step) {
        return Err(format!(
            "injected GLES texture creation failure at step {} ({label})",
            state.current_step
        ));
    }
    Ok(())
}

fn note_texture_created() {
    let mut state = tracker()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    state.counters.created_textures += 1;
}

fn note_texture_destroyed() {
    let mut state = tracker()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    state.counters.destroyed_textures += 1;
}

fn note_pixel_buffer_allocated() {
    let mut state = tracker()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    state.counters.allocated_pixel_buffers += 1;
}

fn note_pixel_buffer_freed() {
    let mut state = tracker()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    state.counters.freed_pixel_buffers += 1;
}

#[doc(hidden)]
pub fn reset_texture_lifecycle_counters() {
    let mut state = tracker()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    state.current_step = 0;
    state.fail_after = None;
    state.counters = TextureLifecycleCounters::default();
}

#[doc(hidden)]
pub fn texture_lifecycle_counters() -> TextureLifecycleCounters {
    tracker()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .counters
}

#[doc(hidden)]
pub fn set_texture_creation_failure_step_for_test(step: Option<usize>) {
    let mut state = tracker()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    state.fail_after = step;
    state.current_step = 0;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GlesTextureKind {
    Rgba,
    YuvPlanar,
    Nv12,
    ExternalOes,
}

#[doc(hidden)]
pub fn texture_creation_step_count_for_test(format: Uint32, access: libc::c_int) -> usize {
    let Some(kind) = texture_kind(format) else {
        return 0;
    };
    let mut steps = if access == SDL_TextureAccess_SDL_TEXTUREACCESS_STREAMING as libc::c_int {
        1
    } else {
        0
    };
    steps += 1;
    match kind {
        GlesTextureKind::YuvPlanar => steps + 2,
        GlesTextureKind::Nv12 => steps + 1,
        GlesTextureKind::ExternalOes | GlesTextureKind::Rgba => steps,
    }
}

type GetProcAddressFn = unsafe extern "C" fn(*const libc::c_char) -> *mut libc::c_void;
type GetCurrentContextFn = unsafe extern "C" fn() -> SDL_GLContext;
type MakeCurrentFn = unsafe extern "C" fn(*mut SDL_Window, SDL_GLContext) -> libc::c_int;

#[derive(Clone, Copy)]
struct GlEnvironment {
    get_proc_address: GetProcAddressFn,
    get_current_context: GetCurrentContextFn,
    make_current: MakeCurrentFn,
}

fn gl_environment() -> &'static GlEnvironment {
    static ENV: OnceLock<GlEnvironment> = OnceLock::new();
    ENV.get_or_init(|| GlEnvironment {
        get_proc_address: crate::video::load_symbol(b"SDL_GL_GetProcAddress\0"),
        get_current_context: crate::video::load_symbol(b"SDL_GL_GetCurrentContext\0"),
        make_current: crate::video::load_symbol(b"SDL_GL_MakeCurrent\0"),
    })
}

type GlGenTexturesFn = unsafe extern "C" fn(libc::c_int, *mut u32);
type GlDeleteTexturesFn = unsafe extern "C" fn(libc::c_int, *const u32);
type GlActiveTextureFn = unsafe extern "C" fn(u32);
type GlBindTextureFn = unsafe extern "C" fn(u32, u32);
type GlTexParameteriFn = unsafe extern "C" fn(u32, u32, libc::c_int);
type GlTexImage2DFn = unsafe extern "C" fn(
    u32,
    libc::c_int,
    libc::c_int,
    libc::c_int,
    libc::c_int,
    libc::c_int,
    u32,
    u32,
    *const libc::c_void,
);
type GlGetErrorFn = unsafe extern "C" fn() -> u32;

#[derive(Clone, Copy)]
struct GlFunctions {
    gen_textures: GlGenTexturesFn,
    delete_textures: GlDeleteTexturesFn,
    active_texture: GlActiveTextureFn,
    bind_texture: GlBindTextureFn,
    tex_parameteri: GlTexParameteriFn,
    tex_image_2d: GlTexImage2DFn,
    get_error: GlGetErrorFn,
}

unsafe fn load_gl_proc<T>(name: &[u8]) -> Result<T, String> {
    let proc_address = (gl_environment().get_proc_address)(name.as_ptr().cast());
    if proc_address.is_null() {
        return Err(format!(
            "SDL_GL_GetProcAddress({}) failed",
            String::from_utf8_lossy(&name[..name.len().saturating_sub(1)])
        ));
    }
    Ok(std::mem::transmute_copy(&proc_address))
}

unsafe fn load_gl_functions() -> Result<GlFunctions, String> {
    Ok(GlFunctions {
        gen_textures: load_gl_proc(b"glGenTextures\0")?,
        delete_textures: load_gl_proc(b"glDeleteTextures\0")?,
        active_texture: load_gl_proc(b"glActiveTexture\0")?,
        bind_texture: load_gl_proc(b"glBindTexture\0")?,
        tex_parameteri: load_gl_proc(b"glTexParameteri\0")?,
        tex_image_2d: load_gl_proc(b"glTexImage2D\0")?,
        get_error: load_gl_proc(b"glGetError\0")?,
    })
}

struct PixelBuffer {
    ptr: *mut libc::c_void,
}

impl PixelBuffer {
    unsafe fn allocate(size: usize) -> Result<Option<Self>, String> {
        if size == 0 {
            return Ok(None);
        }
        inject_failure_if_requested("pixel buffer allocation")?;
        let ptr = libc::calloc(1, size);
        if ptr.is_null() {
            return Err("Out of memory".to_string());
        }
        note_pixel_buffer_allocated();
        Ok(Some(Self { ptr }))
    }
}

impl Drop for PixelBuffer {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                libc::free(self.ptr);
            }
            note_pixel_buffer_freed();
            self.ptr = std::ptr::null_mut();
        }
    }
}

pub(crate) struct ShadowTexture {
    window: *mut SDL_Window,
    context: SDL_GLContext,
    gl: GlFunctions,
    texture_type: u32,
    texture: u32,
    texture_u: u32,
    texture_v: u32,
    pixel_buffer: Option<PixelBuffer>,
}

impl ShadowTexture {
    unsafe fn create(
        window: *mut SDL_Window,
        format: Uint32,
        access: libc::c_int,
        scale_mode: SDL_ScaleMode,
        w: libc::c_int,
        h: libc::c_int,
    ) -> Result<Self, String> {
        let Some(kind) = texture_kind(format) else {
            return Err("Texture format not supported".to_string());
        };
        if format == SDL_PixelFormatEnum_SDL_PIXELFORMAT_EXTERNAL_OES
            && access != SDL_TextureAccess_SDL_TEXTUREACCESS_STATIC as libc::c_int
        {
            return Err("Unsupported texture access for SDL_PIXELFORMAT_EXTERNAL_OES".to_string());
        }

        let gl = load_gl_functions()?;
        let context = (gl_environment().get_current_context)();
        if context.is_null() {
            return Err("No current GLES context is available for texture creation".to_string());
        }

        let mut shadow = Self {
            window,
            context,
            gl,
            texture_type: if kind == GlesTextureKind::ExternalOes {
                GL_TEXTURE_EXTERNAL_OES
            } else {
                GL_TEXTURE_2D
            },
            texture: 0,
            texture_u: 0,
            texture_v: 0,
            pixel_buffer: None,
        };

        reset_tracker_steps();
        shadow.pixel_buffer = PixelBuffer::allocate(pixel_buffer_size(kind, access, w, h))?;

        let gl_scale_mode = if scale_mode == SDL_ScaleMode_SDL_ScaleModeNearest {
            GL_NEAREST
        } else {
            GL_LINEAR
        };

        match kind {
            GlesTextureKind::Rgba => {
                shadow.texture = shadow.allocate_texture(
                    GL_TEXTURE0,
                    GL_RGBA as libc::c_int,
                    GL_RGBA,
                    GL_UNSIGNED_BYTE,
                    w,
                    h,
                    gl_scale_mode,
                    true,
                )?;
            }
            GlesTextureKind::YuvPlanar => {
                shadow.texture_v = shadow.allocate_texture(
                    GL_TEXTURE2,
                    GL_LUMINANCE as libc::c_int,
                    GL_LUMINANCE,
                    GL_UNSIGNED_BYTE,
                    (w + 1) / 2,
                    (h + 1) / 2,
                    gl_scale_mode,
                    true,
                )?;
                shadow.texture_u = shadow.allocate_texture(
                    GL_TEXTURE1,
                    GL_LUMINANCE as libc::c_int,
                    GL_LUMINANCE,
                    GL_UNSIGNED_BYTE,
                    (w + 1) / 2,
                    (h + 1) / 2,
                    gl_scale_mode,
                    true,
                )?;
                shadow.texture = shadow.allocate_texture(
                    GL_TEXTURE0,
                    GL_LUMINANCE as libc::c_int,
                    GL_LUMINANCE,
                    GL_UNSIGNED_BYTE,
                    w,
                    h,
                    gl_scale_mode,
                    true,
                )?;
            }
            GlesTextureKind::Nv12 => {
                shadow.texture_u = shadow.allocate_texture(
                    GL_TEXTURE1,
                    GL_LUMINANCE_ALPHA as libc::c_int,
                    GL_LUMINANCE_ALPHA,
                    GL_UNSIGNED_BYTE,
                    (w + 1) / 2,
                    (h + 1) / 2,
                    gl_scale_mode,
                    true,
                )?;
                shadow.texture = shadow.allocate_texture(
                    GL_TEXTURE0,
                    GL_LUMINANCE as libc::c_int,
                    GL_LUMINANCE,
                    GL_UNSIGNED_BYTE,
                    w,
                    h,
                    gl_scale_mode,
                    true,
                )?;
            }
            GlesTextureKind::ExternalOes => {
                shadow.texture = shadow.allocate_texture(
                    GL_TEXTURE0,
                    GL_RGBA as libc::c_int,
                    GL_RGBA,
                    GL_UNSIGNED_BYTE,
                    w,
                    h,
                    gl_scale_mode,
                    false,
                )?;
            }
        }

        (shadow.gl.active_texture)(GL_TEXTURE0);
        (shadow.gl.bind_texture)(shadow.texture_type, 0);
        Ok(shadow)
    }

    unsafe fn allocate_texture(
        &self,
        unit: u32,
        internal_format: libc::c_int,
        format: u32,
        pixel_type: u32,
        width: libc::c_int,
        height: libc::c_int,
        scale_mode: libc::c_int,
        allocate_image: bool,
    ) -> Result<u32, String> {
        inject_failure_if_requested("glGenTextures")?;

        while (self.gl.get_error)() != GL_NO_ERROR {}

        let mut texture = 0u32;
        (self.gl.gen_textures)(1, &mut texture);
        let mut error = (self.gl.get_error)();
        if error != GL_NO_ERROR || texture == 0 {
            if error == GL_NO_ERROR {
                error = 1;
            }
            return Err(format!("glGenTextures failed with error 0x{error:X}"));
        }
        note_texture_created();

        (self.gl.active_texture)(unit);
        (self.gl.bind_texture)(self.texture_type, texture);
        (self.gl.tex_parameteri)(self.texture_type, GL_TEXTURE_MIN_FILTER, scale_mode);
        (self.gl.tex_parameteri)(self.texture_type, GL_TEXTURE_MAG_FILTER, scale_mode);
        (self.gl.tex_parameteri)(self.texture_type, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
        (self.gl.tex_parameteri)(self.texture_type, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
        if allocate_image {
            (self.gl.tex_image_2d)(
                self.texture_type,
                0,
                internal_format,
                width,
                height,
                0,
                format,
                pixel_type,
                std::ptr::null(),
            );
        }

        error = (self.gl.get_error)();
        if error != GL_NO_ERROR {
            (self.gl.delete_textures)(1, &texture);
            note_texture_destroyed();
            return Err(format!("glTexImage2D failed with error 0x{error:X}"));
        }

        Ok(texture)
    }

    unsafe fn make_context_current(&self) {
        if self.context.is_null() || self.window.is_null() {
            return;
        }
        let current = (gl_environment().get_current_context)();
        if current != self.context {
            let _ = (gl_environment().make_current)(self.window, self.context);
        }
    }
}

impl Drop for ShadowTexture {
    fn drop(&mut self) {
        unsafe {
            self.make_context_current();
            let gl = self.gl;
            delete_texture_id(gl, &mut self.texture);
            delete_texture_id(gl, &mut self.texture_u);
            delete_texture_id(gl, &mut self.texture_v);
        }
    }
}

unsafe fn delete_texture_id(gl: GlFunctions, texture: &mut u32) {
    if *texture != 0 {
        (gl.delete_textures)(1, texture);
        note_texture_destroyed();
        *texture = 0;
    }
}

fn texture_kind(format: Uint32) -> Option<GlesTextureKind> {
    match format {
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGRA32
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBA32
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_BGRX32
        | SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBX32 => Some(GlesTextureKind::Rgba),
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_IYUV | SDL_PixelFormatEnum_SDL_PIXELFORMAT_YV12 => {
            Some(GlesTextureKind::YuvPlanar)
        }
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV12 | SDL_PixelFormatEnum_SDL_PIXELFORMAT_NV21 => {
            Some(GlesTextureKind::Nv12)
        }
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_EXTERNAL_OES => Some(GlesTextureKind::ExternalOes),
        _ => None,
    }
}

fn pixel_buffer_size(
    kind: GlesTextureKind,
    access: libc::c_int,
    w: libc::c_int,
    h: libc::c_int,
) -> usize {
    if access != SDL_TextureAccess_SDL_TEXTUREACCESS_STREAMING as libc::c_int || w <= 0 || h <= 0 {
        return 0;
    }

    let w = w as usize;
    let h = h as usize;
    match kind {
        GlesTextureKind::Rgba => w.saturating_mul(h).saturating_mul(4),
        GlesTextureKind::YuvPlanar | GlesTextureKind::Nv12 => {
            let pitch = w;
            let chroma_h = (h + 1) / 2;
            let chroma_pitch = (pitch + 1) / 2;
            h.saturating_mul(pitch)
                .saturating_add(2usize.saturating_mul(chroma_h.saturating_mul(chroma_pitch)))
        }
        GlesTextureKind::ExternalOes => 0,
    }
}

pub(crate) unsafe fn renderer_uses_gles_texture_path(renderer: *mut SDL_Renderer) -> bool {
    let Some(name) = crate::render::software::renderer_name(renderer) else {
        return false;
    };
    let name = name.to_ascii_lowercase();
    matches!(name.as_str(), "opengles" | "opengles2" | "gles" | "gles2")
        || name.contains("opengl es")
}

pub(crate) unsafe fn prepare_texture_shadow(
    renderer: *mut SDL_Renderer,
    window: *mut SDL_Window,
    format: Uint32,
    access: libc::c_int,
    scale_mode: SDL_ScaleMode,
    w: libc::c_int,
    h: libc::c_int,
) -> Result<Option<ShadowTexture>, String> {
    if !renderer_uses_gles_texture_path(renderer) {
        return Ok(None);
    }
    if texture_kind(format).is_none() {
        return Ok(None);
    }

    match ShadowTexture::create(window, format, access, scale_mode, w, h) {
        Ok(shadow) => Ok(Some(shadow)),
        Err(err) if !failure_injection_enabled() && err.contains("SDL_GL_GetProcAddress") => {
            Ok(None)
        }
        Err(err) => Err(err),
    }
}
