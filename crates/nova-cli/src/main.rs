use nova_diagnostics::{Diagnostic, Severity, render_human_all, render_json_lines};
use nova_inspect::{
    render_json as render_semantic_json, render_json_v2 as render_semantic_json_v2,
    render_json_v3 as render_semantic_json_v3,
};
use nova_interpreter::execute;
use nova_lexer::lex;
use nova_parser::{format_ast, parse};
use nova_sema::analyze;
use nova_source::{SourceFile, SourceId};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "Nova bootstrap compiler

Usage:
  nova check <file|-> [--source-name name] [--message-format human|json] [--fail-on-warnings]
  nova run <file|-> [--source-name name] [--message-format human|json] [--fail-on-warnings]
  nova ast <file|-> [--source-name name] [--message-format human|json]
  nova inspect <file|-> --format json [--schema-version 1|2|3] [--source-name name] [--message-format human|json] [--fail-on-warnings]
  nova --help

`check` validates UTF-8, tokens, syntax, names, types, and definite assignment.
`run` performs the same checks and executes zero-argument `main` in the bootstrap interpreter.
`ast` prints the parsed syntax tree after lexical and syntactic validation.
`inspect` emits versioned semantic facts for a successfully checked program.
`-` reads one source from standard input; `--source-name` overrides its `<stdin>` display name.
`--fail-on-warnings` returns status 1 without promoting warning diagnostics to errors.";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Command {
    Check,
    Run,
    Ast,
    Inspect,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum MessageFormat {
    #[default]
    Human,
    Json,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InspectFormat {
    Json,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InspectSchemaVersion {
    V1,
    V2,
    V3,
}

#[derive(Debug, Eq, PartialEq)]
enum SourceInput {
    File(PathBuf),
    Stdin { display_name: String },
}

#[derive(Debug, Eq, PartialEq)]
struct Options {
    command: Command,
    source: SourceInput,
    message_format: MessageFormat,
    inspect_format: Option<InspectFormat>,
    inspect_schema_version: Option<InspectSchemaVersion>,
    fail_on_warnings: bool,
}

enum ParsedArguments {
    Run(Options),
    Help,
}

fn main() -> ExitCode {
    let arguments = env::args_os().skip(1).collect::<Vec<_>>();
    let stdin = io::stdin();
    let stdout = io::stdout();
    let stderr = io::stderr();
    let mut stdin = stdin.lock();
    let mut stdout = stdout.lock();
    let mut stderr = stderr.lock();

    match run(&arguments, &mut stdin, &mut stdout, &mut stderr) {
        Ok(status) => ExitCode::from(status),
        Err(_) => ExitCode::FAILURE,
    }
}

fn run(
    arguments: &[OsString],
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> io::Result<u8> {
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

    let (display_name, read_failure, invalid_utf8, bytes) = match &options.source {
        SourceInput::File(path) => (
            path.to_string_lossy().into_owned(),
            "could not read source file",
            "source file is not valid UTF-8",
            fs::read(path),
        ),
        SourceInput::Stdin { display_name } => {
            let mut bytes = Vec::new();
            let result = stdin.read_to_end(&mut bytes).map(|_| bytes);
            (
                display_name.clone(),
                "could not read standard input",
                "standard input is not valid UTF-8",
                result,
            )
        }
    };
    let bytes = match bytes {
        Ok(bytes) => bytes,
        Err(error) => {
            let source = SourceFile::new(SourceId::new(0), display_name.clone(), "");
            let diagnostic = Diagnostic::error("N0002", read_failure)
                .with_note(format!("{display_name}: {error}"));
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
            let diagnostic = Diagnostic::error("N0001", invalid_utf8).with_note(format!(
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

    if matches!(options.command, Command::Ast) {
        writeln!(stdout, "{}", format_ast(&parsed.program))?;
        return Ok(0);
    }

    let analyzed = analyze(&parsed.program);
    let accepted = analyzed.is_success();
    let warnings_rejected = options.fail_on_warnings
        && analyzed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Warning);
    if !analyzed.diagnostics.is_empty() {
        emit_diagnostics(
            &analyzed.diagnostics,
            &source,
            options.message_format,
            stderr,
        )?;
    }
    if !accepted || warnings_rejected {
        return Ok(1);
    }

    if matches!(
        (options.command, options.inspect_format),
        (Command::Inspect, Some(InspectFormat::Json))
    ) {
        let rendered = match options
            .inspect_schema_version
            .unwrap_or(InspectSchemaVersion::V1)
        {
            InspectSchemaVersion::V1 => render_semantic_json(&analyzed.program, &source),
            InspectSchemaVersion::V2 => render_semantic_json_v2(&analyzed, &source),
            InspectSchemaVersion::V3 => render_semantic_json_v3(&analyzed, &source),
        };
        match rendered {
            Ok(document) => writeln!(stdout, "{document}")?,
            Err(error) => {
                let diagnostic =
                    Diagnostic::error("N5001", "semantic inspection invariant failure")
                        .with_primary(analyzed.program.span, "inspection stopped for this source")
                        .with_note(error.to_string());
                emit_diagnostics(
                    std::slice::from_ref(&diagnostic),
                    &source,
                    options.message_format,
                    stderr,
                )?;
                return Ok(1);
            }
        }
        return Ok(0);
    }

    if matches!(options.command, Command::Run) {
        match execute(&analyzed.program) {
            Ok(value) => writeln!(stdout, "{value}")?,
            Err(diagnostic) => {
                emit_diagnostics(
                    std::slice::from_ref(&diagnostic),
                    &source,
                    options.message_format,
                    stderr,
                )?;
                return Ok(1);
            }
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
        "run" => Command::Run,
        "ast" => Command::Ast,
        "inspect" => Command::Inspect,
        unknown => return Err(format!("unknown command `{unknown}`")),
    };
    let mut source = None;
    let mut message_format = MessageFormat::Human;
    let mut inspect_format = None;
    let mut inspect_schema_version = None;
    let mut fail_on_warnings = false;
    let mut source_name = None;
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
        } else if text == Some("--format") {
            index += 1;
            let Some(value) = arguments.get(index).and_then(|value| value.to_str()) else {
                return Err("`--format` requires `json`".to_owned());
            };
            inspect_format = Some(parse_inspect_format(value)?);
        } else if let Some(value) = text.and_then(|value| value.strip_prefix("--format=")) {
            inspect_format = Some(parse_inspect_format(value)?);
        } else if text == Some("--schema-version") {
            index += 1;
            let Some(value) = arguments.get(index).and_then(|value| value.to_str()) else {
                return Err("`--schema-version` requires `1`, `2`, or `3`".to_owned());
            };
            inspect_schema_version = Some(parse_inspect_schema_version(value)?);
        } else if let Some(value) = text.and_then(|value| value.strip_prefix("--schema-version=")) {
            inspect_schema_version = Some(parse_inspect_schema_version(value)?);
        } else if text == Some("--source-name") {
            index += 1;
            let Some(value) = arguments.get(index).and_then(|value| value.to_str()) else {
                return Err(source_name_requirement());
            };
            source_name = Some(parse_source_name(value)?);
        } else if let Some(value) = text.and_then(|value| value.strip_prefix("--source-name=")) {
            source_name = Some(parse_source_name(value)?);
        } else if text == Some("--fail-on-warnings") {
            fail_on_warnings = true;
        } else if text == Some("-") {
            if source
                .replace(SourceInput::Stdin {
                    display_name: "<stdin>".to_owned(),
                })
                .is_some()
            {
                return Err("expected exactly one source input".to_owned());
            }
        } else if text.is_some_and(|value| value.starts_with('-')) {
            return Err(format!("unknown option `{}`", argument.to_string_lossy()));
        } else if source
            .replace(SourceInput::File(PathBuf::from(argument)))
            .is_some()
        {
            return Err("expected exactly one source input".to_owned());
        }
        index += 1;
    }

    let Some(mut source) = source else {
        return Err("missing source input".to_owned());
    };
    match (&mut source, source_name) {
        (SourceInput::Stdin { display_name }, Some(source_name)) => {
            *display_name = source_name;
        }
        (SourceInput::File(_), Some(_)) => {
            return Err("`--source-name` is only valid when the source input is `-`".to_owned());
        }
        (_, None) => {}
    }
    match (command, inspect_format) {
        (Command::Inspect, None) => return Err("`inspect` requires `--format json`".to_owned()),
        (Command::Inspect, Some(_)) | (_, None) => {}
        (_, Some(_)) => return Err("`--format` is only valid with `inspect`".to_owned()),
    }
    if command != Command::Inspect && inspect_schema_version.is_some() {
        return Err("`--schema-version` is only valid with `inspect`".to_owned());
    }
    if command == Command::Ast && fail_on_warnings {
        return Err("`--fail-on-warnings` is not valid with `ast`".to_owned());
    }
    Ok(ParsedArguments::Run(Options {
        command,
        source,
        message_format,
        inspect_format,
        inspect_schema_version,
        fail_on_warnings,
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

fn parse_source_name(value: &str) -> Result<String, String> {
    if value.is_empty() || value.contains('\n') || value.contains('\r') {
        return Err(source_name_requirement());
    }
    Ok(value.to_owned())
}

fn source_name_requirement() -> String {
    "`--source-name` requires a non-empty, single-line UTF-8 display name".to_owned()
}

fn parse_inspect_format(value: &str) -> Result<InspectFormat, String> {
    match value {
        "json" => Ok(InspectFormat::Json),
        _ => Err(format!(
            "unsupported inspection format `{value}`; expected `json`"
        )),
    }
}

fn parse_inspect_schema_version(value: &str) -> Result<InspectSchemaVersion, String> {
    match value {
        "1" => Ok(InspectSchemaVersion::V1),
        "2" => Ok(InspectSchemaVersion::V2),
        "3" => Ok(InspectSchemaVersion::V3),
        _ => Err(format!(
            "unsupported inspection schema version `{value}`; expected `1`, `2`, or `3`"
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
    use super::{
        Command, InspectFormat, InspectSchemaVersion, MessageFormat, Options, ParsedArguments,
        SourceInput, parse_arguments, run,
    };
    use std::ffi::OsString;
    use std::io::{self, Read};
    use std::path::Path;

    fn arguments(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn parses_commands_and_both_message_format_spellings() {
        let spaced = parse_arguments(&arguments(&[
            "run",
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
                command: Command::Run,
                source: SourceInput::File(path),
                message_format: MessageFormat::Json,
                inspect_format: None,
                inspect_schema_version: None,
                fail_on_warnings: false,
            }) if path.as_path() == Path::new("sample.nv")
        ));
        assert!(matches!(
            joined,
            ParsedArguments::Run(Options {
                command: Command::Ast,
                source: SourceInput::File(path),
                message_format: MessageFormat::Human,
                inspect_format: None,
                inspect_schema_version: None,
                fail_on_warnings: false,
            }) if path.as_path() == Path::new("sample.nv")
        ));

        let inspected = parse_arguments(&arguments(&["inspect", "sample.nv", "--format=json"]))
            .expect("valid inspection arguments");
        assert!(matches!(
            inspected,
            ParsedArguments::Run(Options {
                command: Command::Inspect,
                source: SourceInput::File(path),
                message_format: MessageFormat::Human,
                inspect_format: Some(InspectFormat::Json),
                inspect_schema_version: None,
                fail_on_warnings: false,
            }) if path.as_path() == Path::new("sample.nv")
        ));

        let inspected_v2 = parse_arguments(&arguments(&[
            "inspect",
            "--schema-version=2",
            "sample.nv",
            "--format=json",
        ]))
        .expect("valid schema-v2 inspection arguments");
        assert!(matches!(
            inspected_v2,
            ParsedArguments::Run(Options {
                command: Command::Inspect,
                source: SourceInput::File(path),
                message_format: MessageFormat::Human,
                inspect_format: Some(InspectFormat::Json),
                inspect_schema_version: Some(InspectSchemaVersion::V2),
                fail_on_warnings: false,
            }) if path.as_path() == Path::new("sample.nv")
        ));

        let inspected_v3 = parse_arguments(&arguments(&[
            "inspect",
            "--schema-version=3",
            "sample.nv",
            "--format=json",
        ]))
        .expect("valid schema-v3 inspection arguments");
        assert!(matches!(
            inspected_v3,
            ParsedArguments::Run(Options {
                command: Command::Inspect,
                source: SourceInput::File(path),
                message_format: MessageFormat::Human,
                inspect_format: Some(InspectFormat::Json),
                inspect_schema_version: Some(InspectSchemaVersion::V3),
                fail_on_warnings: false,
            }) if path.as_path() == Path::new("sample.nv")
        ));

        let strict = parse_arguments(&arguments(&["check", "--fail-on-warnings", "sample.nv"]))
            .expect("valid strict warning arguments");
        assert!(matches!(
            strict,
            ParsedArguments::Run(Options {
                command: Command::Check,
                source: SourceInput::File(path),
                message_format: MessageFormat::Human,
                inspect_format: None,
                inspect_schema_version: None,
                fail_on_warnings: true,
            }) if path.as_path() == Path::new("sample.nv")
        ));

        let stdin = parse_arguments(&arguments(&["check", "-"])).expect("valid stdin arguments");
        assert!(matches!(
            stdin,
            ParsedArguments::Run(Options {
                command: Command::Check,
                source: SourceInput::Stdin { display_name },
                message_format: MessageFormat::Human,
                inspect_format: None,
                inspect_schema_version: None,
                fail_on_warnings: false,
            }) if display_name == "<stdin>"
        ));
    }

    #[test]
    fn parses_explicit_standard_input_display_names() {
        for (values, expected) in [
            (
                vec!["check", "-", "--source-name", "editor:///main.nv"],
                "editor:///main.nv",
            ),
            (
                vec!["ast", "--source-name=virtual/input.nv", "-"],
                "virtual/input.nv",
            ),
        ] {
            let parsed = parse_arguments(&arguments(&values)).expect("valid source name");
            assert!(matches!(
                parsed,
                ParsedArguments::Run(Options {
                    source: SourceInput::Stdin { display_name },
                    ..
                }) if display_name == expected
            ));
        }
    }

    #[test]
    fn rejects_ambiguous_or_incomplete_invocations() {
        for values in [
            vec![],
            vec!["check"],
            vec!["execute", "x.nv"],
            vec!["check", "a.nv", "b.nv"],
            vec!["check", "-", "b.nv"],
            vec!["check", "x.nv", "--message-format", "xml"],
            vec!["inspect", "x.nv"],
            vec!["inspect", "x.nv", "--format", "text"],
            vec![
                "inspect",
                "x.nv",
                "--format",
                "json",
                "--schema-version",
                "4",
            ],
            vec!["check", "x.nv", "--format", "json"],
            vec!["check", "x.nv", "--schema-version", "2"],
            vec!["ast", "x.nv", "--fail-on-warnings"],
            vec!["check", "x.nv", "--fail-on-warnings=true"],
            vec!["check", "x.nv", "--source-name", "virtual/input.nv"],
            vec!["check", "-", "--source-name"],
            vec!["check", "-", "--source-name="],
            vec!["check", "-", "--source-name=line\nbreak"],
        ] {
            assert!(parse_arguments(&arguments(&values)).is_err(), "{values:?}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn rejects_non_utf8_standard_input_display_names() {
        use std::os::unix::ffi::OsStringExt;

        let arguments = vec![
            OsString::from("check"),
            OsString::from("-"),
            OsString::from("--source-name"),
            OsString::from_vec(vec![0xff]),
        ];

        assert!(parse_arguments(&arguments).is_err());
    }

    struct FailingStdin;

    impl Read for FailingStdin {
        fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("injected stdin failure"))
        }
    }

    #[test]
    fn reports_standard_input_read_failures_as_source_diagnostics() {
        let mut stdin = FailingStdin;
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            &arguments(&["check", "-", "--source-name", "pipe/main.nv"]),
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("diagnostic rendering succeeds");

        assert_eq!(status, 1);
        assert!(stdout.is_empty());
        let stderr = String::from_utf8(stderr).expect("diagnostic is UTF-8");
        assert!(stderr.contains("error[N0002]: could not read standard input"));
        assert!(stderr.contains("pipe/main.nv: injected stdin failure"));
    }
}
