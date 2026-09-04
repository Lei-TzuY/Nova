use nova_inspect::{build_document, build_document_v5, build_document_v6, render_json_v6};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::hir::{ExpressionKind, FunctionId, ModuleId};
use nova_sema::{AnalysisOutput, analyze_in_module};
use nova_source::{SourceFile, SourceId, Span};

fn analyzed(text: &str, module: ModuleId) -> (SourceFile, AnalysisOutput) {
    let source = SourceFile::new(SourceId::new(0), "module-inspection.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze_in_module(&parsed.program, module);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    (source, analyzed)
}

#[test]
fn v6_exposes_complete_module_ownership_while_legacy_schemas_fail_closed() {
    let module = ModuleId::new(17);
    let (source, analysis) = analyzed(
        "record Box { value: String }\n\
         enum Maybe { None, Some(Box) }\n\
         fn main() -> String {\n\
             let greeting = \"Nova\";\n\
             let get = fn() -> String { greeting };\n\
             let wrapped = Maybe::Some(new Box { value: get() });\n\
             match wrapped { Maybe::None => \"\", Maybe::Some(value) => value.value }\n\
         }",
        module,
    );

    let v1 = build_document(&analysis.program, &source).expect_err("v1 has no module field");
    let v5 = build_document_v5(&analysis, &source).expect_err("v5 has no module field");
    for error in [v1, v5] {
        assert!(error.message().contains("select schema v6"), "{error}");
    }

    let document = build_document_v6(&analysis, &source).expect("v6 represents module ownership");
    assert_eq!(document.schema_version, 6);
    assert_eq!(document.module.id, "module:17");
    assert!(!document.module.implicit_root);
    assert_eq!(document.module.source, document.source.id);
    assert_eq!(document.module.span, document.program.span);
    assert_eq!(document.module.records, ["record:0"]);
    assert_eq!(document.module.enums, ["enum:0"]);
    assert_eq!(document.module.functions, ["function:0"]);
    assert_eq!(
        document.module.bindings,
        ["binding:0", "binding:1", "binding:2", "binding:3"]
    );
    assert_eq!(document.module.closures, ["closure:0"]);

    let first = render_json_v6(&analysis, &source).expect("first render");
    let second = render_json_v6(&analysis, &source).expect("second render");
    assert_eq!(first, second, "schema v6 rendering must be deterministic");
}

#[test]
fn v6_rejects_cross_module_reference_and_module_span_drift() {
    let module = ModuleId::new(17);
    let (source, mut analysis) = analyzed(
        "fn helper() -> Int { 42 } fn main() -> Int { helper() }",
        module,
    );
    let tail = analysis.program.functions[1]
        .body
        .tail
        .as_deref_mut()
        .expect("main tail");
    let ExpressionKind::Call { callee, .. } = &mut tail.kind else {
        panic!("expected call");
    };
    let ExpressionKind::Function { function, .. } = &mut callee.kind else {
        panic!("expected function reference");
    };
    *function = FunctionId::in_module(ModuleId::new(18), 0);
    let error = build_document_v6(&analysis, &source)
        .expect_err("cross-module same-index reference must fail closed");
    assert!(error.message().contains("module:18"), "{error}");

    let (source, mut analysis) = analyzed("fn main() -> Int { 42 }", module);
    analysis.program.module.span = Span::empty(source.id(), source.len());
    let error =
        build_document_v6(&analysis, &source).expect_err("module span drift must fail closed");
    assert!(error.message().contains("module span"), "{error}");
}

#[test]
fn published_v6_schema_requires_the_module_fact() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/schemas/semantic-inspection-v6.schema.json"
    ))
    .expect("published schema is JSON");
    assert_eq!(schema["$id"], "urn:nova:semantic-inspection:v6");
    assert_eq!(schema["properties"]["schema_version"]["const"], 6);
    assert_eq!(schema["properties"]["module"]["$ref"], "#/$defs/module");
    assert!(
        schema["required"]
            .as_array()
            .expect("required fields")
            .iter()
            .any(|field| field == "module")
    );
}
