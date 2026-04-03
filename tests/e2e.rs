use redstone_compiler::{
    backend::{Compiler, Output, Target},
    frontend::{Parser, tokenize},
};

#[test]
fn mcn16_simple() {
    let code = r"
        x = 5
        y = 10
        z = x + y
    ";

    let expected_assembly = r"
    LAL 5
    SVA 0
    LAL 10
    SVA 1
    LB 0
    ADD
    SVA 2
    ";

    let mut parser = Parser::new();
    let ast = parser.produce_ast(tokenize(code).unwrap()).unwrap();
    let mut compiler = Compiler::new();
    let assembly = compiler.compile_program(ast).unwrap();

    assert_eq!(
        assembly
            .repr()
            .trim()
            .lines()
            .map(str::trim)
            .collect::<Vec<_>>(),
        expected_assembly
            .trim()
            .lines()
            .map(str::trim)
            .collect::<Vec<_>>()
    );
}
