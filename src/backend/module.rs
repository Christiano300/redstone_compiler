use super::{compiler::ModuleCall, Compiler, CompilerError, Instr};

type Res<T = ()> = Result<T, CompilerError>;

fn io_module(call: ModuleCall) -> Res<Vec<Instr>> {
    Ok(vec![])
}

pub fn register_modules(compiler: &mut Compiler) {
    compiler.register_module("io", io_module)
}
