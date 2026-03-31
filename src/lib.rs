#[macro_use]
mod error;

pub mod backend;
pub mod frontend;

pub use error::Error;

#[macro_use]
extern crate static_assertions;
