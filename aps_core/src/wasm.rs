use crate::{
    lexer::Token,
    parser::Parser,
    stdlib,
    vm::{Env, Value, VM},
};
use codespan_reporting::{
    diagnostic::{Diagnostic, Label},
    files::{self, SimpleFiles},
    term::{
        self,
        termcolor::{ColorChoice, StandardStream, WriteColor},
    },
};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap, io::Write as WriteIO, rc::Rc};
use std::{fmt::Write as WriteFmt, slice::Iter};
use wasm_bindgen::{prelude::*, JsObject};

struct S(String);

impl WriteIO for S {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0
            .write_str(unsafe { std::str::from_utf8_unchecked(buf) });
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl WriteColor for S {
    fn supports_color(&self) -> bool {
        false
    }

    fn set_color(&mut self, spec: &term::termcolor::ColorSpec) -> std::io::Result<()> {
        Ok(())
    }

    fn reset(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[wasm_bindgen]
pub fn validate(input: &str) -> Result<JsValue, JsValue> {
    let mut files = SimpleFiles::new();
    let fid = files.add("<file>", &input);

    let mut parser = Parser::new(fid, input.as_bytes());
    parser.lex.next();

    _ = parser.parse_scope(true);

    if parser.diagnostics.len() == 0 && parser.lex.token != Token::EOF {
        parser.diagnostics.push(
            Diagnostic::error()
                .with_message(format!(
                    "expected statement, found {}",
                    parser.lex.token.as_ref(),
                ))
                .with_labels(vec![Label::primary(parser.fid, parser.lex.span())
                    .with_message(format!("expected statement"))]),
        );
    }

    Ok(serde_wasm_bindgen::to_value(&parser.diagnostics)?)
}

#[derive(Serialize, Deserialize)]
enum RunStatus {
    Ok,
    Data {
        log: String,
        errors: Vec<Diagnostic<usize>>,
    },
}

#[wasm_bindgen]
pub fn interpret(input: &str) -> Result<JsValue, JsValue> {
    let mut files = SimpleFiles::new();
    let fid = files.add("<file>", &input);

    let mut parser = Parser::new(fid, input.as_bytes());
    parser.lex.next();

    let scope = parser.parse_scope(true);

    if parser.diagnostics.len() != 0 || parser.lex.token != Token::EOF {
        let mut writer = S(String::new());
        let config = codespan_reporting::term::Config::default();

        if parser.lex.token != Token::EOF {
            parser.diagnostics.push(
                Diagnostic::error()
                    .with_message(format!(
                        "expected statement, found {}",
                        parser.lex.token.as_ref(),
                    ))
                    .with_labels(vec![Label::primary(parser.fid, parser.lex.span())
                        .with_message(format!("expected statement"))]),
            );
        }

        for diagnostic in parser.diagnostics.iter() {
            term::emit(&mut writer, &config, &files, diagnostic);
        }

        return Ok(serde_wasm_bindgen::to_value(&RunStatus::Data {
            log: writer.0,
            errors: parser.diagnostics,
        })?);
    }

    let mut vm = VM::new(input);

    let mut env = Env::new();
    stdlib::inject(&mut env);
    let value = vm.eval_scope(&scope.unwrap(), Rc::new(RefCell::new(env)));

    if let Value::Exception(e) = &value {
        let config = codespan_reporting::term::Config::default();
        let mut writer = S(String::new());

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

        let mut diags = [Diagnostic::error()
            .with_message(&e.message)
            .with_labels(vec![Label::primary(fid, e.span)])]
        .into_iter()
        .chain(e.stack.iter().map(|itm| {
            Diagnostic::note()
                .with_message("called here")
                .with_labels(vec![Label::primary(fid, *itm)])
        }));

        return Ok(serde_wasm_bindgen::to_value(&RunStatus::Data {
            log: writer.0,
            errors: diags.collect(),
        })?);
    }

    Ok(serde_wasm_bindgen::to_value(&RunStatus::Ok)?)
}
