# aps_core

The core lexer/parser/virtual machine/WASM bindings for aps.

## Building

### JS (WASM)

```sh
$ cargo build --package aps_core --no-default-features --features=js --target=wasm32-unknown-unknown --release
$ wasm-bindgen target/wasm32-unknown-unknown/release/aps_core.wasm --web --out-dir=editor/lib
```

Watching mode:

```sh
$ cargo watch -s "cargo build --package aps_core --no-default-features --features=js --target=wasm32-unknown-unknown --release && wasm-bindgen target/wasm32-unknown-unknown/release/aps_core.wasm --web --out-dir=editor/lib"
```

### Regular

```sh
$ cargo build
```
