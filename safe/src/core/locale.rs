use std::ffi::CString;
use std::mem::size_of;
use std::ptr;

use crate::abi::generated_types::SDL_Locale;

fn best_locale_string() -> Option<String> {
    for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(value) = std::env::var(key) {
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetPreferredLocales() -> *mut SDL_Locale {
    let locale = match best_locale_string() {
        Some(value) => value,
        None => {
            let _ = crate::core::error::set_error_message("No locale information is available");
            return std::ptr::null_mut();
        }
    };

    let trimmed = locale
        .split(':')
        .next()
        .unwrap_or(locale.as_str())
        .split('.')
        .next()
        .unwrap_or(locale.as_str())
        .split('@')
        .next()
        .unwrap_or(locale.as_str());

    let (language, country) = match trimmed.split_once('_') {
        Some((language, country)) => (language.to_string(), Some(country.to_string())),
        None => (trimmed.to_string(), None),
    };

    let language = CString::new(language).unwrap_or_default();
    let country = country.and_then(|country| CString::new(country).ok());
    let strings_len = language.as_bytes_with_nul().len()
        + country
            .as_ref()
            .map(|country| country.as_bytes_with_nul().len())
            .unwrap_or(0);
    let total = size_of::<SDL_Locale>() * 2 + strings_len;
    let block = crate::core::memory::SDL_malloc(total) as *mut u8;
    if block.is_null() {
        let _ = crate::core::error::out_of_memory_error();
        return std::ptr::null_mut();
    }

    let locales = block as *mut SDL_Locale;
    let mut string_cursor = block.add(size_of::<SDL_Locale>() * 2);

    ptr::copy_nonoverlapping(
        language.as_ptr().cast::<u8>(),
        string_cursor,
        language.as_bytes_with_nul().len(),
    );
    (*locales).language = string_cursor.cast();
    string_cursor = string_cursor.add(language.as_bytes_with_nul().len());

    if let Some(country) = country.as_ref() {
        ptr::copy_nonoverlapping(
            country.as_ptr().cast::<u8>(),
            string_cursor,
            country.as_bytes_with_nul().len(),
        );
        (*locales).country = string_cursor.cast();
        string_cursor = string_cursor.add(country.as_bytes_with_nul().len());
    } else {
        (*locales).country = std::ptr::null();
    }

    let terminator = locales.add(1);
    (*terminator).language = std::ptr::null();
    (*terminator).country = std::ptr::null();

    let _ = string_cursor;
    locales
}
