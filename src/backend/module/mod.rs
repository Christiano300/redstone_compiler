mod io;
mod list;
mod ram;
mod screen;

use std::cell::RefCell;
use std::collections::HashMap;

// don't ask
thread_local! {
    pub static MODULES: RefCell<HashMap<String, Module>> = RefCell::new({
        let mut map = HashMap::new();
        register(&mut map, "io", io::module, None);
        register(&mut map, "screen", screen::module, None);
        register(&mut map, "ram", ram::module, None);
        register(&mut map, "list", list::module, Some(Box::from(list::init)));
        map
    })
}

use crate::frontend::Expression;

use super::{Compiler, Error, ModuleCall};

pub type Handler = dyn FnMut(&mut Compiler, &ModuleCall) -> Res;

pub type Init = dyn FnMut(&mut Compiler) -> Res;

pub struct Module {
    pub name: String,
    pub handler: Box<Handler>,
    pub init: Option<Box<Init>>,
}

fn register<H>(
    modules: &mut HashMap<String, Module>,
    name: &'static str,
    handler: H,
    init: Option<Box<Init>>,
) where
    H: FnMut(&mut Compiler, &ModuleCall) -> Res + 'static,
{
    modules.insert(
        name.to_string(),
        Module {
            name: name.to_string(),
            handler: Box::from(handler),
            init,
        },
    );
}

enum Arg {
    Number(&'static str),
    Constant(&'static str),
}

fn arg_parse<'a, const COUNT: usize>(
    compiler: &mut Compiler,
    types: [Arg; COUNT],
    args: &'a [Expression],
) -> Res<[&'a Expression; COUNT]> {
    if types.len() != args.len() {
        return Err(Error::InvalidArgs("Wrong number of Arguments".to_string()));
    }
    types
        .into_iter()
        .zip(args.iter())
        .try_for_each(|(typ, arg)| match typ {
            Arg::Constant(name) => match compiler.try_get_constant(arg) {
                Ok(Some(_)) => Ok(()), // if we can get the value at compile-time, its ok
                Ok(None) => Err(Error::InvalidArgs(format!(
                    "{name} has to be known at compile-time"
                ))), // otherwise we error
                err => err.map(|_res| ()), // return any other error
            },
            Arg::Number(..) => Ok(()),
        })?;

    let mut iter = args.iter();
    let res = [(); COUNT].map(|_res| iter.next().unwrap());
    assert_eq!(res.len(), COUNT);
    Ok(res)
}

type Res<T = ()> = Result<T, Error>;
