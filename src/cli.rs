use std::fs;
use std::path::PathBuf;

use crate::diagnostics::{Diagnostic, Phase};

pub fn run(args: Vec<String>) -> i32 {
    match dispatch(args) {
        Ok(()) => 0,
        Err(diagnostics) => {
            for diagnostic in diagnostics {
                eprintln!("{diagnostic}");
            }
            1
        }
    }
}

fn dispatch(args: Vec<String>) -> Result<(), Vec<Diagnostic>> {
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    match args[1].as_str() {
        "parse" => {
            let path = require_input_path(&args, "parse")?;
            let source = read_source(&path)?;
            let module = crate::parse_source(&path, &source)?;
            println!("{module:#?}");
            Ok(())
        }
        "check" => {
            let path = require_input_path(&args, "check")?;
            let source = read_source(&path)?;
            let lowered = crate::lower_source(&path, &source)?;
            println!(
                "check succeeded: module {:?} lowered {} declarations",
                lowered.module_name,
                lowered.declarations.len()
            );
            Ok(())
        }
        "run" => {
            let path = require_input_path(&args, "run")?;
            let source = read_source(&path)?;
            let result = crate::run_source(&path, &source)?;
            println!("{:?}", result.value);
            Ok(())
        }
        "test" => {
            let path = require_input_path(&args, "test")?;
            let source = read_source(&path)?;
            let results = crate::test_source(&path, &source)?;
            let mut failed = 0;
            for result in &results {
                if result.passed {
                    println!("PASS {}", result.name);
                } else {
                    failed += 1;
                    println!("FAIL {}", result.name);
                    for failure in &result.failures {
                        println!("  {} {}", failure.span, failure.message);
                    }
                }
            }
            if failed == 0 {
                Ok(())
            } else {
                Err(vec![Diagnostic::new(
                    crate::diagnostics::SourceSpan::for_path(&path, 1, 1),
                    Phase::Runtime,
                    format!("{failed} test(s) failed"),
                )])
            }
        }
        "compile" => {
            let path = require_input_path(&args, "compile")?;
            let source = read_source(&path)?;
            let emitted = crate::compile_source(&path, &source)?;
            let output_path = path.with_extension("rs");
            fs::write(&output_path, emitted).map_err(|error| {
                vec![Diagnostic::new(
                    crate::diagnostics::SourceSpan::for_path(&output_path, 1, 1),
                    Phase::Cli,
                    format!("failed to write generated Rust source: {error}"),
                )]
            })?;
            println!("{}", output_path.display());
            Ok(())
        }
        "repl" => crate::repl::run_repl_terminal(),
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        other => Err(vec![Diagnostic::new(
            crate::diagnostics::SourceSpan::new("<cli>", 1, 1),
            Phase::Cli,
            format!("unknown subcommand `{other}`"),
        )]),
    }
}

fn require_input_path(args: &[String], command: &str) -> Result<PathBuf, Vec<Diagnostic>> {
    args.get(2).map(PathBuf::from).ok_or_else(|| {
        vec![Diagnostic::new(
            crate::diagnostics::SourceSpan::new("<cli>", 1, 1),
            Phase::Cli,
            format!("usage: vulgata {command} <source-file>"),
        )]
    })
}

fn read_source(path: &PathBuf) -> Result<String, Vec<Diagnostic>> {
    fs::read_to_string(path).map_err(|error| {
        vec![Diagnostic::new(
            crate::diagnostics::SourceSpan::for_path(path, 1, 1),
            Phase::Cli,
            format!("failed to read source file: {error}"),
        )]
    })
}

fn print_usage() {
    println!("Usage: vulgata <parse|check|run|test|compile> <source-file>");
    println!("       vulgata repl");
}
