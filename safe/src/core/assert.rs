use crate::abi::generated_types::{
    SDL_AssertData, SDL_AssertState, SDL_AssertionHandler,
};

crate::forward_sdl! {
    fn SDL_ReportAssertion(
        arg1: *mut SDL_AssertData,
        arg2: *const libc::c_char,
        arg3: *const libc::c_char,
        arg4: libc::c_int
    ) -> SDL_AssertState;
    fn SDL_SetAssertionHandler(handler: SDL_AssertionHandler, userdata: *mut libc::c_void);
    fn SDL_GetDefaultAssertionHandler() -> SDL_AssertionHandler;
    fn SDL_GetAssertionHandler(puserdata: *mut *mut libc::c_void) -> SDL_AssertionHandler;
    fn SDL_GetAssertionReport() -> *const SDL_AssertData;
    fn SDL_ResetAssertionReport();
}
