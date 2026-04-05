use std::sync::OnceLock;

use crate::abi::generated_types::{self as sdl, SDL_Rect, SDL_Renderer, Uint32};
use crate::testsupport::{SDLTest_TextWindow, FONT_CHARACTER_SIZE, FONT_LINE_HEIGHT};

const FONT_SOURCE: &str = include_str!("../../../original/src/test/SDL_test_font.c");

fn font_data() -> &'static Vec<u8> {
    static FONT: OnceLock<Vec<u8>> = OnceLock::new();
    FONT.get_or_init(|| {
        let start = FONT_SOURCE
            .find("static unsigned char SDLTest_FontData[] = {")
            .unwrap();
        let body_start = FONT_SOURCE[start..].find('{').unwrap() + start + 1;
        let body_end = FONT_SOURCE[body_start..].find("};").unwrap() + body_start;
        let mut bytes = Vec::new();
        for token in FONT_SOURCE[body_start..body_end].split(',') {
            let token = token.trim();
            if let Some(hex) = token.strip_prefix("0x") {
                let value = hex
                    .chars()
                    .take_while(|ch| ch.is_ascii_hexdigit())
                    .collect::<String>();
                if !value.is_empty() {
                    bytes.push(u8::from_str_radix(&value, 16).unwrap());
                }
            }
        }
        bytes
    })
}

fn utf8_getch(src: &[u8]) -> (u32, usize) {
    const UNKNOWN_UNICODE: u32 = 0xfffd;
    if src.is_empty() {
        return (UNKNOWN_UNICODE, 0);
    }
    let first = src[0];
    if first < 0x80 {
        return (first as u32, 1);
    }
    for width in 2..=4 {
        if src.len() < width {
            break;
        }
        if let Ok(text) = std::str::from_utf8(&src[..width]) {
            if let Some(ch) = text.chars().next() {
                return (ch as u32, width);
            }
        }
    }
    (UNKNOWN_UNICODE, 1)
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_DrawCharacter(
    renderer: *mut SDL_Renderer,
    x: libc::c_int,
    y: libc::c_int,
    c: Uint32,
) -> libc::c_int {
    if renderer.is_null() {
        return crate::testsupport::invalid_param_error("renderer");
    }
    let index = (c as usize).min(255) * FONT_CHARACTER_SIZE as usize;
    let data = font_data();
    if index + FONT_CHARACTER_SIZE as usize > data.len() {
        return 0;
    }
    for (row, bits) in data[index..index + FONT_CHARACTER_SIZE as usize]
        .iter()
        .copied()
        .enumerate()
    {
        for column in 0..FONT_CHARACTER_SIZE {
            if bits & (1 << column) != 0 {
                sdl::SDL_RenderDrawPoint(renderer, x + column, y + row as i32);
            }
        }
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_DrawString(
    renderer: *mut SDL_Renderer,
    x: libc::c_int,
    y: libc::c_int,
    s: *const libc::c_char,
) -> libc::c_int {
    if renderer.is_null() {
        return crate::testsupport::invalid_param_error("renderer");
    }
    if s.is_null() {
        return crate::testsupport::invalid_param_error("s");
    }
    let bytes = std::ffi::CStr::from_ptr(s).to_bytes();
    let mut curx = x;
    let mut index = 0usize;
    while index < bytes.len() {
        let (ch, advance) = utf8_getch(&bytes[index..]);
        if ch < 256 {
            SDLTest_DrawCharacter(renderer, curx, y, ch);
        }
        curx += FONT_CHARACTER_SIZE;
        index += advance.max(1);
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_TextWindowCreate(
    x: libc::c_int,
    y: libc::c_int,
    w: libc::c_int,
    h: libc::c_int,
) -> *mut SDLTest_TextWindow {
    let lines = sdl::SDL_calloc(
        (h / FONT_LINE_HEIGHT).max(0) as usize,
        std::mem::size_of::<*mut libc::c_char>(),
    )
    .cast::<*mut libc::c_char>();
    if lines.is_null() {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new(SDLTest_TextWindow {
        rect: SDL_Rect { x, y, w, h },
        current: 0,
        numlines: h / FONT_LINE_HEIGHT,
        lines,
    }))
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_TextWindowDisplay(
    textwin: *mut SDLTest_TextWindow,
    renderer: *mut SDL_Renderer,
) {
    let Some(textwin) = textwin.as_ref() else {
        return;
    };
    let mut y = textwin.rect.y;
    for index in 0..textwin.numlines {
        let line = *textwin.lines.add(index as usize);
        if !line.is_null() {
            SDLTest_DrawString(renderer, textwin.rect.x, y, line);
        }
        y += FONT_LINE_HEIGHT;
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_TextWindowAddTextFromBuffer(
    textwin: *mut SDLTest_TextWindow,
    text: *const libc::c_char,
) {
    let Some(textwin) = textwin.as_mut() else {
        return;
    };
    if text.is_null() {
        return;
    }
    SDLTest_TextWindowAddTextWithLength(
        textwin,
        text,
        std::ffi::CStr::from_ptr(text).to_bytes().len(),
    );
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_TextWindowAddTextWithLength(
    textwin: *mut SDLTest_TextWindow,
    text: *const libc::c_char,
    mut len: usize,
) {
    let Some(textwin) = textwin.as_mut() else {
        return;
    };
    if text.is_null() {
        return;
    }
    let bytes = std::slice::from_raw_parts(text.cast::<u8>(), len);
    let mut newline = false;
    if len > 0 && bytes[len - 1] == b'\n' {
        len -= 1;
        newline = true;
    }
    let current = textwin.current as usize;
    let current_line = *textwin.lines.add(current);
    let existing = if current_line.is_null() {
        0
    } else {
        std::ffi::CStr::from_ptr(current_line).to_bytes().len()
    };
    if bytes.first().copied() == Some(b'\x08') {
        if existing > 0 {
            let line = *textwin.lines.add(current);
            let mut new_len = existing;
            while new_len > 1
                && (std::slice::from_raw_parts(line.cast::<u8>(), existing)[new_len - 1] & 0xc0)
                    == 0x80
            {
                new_len -= 1;
            }
            new_len -= 1;
            *line.add(new_len) = 0;
        } else if textwin.current > 0 {
            sdl::SDL_free((*textwin.lines.add(current)).cast());
            *textwin.lines.add(current) = std::ptr::null_mut();
            textwin.current -= 1;
        }
        return;
    }
    let line = sdl::SDL_realloc(current_line.cast(), existing + len + 1).cast::<libc::c_char>();
    if line.is_null() {
        return;
    }
    std::ptr::copy_nonoverlapping(text.cast::<u8>(), line.add(existing).cast::<u8>(), len);
    *line.add(existing + len) = 0;
    *textwin.lines.add(current) = line;
    if newline {
        if textwin.current == textwin.numlines - 1 {
            sdl::SDL_free((*textwin.lines).cast());
            std::ptr::copy(
                textwin.lines.add(1),
                textwin.lines,
                (textwin.numlines - 1) as usize,
            );
            *textwin.lines.add(current) = std::ptr::null_mut();
        } else {
            textwin.current += 1;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_TextWindowClear(textwin: *mut SDLTest_TextWindow) {
    let Some(textwin) = textwin.as_mut() else {
        return;
    };
    for index in 0..textwin.numlines {
        let line = *textwin.lines.add(index as usize);
        if !line.is_null() {
            sdl::SDL_free(line.cast());
            *textwin.lines.add(index as usize) = std::ptr::null_mut();
        }
    }
    textwin.current = 0;
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_TextWindowDestroy(textwin: *mut SDLTest_TextWindow) {
    if textwin.is_null() {
        return;
    }
    SDLTest_TextWindowClear(textwin);
    let textwin = Box::from_raw(textwin);
    sdl::SDL_free(textwin.lines.cast());
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_CleanupTextDrawing() {}
