#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clashing_extern_declarations)]
#![allow(clippy::all)]

pub mod abi {
    #[allow(clippy::all)]
    pub mod generated_types;
}

pub mod audio;
pub mod core;
pub mod events;
pub mod input;
pub mod security {
    pub mod checked_math;
}
pub mod dynapi {
    pub mod generated;
}
pub mod render;
pub mod video;

pub mod main_archive;

pub mod exports {
    #[inline(never)]
    pub fn abort_unimplemented(symbol: &str) -> ! {
        eprintln!("safe-sdl bootstrap stub called: {symbol}");
        std::process::abort();
    }

    pub mod generated_linux_stubs;
}
