use std::fs;
use std::path::{Path, PathBuf};

use crate::diagnostics::{Diagnostic, Phase};
use crate::runtime::ExecutionMode;

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
            let options = parse_file_command_options(&args, "parse", false)?;
            let source = read_source(&options.source_path)?;
            let module = crate::parse_source(&options.source_path, &source)?;
            maybe_write_metadata(&options, &options.source_path, &source)?;
            println!("{module:#?}");
            Ok(())
        }
        "check" => {
            let options = parse_file_command_options(&args, "check", false)?;
            let source = read_source(&options.source_path)?;
            let lowered = crate::lower_source(&options.source_path, &source)?;
            maybe_write_metadata(&options, &options.source_path, &source)?;
            println!(
                "check succeeded: module {:?} lowered {} declarations",
                lowered.module_name,
                lowered.declarations.len()
            );
            Ok(())
        }
        "run" => {
            let options = parse_file_command_options(&args, "run", true)?;
            let source = read_source(&options.source_path)?;
            maybe_write_metadata(&options, &options.source_path, &source)?;
            let result = crate::run_source_in_mode(
                &options.source_path,
                &source,
                options.mode.unwrap_or(ExecutionMode::Release),
            )?;
            println!("{:?}", result.value);
            Ok(())
        }
        "test" => {
            let options = parse_file_command_options(&args, "test", true)?;
            let source = read_source(&options.source_path)?;
            maybe_write_metadata(&options, &options.source_path, &source)?;
            let results = crate::test_source_in_mode(
                &options.source_path,
                &source,
                options.mode.unwrap_or(ExecutionMode::Checked),
            )?;
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
                    crate::diagnostics::SourceSpan::for_path(&options.source_path, 1, 1),
                    Phase::Runtime,
                    format!("{failed} test(s) failed"),
                )])
            }
        }
        "compile" => {
            let options = parse_file_command_options(&args, "compile", false)?;
            let source = read_source(&options.source_path)?;
            maybe_write_metadata(&options, &options.source_path, &source)?;
            let emitted = crate::compile_source(&options.source_path, &source)?;
            let output_path = options.source_path.with_extension("rs");
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
        "repl" => {
            let mode = parse_repl_mode(&args)?.unwrap_or(ExecutionMode::Tooling);
            crate::repl::run_repl_terminal_with_mode(mode)
        }
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

struct FileCommandOptions {
    source_path: PathBuf,
    mode: Option<ExecutionMode>,
    emit_metadata_path: Option<PathBuf>,
}

fn parse_file_command_options(
    args: &[String],
    command: &str,
    allow_mode: bool,
) -> Result<FileCommandOptions, Vec<Diagnostic>> {
    let mut source_path = None;
    let mut mode = None;
    let mut emit_metadata_path = None;
    let mut index = 2;

    while index < args.len() {
        match args[index].as_str() {
            "--mode" => {
                if !allow_mode {
                    return Err(vec![cli_error(
                        format!("`{command}` does not support `--mode`"),
                    )]);
                }
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| vec![cli_error("missing value after `--mode`")])?;
                mode = Some(parse_mode(value)?);
                index += 2;
            }
            "--emit-metadata" => {
                let value = args.get(index + 1).ok_or_else(|| {
                    vec![cli_error("missing output path after `--emit-metadata`")]
                })?;
                emit_metadata_path = Some(PathBuf::from(value));
                index += 2;
            }
            value if value.starts_with('-') => {
                return Err(vec![cli_error(format!("unknown option `{value}`"))]);
            }
            value => {
                if source_path.is_some() {
                    return Err(vec![cli_error(format!(
                        "unexpected extra positional argument `{value}`"
                    ))]);
                }
                source_path = Some(PathBuf::from(value));
                index += 1;
            }
        }
    }

    let source_path = source_path.ok_or_else(|| {
        vec![Diagnostic::new(
            crate::diagnostics::SourceSpan::new("<cli>", 1, 1),
            Phase::Cli,
            format!("usage: vulgata {command} [options] <source-file>"),
        )]
    })?;

    Ok(FileCommandOptions {
        source_path,
        mode,
        emit_metadata_path,
    })
}

fn parse_repl_mode(args: &[String]) -> Result<Option<ExecutionMode>, Vec<Diagnostic>> {
    let mut mode = None;
    let mut index = 2;
    while index < args.len() {
        match args[index].as_str() {
            "--mode" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| vec![cli_error("missing value after `--mode`")])?;
                mode = Some(parse_mode(value)?);
                index += 2;
            }
            value => return Err(vec![cli_error(format!("unknown repl argument `{value}`"))]),
        }
    }
    Ok(mode)
}

fn parse_mode(value: &str) -> Result<ExecutionMode, Vec<Diagnostic>> {
    ExecutionMode::parse_cli(value).ok_or_else(|| {
        vec![cli_error(format!(
            "unknown execution mode `{value}`; expected one of: release, checked, debug, tooling"
        ))]
    })
}

fn maybe_write_metadata(
    options: &FileCommandOptions,
    source_path: &Path,
    source: &str,
) -> Result<(), Vec<Diagnostic>> {
    let Some(output_path) = &options.emit_metadata_path else {
        return Ok(());
    };

    let metadata = crate::emit_metadata_source(source_path, source)?;
    let mut bytes = serde_json::to_vec_pretty(&metadata).map_err(|error| {
        vec![Diagnostic::new(
            crate::diagnostics::SourceSpan::for_path(source_path, 1, 1),
            Phase::Cli,
            format!("failed to serialise metadata JSON: {error}"),
        )]
    })?;
    bytes.push(b'\n');

    fs::write(output_path, bytes).map_err(|error| {
        vec![Diagnostic::new(
            crate::diagnostics::SourceSpan::for_path(output_path, 1, 1),
            Phase::Cli,
            format!("failed to write metadata file: {error}"),
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

fn cli_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        crate::diagnostics::SourceSpan::new("<cli>", 1, 1),
        Phase::Cli,
        message,
    )
}

fn print_usage() {
    println!("Usage: vulgata <parse|check|run|test|compile> [options] <source-file>");
    println!("       vulgata repl [--mode <release|checked|debug|tooling>]");
    println!("Options:");
    println!("  --mode <release|checked|debug|tooling>  Select execution mode for run/test/repl");
    println!("  --emit-metadata <path>                   Write semantic-layer metadata JSON");
}
