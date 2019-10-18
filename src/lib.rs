#![deny(clippy::all)]
#![feature(box_syntax)]
#![feature(crate_visibility_modifier)]
#![feature(try_trait)]
#![feature(wrapping_int_impl)]
#![warn(rust_2018_idioms)]

pub mod bytes;
pub mod header;
pub mod io;
pub mod ipl3;
pub mod rom;
crate mod util;
