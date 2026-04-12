use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use crate::ast::Decl;
use crate::diagnostics::{Diagnostic, Phase, SourceSpan};
use crate::runtime::TestResult;

const REPL_PATH: &str = "<repl>/session.vg";

pub struct ReplSession {
    path: PathBuf,
    source: String,
}

#[derive(Debug)]
pub enum ReplCommand {
    Continue(Vec<String>),
    Quit(Vec<String>),
}

impl ReplSession {
    pub fn new() -> Self {
        Self {
            path: PathBuf::from(REPL_PATH),
            source: String::new(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn submit_block(&mut self, block: &str) -> Result<String, Vec<Diagnostic>> {
        let candidate = if self.source.trim().is_empty() {
            block.trim_end().to_string()
        } else {
            format!("{}\n\n{}", self.source.trim_end(), block.trim_end())
        };
        crate::check_source(&self.path, &candidate)?;
        self.source = candidate;
        Ok("ok: block added".to_string())
    }

    pub fn handle_command(&mut self, command: &str) -> Result<ReplCommand, Vec<Diagnostic>> {
        match command.trim() {
            ":help" => Ok(ReplCommand::Continue(vec![help_text().to_string()])),
            ":show" => Ok(ReplCommand::Continue(vec![if self.source.trim().is_empty() {
                "(empty session)".to_string()
            } else {
                self.source.clone()
            }])),
            ":reset" => {
                self.source.clear();
                Ok(ReplCommand::Continue(vec!["session reset".to_string()]))
            }
            ":parse" => {
                let module = crate::parse_source(&self.path, &self.source)?;
                Ok(ReplCommand::Continue(vec![format!("{module:#?}")]))
            }
            ":check" => {
                let lowered = crate::lower_source(&self.path, &self.source)?;
                Ok(ReplCommand::Continue(vec![format!(
                    "check succeeded: module {:?} lowered {} declarations",
                    lowered.module_name,
                    lowered.declarations.len()
                )]))
            }
            ":run" => {
                self.ensure_repl_execution_supported()?;
                let result = crate::run_source(&self.path, &self.source)?;
                Ok(ReplCommand::Continue(vec![format!("{:?}", result.value)]))
            }
            ":test" => {
                self.ensure_repl_execution_supported()?;
                let results = crate::test_source(&self.path, &self.source)?;
                Ok(ReplCommand::Continue(format_test_results(&results)))
            }
            ":quit" => Ok(ReplCommand::Quit(Vec::new())),
            other => Err(vec![Diagnostic::new(
                SourceSpan::new(REPL_PATH, 1, 1),
                Phase::Cli,
                format!("unknown repl command `{other}`"),
            )]),
        }
    }

    fn ensure_repl_execution_supported(&self) -> Result<(), Vec<Diagnostic>> {
        let module = crate::parse_source(&self.path, &self.source)?;
        if module
            .declarations
            .iter()
            .any(|decl| matches!(decl, Decl::Extern(_)))
        {
            return Err(vec![Diagnostic::new(
                SourceSpan::for_path(&self.path, 1, 1),
                Phase::Cli,
                "repl does not yet support extern-backed execution; remove extern declarations or use file-based commands",
            )]);
        }
        Ok(())
    }
}

pub fn run_repl<R: BufRead, W: Write>(
    input: &mut R,
    output: &mut W,
) -> Result<(), Vec<Diagnostic>> {
    writeln!(output, "vulgata repl").map_err(io_diag)?;
    writeln!(output, "type :help for commands").map_err(io_diag)?;

    let mut session = ReplSession::new();
    let mut pending = Vec::new();
    let mut line = String::new();

    loop {
        let prompt = if pending.is_empty() { "> " } else { "| " };
        write!(output, "{prompt}").map_err(io_diag)?;
        output.flush().map_err(io_diag)?;

        line.clear();
        let bytes = input.read_line(&mut line).map_err(io_diag)?;
        if bytes == 0 {
            break;
        }

        let trimmed = line.trim_end_matches(['\n', '\r']);
        if pending.is_empty() && trimmed.starts_with(':') {
            match session.handle_command(trimmed)? {
                ReplCommand::Continue(lines) => {
                    for line in lines {
                        writeln!(output, "{line}").map_err(io_diag)?;
                    }
                }
                ReplCommand::Quit(lines) => {
                    for line in lines {
                        writeln!(output, "{line}").map_err(io_diag)?;
                    }
                    break;
                }
            }
            continue;
        }

        if trimmed.is_empty() {
            if pending.is_empty() {
                continue;
            }

            let block = pending.join("\n");
            match session.submit_block(&block) {
                Ok(message) => writeln!(output, "{message}").map_err(io_diag)?,
                Err(diagnostics) => {
                    for diagnostic in diagnostics {
                        writeln!(output, "{diagnostic}").map_err(io_diag)?;
                    }
                }
            }
            pending.clear();
            continue;
        }

        pending.push(trimmed.to_string());
    }

    Ok(())
}

fn format_test_results(results: &[TestResult]) -> Vec<String> {
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
    lines
}

fn help_text() -> &'static str {
    "Commands: :help :show :reset :parse :check :run :test :quit\nSubmit a source block with an empty line."
}

fn io_diag(error: std::io::Error) -> Vec<Diagnostic> {
    vec![Diagnostic::new(
        SourceSpan::new("<repl>", 1, 1),
        Phase::Cli,
        format!("repl I/O error: {error}"),
    )]
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{run_repl, ReplCommand, ReplSession};

    #[test]
    fn accepts_valid_block_and_shows_source() {
        let mut session = ReplSession::new();
        session
            .submit_block("action main() -> Int:\n  return 1")
            .expect("submit");
        match session.handle_command(":show").expect("show") {
            ReplCommand::Continue(lines) => {
                assert!(lines[0].contains("action main() -> Int:"));
            }
            ReplCommand::Quit(_) => panic!("unexpected quit"),
        }
    }

    #[test]
    fn rejects_invalid_block_without_mutating_session() {
        let mut session = ReplSession::new();
        session
            .submit_block("action main() -> Int:\n  return 1")
            .expect("submit");
        let before = session.source().to_string();
        let err = session
            .submit_block("action broken( -> None:\n  return")
            .expect_err("submit should fail");
        assert!(!err.is_empty());
        assert_eq!(session.source(), before);
    }

    #[test]
    fn runs_main_and_tests_from_session_source() {
        let mut session = ReplSession::new();
        session
            .submit_block(
                "action main() -> Int:\n  return 42\n\ntest smoke:\n  expect main() == 42",
            )
            .expect("submit");

        match session.handle_command(":run").expect("run") {
            ReplCommand::Continue(lines) => assert_eq!(lines, vec!["Int(42)"]),
            ReplCommand::Quit(_) => panic!("unexpected quit"),
        }

        match session.handle_command(":test").expect("test") {
            ReplCommand::Continue(lines) => assert_eq!(lines, vec!["PASS smoke"]),
            ReplCommand::Quit(_) => panic!("unexpected quit"),
        }
    }

    #[test]
    fn rejects_extern_backed_execution_in_repl() {
        let mut session = ReplSession::new();
        session
            .submit_block("extern action add(a: Int, b: Int) -> Int\n")
            .expect("submit");
        let diagnostics = session.handle_command(":run").expect_err("run should fail");
        assert!(
            diagnostics[0]
                .message
                .contains("repl does not yet support extern-backed execution")
        );
    }

    #[test]
    fn interactive_loop_collects_blocks_and_commands() {
        let input = b"action main() -> Int:\n  return 7\n\n:run\n:quit\n";
        let mut input = Cursor::new(&input[..]);
        let mut output = Vec::new();
        run_repl(&mut input, &mut output).expect("repl");
        let rendered = String::from_utf8(output).expect("utf8");
        assert!(rendered.contains("vulgata repl"));
        assert!(rendered.contains("ok: block added"));
        assert!(rendered.contains("Int(7)"));
    }
}
