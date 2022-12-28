# aps_core

The core lexer/parser/virtual machine/WASM bindings for aps.

## Building

### JS (WASM)

Run the following command in the root of the repository to build the package in
the `editor` directory.

```sh
$ wasm-pack build aps_core --target web --out-dir=../editor/lib --features=js --no-default-features
```

### Regular

```sh
$ cargo build
```
