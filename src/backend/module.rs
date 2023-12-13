use super::Compiler;

trait Module {
    fn get_name(&self) -> String;

    fn compile(&mut self, compiler: &mut Compiler);
}
