use nova_diagnostics::{Diagnostic, render_human_all, render_json_lines};
use nova_hir::lower;
use nova_lexer::lex;
use nova_parser::{format_ast, parse};
use nova_source::{SourceFile, SourceId};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "Nova bootstrap frontend

Usage:
  nova check <file> [--message-format human|json]
  nova ast <file> [--message-format human|json]
  nova --help

`check` validates UTF-8, tokens, syntax, and the implemented name rules.
It does not yet perform type inference or type checking.";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Command {
    Check,
    Ast,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum MessageFormat {
    #[default]
    Human,
    Json,
}

#[derive(Debug, Eq, PartialEq)]
struct Options {
    command: Command,
    path: PathBuf,
    message_format: MessageFormat,
}

enum ParsedArguments {
    Run(Options),
    Help,
}

fn main() -> ExitCode {
    let arguments = env::args_os().skip(1).collect::<Vec<_>>();
    let stdout = io::stdout();
    let stderr = io::stderr();
    let mut stdout = stdout.lock();
    let mut stderr = stderr.lock();

    match run(&arguments, &mut stdout, &mut stderr) {
        Ok(status) => ExitCode::from(status),
        Err(_) => ExitCode::FAILURE,
    }
}

fn run(arguments: &[OsString], stdout: &mut dyn Write, stderr: &mut dyn Write) -> io::Result<u8> {
    let options = match parse_arguments(arguments) {
        Ok(ParsedArguments::Run(options)) => options,
        Ok(ParsedArguments::Help) => {
            writeln!(stdout, "{USAGE}")?;
            return Ok(0);
        }
        Err(message) => {
            writeln!(stderr, "error: {message}\n\n{USAGE}")?;
            return Ok(2);
        }
    };

    let display_name = options.path.to_string_lossy().into_owned();
    let bytes = match fs::read(&options.path) {
        Ok(bytes) => bytes,
        Err(error) => {
            let source = SourceFile::new(SourceId::new(0), display_name, "");
            let diagnostic = Diagnostic::error("N0002", "could not read source file")
                .with_note(error.to_string());
            emit_diagnostics(
                std::slice::from_ref(&diagnostic),
                &source,
                options.message_format,
                stderr,
            )?;
            return Ok(1);
        }
    };

    let text = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => {
            let valid_up_to = error.utf8_error().valid_up_to();
            let source = SourceFile::new(SourceId::new(0), display_name.clone(), "");
            let diagnostic = Diagnostic::error("N0001", "source file is not valid UTF-8")
                .with_note(format!(
                    "{display_name}: first invalid byte sequence begins at byte offset {valid_up_to}"
                ));
            emit_diagnostics(
                std::slice::from_ref(&diagnostic),
                &source,
                options.message_format,
                stderr,
            )?;
            return Ok(1);
        }
    };

    let source = SourceFile::new(SourceId::new(0), display_name, text);
    let lexed = lex(&source);
    if !lexed.is_success() {
        emit_diagnostics(&lexed.diagnostics, &source, options.message_format, stderr)?;
        return Ok(1);
    }

    let parsed = parse(&source, &lexed.tokens);
    if !parsed.is_success() {
        emit_diagnostics(&parsed.diagnostics, &source, options.message_format, stderr)?;
        return Ok(1);
    }

    match options.command {
        Command::Check => {
            let lowered = lower(&parsed.program);
            if !lowered.is_success() {
                emit_diagnostics(
                    &lowered.diagnostics,
                    &source,
                    options.message_format,
                    stderr,
                )?;
                return Ok(1);
            }
        }
        Command::Ast => {
            writeln!(stdout, "{}", format_ast(&parsed.program))?;
        }
    }
    Ok(0)
}

fn parse_arguments(arguments: &[OsString]) -> Result<ParsedArguments, String> {
    let Some(first) = arguments.first().and_then(|argument| argument.to_str()) else {
        return Err("missing command".to_owned());
    };
    if matches!(first, "--help" | "-h") {
        if arguments.len() == 1 {
            return Ok(ParsedArguments::Help);
        }
        return Err("`--help` does not accept additional arguments".to_owned());
    }

    let command = match first {
        "check" => Command::Check,
        "ast" => Command::Ast,
        unknown => return Err(format!("unknown command `{unknown}`")),
    };
    let mut path = None;
    let mut message_format = MessageFormat::Human;
    let mut index = 1;

    while index < arguments.len() {
        let argument = &arguments[index];
        let text = argument.to_str();
        if text == Some("--message-format") {
            index += 1;
            let Some(value) = arguments.get(index).and_then(|value| value.to_str()) else {
                return Err("`--message-format` requires `human` or `json`".to_owned());
            };
            message_format = parse_message_format(value)?;
        } else if let Some(value) = text.and_then(|value| value.strip_prefix("--message-format=")) {
            message_format = parse_message_format(value)?;
        } else if text.is_some_and(|value| value.starts_with('-')) {
            return Err(format!("unknown option `{}`", argument.to_string_lossy()));
        } else if path.replace(PathBuf::from(argument)).is_some() {
            return Err("expected exactly one source file".to_owned());
        }
        index += 1;
    }

    let Some(path) = path else {
        return Err("missing source file".to_owned());
    };
    Ok(ParsedArguments::Run(Options {
        command,
        path,
        message_format,
    }))
}

fn parse_message_format(value: &str) -> Result<MessageFormat, String> {
    match value {
        "human" => Ok(MessageFormat::Human),
        "json" => Ok(MessageFormat::Json),
        _ => Err(format!(
            "unsupported message format `{value}`; expected `human` or `json`"
        )),
    }
}

fn emit_diagnostics(
    diagnostics: &[Diagnostic],
    source: &SourceFile,
    format: MessageFormat,
    writer: &mut dyn Write,
) -> io::Result<()> {
    let rendered = match format {
        MessageFormat::Human => render_human_all(diagnostics, source),
        MessageFormat::Json => render_json_lines(diagnostics, source),
    };
    if !rendered.is_empty() {
        writeln!(writer, "{rendered}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Command, MessageFormat, Options, ParsedArguments, parse_arguments};
    use std::ffi::OsString;
    use std::path::Path;

    fn arguments(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn parses_both_message_format_spellings() {
        let spaced = parse_arguments(&arguments(&[
            "check",
            "sample.nv",
            "--message-format",
            "json",
        ]))
        .expect("valid arguments");
        let joined = parse_arguments(&arguments(&["ast", "--message-format=human", "sample.nv"]))
            .expect("valid arguments");

        assert!(matches!(
            spaced,
            ParsedArguments::Run(Options {
                command: Command::Check,
                path,
                message_format: MessageFormat::Json,
            }) if path.as_path() == Path::new("sample.nv")
        ));
        assert!(matches!(
            joined,
            ParsedArguments::Run(Options {
                command: Command::Ast,
                path,
                message_format: MessageFormat::Human,
            }) if path.as_path() == Path::new("sample.nv")
        ));
    }

    #[test]
    fn rejects_ambiguous_or_incomplete_invocations() {
        for values in [
            vec![],
            vec!["check"],
            vec!["run", "x.nv"],
            vec!["check", "a.nv", "b.nv"],
            vec!["check", "x.nv", "--message-format", "xml"],
        ] {
            assert!(parse_arguments(&arguments(&values)).is_err(), "{values:?}");
        }
    }
}
