mod io;
mod ram;
mod screen;

use std::cell::RefCell;
use std::collections::HashMap;

// don't ask
thread_local! {
    pub static MODULES: RefCell<HashMap<String, Module>> = RefCell::new({
        let mut map = HashMap::new();
        register(&mut map, "io", io::module);
        register(&mut map, "screen", screen::module);
        register(&mut map, "ram", ram::module);
        map
    })
}

use crate::frontend::Expression;

use super::{Compiler, Error, ModuleCall};

pub type Handler = dyn FnMut(&mut Compiler, &ModuleCall) -> Res;

pub struct Module {
    pub name: String,
    pub handler: Box<Handler>,
}

fn register<F>(modules: &mut HashMap<String, Module>, name: &'static str, handler: F)
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

enum Arg {
    Number(&'static str),
    Constant(&'static str),
}

fn arg_parse<'a, const COUNT: usize>(
    compiler: &mut Compiler,
    types: [Arg; COUNT],
    args: &'a Vec<Expression>,
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
    let res = [(); COUNT].map(|_res| iter.next().unwrap_or(&Expression::Pass));
    assert_eq!(res.len(), COUNT);
    Ok(res)
}

type Res<T = ()> = Result<T, Error>;
