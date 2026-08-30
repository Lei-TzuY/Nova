from pathlib import Path

p = Path('crates/nova-cli/src/main.rs')
text = p.read_text()
anchor = '''        assert!(matches!(
            inspected_v2,
            ParsedArguments::Run(Options {
                command: Command::Inspect,
                path,
                message_format: MessageFormat::Human,
                inspect_format: Some(InspectFormat::Json),
                inspect_schema_version: Some(InspectSchemaVersion::V2),
            }) if path.as_path() == Path::new("sample.nv")
        ));
'''
addition = anchor + '''

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
                path,
                message_format: MessageFormat::Human,
                inspect_format: Some(InspectFormat::Json),
                inspect_schema_version: Some(InspectSchemaVersion::V3),
            }) if path.as_path() == Path::new("sample.nv")
        ));
'''
if text.count(anchor) != 1:
    raise RuntimeError(f'expected v2 assertion once, found {text.count(anchor)}')
text = text.replace(anchor, addition, 1)
reject = '''                "--schema-version",
                "3",
'''
if text.count(reject) != 1:
    raise RuntimeError(f'expected rejected schema 3 once, found {text.count(reject)}')
text = text.replace(reject, '''                "--schema-version",
                "4",
''', 1)
p.write_text(text)
