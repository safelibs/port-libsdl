use crate::abi::generated_types::SDL_Locale;

crate::forward_sdl! {
    fn SDL_GetPreferredLocales() -> *mut SDL_Locale;
}
