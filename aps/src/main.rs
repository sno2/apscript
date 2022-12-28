use std::{cell::RefCell, collections::HashMap, rc::Rc};

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
    vm::{Env, Value, VM},
};

use clap::{Parser as ClapParser, Subcommand};

#[derive(Debug, ClapParser)]
#[command(name = "aps")]
#[command(about = "An interpreter for the AP Pseudocode language", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Runs a given file.
    #[command(arg_required_else_help = true)]
    Run { file: String },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    match args.command {
        Commands::Run { file } => {
            let input = std::fs::read_to_string(&file)?;

            let mut files = SimpleFiles::new();
            let fid = files.add(&file, &input);

            let mut parser = Parser::new(fid, input.as_bytes());
            parser.lex.next();

            let value = parser.parse_scope(true);

            if parser.diagnostics.len() != 0 {
                let writer = StandardStream::stderr(ColorChoice::Always);
                let config = codespan_reporting::term::Config::default();
                let mut writer = writer.lock();

                for diagnostic in parser.diagnostics.iter() {
                    term::emit(&mut writer, &config, &files, diagnostic)?;
                }
                std::process::exit(1);
            }

            let mut vm = VM::new(&input);

            let mut env = Env::new();
            stdlib::inject(&mut env);
            let value = vm.eval_scope(&value.unwrap(), Rc::new(RefCell::new(env)));

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
                )?;

                for itm in e.stack.iter() {
                    term::emit(
                        &mut writer,
                        &config,
                        &files,
                        &Diagnostic::note()
                            .with_message("called here")
                            .with_labels(vec![Label::primary(fid, *itm)]),
                    )?;
                }

                std::process::exit(1);
            }
        }
    }

    Ok(())
}
