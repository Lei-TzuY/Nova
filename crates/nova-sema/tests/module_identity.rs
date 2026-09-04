use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::hir::{ExpressionKind, ModuleId, Type};
use nova_sema::{analyze, analyze_in_module};
use nova_source::{SourceFile, SourceId};

#[test]
fn one_ast_can_be_resolved_under_distinct_module_qualified_identities() {
    let source = SourceFile::new(
        SourceId::new(0),
        "module-identity.nv",
        "record Box { value: Int }\n\
         enum Maybe { None, Some(Int) }\n\
         fn helper(value: Int) -> Int { value }\n\
         fn main() -> Int {\n\
             let base = 40;\n\
             let add: fn(Int) -> Int = fn(delta: Int) -> Int { helper(base + delta) };\n\
             let boxed = new Box { value: add(2) };\n\
             match Maybe::Some(boxed.value) {\n\
                 Maybe::None => 0,\n\
                 Maybe::Some(value) => value,\n\
             }\n\
         }",
    );
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);

    let root = analyze(&parsed.program);
    let assigned_id = ModuleId::new(17);
    let assigned = analyze_in_module(&parsed.program, assigned_id);
    assert!(root.is_success(), "{:?}", root.diagnostics);
    assert!(assigned.is_success(), "{:?}", assigned.diagnostics);
    assert_eq!(root.program.module.id, ModuleId::ROOT);
    assert_eq!(assigned.program.module.id, assigned_id);
    assert_eq!(root.program.module.span, root.program.span);
    assert_eq!(assigned.program.module.span, assigned.program.span);

    assert_eq!(root.program.records[0].id.index(), 0);
    assert_eq!(assigned.program.records[0].id.index(), 0);
    assert_ne!(root.program.records[0].id, assigned.program.records[0].id);
    assert_eq!(assigned.program.records[0].id.module(), assigned_id);
    assert_eq!(assigned.program.enums[0].id.module(), assigned_id);
    assert!(
        assigned
            .program
            .functions
            .iter()
            .all(|function| function.id.module() == assigned_id)
    );

    let main = &assigned.program.functions[1];
    let closure_initializer = match &main.body.statements[1].kind {
        nova_sema::hir::StatementKind::Binding { initializer, .. } => initializer,
        other => panic!("expected closure binding, found {other:?}"),
    };
    let ExpressionKind::Closure(closure) = &closure_initializer.kind else {
        panic!("expected closure initializer");
    };
    assert_eq!(closure.id.module(), assigned_id);
    assert_eq!(closure.captures.len(), 1);
    assert_eq!(closure.captures[0].reference.binding.module(), assigned_id);
    assert!(
        closure
            .parameters
            .iter()
            .all(|parameter| parameter.id.module() == assigned_id)
    );

    let boxed_binding = match &main.body.statements[2].kind {
        nova_sema::hir::StatementKind::Binding { binding, .. } => binding,
        other => panic!("expected record binding, found {other:?}"),
    };
    let Type::Record(box_type) = &boxed_binding.ty else {
        panic!("expected nominal record type");
    };
    assert_eq!(box_type.id.module(), assigned_id);

    assert!(assigned.control_flow.functions().iter().all(|graph| {
        graph.function().module() == assigned_id
            && graph
                .bindings()
                .iter()
                .all(|binding| binding.id.module() == assigned_id)
    }));
    assert!(assigned.control_flow.closures().iter().all(|graph| {
        graph.closure().module() == assigned_id
            && graph
                .bindings()
                .iter()
                .all(|binding| binding.id.module() == assigned_id)
    }));
}
