# Assignment 3 – Cobra Compiler

## Overview
This project implements the **Cobra language compiler** for Assignment 3.  
It extends the previous compiler to support:

- Booleans
- Conditionals (`if`)
- Loops (`loop`, `break`)
- Mutation (`set!`)
- Runtime type checking
- Tagged value representation

The compiler translates Cobra programs into **x86-64 assembly** that can be assembled and executed.

---

## Tagged Value Representation

The language uses tagged values to distinguish between numbers and booleans.

| Type | Representation |
|------|---------------|
| Numbers | Shift left by 1 (LSB = 0) |
| true | `0b11` (3) |
| false | `0b01` (1) |

Example:

```
5  -> 10
true  -> 3
false -> 1
```

---

## Supported Language Features

The compiler supports the following Cobra constructs:

### Values
- Numbers
- Booleans (`true`, `false`)
- `input`

### Variables
- Identifiers
- `let` bindings
- Variable mutation using `set!`

### Unary Operations
- `add1`
- `sub1`
- `negate`
- `isnum`
- `isbool`

### Binary Operations
- `+`
- `-`
- `*`
- `<`
- `>`
- `<=`
- `>=`
- `=`

### Control Flow
- `if` expressions
- `block`
- `loop`
- `break`

---

## Runtime Error Handling

The compiler implements runtime checks for:

### Invalid Argument
Occurs when operations are used with incorrect types.

Example:

```
(+ true 5)
```

Output:

```
invalid argument
```

### Overflow
Occurs when arithmetic operations exceed the integer range.

---

## Example Programs

### Example 1 – Conditionals
```
(if true 5 10)
```

Result:
```
5
```

---

### Example 2 – Comparison
```
(< 3 5)
```

Result:
```
true
```

---

### Example 3 – Loop with Break
```
(let ((x 0))
  (loop
    (if (= x 10)
        (break x)
        (set! x (+ x 1)))))
```

Result:
```
10
```

---

## Implementation Details

### Compiler Components
The compiler includes:

- Parser for Cobra syntax
- Abstract Syntax Tree (AST)
- Code generation to x86 assembly
- Runtime error handling
- Stack management

### Label Generation
Unique labels are generated for:
- `if`
- `loop`
- `break`

This ensures correct control flow in generated assembly.

### Stack Allocation
The compiler calculates the maximum stack usage before execution and allocates space accordingly.

---

## Testing

The project includes tests covering:

- Boolean operations
- Arithmetic operations
- Comparisons
- Nested conditionals
- Loops with break
- Mutation using `set!`
- Runtime error cases

Total tests implemented: **20+**

---

## How to Build

Build the compiler:

```
cargo build
```

---

## How to Compile a Program

```
make <program>
```

Example:

```
make test/test1.snek
```

---

## Run the Program

```
./test/test1.run
```

---

## Project Structure

```
assignment3-cobra
│
├── starter-code
│   ├── src
│   │   └── main.rs
│   ├── runtime
│   │   └── start.rs
│   ├── tests
│   └── Makefile
│
└── README.md
```

---

## What I Learned

Through this assignment, I learned:

- How tagged values work in dynamic languages
- How compilers generate assembly code
- Implementing control flow in assembly
- Runtime type checking
- Handling mutable variables in a compiler

---

## Author

Shubham Kafle  
CSCI 282 – Compiler Construction