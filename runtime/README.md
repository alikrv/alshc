ALSH runtime helpers

This directory contains a minimal C runtime with arena-based allocation and string helpers used by the compiler codegen.

Build and link:

```sh
cc -c alsh_runtime.c -o alsh_runtime.o
# The compiler will link this object when producing the final executable
```