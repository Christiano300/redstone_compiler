use core::panic;
use std::collections::HashMap;

use super::ValueType;

pub struct Environment {
    parent: Option<Box<Environment>>,
    variables: HashMap<String, ValueType>,
}

impl Environment {
    pub fn new(parent: Option<Box<Environment>>) -> Environment {
        Environment {
            parent,
            variables: HashMap::new(),
        }
    }

    pub fn assign(&mut self, name: String, value: ValueType) -> ValueType {
        let env = self.resolve(&name);
        match env {
            None => self.variables.insert(name, value),
            Some(env) => env.variables.insert(name, value),
        };
        value
    }

    pub fn lookup(&mut self, name: &String) -> &mut ValueType {
        let env = self.resolve(name);
        match env {
            None => panic!("Variable {name} doesn't exist"),
            Some(env) => env.variables.get_mut(name).unwrap(),
        }
    }

    fn resolve(&mut self, name: &String) -> Option<&mut Environment> {
        if self.variables.contains_key(name) {
            Some(self)
        } else if self.parent.is_none() {
            None
        } else {
            self.parent.as_mut().unwrap().resolve(name)
        }
    }
}
