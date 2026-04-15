use std::fmt::Debug;

use crate::{Error, frontend::Fragment};

pub trait Target {
    type Output: Output;

    fn compile_program(&mut self, program: Fragment) -> Result<Self::Output, Vec<Error>>;

    fn reset(&mut self) {}
}

pub trait Output: Debug {
    fn repr(&self) -> String;

    fn repr_bin(&self) -> Option<String>;

    fn repr_loc(&self) -> Option<String>;
}
