mod io;

use std::cell::RefCell;
use std::collections::HashMap;

// don't ask
thread_local! {
    pub static MODULES: RefCell<HashMap<String, Module>> = RefCell::new({
        let mut map = HashMap::new();
        register(&mut map, "io", io::module);
        map
    })
}

use super::{Compiler, Error, ModuleCall};

pub type Handler = dyn FnMut(&mut Compiler, &ModuleCall) -> Res;

pub struct Module {
    pub name: String,
    pub handler: Box<Handler>,
}

pub fn register<F>(modules: &mut HashMap<String, Module>, name: &'static str, handler: F)
where
    F: FnMut(&mut Compiler, &ModuleCall) -> Res + 'static,
{
    modules.insert(
        name.to_string(),
        Module {
            name: name.to_string(),
            handler: Box::from(handler),
        },
    );
}

type Res<T = ()> = Result<T, Error>;
