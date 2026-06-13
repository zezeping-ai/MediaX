#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::approx_constant)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::redundant_static_lifetimes)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
// bindgen 生成的 FFI 结构体含函数指针 PartialEq 与类型转换，Rust 1.86+ 会对此报警。
#![allow(unpredictable_function_pointer_comparisons)]
#![allow(unnecessary_transmutes)]

extern crate libc;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[macro_use]
mod avutil;
pub use avutil::*;
