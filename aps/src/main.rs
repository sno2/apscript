use codespan_reporting::{
    diagnostic::{Diagnostic, Label},
    files::SimpleFiles,
    term::{
        self,
        termcolor::{ColorChoice, StandardStream},
    },
};

use aps_core::{
    parser::Parser,
    stdlib,
    vm::{Value, VM},
};

fn main() {
    let input = std::fs::read_to_string("foo.aps").unwrap();

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

        for itm in e.stack.iter() {
            term::emit(
                &mut writer,
                &config,
                &files,
                &Diagnostic::note()
                    .with_message("called here")
                    .with_labels(vec![Label::primary(fid, *itm)]),
            )
            .unwrap();
        }
    }
}
