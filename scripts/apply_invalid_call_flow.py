from pathlib import Path


def patch(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count}: {old[:120]!r}")
    file.write_text(text.replace(old, new, 1))


call_old = '''            ast::ExpressionKind::Call { callee, arguments } => {
                let callee = self.lower_expression(callee, return_type);
                let mut can_continue = !callee.ty.is_never();
                let mut lowered_arguments = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    let argument = if can_continue {
                        self.lower_expression(argument, return_type)
                    } else {
                        self.lower_expression_for_diagnostics(argument, return_type)
                    };
                    if can_continue && argument.ty.is_never() {
                        can_continue = false;
                    }
                    lowered_arguments.push(argument);
                }
                let ty = self.check_call(&callee, &lowered_arguments, expression.span);
                (
                    ExpressionKind::Call {
                        callee: Box::new(callee),
                        arguments: lowered_arguments,
                    },
                    ty,
                )
            }
'''
call_new = '''            ast::ExpressionKind::Call { callee, arguments } => {
                let call_entry_state = self.capture_reachable_state();
                let callee = self.lower_expression(callee, return_type);
                let mut can_continue = !callee.ty.is_never();
                let mut lowered_arguments = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    let argument = if can_continue {
                        self.lower_expression(argument, return_type)
                    } else {
                        self.lower_expression_for_diagnostics(argument, return_type)
                    };
                    if can_continue && argument.ty.is_never() {
                        can_continue = false;
                    }
                    lowered_arguments.push(argument);
                }
                let ty = self.check_call(&callee, &lowered_arguments, expression.span);
                if ty.is_error() {
                    self.restore_reachable_state(call_entry_state);
                }
                (
                    ExpressionKind::Call {
                        callee: Box::new(callee),
                        arguments: lowered_arguments,
                    },
                    ty,
                )
            }
'''
patch("crates/nova-sema/src/analyzer.rs", call_old, call_new)

check_old = '''    fn check_call(
        &mut self,
        callee: &hir::Expression,
        arguments: &[hir::Expression],
        span: Span,
    ) -> Type {
        if callee.ty.is_never() {
            return Type::Never;
        }
        let Type::Function(signature) = callee.ty.clone() else {
            if callee.ty.is_error() {
                return Type::Error;
            }
            self.diagnostics.push(
                Diagnostic::error("N3005", "expression is not callable").with_primary(
                    callee.span,
                    format!("found {} instead of a function", callee.ty),
                ),
            );
            return Type::Error;
        };

        if arguments.len() != signature.parameters.len() {
            self.diagnostics.push(
                Diagnostic::error("N3006", "wrong number of arguments").with_primary(
                    span,
                    format!(
                        "expected {} argument(s), found {}",
                        signature.parameters.len(),
                        arguments.len()
                    ),
                ),
            );
        }
        for (index, (argument, expected)) in arguments
            .iter()
            .zip(signature.parameters.iter())
            .enumerate()
        {
            self.require_type(
                &argument.ty,
                expected,
                argument.span,
                &format!("argument {}", index + 1),
            );
        }
        if arguments.iter().any(|argument| argument.ty.is_never()) {
            Type::Never
        } else {
            *signature.return_type
        }
    }
'''
check_new = '''    fn check_call(
        &mut self,
        callee: &hir::Expression,
        arguments: &[hir::Expression],
        span: Span,
    ) -> Type {
        if callee.ty.is_never() {
            return Type::Never;
        }

        let arguments_never = arguments.iter().any(|argument| argument.ty.is_never());
        let arguments_error = arguments.iter().any(|argument| argument.ty.is_error());
        let Type::Function(signature) = callee.ty.clone() else {
            if callee.ty.is_error() {
                return if arguments_never {
                    Type::Never
                } else {
                    Type::Error
                };
            }
            self.diagnostics.push(
                Diagnostic::error("N3005", "expression is not callable").with_primary(
                    callee.span,
                    format!("found {} instead of a function", callee.ty),
                ),
            );
            return if arguments_never {
                Type::Never
            } else {
                Type::Error
            };
        };

        let arity_matches = arguments.len() == signature.parameters.len();
        if !arity_matches {
            self.diagnostics.push(
                Diagnostic::error("N3006", "wrong number of arguments").with_primary(
                    span,
                    format!(
                        "expected {} argument(s), found {}",
                        signature.parameters.len(),
                        arguments.len()
                    ),
                ),
            );
        }

        let mut argument_types_match = true;
        for (index, (argument, expected)) in arguments
            .iter()
            .zip(signature.parameters.iter())
            .enumerate()
        {
            let type_matches = expected_type_compatible(&argument.ty, expected);
            self.require_type(
                &argument.ty,
                expected,
                argument.span,
                &format!("argument {}", index + 1),
            );
            argument_types_match &= type_matches;
        }

        if arguments_never {
            Type::Never
        } else if arguments_error || !arity_matches || !argument_types_match {
            Type::Error
        } else {
            *signature.return_type
        }
    }
'''
patch("crates/nova-sema/src/analyzer.rs", check_old, check_new)

patch(
    "README.md",
    "Function parameters and a function body's outermost bindings share one scope.\n\n`record Name { field: Type, ... }` declares a nominal type: two separately\n",
    "Function parameters and a function body's outermost bindings share one scope.\n\nRejected calls are fail-closed for continuing flow recovery. Callees and arguments\nare still lowered left-to-right for deterministic diagnostics, but a non-callable\ncallee, wrong arity, argument type mismatch, or erroneous argument yields Error HIR\nand cannot export assignments or loop-exit facts. An actually evaluated child that\nis already non-continuing keeps its `!` flow.\n\n`record Name { field: Type, ... }` declares a nominal type: two separately\n",
)

patch(
    "docs/language-constitution.md",
    "Continuing record or enum construction that is itself rejected by type-head,\n",
    "A continuing call rejected because its callee is not callable, its arity is wrong,\nan argument has the wrong type, or an evaluated argument is erroneous is fail-closed\nfor flow recovery. Callee/argument diagnostics and HIR children are retained, but\nassignments and loop-exit facts produced only inside that rejected call do not become\npost-call facts. An actually evaluated non-continuing callee or argument keeps `!`\nprecedence.\n\nContinuing record or enum construction that is itself rejected by type-head,\n",
)

roadmap = "docs/roadmap.md"
patch(
    roadmap,
    "**Status: sixteen vertical slices implemented; broader type-system work remains.**\n",
    "**Status: seventeen vertical slices implemented; broader type-system work remains.**\n",
)
marker = "The next Phase 2 slices should address semantic depth rather than widen syntax\nprematurely. In particular:\n"
addition = '''Implemented in the seventeenth Phase 2 slice:

- call lowering snapshots reachable state before evaluating the callee and arguments,
  so a continuing rejected call cannot export assignments or loop-exit facts;
- wrong arity, non-callable callees, argument type mismatches, and evaluated argument
  errors now produce `Type::Error` HIR rather than retaining a normal return type;
- callee and argument HIR remain intact and are still lowered left-to-right for
  deterministic diagnostics and source-qualified recovery;
- an actually evaluated non-continuing callee or argument keeps `!` precedence even
  when the call is otherwise invalid, preserving reachable return/break/continue; and
- red-to-green adversarial tests lock definite-initialization, conditional break,
  recovery typing, child-error, and non-continuation behavior across invalid calls.

'''
patch(roadmap, marker, addition + marker)
