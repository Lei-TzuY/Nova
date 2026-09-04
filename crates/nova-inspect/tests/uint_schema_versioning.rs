use nova_inspect::{
    build_document, build_document_v2, build_document_v3, build_document_v4, build_document_v5,
    build_document_v6, build_document_v7, render_json_v7, v1,
};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::hir::{ExpressionKind as HirExpressionKind, ModuleId, Type};
use nova_sema::{AnalysisOutput, analyze_in_module};
use nova_source::{SourceFile, SourceId};

fn analyzed(text: &str, module: ModuleId) -> (SourceFile, AnalysisOutput) {
    let source = SourceFile::new(SourceId::new(0), "uint-inspection.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze_in_module(&parsed.program, module);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    (source, analyzed)
}

#[test]
fn v1_through_v6_remain_frozen_and_v7_projects_uint() {
    let (source, analysis) = analyzed(
        "fn narrow(value: UInt) -> Int { Int::from_uint(value) }\n\
         fn main() -> UInt { UInt::MIN + UInt::from(narrow(UInt::from(42))) }",
        ModuleId::ROOT,
    );

    let errors = [
        build_document(&analysis.program, &source).expect_err("v1 must reject UInt"),
        build_document_v2(&analysis, &source).expect_err("v2 must reject UInt"),
        build_document_v3(&analysis, &source).expect_err("v3 must reject UInt"),
        build_document_v4(&analysis, &source).expect_err("v4 must reject UInt"),
        build_document_v5(&analysis, &source).expect_err("v5 must reject UInt"),
        build_document_v6(&analysis, &source).expect_err("v6 must reject UInt"),
    ];
    for error in errors {
        assert!(error.message().contains("schema v1-v6"), "{error}");
        assert!(error.message().contains("select schema v7"), "{error}");
    }

    let document = build_document_v7(&analysis, &source).expect("v7 represents UInt");
    assert_eq!(document.schema_version, 7);
    assert_eq!(document.module.id, "module:0");
    assert!(
        document
            .program
            .types
            .iter()
            .any(|ty| ty.kind == v1::TypeKind::UInt && ty.display == "UInt")
    );
    assert!(
        document
            .program
            .expressions
            .iter()
            .any(|expression| expression.kind == v1::ExpressionKind::UnsignedInteger)
    );
    assert!(document.program.expressions.iter().any(|expression| {
        expression.kind == v1::ExpressionKind::NumericConversion
            && expression.operator.as_deref() == Some("int_to_uint")
    }));
    assert!(document.program.expressions.iter().any(|expression| {
        expression.kind == v1::ExpressionKind::NumericConversion
            && expression.operator.as_deref() == Some("uint_to_int")
    }));

    let first = render_json_v7(&analysis, &source).expect("first v7 render");
    let second = render_json_v7(&analysis, &source).expect("second v7 render");
    assert_eq!(first, second, "v7 rendering must be deterministic");

    let (source, analysis) = analyzed("fn main() -> UInt { UInt::MAX }", ModuleId::new(17));
    let document = build_document_v7(&analysis, &source).expect("v7 preserves module identity");
    assert_eq!(document.module.id, "module:17");
}

#[test]
fn v7_rejects_malformed_numeric_conversion_and_literal_hir() {
    let (source, mut conversion) = analyzed("fn main() -> UInt { UInt::from(42) }", ModuleId::ROOT);
    let tail = conversion.program.functions[0]
        .body
        .tail
        .as_deref_mut()
        .expect("main tail");
    assert!(matches!(&tail.kind, HirExpressionKind::IntToUInt { .. }));
    tail.ty = Type::Int;
    let error = build_document_v7(&conversion, &source)
        .expect_err("conversion result type drift must fail closed");
    assert!(error.message().contains("inconsistent types"), "{error}");

    let (source, mut literal) = analyzed("fn main() -> UInt { UInt::MAX }", ModuleId::ROOT);
    let tail = literal.program.functions[0]
        .body
        .tail
        .as_deref_mut()
        .expect("main tail");
    assert!(matches!(&tail.kind, HirExpressionKind::Unsigned(_)));
    tail.ty = Type::Int;
    let error =
        build_document_v7(&literal, &source).expect_err("literal type drift must fail closed");
    assert!(error.message().contains("instead of UInt"), "{error}");

    let (source, mut conversion) = analyzed(
        "fn main() -> Int { Int::from_uint(UInt::MAX) }",
        ModuleId::ROOT,
    );
    let tail = conversion.program.functions[0]
        .body
        .tail
        .as_deref_mut()
        .expect("main tail");
    assert!(matches!(&tail.kind, HirExpressionKind::UIntToInt { .. }));
    tail.ty = Type::UInt;
    let error = build_document_v7(&conversion, &source)
        .expect_err("reverse conversion result type drift must fail closed");
    assert!(error.message().contains("inconsistent types"), "{error}");
}

#[test]
fn published_schemas_keep_old_enums_frozen_and_add_uint_only_in_v7() {
    let v1: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/schemas/semantic-inspection-v1.schema.json"
    ))
    .expect("published v1 schema is JSON");
    let v5: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/schemas/semantic-inspection-v5.schema.json"
    ))
    .expect("published v5 schema is JSON");
    let v6: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/schemas/semantic-inspection-v6.schema.json"
    ))
    .expect("published v6 schema is JSON");
    let v7: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/schemas/semantic-inspection-v7.schema.json"
    ))
    .expect("published v7 schema is JSON");

    let v1_types = v1["$defs"]["type"]["properties"]["kind"]["enum"]
        .as_array()
        .expect("v1 type enum");
    assert!(!v1_types.iter().any(|kind| kind == "uint"));

    for (version, schema) in [(5, &v5), (6, &v6)] {
        let kinds = schema["$defs"]["expression"]["properties"]["kind"]["enum"]
            .as_array()
            .expect("expression kind enum");
        assert!(
            !kinds.iter().any(|kind| kind == "unsigned_integer"),
            "v{version} must not be mutated"
        );
        assert!(
            !kinds.iter().any(|kind| kind == "numeric_conversion"),
            "v{version} must not be mutated"
        );
    }

    assert_eq!(v7["$id"], "urn:nova:semantic-inspection:v7");
    assert_eq!(v7["properties"]["schema_version"]["const"], 7);
    let v7_types = v7["$defs"]["type"]["properties"]["kind"]["enum"]
        .as_array()
        .expect("v7 type enum");
    assert!(v7_types.iter().any(|kind| kind == "uint"));
    let v7_kinds = v7["$defs"]["expression"]["properties"]["kind"]["enum"]
        .as_array()
        .expect("v7 expression kind enum");
    assert!(v7_kinds.iter().any(|kind| kind == "unsigned_integer"));
    assert!(v7_kinds.iter().any(|kind| kind == "numeric_conversion"));
    assert_eq!(
        v7["$defs"]["closureCapture"]["properties"]["mode"]["const"],
        "by_value"
    );
}
