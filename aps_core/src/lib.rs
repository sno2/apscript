pub mod ast;
pub mod lexer;
pub mod parser;
pub mod stdlib;
pub mod vm;

#[cfg(feature = "js")]
mod wasm;

#[cfg(feature = "js")]
pub use wasm::*;

#[macro_export]
macro_rules! tee {
    ($e: expr) => {{
        let value = $e;
        if let Value::Exception(_) = value {
            return value;
        } else {
            value
        }
    }};
}

#[macro_export]
macro_rules! fail {
    ($msg: expr, BUILTIN) => {{
        return Value::Exception(Box::new(crate::vm::Exception {
            message: $msg.into(),
            span: crate::ast::Span { start: 0, end: 0 },
            stack: Vec::new(),
        }));
    }};
    ($msg: expr, $span: expr) => {{
        return Value::Exception(Box::new(crate::vm::Exception {
            message: $msg.into(),
            span: $span,
            stack: Vec::new(),
        }));
    }};
}
