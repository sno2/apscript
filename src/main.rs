use codespan_reporting::{
    diagnostic::{Diagnostic, Label},
    files::SimpleFiles,
    term::{
        self,
        termcolor::{ColorChoice, StandardStream},
    },
};
use parser::Parser;

use crate::vm::{Value, VM};

mod ast;
mod lexer;
mod parser;
mod stdlib;
mod vm;

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
        }));
    }};
    ($msg: expr, $span: expr) => {{
        return Value::Exception(Box::new(crate::vm::Exception {
            message: $msg.into(),
            span: $span,
        }));
    }};
}

fn main() {
    let input = std::fs::read_to_string("foo.aps").unwrap();
    // let mut lex = Lexer::new(input.as_bytes());

    // loop {
    //     lex.next();
    //     println!("{:4} {:?}", lex.start, lex.token);
    //     if lex.token == Token::EOF {
    //         break;
    //     }
    // }

    let mut files = SimpleFiles::new();
    let fid = files.add("foo.aps", &input);

    let mut parser = Parser::new(fid, input.as_bytes());
    parser.lex.next();

    let value = parser.parse_scope(true);

    if parser.diagnostics.len() != 0 {
        let writer = StandardStream::stderr(ColorChoice::Always);
        let config = codespan_reporting::term::Config::default();
        let mut writer = writer.lock();

        for diagnostic in parser.diagnostics.iter() {
            term::emit(&mut writer, &config, &files, diagnostic).unwrap();
        }

        return;
    }

    let mut vm = VM::new(&input);
    stdlib::inject(&mut vm);

    let value = vm.eval_scope(&value.unwrap());
    if let Value::Exception(e) = &value {
        let writer = StandardStream::stderr(ColorChoice::Always);
        let config = codespan_reporting::term::Config::default();
        let mut writer = writer.lock();
        term::emit(
            &mut writer,
            &config,
            &files,
            &Diagnostic::error()
                .with_message(&e.message)
                .with_labels(vec![Label::primary(fid, e.span)]),
        )
        .unwrap();
    }
}
