use std::collections::BTreeMap;
use std::sync::OnceLock;

use crate::abi::generated_types::{
    self as sdl, SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB24,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBA32, SDL_Surface,
};

struct ParsedImage {
    width: i32,
    height: i32,
    bytes_per_pixel: usize,
    pixels: Vec<u8>,
}

const BLIT_SOURCE: &str = include_str!("../../../original/src/test/SDL_test_imageBlit.c");
const BLIT_BLEND_SOURCE: &str =
    include_str!("../../../original/src/test/SDL_test_imageBlitBlend.c");
const FACE_SOURCE: &str = include_str!("../../../original/src/test/SDL_test_imageFace.c");
const PRIMITIVES_SOURCE: &str =
    include_str!("../../../original/src/test/SDL_test_imagePrimitives.c");
const PRIMITIVES_BLEND_SOURCE: &str =
    include_str!("../../../original/src/test/SDL_test_imagePrimitivesBlend.c");

fn decode_c_string_literal(input: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            bytes.push(ch as u8);
            continue;
        }
        let Some(next) = chars.next() else {
            break;
        };
        match next {
            '0'..='7' => {
                let mut octal = String::from(next);
                for _ in 0..2 {
                    if let Some(peek) = chars.peek() {
                        if matches!(peek, '0'..='7') {
                            octal.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                }
                bytes.push(u8::from_str_radix(&octal, 8).unwrap_or(0));
            }
            'n' => bytes.push(b'\n'),
            'r' => bytes.push(b'\r'),
            't' => bytes.push(b'\t'),
            '\\' => bytes.push(b'\\'),
            '"' => bytes.push(b'"'),
            '\'' => bytes.push(b'\''),
            other => bytes.push(other as u8),
        }
    }
    bytes
}

fn skip_ws(input: &str, index: &mut usize) {
    while *index < input.len() && input.as_bytes()[*index].is_ascii_whitespace() {
        *index += 1;
    }
}

fn parse_number(input: &str, index: &mut usize) -> i32 {
    skip_ws(input, index);
    let start = *index;
    while *index < input.len() && input.as_bytes()[*index].is_ascii_digit() {
        *index += 1;
    }
    input[start..*index].trim().parse().unwrap()
}

fn parse_image(source: &str, symbol: &str) -> ParsedImage {
    let needle = format!("static const SDLTest_SurfaceImage_t {symbol} = {{");
    let start = source.find(&needle).unwrap() + needle.len();
    let end = source[start..].find("};").unwrap() + start;
    let body = &source[start..end];
    let mut index = 0usize;
    let width = parse_number(body, &mut index);
    while body.as_bytes()[index] != b',' {
        index += 1;
    }
    index += 1;
    let height = parse_number(body, &mut index);
    while body.as_bytes()[index] != b',' {
        index += 1;
    }
    index += 1;
    let bytes_per_pixel = parse_number(body, &mut index) as usize;
    while body.as_bytes()[index] != b',' {
        index += 1;
    }
    index += 1;

    let mut pixels = Vec::new();
    loop {
        skip_ws(body, &mut index);
        if index >= body.len() || body.as_bytes()[index] == b'}' {
            break;
        }
        if body.as_bytes()[index] == b',' {
            index += 1;
            continue;
        }
        assert_eq!(body.as_bytes()[index], b'"');
        index += 1;
        let string_start = index;
        let mut escaped = false;
        while index < body.len() {
            let byte = body.as_bytes()[index];
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                break;
            }
            index += 1;
        }
        pixels.extend(decode_c_string_literal(&body[string_start..index]));
        index += 1;
    }
    ParsedImage {
        width,
        height,
        bytes_per_pixel,
        pixels,
    }
}

fn images() -> &'static BTreeMap<&'static str, ParsedImage> {
    static IMAGES: OnceLock<BTreeMap<&'static str, ParsedImage>> = OnceLock::new();
    IMAGES.get_or_init(|| {
        BTreeMap::from([
            (
                "SDLTest_imageBlit",
                parse_image(BLIT_SOURCE, "SDLTest_imageBlit"),
            ),
            (
                "SDLTest_imageBlitColor",
                parse_image(BLIT_SOURCE, "SDLTest_imageBlitColor"),
            ),
            (
                "SDLTest_imageBlitAlpha",
                parse_image(BLIT_SOURCE, "SDLTest_imageBlitAlpha"),
            ),
            (
                "SDLTest_imageBlitBlendAdd",
                parse_image(BLIT_BLEND_SOURCE, "SDLTest_imageBlitBlendAdd"),
            ),
            (
                "SDLTest_imageBlitBlend",
                parse_image(BLIT_BLEND_SOURCE, "SDLTest_imageBlitBlend"),
            ),
            (
                "SDLTest_imageBlitBlendMod",
                parse_image(BLIT_BLEND_SOURCE, "SDLTest_imageBlitBlendMod"),
            ),
            (
                "SDLTest_imageBlitBlendNone",
                parse_image(BLIT_BLEND_SOURCE, "SDLTest_imageBlitBlendNone"),
            ),
            (
                "SDLTest_imageBlitBlendAll",
                parse_image(BLIT_BLEND_SOURCE, "SDLTest_imageBlitBlendAll"),
            ),
            (
                "SDLTest_imageFace",
                parse_image(FACE_SOURCE, "SDLTest_imageFace"),
            ),
            (
                "SDLTest_imagePrimitives",
                parse_image(PRIMITIVES_SOURCE, "SDLTest_imagePrimitives"),
            ),
            (
                "SDLTest_imagePrimitivesBlend",
                parse_image(PRIMITIVES_BLEND_SOURCE, "SDLTest_imagePrimitivesBlend"),
            ),
        ])
    })
}

unsafe fn create_surface(symbol: &'static str, format: u32) -> *mut SDL_Surface {
    let image = &images()[symbol];
    let surface = sdl::SDL_CreateRGBSurfaceWithFormat(
        0,
        image.width,
        image.height,
        (image.bytes_per_pixel * 8) as i32,
        format,
    );
    if surface.is_null() {
        return surface;
    }
    crate::testsupport::copy_bytes_to_surface(
        surface,
        &image.pixels,
        image.width as usize * image.bytes_per_pixel,
        image.width as usize * image.bytes_per_pixel,
        image.height as usize,
    );
    surface
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_ImageBlit() -> *mut SDL_Surface {
    create_surface(
        "SDLTest_imageBlit",
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB24,
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_ImageBlitColor() -> *mut SDL_Surface {
    create_surface(
        "SDLTest_imageBlitColor",
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB24,
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_ImageBlitAlpha() -> *mut SDL_Surface {
    create_surface(
        "SDLTest_imageBlitAlpha",
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB24,
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_ImageBlitBlendAdd() -> *mut SDL_Surface {
    create_surface(
        "SDLTest_imageBlitBlendAdd",
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB24,
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_ImageBlitBlend() -> *mut SDL_Surface {
    create_surface(
        "SDLTest_imageBlitBlend",
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB24,
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_ImageBlitBlendMod() -> *mut SDL_Surface {
    create_surface(
        "SDLTest_imageBlitBlendMod",
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB24,
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_ImageBlitBlendNone() -> *mut SDL_Surface {
    create_surface(
        "SDLTest_imageBlitBlendNone",
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB24,
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_ImageBlitBlendAll() -> *mut SDL_Surface {
    create_surface(
        "SDLTest_imageBlitBlendAll",
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB24,
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_ImageFace() -> *mut SDL_Surface {
    create_surface(
        "SDLTest_imageFace",
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGBA32,
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_ImagePrimitives() -> *mut SDL_Surface {
    create_surface(
        "SDLTest_imagePrimitives",
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB24,
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_ImagePrimitivesBlend() -> *mut SDL_Surface {
    create_surface(
        "SDLTest_imagePrimitivesBlend",
        SDL_PixelFormatEnum_SDL_PIXELFORMAT_RGB24,
    )
}
