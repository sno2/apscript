# APScript (aps)

A speedy interpreter for the AP Computer Science Principles pseudocode language built in Rust.

## Features

- Robust garbage collector (using `gc`)
- Pitch-perfect specification conformity
- Beautiful error logging for parsing and runtime errors (using `codespan-reporting`)
- Hackable virtual machine design

### Error handling

The parser is built for accomodating for many syntax errors:

## Language Guide

A brief introductory guide to the language. If you get lost, please file an
issue because it is _our_ fault for not explaining it better and others will
be confused if you are.

#### Table of Contents

1. [Variables](#variables)
2. [Lists](#lists)
3. [Conditionals](#conditionals)
4. [Loops](#loops)
5. [I/O](#io)

### Variables

Variables allow you to store data into named locations and can be created using
the syntax `name <- value`.

```
age <- 20
```

Variables can be complex expressions like math and follow the order of
operations.

```
age <- 20 + 5 * 2
```

We will represent the state of the program using tables like this. For example,
running the above script will result in a state table with the following data:

| Variable | Value |
| -------- | ----- |
| age      | `30`  |

You can also use other variables when assigning to variables.

```
age <- 20
age <- age + 5
```

| State         | Value |
| ------------- | ----- |
| age (initial) | `20`  |
| age (final)   | `25`  |

### Lists

Lists allow you to store multiple values in the same data type. You can create
lists by inserting comma-separated value expressions between brackets.

```
ages1 <- [] # creates an empty list
ages2 <- [2, 3, 4]
```

You can get the number of items in an array by using the `LENGTH` function:

```
ages <- [2, 3, 4]
agesLength <- LENGTH(ages)
```

| Variable   | Value       |
| ---------- | ----------- |
| ages       | `[2, 3, 4]` |
| agesLength | `3`         |

In order to get values at specified indexes (positions) in an array, you can use
include brackets after an array name. Note that the index of the first value is
1, in contrast to many other programming languages.

```
ages <- [40, 24, 36]

ages1 <- ages[1]
ages2 <- ages[2]
ages3 <- ages[3]
```

| Variable | Value          |
| -------- | -------------- |
| ages     | `[40, 24, 36]` |
| ages1    | `40`           |
| ages2    | `24`           |
| ages3    | `36`           |

Also, it is possible to dynamically add items to a list using the `APPEND`
function.

```
ages <- [60]
APPEND(ages, 5)
APPEND(ages, 10)
APPEND(ages, 8)
APPEND(ages, 16)
```

| State          | Value                |
| -------------- | -------------------- |
| ages (initial) | `[60]`               |
| ages (final)   | `[60, 5, 10, 8, 16]` |

See the [Standard Library reference](#standard-library) for information on all
of the functions for manipulating lists.

### Conditionals

The `IF` statement can be used to conditionally run a block of statements based
on if an expression is true or false.

```
IF (TRUE) {
  # this code will run because the expression is true
}
# code after here will still run
```

```
IF (FALSE) {
  # this code will not run because the expression is false
}
# code after here will still run
```

```
age <- 16
IF (age >= 18) {
  #  this code will not run because the expression is false
}
# code after here will still run
```

`IF` statements can be combined with a trailing `ELSE` statement which will only
run if the `IF` statement's expression was false.

### Loops

The pseudocode includes three different kinds of loops. The most basic loop is
the `REPEAT n` loop which will run the block `n` times.

```
foo <- []
REPEAT 5 TIMES {
  APPEND(foo, 1)
}
```

| State         | Value             |
| ------------- | ----------------- |
| foo (initial) | `[]`              |
| foo (final)   | `[1, 1, 1, 1, 1]` |

### I/O

I/O stands for input/output, or methods that the outside parts can interact with
your program. You can interact with your program using the console.

The `DISPLAY` function is used to log out to the console.

```
DISPLAY(5)
```

```
5
```

You can also pass in string arguments and it will log them out with spaces in
between.

```
DISPLAY(6, 8)
```

```
6 8
```

Furthermore, strings can be used to log text to the console.

```
DISPLAY("Hello user!")
DISPLAY("How are you?")
```

```
Hello user!
How are you?
```

The `INPUT` function can be used to get input from the user. The same sort of
arguments can be passed to the function and logged before the input is asked
for. Also, the program will pause until the user enters input.

```
INPUT("What is your name?")
```

```
What is your name? [waits for input]
```

`INPUT` will return a string that you can use in other `INPUT` and `DISPLAY`
calls later.

```
favColor <- INPUT("What is your favorite color?")
DISPLAY("Woah! My favorite color is", favColor, "too!")
```

```

What is your favorite color? blue
Woah! My favorite color is blue too!

```

`INPUT` will return a number if the user entered in a valid number.

```
age <- INPUT("What is your age?")
DISPLAY("Well, you will be", age + 1, "next year!")
```

```
What is your age? 15
Well, you will be 16 next year!
```

## Standard Library

The standard library as specified by the [AP Computer Science Principles Pseudocode Exam Reference Sheet](https://apcentral.collegeboard.org/media/pdf/ap-computer-science-principles-exam-reference-sheet.pdf).

### `LENGTH(list)`

Returns the number of items in `list`.

```
ages <- [16, 24, 3]
agesLength <- LENGTH(ages)
```

| Variable   | Value         |
| ---------- | ------------- |
| ages       | `[16, 24, 3]` |
| agesLength | `3`           |

### `INSERT(list, i, value)`

Any values in `list` at indices greater than or equal to `i` are shifted to the
right. The length of `list` is increased by 1, and `value` is placed at index
`i` in `list`.

```
ages <- [100, 200, 300]
INSERT(ages, 1, 6)
```

| Variable       | Value                |
| -------------- | -------------------- |
| ages (initial) | `[100, 200, 300]`    |
| ages (final)   | `[100, 6, 200, 300]` |

### `APPEND(list, value)`

The length of `list` is increased by 1, and `value` is placed at the end of
`list`.

### `REMOVE(list, i)`

Removes the item at index `i` in `list` and shifts to the left any values at
indices greater than `i`. The length of list is decreased by `1`.

### `DISPLAY(value1, ...)`

Writes all of the given arguments to the console separated by spaces.

### `INPUT(hint1, ...)`

Writes all of the given arguments to the console separated by spaces in the same
format of `DISPLAY`. After that, it waits for input and returns the parsed
input. If the returned input is a number, then it parses the number and returns
it. Otherwise, it returns the input as a string.

```sql
fav <- INPUT("What's your favorite color?")
DISPLAY("Cool! My favorite color is", fav, "too!")
```

```
$ aps run [myfile.aps]
  What's your favorite color? [input: blue]
  Cool! My favorite color is blue too!
```

> Note: All of the builtins are standalone function pointers wrapped as values
> in the interpreter. You can view the source of any of them in `src/stdlib.rs`
> and add your own builtins by appending them to the scope of `VM`.

## Notes

- `FOR EACH _ IN _` only goes through indices that were present at the start of
  the block. It does not, for example, go on forever if you were to append items
  to the list in the middle of the loop because it uses a cached length of the
  array. If the array had a few items removed while iterating, then the loop
  will simply terminate silently.

## License

aps is licensed under the MIT License.
