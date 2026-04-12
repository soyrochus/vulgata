use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use vulgata::diagnostics::Diagnostic;
use vulgata::externs::ExternRegistry;
use vulgata::runtime::{Interpreter, TestResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Ok,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Run,
    Test,
}

#[derive(Debug)]
struct Fixture {
    name: String,
    source_path: PathBuf,
    source: String,
    externs_path: Option<PathBuf>,
    externs: Option<String>,
    mode: Option<Mode>,
    parse_status: Status,
    parse_error: Option<String>,
    check_status: Status,
    check_error: Option<String>,
    run_output: Option<String>,
    run_error: Option<String>,
    compile_output: Option<String>,
    compile_error: Option<String>,
    equivalent: bool,
}

#[test]
fn conformance_fixtures_pass() {
    let fixtures = load_fixtures().expect("load fixtures");
    assert!(!fixtures.is_empty(), "expected at least one conformance fixture");

    for fixture in fixtures {
        assert_fixture(&fixture);
    }
}

fn assert_fixture(fixture: &Fixture) {
    match vulgata::parse_source(&fixture.source_path, &fixture.source) {
        Ok(_) => assert_eq!(
            fixture.parse_status,
            Status::Ok,
            "fixture `{}` parse unexpectedly succeeded",
            fixture.name
        ),
        Err(diagnostics) => {
            assert_eq!(
                fixture.parse_status,
                Status::Error,
                "fixture `{}` parse unexpectedly failed: {}",
                fixture.name,
                format_diagnostics(&diagnostics)
            );
            if let Some(expected) = &fixture.parse_error {
                assert_eq!(
                    normalize_output(expected),
                    normalize_output(&format_diagnostics(&diagnostics)),
                    "fixture `{}` parse diagnostic mismatch",
                    fixture.name
                );
            }
        }
    }

    match vulgata::check_source(&fixture.source_path, &fixture.source) {
        Ok(_) => assert_eq!(
            fixture.check_status,
            Status::Ok,
            "fixture `{}` check unexpectedly succeeded",
            fixture.name
        ),
        Err(diagnostics) => {
            assert_eq!(
                fixture.check_status,
                Status::Error,
                "fixture `{}` check unexpectedly failed: {}",
                fixture.name,
                format_diagnostics(&diagnostics)
            );
            if let Some(expected) = &fixture.check_error {
                assert_eq!(
                    normalize_output(expected),
                    normalize_output(&format_diagnostics(&diagnostics)),
                    "fixture `{}` check diagnostic mismatch",
                    fixture.name
                );
            }
        }
    }

    let mut interpreter_output = None;
    if fixture.run_output.is_some() || fixture.run_error.is_some() {
        match execute_interpreter(fixture) {
            Ok(output) => {
                if let Some(expected) = &fixture.run_output {
                    assert_eq!(
                        normalize_output(expected),
                        normalize_output(&output),
                        "fixture `{}` interpreter output mismatch",
                        fixture.name
                    );
                } else {
                    panic!(
                        "fixture `{}` interpreter unexpectedly succeeded: {}",
                        fixture.name, output
                    );
                }
                interpreter_output = Some(output);
            }
            Err(error) => {
                if let Some(expected) = &fixture.run_error {
                    assert_eq!(
                        normalize_output(expected),
                        normalize_output(&error),
                        "fixture `{}` interpreter error mismatch",
                        fixture.name
                    );
                } else {
                    panic!(
                        "fixture `{}` interpreter unexpectedly failed: {}",
                        fixture.name, error
                    );
                }
            }
        }
    }

    let mut compiled_output = None;
    if fixture.compile_output.is_some() || fixture.compile_error.is_some() {
        match execute_compiled(fixture) {
            Ok(output) => {
                if let Some(expected) = &fixture.compile_output {
                    assert_eq!(
                        normalize_output(expected),
                        normalize_output(&output),
                        "fixture `{}` compiled output mismatch",
                        fixture.name
                    );
                } else {
                    panic!(
                        "fixture `{}` compile unexpectedly succeeded: {}",
                        fixture.name, output
                    );
                }
                compiled_output = Some(output);
            }
            Err(error) => {
                if let Some(expected) = &fixture.compile_error {
                    assert_eq!(
                        normalize_output(expected),
                        normalize_output(&error),
                        "fixture `{}` compile error mismatch",
                        fixture.name
                    );
                } else {
                    panic!("fixture `{}` compile unexpectedly failed: {}", fixture.name, error);
                }
            }
        }
    }

    if fixture.equivalent {
        let interpreter_output = interpreter_output
            .as_ref()
            .unwrap_or_else(|| panic!("fixture `{}` missing interpreter output", fixture.name));
        let compiled_output = compiled_output
            .as_ref()
            .unwrap_or_else(|| panic!("fixture `{}` missing compiled output", fixture.name));
        assert_eq!(
            normalize_output(interpreter_output),
            normalize_output(compiled_output),
            "fixture `{}` interpreter/compiled equivalence mismatch",
            fixture.name
        );
    }
}

fn execute_interpreter(fixture: &Fixture) -> Result<String, String> {
    let mode = fixture
        .mode
        .unwrap_or_else(|| panic!("fixture `{}` missing mode for interpreter execution", fixture.name));

    if let Some(externs) = &fixture.externs {
        let lowered = vulgata::lower_source(&fixture.source_path, &fixture.source)
            .map_err(|diagnostics| format_diagnostics(&diagnostics))?;
        let registry = ExternRegistry::from_config_text(
            &lowered,
            fixture.externs_path.as_deref(),
            Some(externs),
        )
        .map_err(|diagnostics| format_diagnostics(&diagnostics))?;
        let interpreter =
            Interpreter::with_externs(&lowered, registry).map_err(|diagnostics| format_diagnostics(&diagnostics))?;
        return match mode {
            Mode::Run => interpreter
                .run_main()
                .map(|result| format!("{:?}", result.value))
                .map_err(|diagnostics| format_diagnostics(&diagnostics)),
            Mode::Test => interpreter
                .run_tests()
                .map(|results| format_test_results(&results))
                .map_err(|diagnostics| format_diagnostics(&diagnostics)),
        };
    }

    match mode {
        Mode::Run => vulgata::run_source(&fixture.source_path, &fixture.source)
            .map(|result| format!("{:?}", result.value))
            .map_err(|diagnostics| format_diagnostics(&diagnostics)),
        Mode::Test => vulgata::test_source(&fixture.source_path, &fixture.source)
            .map(|results| format_test_results(&results))
            .map_err(|diagnostics| format_diagnostics(&diagnostics)),
    }
}

fn execute_compiled(fixture: &Fixture) -> Result<String, String> {
    let emitted = vulgata::compile_source(&fixture.source_path, &fixture.source)
        .map_err(|diagnostics| format_diagnostics(&diagnostics))?;

    let dir = std::env::temp_dir().join("vulgata-conformance").join(&fixture.name);
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let source_path = dir.join("generated.rs");
    let binary_path = dir.join("generated-bin");
    fs::write(&source_path, emitted).map_err(|error| error.to_string())?;

    let compile = Command::new("rustc")
        .arg("--edition=2024")
        .arg(&source_path)
        .arg("-o")
        .arg(&binary_path)
        .output()
        .map_err(|error| error.to_string())?;
    if !compile.status.success() {
        return Err(normalize_output(&String::from_utf8_lossy(&compile.stderr)));
    }

    let run = Command::new(&binary_path)
        .output()
        .map_err(|error| error.to_string())?;
    let stdout = normalize_output(&String::from_utf8_lossy(&run.stdout));
    if !stdout.is_empty() {
        return Ok(stdout);
    }

    if run.status.success() {
        Ok(stdout)
    } else {
        Err(normalize_output(&String::from_utf8_lossy(&run.stderr)))
    }
}

fn format_diagnostics(diagnostics: &[Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_test_results(results: &[TestResult]) -> String {
    let mut lines = Vec::new();
    for result in results {
        if result.passed {
            lines.push(format!("PASS {}", result.name));
        } else {
            lines.push(format!("FAIL {}", result.name));
            for failure in &result.failures {
                lines.push(format!("  {} {}", failure.span, failure.message));
            }
        }
    }
    lines.join("\n")
}

fn load_fixtures() -> Result<Vec<Fixture>, String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/conformance");
    let mut dirs = fs::read_dir(&root)
        .map_err(|error| format!("failed to read fixture root {}: {error}", root.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    dirs.sort_by_key(|entry| entry.path());

    let mut fixtures = Vec::new();
    for entry in dirs {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let config_path = path.join("fixture.conf");
        let source_path = path.join("source.vg");
        let source = fs::read_to_string(&source_path)
            .map_err(|error| format!("failed to read {}: {error}", source_path.display()))?;
        let config_text = fs::read_to_string(&config_path)
            .map_err(|error| format!("failed to read {}: {error}", config_path.display()))?;
        let config = parse_config(&config_text)?;

        let externs_path = path.join("externs.toml");
        let externs = if externs_path.exists() {
            Some(
                fs::read_to_string(&externs_path)
                    .map_err(|error| format!("failed to read {}: {error}", externs_path.display()))?,
            )
        } else {
            None
        };

        fixtures.push(Fixture {
            name: path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| format!("invalid fixture name for {}", path.display()))?
                .to_string(),
            source_path,
            source,
            externs_path: if externs.is_some() { Some(externs_path) } else { None },
            externs,
            mode: config.get("mode").map(|value| parse_mode(value)).transpose()?,
            parse_status: parse_status(config.get("parse_status").map(String::as_str).unwrap_or("ok"))?,
            parse_error: config.get("parse_error").cloned(),
            check_status: parse_status(config.get("check_status").map(String::as_str).unwrap_or("ok"))?,
            check_error: config.get("check_error").cloned(),
            run_output: config.get("run_output").cloned(),
            run_error: config.get("run_error").cloned(),
            compile_output: config.get("compile_output").cloned(),
            compile_error: config.get("compile_error").cloned(),
            equivalent: config
                .get("equivalent")
                .map(|value| parse_bool(value))
                .transpose()?
                .unwrap_or(false),
        });
    }

    Ok(fixtures)
}

fn parse_config(text: &str) -> Result<std::collections::HashMap<String, String>, String> {
    let mut config = std::collections::HashMap::new();
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("invalid config line `{line}`"))?;
        config.insert(key.trim().to_string(), parse_value(value.trim())?);
    }
    Ok(config)
}

fn parse_value(raw: &str) -> Result<String, String> {
    if raw.starts_with('"') && raw.ends_with('"') && raw.len() >= 2 {
        let inner = &raw[1..raw.len() - 1];
        let mut value = String::new();
        let mut chars = inner.chars();
        while let Some(ch) = chars.next() {
            if ch != '\\' {
                value.push(ch);
                continue;
            }

            let escaped = chars.next().ok_or_else(|| "unterminated escape sequence".to_string())?;
            match escaped {
                'n' => value.push('\n'),
                'r' => value.push('\r'),
                't' => value.push('\t'),
                '\\' => value.push('\\'),
                '"' => value.push('"'),
                other => return Err(format!("unsupported escape `\\{other}`")),
            }
        }
        Ok(value)
    } else {
        Ok(raw.to_string())
    }
}

fn parse_status(raw: &str) -> Result<Status, String> {
    match raw {
        "ok" => Ok(Status::Ok),
        "error" => Ok(Status::Error),
        other => Err(format!("unsupported status `{other}`")),
    }
}

fn parse_mode(raw: &str) -> Result<Mode, String> {
    match raw {
        "run" => Ok(Mode::Run),
        "test" => Ok(Mode::Test),
        other => Err(format!("unsupported mode `{other}`")),
    }
}

fn parse_bool(raw: &str) -> Result<bool, String> {
    match raw {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(format!("unsupported boolean `{other}`")),
    }
}

fn normalize_output(text: impl AsRef<str>) -> String {
    let normalized = text.as_ref().replace("\r\n", "\n");
    let manifest_dir = env!("CARGO_MANIFEST_DIR").replace('\\', "/");
    normalized
        .replace(&format!("{manifest_dir}/"), "")
        .replace(&manifest_dir, ".")
        .trim_end()
        .to_string()
}
