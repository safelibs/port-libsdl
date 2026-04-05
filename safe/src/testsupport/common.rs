use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;

use crate::abi::generated_types::{
    self as sdl, SDL_AudioSpec, SDL_Event, SDL_EventType_SDL_DROPFILE, SDL_EventType_SDL_DROPTEXT,
    SDL_EventType_SDL_KEYDOWN, SDL_EventType_SDL_QUIT, SDL_EventType_SDL_WINDOWEVENT,
    SDL_GLattr_SDL_GL_ACCELERATED_VISUAL, SDL_GLattr_SDL_GL_ALPHA_SIZE,
    SDL_GLattr_SDL_GL_BLUE_SIZE, SDL_GLattr_SDL_GL_BUFFER_SIZE, SDL_GLattr_SDL_GL_CONTEXT_FLAGS,
    SDL_GLattr_SDL_GL_CONTEXT_MAJOR_VERSION, SDL_GLattr_SDL_GL_CONTEXT_MINOR_VERSION,
    SDL_GLattr_SDL_GL_CONTEXT_PROFILE_MASK, SDL_GLattr_SDL_GL_DEPTH_SIZE,
    SDL_GLattr_SDL_GL_DOUBLEBUFFER, SDL_GLattr_SDL_GL_GREEN_SIZE,
    SDL_GLattr_SDL_GL_MULTISAMPLEBUFFERS, SDL_GLattr_SDL_GL_MULTISAMPLESAMPLES,
    SDL_GLattr_SDL_GL_RED_SIZE, SDL_GLattr_SDL_GL_STENCIL_SIZE, SDL_KeyCode_SDLK_ESCAPE,
    SDL_KeyCode_SDLK_q, SDL_LogCategory_SDL_LOG_CATEGORY_AUDIO,
    SDL_LogCategory_SDL_LOG_CATEGORY_ERROR, SDL_LogCategory_SDL_LOG_CATEGORY_INPUT,
    SDL_LogCategory_SDL_LOG_CATEGORY_RENDER, SDL_LogCategory_SDL_LOG_CATEGORY_SYSTEM,
    SDL_LogCategory_SDL_LOG_CATEGORY_VIDEO, SDL_LogPriority_SDL_LOG_PRIORITY_VERBOSE,
    SDL_RendererInfo, SDL_WindowEventID_SDL_WINDOWEVENT_CLOSE,
    SDL_WindowFlags_SDL_WINDOW_ALLOW_HIGHDPI, SDL_WindowFlags_SDL_WINDOW_BORDERLESS,
    SDL_WindowFlags_SDL_WINDOW_FULLSCREEN, SDL_WindowFlags_SDL_WINDOW_FULLSCREEN_DESKTOP,
    SDL_WindowFlags_SDL_WINDOW_HIDDEN, SDL_WindowFlags_SDL_WINDOW_INPUT_FOCUS,
    SDL_WindowFlags_SDL_WINDOW_INPUT_GRABBED, SDL_WindowFlags_SDL_WINDOW_KEYBOARD_GRABBED,
    SDL_WindowFlags_SDL_WINDOW_MAXIMIZED, SDL_WindowFlags_SDL_WINDOW_METAL,
    SDL_WindowFlags_SDL_WINDOW_MINIMIZED, SDL_WindowFlags_SDL_WINDOW_MOUSE_FOCUS,
    SDL_WindowFlags_SDL_WINDOW_OPENGL, SDL_WindowFlags_SDL_WINDOW_RESIZABLE,
    SDL_WindowFlags_SDL_WINDOW_VULKAN, SDL_bool, SDL_bool_SDL_FALSE, SDL_bool_SDL_TRUE,
    SDL_HINT_AUDIODRIVER, SDL_HINT_VIDEODRIVER, SDL_INIT_AUDIO, SDL_INIT_VIDEO,
};
use crate::testsupport::{
    c_string, lock_usage_cache, windowpos_centered, windowpos_undefined, SDLTest_CommonState,
    FONT_LINE_HEIGHT, VERBOSE_EVENT, VERBOSE_MODES, VERBOSE_MOTION, VERBOSE_RENDER, VERBOSE_VIDEO,
};

const VIDEO_USAGE: &[&str] = &[
    "[--video driver]",
    "[--renderer driver]",
    "[--gldebug]",
    "[--info all|video|modes|render|event|event_motion]",
    "[--log all|error|system|audio|video|render|input]",
    "[--display N]",
    "[--metal-window | --opengl-window | --vulkan-window]",
    "[--fullscreen | --fullscreen-desktop | --windows N]",
    "[--title title]",
    "[--icon icon.bmp]",
    "[--center | --position X,Y]",
    "[--geometry WxH]",
    "[--min-geometry WxH]",
    "[--max-geometry WxH]",
    "[--logical WxH]",
    "[--scale N]",
    "[--depth N]",
    "[--refresh R]",
    "[--vsync]",
    "[--noframe]",
    "[--resizable]",
    "[--minimize]",
    "[--maximize]",
    "[--grab]",
    "[--keyboard-grab]",
    "[--shown]",
    "[--hidden]",
    "[--input-focus]",
    "[--mouse-focus]",
    "[--flash-on-focus-loss]",
    "[--allow-highdpi]",
    "[--confine-cursor X,Y,W,H]",
    "[--usable-bounds]",
];

const AUDIO_USAGE: &[&str] = &[
    "[--audio driver]",
    "[--rate N]",
    "[--format U8|S8|U16|U16LE|U16BE|S16|S16LE|S16BE]",
    "[--channels N]",
    "[--samples N]",
];

fn parse_pair(text: &str, separator: char) -> Option<(i32, i32)> {
    let (lhs, rhs) = text.split_once(separator)?;
    Some((lhs.trim().parse().ok()?, rhs.trim().parse().ok()?))
}

unsafe fn render_usage(
    state: *mut SDLTest_CommonState,
    argv0: *const c_char,
    options: *const *const c_char,
) -> *const c_char {
    let mut lines = Vec::new();
    let app = if argv0.is_null() {
        "app".to_string()
    } else {
        c_string(argv0)
    };
    lines.push(format!("Usage: {app} [--trackmem]"));
    if (*state).flags & SDL_INIT_VIDEO != 0 {
        lines.push(VIDEO_USAGE.join(" "));
    }
    if (*state).flags & SDL_INIT_AUDIO != 0 {
        lines.push(AUDIO_USAGE.join(" "));
    }
    if !options.is_null() {
        let mut index = 0usize;
        while !(*options.add(index)).is_null() {
            lines.push(c_string(*options.add(index)));
            index += 1;
        }
    }
    let mut bytes = lines.join("\n").into_bytes();
    bytes.push(0);
    let ptr_value = state as usize;
    let mut cache = lock_usage_cache();
    cache.insert(ptr_value, bytes);
    cache[&ptr_value].as_ptr().cast()
}

unsafe fn renderer_index_by_name(name: *const c_char) -> c_int {
    if name.is_null() {
        return -1;
    }
    let requested = CStr::from_ptr(name).to_string_lossy();
    let count = sdl::SDL_GetNumRenderDrivers();
    for index in 0..count {
        let mut info = std::mem::zeroed::<SDL_RendererInfo>();
        if sdl::SDL_GetRenderDriverInfo(index, &mut info) == 0
            && !info.name.is_null()
            && CStr::from_ptr(info.name).to_string_lossy() == requested
        {
            return index;
        }
    }
    -1
}

unsafe fn cleanup_windows(state: *mut SDLTest_CommonState) {
    if !(*state).targets.is_null() {
        for index in 0..(*state).num_windows.max(0) as usize {
            let texture = *(*state).targets.add(index);
            if !texture.is_null() {
                sdl::SDL_DestroyTexture(texture);
            }
        }
        sdl::SDL_free((*state).targets.cast());
        (*state).targets = ptr::null_mut();
    }
    if !(*state).renderers.is_null() {
        for index in 0..(*state).num_windows.max(0) as usize {
            let renderer = *(*state).renderers.add(index);
            if !renderer.is_null() {
                sdl::SDL_DestroyRenderer(renderer);
            }
        }
        sdl::SDL_free((*state).renderers.cast());
        (*state).renderers = ptr::null_mut();
    }
    if !(*state).windows.is_null() {
        for index in 0..(*state).num_windows.max(0) as usize {
            let window = *(*state).windows.add(index);
            if !window.is_null() {
                sdl::SDL_DestroyWindow(window);
            }
        }
        sdl::SDL_free((*state).windows.cast());
        (*state).windows = ptr::null_mut();
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_CommonCreateState(
    argv: *mut *mut c_char,
    flags: u32,
) -> *mut SDLTest_CommonState {
    let mut index = 1usize;
    while !crate::testsupport::argv_at(argv, index).is_null() {
        let value = c_string(crate::testsupport::argv_at(argv, index));
        if value.eq_ignore_ascii_case("--trackmem") {
            crate::testsupport::memory::SDLTest_TrackAllocations();
            break;
        }
        index += 1;
    }

    let title = crate::testsupport::argv_at(argv, 0);
    Box::into_raw(Box::new(SDLTest_CommonState {
        argv,
        flags,
        verbose: 0,
        videodriver: ptr::null(),
        display: 0,
        window_title: if title.is_null() {
            b"sdl-test\0".as_ptr().cast()
        } else {
            title
        },
        window_icon: ptr::null(),
        window_flags: 0,
        flash_on_focus_loss: SDL_bool_SDL_FALSE,
        window_x: windowpos_undefined(),
        window_y: windowpos_undefined(),
        window_w: 640,
        window_h: 480,
        window_minW: 0,
        window_minH: 0,
        window_maxW: 0,
        window_maxH: 0,
        logical_w: 0,
        logical_h: 0,
        scale: 1.0,
        depth: 0,
        refresh_rate: 0,
        num_windows: 1,
        windows: ptr::null_mut(),
        renderdriver: ptr::null(),
        render_flags: 0,
        skip_renderer: SDL_bool_SDL_FALSE,
        renderers: ptr::null_mut(),
        targets: ptr::null_mut(),
        audiodriver: ptr::null(),
        audiospec: SDL_AudioSpec {
            freq: 22_050,
            format: sdl::AUDIO_S16 as u16,
            channels: 2,
            silence: 0,
            samples: 2048,
            padding: 0,
            size: 0,
            callback: None,
            userdata: ptr::null_mut(),
        },
        gl_red_size: 3,
        gl_green_size: 3,
        gl_blue_size: 2,
        gl_alpha_size: 0,
        gl_buffer_size: 0,
        gl_depth_size: 16,
        gl_stencil_size: 0,
        gl_double_buffer: 1,
        gl_accum_red_size: 0,
        gl_accum_green_size: 0,
        gl_accum_blue_size: 0,
        gl_accum_alpha_size: 0,
        gl_stereo: 0,
        gl_multisamplebuffers: 0,
        gl_multisamplesamples: 0,
        gl_retained_backing: 1,
        gl_accelerated: -1,
        gl_major_version: 0,
        gl_minor_version: 0,
        gl_debug: 0,
        gl_profile_mask: 0,
        confine: sdl::SDL_Rect {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        },
    }))
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_CommonArg(state: *mut SDLTest_CommonState, index: c_int) -> c_int {
    if state.is_null() {
        return -1;
    }
    let argv = (*state).argv;
    let arg = crate::testsupport::argv_at(argv, index as usize);
    if arg.is_null() {
        return -1;
    }
    let value = c_string(arg);
    let next = crate::testsupport::argv_at(argv, index as usize + 1);
    let next_string = (!next.is_null()).then(|| c_string(next));

    match value.as_str() {
        "--trackmem" => return 1,
        "--video" => {
            let Some(_driver) = next_string else {
                return -1;
            };
            (*state).videodriver = next;
            sdl::SDL_SetHint(SDL_HINT_VIDEODRIVER.as_ptr().cast(), next);
            return 2;
        }
        "--audio" => {
            let Some(_driver) = next_string else {
                return -1;
            };
            (*state).audiodriver = next;
            sdl::SDL_SetHint(SDL_HINT_AUDIODRIVER.as_ptr().cast(), next);
            return 2;
        }
        "--renderer" => {
            if next.is_null() {
                return -1;
            }
            (*state).renderdriver = next;
            return 2;
        }
        "--gldebug" => {
            (*state).gl_debug = 1;
            return 1;
        }
        "--info" => {
            let Some(mode) = next_string else {
                return -1;
            };
            match mode.as_str() {
                "all" => {
                    (*state).verbose |=
                        VERBOSE_VIDEO | VERBOSE_MODES | VERBOSE_RENDER | VERBOSE_EVENT;
                    return 2;
                }
                "video" => (*state).verbose |= VERBOSE_VIDEO,
                "modes" => (*state).verbose |= VERBOSE_MODES,
                "render" => (*state).verbose |= VERBOSE_RENDER,
                "event" => (*state).verbose |= VERBOSE_EVENT,
                "event_motion" => (*state).verbose |= VERBOSE_EVENT | VERBOSE_MOTION,
                _ => return -1,
            }
            return 2;
        }
        "--log" => {
            let Some(channel) = next_string else {
                return -1;
            };
            match channel.as_str() {
                "all" => sdl::SDL_LogSetAllPriority(SDL_LogPriority_SDL_LOG_PRIORITY_VERBOSE),
                "error" => sdl::SDL_LogSetPriority(
                    SDL_LogCategory_SDL_LOG_CATEGORY_ERROR as c_int,
                    SDL_LogPriority_SDL_LOG_PRIORITY_VERBOSE,
                ),
                "system" => sdl::SDL_LogSetPriority(
                    SDL_LogCategory_SDL_LOG_CATEGORY_SYSTEM as c_int,
                    SDL_LogPriority_SDL_LOG_PRIORITY_VERBOSE,
                ),
                "audio" => sdl::SDL_LogSetPriority(
                    SDL_LogCategory_SDL_LOG_CATEGORY_AUDIO as c_int,
                    SDL_LogPriority_SDL_LOG_PRIORITY_VERBOSE,
                ),
                "video" => sdl::SDL_LogSetPriority(
                    SDL_LogCategory_SDL_LOG_CATEGORY_VIDEO as c_int,
                    SDL_LogPriority_SDL_LOG_PRIORITY_VERBOSE,
                ),
                "render" => sdl::SDL_LogSetPriority(
                    SDL_LogCategory_SDL_LOG_CATEGORY_RENDER as c_int,
                    SDL_LogPriority_SDL_LOG_PRIORITY_VERBOSE,
                ),
                "input" => sdl::SDL_LogSetPriority(
                    SDL_LogCategory_SDL_LOG_CATEGORY_INPUT as c_int,
                    SDL_LogPriority_SDL_LOG_PRIORITY_VERBOSE,
                ),
                _ => return -1,
            }
            return 2;
        }
        "--display" => {
            let Some(number) = next_string else {
                return -1;
            };
            (*state).display = number.parse().unwrap_or(0);
            return 2;
        }
        "--windows" => {
            let Some(number) = next_string else {
                return -1;
            };
            (*state).num_windows = number.parse().unwrap_or(1).max(0);
            return 2;
        }
        "--title" => {
            if next.is_null() {
                return -1;
            }
            (*state).window_title = next;
            return 2;
        }
        "--icon" => {
            if next.is_null() {
                return -1;
            }
            (*state).window_icon = next;
            return 2;
        }
        "--center" => {
            (*state).window_x = windowpos_centered();
            (*state).window_y = windowpos_centered();
            return 1;
        }
        "--position" => {
            let Some(position) = next_string else {
                return -1;
            };
            let Some((x, y)) = parse_pair(&position, ',') else {
                return -1;
            };
            (*state).window_x = x;
            (*state).window_y = y;
            return 2;
        }
        "--geometry" => {
            let Some(geometry) = next_string else {
                return -1;
            };
            let Some((w, h)) = parse_pair(&geometry, 'x') else {
                return -1;
            };
            (*state).window_w = w;
            (*state).window_h = h;
            return 2;
        }
        "--min-geometry" => {
            let Some(geometry) = next_string else {
                return -1;
            };
            let Some((w, h)) = parse_pair(&geometry, 'x') else {
                return -1;
            };
            (*state).window_minW = w;
            (*state).window_minH = h;
            return 2;
        }
        "--max-geometry" => {
            let Some(geometry) = next_string else {
                return -1;
            };
            let Some((w, h)) = parse_pair(&geometry, 'x') else {
                return -1;
            };
            (*state).window_maxW = w;
            (*state).window_maxH = h;
            return 2;
        }
        "--logical" => {
            let Some(geometry) = next_string else {
                return -1;
            };
            let Some((w, h)) = parse_pair(&geometry, 'x') else {
                return -1;
            };
            (*state).logical_w = w;
            (*state).logical_h = h;
            return 2;
        }
        "--scale" => {
            let Some(scale) = next_string else {
                return -1;
            };
            (*state).scale = scale.parse().unwrap_or(1.0);
            return 2;
        }
        "--depth" => {
            let Some(depth) = next_string else {
                return -1;
            };
            (*state).depth = depth.parse().unwrap_or(0);
            return 2;
        }
        "--refresh" => {
            let Some(refresh) = next_string else {
                return -1;
            };
            (*state).refresh_rate = refresh.parse().unwrap_or(0);
            return 2;
        }
        "--fullscreen" => {
            (*state).window_flags |= SDL_WindowFlags_SDL_WINDOW_FULLSCREEN;
            return 1;
        }
        "--fullscreen-desktop" => {
            (*state).window_flags |= SDL_WindowFlags_SDL_WINDOW_FULLSCREEN_DESKTOP;
            return 1;
        }
        "--opengl-window" => {
            (*state).window_flags |= SDL_WindowFlags_SDL_WINDOW_OPENGL;
            return 1;
        }
        "--vulkan-window" => {
            (*state).window_flags |= SDL_WindowFlags_SDL_WINDOW_VULKAN;
            return 1;
        }
        "--metal-window" => {
            (*state).window_flags |= SDL_WindowFlags_SDL_WINDOW_METAL;
            return 1;
        }
        "--noframe" => {
            (*state).window_flags |= SDL_WindowFlags_SDL_WINDOW_BORDERLESS;
            return 1;
        }
        "--resizable" => {
            (*state).window_flags |= SDL_WindowFlags_SDL_WINDOW_RESIZABLE;
            return 1;
        }
        "--minimize" => {
            (*state).window_flags |= SDL_WindowFlags_SDL_WINDOW_MINIMIZED;
            return 1;
        }
        "--maximize" => {
            (*state).window_flags |= SDL_WindowFlags_SDL_WINDOW_MAXIMIZED;
            return 1;
        }
        "--grab" => {
            (*state).window_flags |= SDL_WindowFlags_SDL_WINDOW_INPUT_GRABBED;
            return 1;
        }
        "--keyboard-grab" => {
            (*state).window_flags |= SDL_WindowFlags_SDL_WINDOW_KEYBOARD_GRABBED;
            return 1;
        }
        "--shown" => {
            (*state).window_flags &= !SDL_WindowFlags_SDL_WINDOW_HIDDEN;
            return 1;
        }
        "--hidden" => {
            (*state).window_flags |= SDL_WindowFlags_SDL_WINDOW_HIDDEN;
            return 1;
        }
        "--input-focus" => {
            (*state).window_flags |= SDL_WindowFlags_SDL_WINDOW_INPUT_FOCUS;
            return 1;
        }
        "--mouse-focus" => {
            (*state).window_flags |= SDL_WindowFlags_SDL_WINDOW_MOUSE_FOCUS;
            return 1;
        }
        "--flash-on-focus-loss" => {
            (*state).flash_on_focus_loss = SDL_bool_SDL_TRUE;
            return 1;
        }
        "--allow-highdpi" => {
            (*state).window_flags |= SDL_WindowFlags_SDL_WINDOW_ALLOW_HIGHDPI;
            return 1;
        }
        "--skip-renderer" => {
            (*state).skip_renderer = SDL_bool_SDL_TRUE;
            return 1;
        }
        "--rate" => {
            let Some(rate) = next_string else {
                return -1;
            };
            (*state).audiospec.freq = rate.parse().unwrap_or((*state).audiospec.freq);
            return 2;
        }
        "--channels" => {
            let Some(channels) = next_string else {
                return -1;
            };
            (*state).audiospec.channels = channels.parse().unwrap_or((*state).audiospec.channels);
            return 2;
        }
        "--samples" => {
            let Some(samples) = next_string else {
                return -1;
            };
            (*state).audiospec.samples = samples.parse().unwrap_or((*state).audiospec.samples);
            return 2;
        }
        "--format" => {
            let Some(format) = next_string else {
                return -1;
            };
            (*state).audiospec.format = match format.as_str() {
                "U8" => sdl::AUDIO_U8 as u16,
                "S8" => sdl::AUDIO_S8 as u16,
                "U16" | "U16LSB" | "U16LE" => sdl::AUDIO_U16 as u16,
                "U16MSB" | "U16BE" => sdl::AUDIO_U16MSB as u16,
                "S16" | "S16LSB" | "S16LE" => sdl::AUDIO_S16 as u16,
                "S16MSB" | "S16BE" => sdl::AUDIO_S16MSB as u16,
                _ => return -1,
            };
            return 2;
        }
        "--usable-bounds" => return 1,
        _ => {}
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_CommonLogUsage(
    state: *mut SDLTest_CommonState,
    argv0: *const c_char,
    options: *const *const c_char,
) {
    let usage = render_usage(state, argv0, options);
    crate::testsupport::log::SDLTest_LogFromBuffer(usage);
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_CommonUsage(state: *mut SDLTest_CommonState) -> *const c_char {
    render_usage(state, (*state).window_title, ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_CommonInit(state: *mut SDLTest_CommonState) -> SDL_bool {
    if state.is_null() {
        return SDL_bool_SDL_FALSE;
    }
    if !(*state).videodriver.is_null() {
        sdl::SDL_SetHint(SDL_HINT_VIDEODRIVER.as_ptr().cast(), (*state).videodriver);
    }
    if !(*state).audiodriver.is_null() {
        sdl::SDL_SetHint(SDL_HINT_AUDIODRIVER.as_ptr().cast(), (*state).audiodriver);
    }
    if sdl::SDL_InitSubSystem((*state).flags) != 0 {
        return SDL_bool_SDL_FALSE;
    }
    if (*state).num_windows <= 0 || ((*state).flags & SDL_INIT_VIDEO == 0) {
        return SDL_bool_SDL_TRUE;
    }

    if (*state).window_flags & SDL_WindowFlags_SDL_WINDOW_OPENGL != 0 {
        sdl::SDL_GL_SetAttribute(SDL_GLattr_SDL_GL_RED_SIZE, (*state).gl_red_size);
        sdl::SDL_GL_SetAttribute(SDL_GLattr_SDL_GL_GREEN_SIZE, (*state).gl_green_size);
        sdl::SDL_GL_SetAttribute(SDL_GLattr_SDL_GL_BLUE_SIZE, (*state).gl_blue_size);
        sdl::SDL_GL_SetAttribute(SDL_GLattr_SDL_GL_ALPHA_SIZE, (*state).gl_alpha_size);
        sdl::SDL_GL_SetAttribute(SDL_GLattr_SDL_GL_BUFFER_SIZE, (*state).gl_buffer_size);
        sdl::SDL_GL_SetAttribute(SDL_GLattr_SDL_GL_DEPTH_SIZE, (*state).gl_depth_size);
        sdl::SDL_GL_SetAttribute(SDL_GLattr_SDL_GL_STENCIL_SIZE, (*state).gl_stencil_size);
        sdl::SDL_GL_SetAttribute(SDL_GLattr_SDL_GL_DOUBLEBUFFER, (*state).gl_double_buffer);
        sdl::SDL_GL_SetAttribute(
            SDL_GLattr_SDL_GL_MULTISAMPLEBUFFERS,
            (*state).gl_multisamplebuffers,
        );
        sdl::SDL_GL_SetAttribute(
            SDL_GLattr_SDL_GL_MULTISAMPLESAMPLES,
            (*state).gl_multisamplesamples,
        );
        sdl::SDL_GL_SetAttribute(
            SDL_GLattr_SDL_GL_ACCELERATED_VISUAL,
            (*state).gl_accelerated,
        );
        sdl::SDL_GL_SetAttribute(
            SDL_GLattr_SDL_GL_CONTEXT_MAJOR_VERSION,
            (*state).gl_major_version,
        );
        sdl::SDL_GL_SetAttribute(
            SDL_GLattr_SDL_GL_CONTEXT_MINOR_VERSION,
            (*state).gl_minor_version,
        );
        sdl::SDL_GL_SetAttribute(SDL_GLattr_SDL_GL_CONTEXT_FLAGS, (*state).gl_debug);
        sdl::SDL_GL_SetAttribute(
            SDL_GLattr_SDL_GL_CONTEXT_PROFILE_MASK,
            (*state).gl_profile_mask,
        );
    }

    let count = (*state).num_windows as usize;
    (*state).windows = sdl::SDL_calloc(count, std::mem::size_of::<*mut sdl::SDL_Window>()).cast();
    (*state).renderers =
        sdl::SDL_calloc(count, std::mem::size_of::<*mut sdl::SDL_Renderer>()).cast();
    (*state).targets = sdl::SDL_calloc(count, std::mem::size_of::<*mut sdl::SDL_Texture>()).cast();
    if (*state).windows.is_null() || (*state).renderers.is_null() || (*state).targets.is_null() {
        cleanup_windows(state);
        return SDL_bool_SDL_FALSE;
    }

    let renderer_index = renderer_index_by_name((*state).renderdriver);
    for index in 0..count {
        let window = sdl::SDL_CreateWindow(
            (*state).window_title,
            (*state).window_x,
            (*state).window_y,
            (*state).window_w.max(1),
            (*state).window_h.max(1),
            (*state).window_flags,
        );
        if window.is_null() {
            cleanup_windows(state);
            return SDL_bool_SDL_FALSE;
        }
        if (*state).window_minW > 0 || (*state).window_minH > 0 {
            sdl::SDL_SetWindowMinimumSize(window, (*state).window_minW, (*state).window_minH);
        }
        if (*state).window_maxW > 0 || (*state).window_maxH > 0 {
            sdl::SDL_SetWindowMaximumSize(window, (*state).window_maxW, (*state).window_maxH);
        }
        *(*state).windows.add(index) = window;
        if (*state).skip_renderer == SDL_bool_SDL_FALSE {
            let renderer = sdl::SDL_CreateRenderer(window, renderer_index, (*state).render_flags);
            if renderer.is_null() {
                cleanup_windows(state);
                return SDL_bool_SDL_FALSE;
            }
            if (*state).logical_w > 0 && (*state).logical_h > 0 {
                sdl::SDL_RenderSetLogicalSize(renderer, (*state).logical_w, (*state).logical_h);
            }
            if (*state).scale > 0.0 {
                sdl::SDL_RenderSetScale(renderer, (*state).scale, (*state).scale);
            }
            *(*state).renderers.add(index) = renderer;
        }
    }
    SDL_bool_SDL_TRUE
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_CommonDefaultArgs(
    state: *mut SDLTest_CommonState,
    argc: c_int,
    argv: *mut *mut c_char,
) -> SDL_bool {
    let mut index = 1;
    while index < argc {
        let consumed = SDLTest_CommonArg(state, index);
        if consumed < 0 {
            SDLTest_CommonLogUsage(state, crate::testsupport::argv_at(argv, 0), ptr::null());
            return SDL_bool_SDL_FALSE;
        }
        if consumed == 0 {
            SDLTest_CommonLogUsage(state, crate::testsupport::argv_at(argv, 0), ptr::null());
            return SDL_bool_SDL_FALSE;
        }
        index += consumed;
    }
    SDLTest_CommonInit(state)
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_CommonEvent(
    _state: *mut SDLTest_CommonState,
    event: *mut SDL_Event,
    done: *mut c_int,
) {
    if event.is_null() {
        return;
    }
    let event_type = (*event).type_;
    if event_type == SDL_EventType_SDL_QUIT {
        if !done.is_null() {
            *done = 1;
        }
    } else if event_type == SDL_EventType_SDL_WINDOWEVENT {
        if (*event).window.event as u32 == SDL_WindowEventID_SDL_WINDOWEVENT_CLOSE {
            if !done.is_null() {
                *done = 1;
            }
        }
    } else if event_type == SDL_EventType_SDL_KEYDOWN {
        let sym = (*event).key.keysym.sym;
        if sym == SDL_KeyCode_SDLK_ESCAPE as i32 || sym == SDL_KeyCode_SDLK_q as i32 {
            if !done.is_null() {
                *done = 1;
            }
        }
    } else if event_type == SDL_EventType_SDL_DROPFILE || event_type == SDL_EventType_SDL_DROPTEXT {
        if !(*event).drop.file.is_null() {
            sdl::SDL_free((*event).drop.file.cast());
            (*event).drop.file = ptr::null_mut();
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_CommonQuit(state: *mut SDLTest_CommonState) {
    if state.is_null() {
        return;
    }
    cleanup_windows(state);
    crate::testsupport::font::SDLTest_CleanupTextDrawing();
    crate::testsupport::memory::SDLTest_LogAllocations();
    sdl::SDL_Quit();
    lock_usage_cache().remove(&(state as usize));
    drop(Box::from_raw(state));
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_CommonDrawWindowInfo(
    renderer: *mut sdl::SDL_Renderer,
    window: *mut sdl::SDL_Window,
    usedHeight: *mut c_int,
) {
    if !usedHeight.is_null() {
        *usedHeight = 0;
    }
    if renderer.is_null() || window.is_null() {
        return;
    }
    let mut x = 0;
    let mut y = 0;
    let mut w = 0;
    let mut h = 0;
    sdl::SDL_GetWindowPosition(window, &mut x, &mut y);
    sdl::SDL_GetWindowSize(window, &mut w, &mut h);
    let lines = [
        format!("pos: {x},{y}"),
        format!("size: {w}x{h}"),
        format!("flags: 0x{:x}", sdl::SDL_GetWindowFlags(window)),
    ];
    let mut offset = 0;
    for line in lines {
        let text = CString::new(line).unwrap();
        crate::testsupport::font::SDLTest_DrawString(renderer, 0, offset, text.as_ptr());
        offset += FONT_LINE_HEIGHT;
    }
    if !usedHeight.is_null() {
        *usedHeight = offset;
    }
}
