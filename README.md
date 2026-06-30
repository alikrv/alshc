# ALSHC - ALSH Compiler

## Experimental ALSH compiler (LLVM frontend)

- implements the "COMPILER" ALSH spec, not the "NORMAL" one. Available in this repo as ALSHSPEC.md, or in the spec repo as COMPILER_SPEC.md
- implemented in Rust
- uses inkwell to emit LLVM IR
- cosplays as a serious compiler:
```
> alshc
alshc: fatal: no input files
> alshc test.alsh
> ls
test.alsh a.out
```
- can get the assembly with `-S`
- can get the LLVM IR with `-ir`
- debug mode with `-d`:
```
[ali@baguette alshc]$ cargo run -- -d ./tests/test_arith.alsh
ALSH Compiler
============
Compiling code from ./tests/test_arith.alsh:
@justrunit

# test_arith.alsh
let x = 10
let y = 3
let add = ($x + $y)
let sub = ($x - $y)
let mul = ($x * $y)
let div = ($x / $y)
std::println($add)
std::println($sub)
std::println($mul)
std::println($div)
Parsed 11 statements
  Statement 0: Command("")
  Statement 1: Let { name: "x", value: Literal(Number(10)) }
  Statement 2: Let { name: "y", value: Literal(Number(3)) }
  Statement 3: Let { name: "add", value: BinaryOp(Variable("x"), Add, Variable("y")) }
  Statement 4: Let { name: "sub", value: BinaryOp(Variable("x"), Sub, Variable("y")) }
  Statement 5: Let { name: "mul", value: BinaryOp(Variable("x"), Mul, Variable("y")) }
  Statement 6: Let { name: "div", value: BinaryOp(Variable("x"), Div, Variable("y")) }
  Statement 7: Expression(FunctionCall("std::println", [Variable("add")]))
  Statement 8: Expression(FunctionCall("std::println", [Variable("sub")]))
  Statement 9: Expression(FunctionCall("std::println", [Variable("mul")]))
  Statement 10: Expression(FunctionCall("std::println", [Variable("div")]))
Compiled to a.out
[ali@baguette alshc]$
```

## Note:
ALSH, the shell (and scripting language),
and
ALSHC, the ALSH compiler,
have been split into different repositories.
Now, this repo only has ALSHC (alshc), the compiler, but more importantly,
the language has two different specs:
NORMAL and COMPILER. The system shell only guarantees to implement the "NORMAL" spec, while the compiler will implement the "COMPILER" spec. The key differences are:
- more strictly typed in the COMPILER spec:
```
// COMPILER spec:
function foo_func(int param1, str param2) bool { // typename of the function type goes between the declaration and the body
    // do stuff here
    return true // keyword true, a bool - has to be since that is the type of the function
}
// NORMAL spec:
function bar_func(param1, param2) {
    // do stuff here
    return $whatever
} // much more vibe-based

```
- and that's really it, for now