use alshc::{CodeGen, ControlFlowParser};
use inkwell::context::Context;

#[test]
fn c_calls_use_c_string_arguments_for_string_literals() {
    let source = r#"@main
function main() int {
    c::puts("hello")
    return 0;
}"#;

    let mut parser = ControlFlowParser::new(source);
    let statements = parser.parse().expect("parser should succeed");

    let context = Context::create();
    let mut codegen = CodeGen::new(&context, false);
    codegen.set_has_main(parser.has_main);
    codegen.set_main_function_name(parser.main_function_name.clone());
    codegen.generate(&statements).expect("codegen should succeed");

    let ir = codegen.get_ir();
    let call_line = ir
        .lines()
        .find(|line| line.contains("call i32 @puts"))
        .expect("puts call should be emitted");

    assert!(
        call_line.contains("getelementptr"),
        "expected puts call to receive a C string pointer, got: {call_line}"
    );
}
