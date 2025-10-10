pub mod backend;
mod error;
pub mod frontend;

pub use error::Error;

#[macro_use]
extern crate static_assertions;
