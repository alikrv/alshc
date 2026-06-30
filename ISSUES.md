- @stdlib preprocessor directive is doing nothing

- std::println should accept more than one arg `alshc: codegen error: Codegen error in function test_some_types: std::println expects 1 argument`

- other functions cant be used? `alshc: codegen error: Codegen error in function test_some_types: Unknown function: foo_func`

- if statements and their "integration" with int ant bool <- and the implementation of types

- types

- missing std:: functions

- parser is sometimes not good at its job:
```
[ali@baguette alshc]$ cargo run -- -d test_while_simple.alsh
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.05s
     Running `target/debug/alshc -d test_while_simple.alsh`
ALSH Compiler
============
Compiling code from test_while_simple.alsh:
@main
function test_loop() int {
    let i = 0
    while ($i < 3) {
        std::print("loop at iteration:")
        std::println($i)
        $i = ($i + 1)
    }
    return 0
}

// this isnt parsed correctly
// says that there's a missing '}', but where? <- that's another thing compiler error messages should be more helpful

alshc: parse error: Expected matching }
[ali@baguette alshc]$
```
- C interop is still not solid:
```
[ali@baguette alshc]$ cargo run -- -d test_c_varargs.alsh ; ./a.out
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.01s
     Running `target/debug/alshc -d test_c_varargs.alsh`
ALSH Compiler
============
Compiling code from test_c_varargs.alsh:
@main
function test_c_printf_varargs() int {
    std::println("test1")
    let var1 = 43
    let var2 = "heY!"
    c::printf("%d, %s\n", $var1, $var2)
}
Parsed 1 statements
  Statement 0: FunctionDef { name: "test_c_printf_varargs", params: [], body: [Expression(FunctionCall("std::println", [Literal(String("test1"))])), Let { name: "var1", value: Literal(Number(43)) }, Let { name: "var2", value: Literal(String("heY!")) }, Expression(CCall("printf", [Literal(String("\"%d")), Literal(String("%s\\n\"")), Variable("var1"), Variable("var2")]))] }
Compiled to a.out
test1
"2115395708[ali@baguette alshc]$
```
(types not properly implemented could be the reason?)