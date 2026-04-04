use crate::abi::generated_types::{
    SDL_TLSID, SDL_Thread, SDL_ThreadFunction, SDL_ThreadPriority, SDL_threadID, Sint64,
};

crate::forward_sdl! {
    fn SDL_CreateThread(
        fn_: SDL_ThreadFunction,
        name: *const libc::c_char,
        data: *mut libc::c_void
    ) -> *mut SDL_Thread;
    fn SDL_CreateThreadWithStackSize(
        fn_: SDL_ThreadFunction,
        name: *const libc::c_char,
        stacksize: usize,
        data: *mut libc::c_void
    ) -> *mut SDL_Thread;
    fn SDL_GetThreadName(thread: *mut SDL_Thread) -> *const libc::c_char;
    fn SDL_ThreadID() -> SDL_threadID;
    fn SDL_GetThreadID(thread: *mut SDL_Thread) -> SDL_threadID;
    fn SDL_SetThreadPriority(priority: SDL_ThreadPriority) -> libc::c_int;
    fn SDL_WaitThread(thread: *mut SDL_Thread, status: *mut libc::c_int);
    fn SDL_DetachThread(thread: *mut SDL_Thread);
    fn SDL_TLSCreate() -> SDL_TLSID;
    fn SDL_TLSGet(id: SDL_TLSID) -> *mut libc::c_void;
    fn SDL_TLSSet(
        id: SDL_TLSID,
        value: *const libc::c_void,
        destructor: Option<unsafe extern "C" fn(arg1: *mut libc::c_void)>
    ) -> libc::c_int;
    fn SDL_TLSCleanup();
    fn SDL_LinuxSetThreadPriority(threadID: Sint64, priority: libc::c_int) -> libc::c_int;
    fn SDL_LinuxSetThreadPriorityAndPolicy(
        threadID: Sint64,
        sdlPriority: libc::c_int,
        schedPolicy: libc::c_int
    ) -> libc::c_int;
}
