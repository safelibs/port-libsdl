#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::all)]

#[allow(clippy::all)]
#[path = "../../src/abi/generated_types.rs"]
pub mod generated_types_impl;

pub mod abi {
    pub use crate::generated_types_impl as generated_types;
}

#[path = "../../src/testsupport/mod.rs"]
pub mod testsupport;
