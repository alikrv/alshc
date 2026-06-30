## New features to be added
will be deleted from this here note when added

- `std::sizeof($any_var_here)` - size in memory (in bytes) of the provided variable
- `std::sizeoft(<typename>)` - size in memory (in bytes) of the specified type
- `std::typeof($var_here)` - the type of the provided variable.
- type casting
instead of the C syntax:
```c
float i = 10.1f;
int ii = 2 + (int)i;
```
we'll use a more readable syntax: `as` <- new keyword!
```
let i = 10.1f // gets inferred to a float
let ii = 2 + $i as int // an int, and `i` gets casted to int as well
```

that's typecasting - also add new library funcs to verbosely, explcititly convert variables types:
- `std::str_to_int`
- `std::str_to_float`
- `std::str_to_double`
- `std::str_to_bool`
- `std::str_to_char`
- `std::str_to_cstr` // a `char *`

- `std::int_to_float`
- `std::int_to_double`
- `std::int_to_bool`
- `std::int_to_char`
- `std::int_to_str`

- `std::float_to_int`
- `std::float_to_double`
- `std::float_to_bool`
- `std::float_to_char`
- `std::float_to_str`

- `std::double_to_int`
- `std::double_to_float`
- `std::double_to_char`
- `std::double_to_bool`
- `std::double_to_str`

- `std::char_to_int`
- `std::char_to_float`
- `std::char_to_double`
- `std::char_to_bool`
- `std::char_to_str`

- `std::bool_to_int`
- `std::bool_to_float`
- `std::bool_to_double`
- `std::bool_to_char`
- `std::bool_to_str`