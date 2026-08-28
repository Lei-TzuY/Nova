//! Deterministic bootstrap interpreter for semantically accepted Nova HIR.

use nova_diagnostics::Diagnostic;
use nova_parser::ast::{BinaryOperator, UnaryOperator};
use nova_sema::hir::{
    BindingId, Block, EnumId, Expression, ExpressionKind, Function, FunctionId, Program, RecordId,
    Statement, StatementKind, Type,
};
use std::collections::BTreeMap;
use std::fmt;

const MAX_CALL_DEPTH: usize = 64;
const MAX_EXECUTION_STEPS: usize = 100_000;

/// Runtime value produced by the bootstrap interpreter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Value {
    /// Signed bootstrap integer value.
    Int(i64),
    /// Boolean value.
    Bool(bool),
    /// Nominal record value stored in declaration-order slots.
    Record {
        /// Stable nominal record identity.
        record: RecordId,
        /// Field values in declaration order.
        fields: Vec<Value>,
    },
    /// Nominal enum value with an optional single payload.
    Enum {
        /// Stable nominal enum identity.
        enumeration: EnumId,
        /// Variant slot in declaration order.
        variant_index: usize,
        /// Optional boxed payload.
        payload: Option<Box<Value>>,
    },
    /// First-class reference to a top-level function.
    Function(FunctionId),
    /// Internal value for a block with no tail expression.
    Unit,
}

impl fmt::Display for Value {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(value) => write!(formatter, "{value}"),
            Self::Bool(value) => write!(formatter, "{value}"),
            Self::Record { record, .. } => write!(formatter, "<record:{}>", record.index()),
            Self::Enum {
                enumeration,
                variant_index,
                ..
            } => write!(formatter, "<enum:{}:{variant_index}>", enumeration.index()),
            Self::Function(id) => write!(formatter, "<function:{}>", id.index()),
            Self::Unit => formatter.write_str("()"),
        }
    }
}

/// Executes the zero-argument `main` function of a semantically accepted HIR program.
///
/// Runtime failures are returned through the same structured diagnostic model used by
/// earlier compiler phases. The bootstrap interpreter uses checked signed-64-bit
/// integer arithmetic and bounded execution, never relying on host build-profile
/// overflow behavior or intentionally unbounded loops.
pub fn execute(program: &Program) -> Result<Value, Diagnostic> {
    Interpreter::new(program).execute_main()
}

type Frame = BTreeMap<BindingId, Option<Value>>;

enum Flow {
    Value(Value),
    Return(Value),
}

struct Interpreter<'program> {
    program: &'program Program,
    call_depth: usize,
    steps: usize,
}

impl<'program> Interpreter<'program> {
    fn new(program: &'program Program) -> Self {
        Self {
            program,
            call_depth: 0,
            steps: 0,
        }
    }

    fn execute_main(&mut self) -> Result<Value, Diagnostic> {
        let Some(main) = self
            .program
            .functions
            .iter()
            .find(|function| function.name == "main")
        else {
            return Err(Diagnostic::error("N4001", "missing entry point").with_note(
                "`nova run` requires a top-level `fn main() -> Int` or `fn main() -> Bool`",
            ));
        };
        if !main.parameters.is_empty() {
            return Err(
                Diagnostic::error("N4001", "invalid entry point").with_primary(
                    main.span,
                    "`main` must not declare parameters in the bootstrap interpreter",
                ),
            );
        }
        if !matches!(main.return_type, Type::Int | Type::Bool) {
            return Err(
                Diagnostic::error("N4001", "invalid entry point").with_primary(
                    main.span,
                    format!(
                        "`main` cannot return {} in the bootstrap interpreter",
                        main.return_type
                    ),
                ),
            );
        }
        self.call_function(main.id, Vec::new())
    }

    fn call_function(
        &mut self,
        function_id: FunctionId,
        arguments: Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        let Some(function) = self.program.functions.get(function_id.index()).cloned() else {
            return Err(self.invariant(
                self.program.span,
                format!(
                    "resolved function id {} is outside the program",
                    function_id.index()
                ),
            ));
        };
        if function.parameters.len() != arguments.len() {
            return Err(self.invariant(
                function.span,
                format!(
                    "resolved call supplied {} argument(s) to {} parameter(s)",
                    arguments.len(),
                    function.parameters.len()
                ),
            ));
        }
        if self.call_depth >= MAX_CALL_DEPTH {
            return Err(
                Diagnostic::error("N4004", "execution call-depth limit exceeded")
                    .with_primary(
                        function.span,
                        format!(
                            "the bootstrap interpreter allows at most {MAX_CALL_DEPTH} active function calls"
                        ),
                    )
                    .with_note("this guard prevents uncontrolled host-stack recursion"),
            );
        }

        let mut frame = Frame::new();
        for (parameter, argument) in function.parameters.iter().zip(arguments) {
            frame.insert(parameter.id, Some(argument));
        }

        self.call_depth += 1;
        let result = self.eval_function(&function, &mut frame);
        self.call_depth -= 1;
        result
    }

    fn eval_function(
        &mut self,
        function: &Function,
        frame: &mut Frame,
    ) -> Result<Value, Diagnostic> {
        match self.eval_block(&function.body, frame)? {
            Flow::Value(value) | Flow::Return(value) => Ok(value),
        }
    }

    fn eval_block(&mut self, block: &Block, frame: &mut Frame) -> Result<Flow, Diagnostic> {
        for statement in &block.statements {
            if let Some(value) = self.eval_statement(statement, frame)? {
                return Ok(Flow::Return(value));
            }
        }

        match block.tail.as_deref() {
            Some(expression) => self.eval_expression(expression, frame),
            None => Ok(Flow::Value(Value::Unit)),
        }
    }

    fn eval_statement(
        &mut self,
        statement: &Statement,
        frame: &mut Frame,
    ) -> Result<Option<Value>, Diagnostic> {
        self.step(statement.span)?;
        match &statement.kind {
            StatementKind::Binding {
                binding,
                initializer,
            } => match self.eval_expression(initializer, frame)? {
                Flow::Value(value) => {
                    frame.insert(binding.id, Some(value));
                    Ok(None)
                }
                Flow::Return(value) => Ok(Some(value)),
            },
            StatementKind::UninitializedBinding(binding) => {
                frame.insert(binding.id, None);
                Ok(None)
            }
            StatementKind::Assignment { target, value } => {
                let Some(target) = target else {
                    return Err(self.invariant(
                        statement.span,
                        "semantically accepted assignment has no resolved target",
                    ));
                };
                match self.eval_expression(value, frame)? {
                    Flow::Value(value) => {
                        let Some(slot) = frame.get_mut(target) else {
                            return Err(self.invariant(
                                statement.span,
                                format!(
                                    "assignment target {} is absent from the frame",
                                    target.index()
                                ),
                            ));
                        };
                        *slot = Some(value);
                        Ok(None)
                    }
                    Flow::Return(value) => Ok(Some(value)),
                }
            }
            StatementKind::While { condition, body } => loop {
                let condition = match self.eval_expression(condition, frame)? {
                    Flow::Value(value) => value,
                    Flow::Return(value) => return Ok(Some(value)),
                };
                match condition {
                    Value::Bool(false) => return Ok(None),
                    Value::Bool(true) => match self.eval_block(body, frame)? {
                        Flow::Value(_) => {}
                        Flow::Return(value) => return Ok(Some(value)),
                    },
                    _ => {
                        return Err(self.invariant(
                            statement.span,
                            "semantically accepted `while` condition was not Bool",
                        ));
                    }
                }
            },
            StatementKind::Return(expression) => match self.eval_expression(expression, frame)? {
                Flow::Value(value) | Flow::Return(value) => Ok(Some(value)),
            },
            StatementKind::Expression(expression) => {
                match self.eval_expression(expression, frame)? {
                    Flow::Value(_) => Ok(None),
                    Flow::Return(value) => Ok(Some(value)),
                }
            }
        }
    }

    fn eval_expression(
        &mut self,
        expression: &Expression,
        frame: &mut Frame,
    ) -> Result<Flow, Diagnostic> {
        self.step(expression.span)?;
        match &expression.kind {
            ExpressionKind::Integer(value) => Ok(Flow::Value(Value::Int(*value))),
            ExpressionKind::Boolean(value) => Ok(Flow::Value(Value::Bool(*value))),
            ExpressionKind::Binding(binding) => {
                let Some(slot) = frame.get(binding) else {
                    return Err(self.invariant(
                        expression.span,
                        format!(
                            "resolved binding {} is absent from the frame",
                            binding.index()
                        ),
                    ));
                };
                let Some(value) = slot else {
                    return Err(self.invariant(
                        expression.span,
                        format!(
                            "binding {} reached runtime before initialization",
                            binding.index()
                        ),
                    ));
                };
                Ok(Flow::Value(value.clone()))
            }
            ExpressionKind::Function(function) => Ok(Flow::Value(Value::Function(*function))),
            ExpressionKind::RecordLiteral { record, fields } => {
                let Some(definition) = self.program.records.get(record.index()) else {
                    return Err(self.invariant(
                        expression.span,
                        format!(
                            "resolved record id {} is outside the program",
                            record.index()
                        ),
                    ));
                };
                if definition.id != *record {
                    return Err(self.invariant(
                        expression.span,
                        "record declaration index does not match its resolved identity",
                    ));
                }

                let mut slots = vec![None; definition.fields.len()];
                for field in fields {
                    let value = match self.eval_expression(&field.value, frame)? {
                        Flow::Value(value) => value,
                        Flow::Return(value) => return Ok(Flow::Return(value)),
                    };
                    let Some(slot) = slots.get_mut(field.field_index) else {
                        return Err(self.invariant(
                            expression.span,
                            format!(
                                "record initializer targets field slot {} outside record `{}`",
                                field.field_index, definition.name
                            ),
                        ));
                    };
                    if slot.is_some() {
                        return Err(self.invariant(
                            expression.span,
                            format!(
                                "record initializer targets field slot {} more than once",
                                field.field_index
                            ),
                        ));
                    }
                    *slot = Some(value);
                }

                let mut values = Vec::with_capacity(slots.len());
                for (index, slot) in slots.into_iter().enumerate() {
                    let Some(value) = slot else {
                        return Err(self.invariant(
                            expression.span,
                            format!("record field slot {index} was not initialized"),
                        ));
                    };
                    values.push(value);
                }
                Ok(Flow::Value(Value::Record {
                    record: *record,
                    fields: values,
                }))
            }
            ExpressionKind::EnumConstructor {
                enumeration,
                variant_index,
                payload,
            } => {
                let Some(definition) = self.program.enums.get(enumeration.index()) else {
                    return Err(self.invariant(
                        expression.span,
                        format!(
                            "resolved enum id {} is outside the program",
                            enumeration.index()
                        ),
                    ));
                };
                if definition.id != *enumeration {
                    return Err(self.invariant(
                        expression.span,
                        "enum declaration index does not match its resolved identity",
                    ));
                }
                let Some(variant) = definition.variants.get(*variant_index) else {
                    return Err(self.invariant(
                        expression.span,
                        format!(
                            "variant slot {variant_index} is outside enum `{}`",
                            definition.name
                        ),
                    ));
                };
                if variant.payload.is_some() != payload.is_some() {
                    return Err(self.invariant(
                        expression.span,
                        "resolved enum constructor payload arity does not match its variant",
                    ));
                }
                let payload = if let Some(payload) = payload {
                    match self.eval_expression(payload, frame)? {
                        Flow::Value(value) => Some(Box::new(value)),
                        Flow::Return(value) => return Ok(Flow::Return(value)),
                    }
                } else {
                    None
                };
                Ok(Flow::Value(Value::Enum {
                    enumeration: *enumeration,
                    variant_index: *variant_index,
                    payload,
                }))
            }
            ExpressionKind::FieldAccess {
                base,
                record,
                field_index,
            } => {
                let base = match self.eval_expression(base, frame)? {
                    Flow::Value(value) => value,
                    Flow::Return(value) => return Ok(Flow::Return(value)),
                };
                let Value::Record {
                    record: actual,
                    fields,
                } = base
                else {
                    return Err(self.invariant(
                        expression.span,
                        "semantically accepted field access did not evaluate to a record",
                    ));
                };
                if actual != *record {
                    return Err(self.invariant(
                        expression.span,
                        format!(
                            "field access expected record {}, found record {}",
                            record.index(),
                            actual.index()
                        ),
                    ));
                }
                let Some(value) = fields.get(*field_index).cloned() else {
                    return Err(self.invariant(
                        expression.span,
                        format!("field slot {field_index} is outside the runtime record"),
                    ));
                };
                Ok(Flow::Value(value))
            }
            ExpressionKind::Unary { operator, operand } => {
                let operand = match self.eval_expression(operand, frame)? {
                    Flow::Value(value) => value,
                    Flow::Return(value) => return Ok(Flow::Return(value)),
                };
                self.eval_unary(*operator, operand, expression)
                    .map(Flow::Value)
            }
            ExpressionKind::Binary {
                operator,
                left,
                right,
            } => self.eval_binary(*operator, left, right, expression, frame),
            ExpressionKind::Call { callee, arguments } => {
                let callee = match self.eval_expression(callee, frame)? {
                    Flow::Value(value) => value,
                    Flow::Return(value) => return Ok(Flow::Return(value)),
                };
                let Value::Function(function) = callee else {
                    return Err(self.invariant(
                        expression.span,
                        "semantically accepted call did not evaluate to a function",
                    ));
                };
                let mut values = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    match self.eval_expression(argument, frame)? {
                        Flow::Value(value) => values.push(value),
                        Flow::Return(value) => return Ok(Flow::Return(value)),
                    }
                }
                self.call_function(function, values).map(Flow::Value)
            }
            ExpressionKind::Block(block) => self.eval_block(block, frame),
            ExpressionKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition = match self.eval_expression(condition, frame)? {
                    Flow::Value(value) => value,
                    Flow::Return(value) => return Ok(Flow::Return(value)),
                };
                match condition {
                    Value::Bool(true) => self.eval_block(then_branch, frame),
                    Value::Bool(false) => self.eval_expression(else_branch, frame),
                    _ => Err(self.invariant(
                        expression.span,
                        "semantically accepted `if` condition was not Bool",
                    )),
                }
            }
            ExpressionKind::Match {
                scrutinee,
                enumeration,
                arms,
            } => {
                let Some(definition) = self.program.enums.get(enumeration.index()) else {
                    return Err(self.invariant(
                        expression.span,
                        format!(
                            "resolved enum id {} is outside the program",
                            enumeration.index()
                        ),
                    ));
                };
                if definition.id != *enumeration || arms.len() != definition.variants.len() {
                    return Err(self.invariant(
                        expression.span,
                        "resolved match is not exhaustive for its enum declaration",
                    ));
                }
                let mut covered = vec![false; definition.variants.len()];
                for arm in arms {
                    let Some(slot) = covered.get_mut(arm.variant_index) else {
                        return Err(self.invariant(
                            arm.span,
                            "resolved match arm targets a variant outside its enum",
                        ));
                    };
                    if *slot {
                        return Err(self.invariant(
                            arm.span,
                            "resolved match contains a duplicate variant arm",
                        ));
                    }
                    *slot = true;
                    let declared = &definition.variants[arm.variant_index];
                    if declared.payload.is_some() != arm.binding.is_some() {
                        return Err(self.invariant(
                            arm.span,
                            "resolved match binding arity does not match its variant",
                        ));
                    }
                }

                let scrutinee = match self.eval_expression(scrutinee, frame)? {
                    Flow::Value(value) => value,
                    Flow::Return(value) => return Ok(Flow::Return(value)),
                };
                let Value::Enum {
                    enumeration: actual_enum,
                    variant_index,
                    payload,
                } = scrutinee
                else {
                    return Err(self.invariant(
                        expression.span,
                        "semantically accepted match did not evaluate to an enum",
                    ));
                };
                if actual_enum != *enumeration {
                    return Err(self.invariant(
                        expression.span,
                        format!(
                            "match expected enum {}, found enum {}",
                            enumeration.index(),
                            actual_enum.index()
                        ),
                    ));
                }
                let Some(arm) = arms.iter().find(|arm| arm.variant_index == variant_index) else {
                    return Err(self.invariant(
                        expression.span,
                        "exhaustive match has no arm for the runtime variant",
                    ));
                };
                match (&arm.binding, payload) {
                    (Some(binding), Some(payload)) => {
                        frame.insert(binding.id, Some(*payload));
                    }
                    (None, None) => {}
                    _ => {
                        return Err(self.invariant(
                            arm.span,
                            "runtime enum payload arity does not match the selected arm",
                        ));
                    }
                }
                self.eval_expression(&arm.value, frame)
            }
            ExpressionKind::Error => Err(self.invariant(
                expression.span,
                "error-recovery HIR reached the bootstrap interpreter",
            )),
        }
    }

    fn eval_unary(
        &self,
        operator: UnaryOperator,
        operand: Value,
        expression: &Expression,
    ) -> Result<Value, Diagnostic> {
        match (operator, operand) {
            (UnaryOperator::Negate, Value::Int(value)) => value
                .checked_neg()
                .map(Value::Int)
                .ok_or_else(|| self.overflow(expression)),
            (UnaryOperator::Not, Value::Bool(value)) => Ok(Value::Bool(!value)),
            _ => Err(self.invariant(
                expression.span,
                "semantically accepted unary operator received the wrong runtime value",
            )),
        }
    }

    fn eval_binary(
        &mut self,
        operator: BinaryOperator,
        left: &Expression,
        right: &Expression,
        expression: &Expression,
        frame: &mut Frame,
    ) -> Result<Flow, Diagnostic> {
        let left = match self.eval_expression(left, frame)? {
            Flow::Value(value) => value,
            Flow::Return(value) => return Ok(Flow::Return(value)),
        };

        match (operator, &left) {
            (BinaryOperator::And, Value::Bool(false)) => {
                return Ok(Flow::Value(Value::Bool(false)));
            }
            (BinaryOperator::Or, Value::Bool(true)) => {
                return Ok(Flow::Value(Value::Bool(true)));
            }
            _ => {}
        }

        let right = match self.eval_expression(right, frame)? {
            Flow::Value(value) => value,
            Flow::Return(value) => return Ok(Flow::Return(value)),
        };
        self.apply_binary(operator, left, right, expression)
            .map(Flow::Value)
    }

    fn apply_binary(
        &self,
        operator: BinaryOperator,
        left: Value,
        right: Value,
        expression: &Expression,
    ) -> Result<Value, Diagnostic> {
        match (operator, left, right) {
            (BinaryOperator::Add, Value::Int(left), Value::Int(right)) => left
                .checked_add(right)
                .map(Value::Int)
                .ok_or_else(|| self.overflow(expression)),
            (BinaryOperator::Subtract, Value::Int(left), Value::Int(right)) => left
                .checked_sub(right)
                .map(Value::Int)
                .ok_or_else(|| self.overflow(expression)),
            (BinaryOperator::Multiply, Value::Int(left), Value::Int(right)) => left
                .checked_mul(right)
                .map(Value::Int)
                .ok_or_else(|| self.overflow(expression)),
            (BinaryOperator::Divide, Value::Int(_), Value::Int(0))
            | (BinaryOperator::Remainder, Value::Int(_), Value::Int(0)) => {
                Err(Diagnostic::error("N4003", "division by zero")
                    .with_primary(expression.span, "zero divisor is not executable"))
            }
            (BinaryOperator::Divide, Value::Int(left), Value::Int(right)) => left
                .checked_div(right)
                .map(Value::Int)
                .ok_or_else(|| self.overflow(expression)),
            (BinaryOperator::Remainder, Value::Int(left), Value::Int(right)) => left
                .checked_rem(right)
                .map(Value::Int)
                .ok_or_else(|| self.overflow(expression)),
            (BinaryOperator::Less, Value::Int(left), Value::Int(right)) => {
                Ok(Value::Bool(left < right))
            }
            (BinaryOperator::LessEqual, Value::Int(left), Value::Int(right)) => {
                Ok(Value::Bool(left <= right))
            }
            (BinaryOperator::Greater, Value::Int(left), Value::Int(right)) => {
                Ok(Value::Bool(left > right))
            }
            (BinaryOperator::GreaterEqual, Value::Int(left), Value::Int(right)) => {
                Ok(Value::Bool(left >= right))
            }
            (BinaryOperator::Equal, Value::Int(left), Value::Int(right)) => {
                Ok(Value::Bool(left == right))
            }
            (BinaryOperator::NotEqual, Value::Int(left), Value::Int(right)) => {
                Ok(Value::Bool(left != right))
            }
            (BinaryOperator::Equal, Value::Bool(left), Value::Bool(right)) => {
                Ok(Value::Bool(left == right))
            }
            (BinaryOperator::NotEqual, Value::Bool(left), Value::Bool(right)) => {
                Ok(Value::Bool(left != right))
            }
            (BinaryOperator::And, Value::Bool(left), Value::Bool(right)) => {
                Ok(Value::Bool(left && right))
            }
            (BinaryOperator::Or, Value::Bool(left), Value::Bool(right)) => {
                Ok(Value::Bool(left || right))
            }
            _ => Err(self.invariant(
                expression.span,
                "semantically accepted binary operator received incompatible runtime values",
            )),
        }
    }

    fn step(&mut self, span: nova_source::Span) -> Result<(), Diagnostic> {
        if self.steps >= MAX_EXECUTION_STEPS {
            return Err(
                Diagnostic::error("N4006", "execution step limit exceeded")
                    .with_primary(
                        span,
                        format!(
                            "the bootstrap interpreter allows at most {MAX_EXECUTION_STEPS} evaluation steps"
                        ),
                    )
                    .with_note("this guard stops nonterminating loops and runaway execution"),
            );
        }
        self.steps += 1;
        Ok(())
    }

    fn overflow(&self, expression: &Expression) -> Diagnostic {
        Diagnostic::error("N4002", "integer arithmetic overflow").with_primary(
            expression.span,
            "bootstrap Int arithmetic is checked signed 64-bit execution",
        )
    }

    fn invariant(&self, span: nova_source::Span, message: impl Into<String>) -> Diagnostic {
        Diagnostic::error("N4005", "interpreter invariant violated")
            .with_primary(span, message.into())
    }
}

#[cfg(test)]
mod tests {
    use super::{Value, execute};
    use nova_lexer::lex;
    use nova_parser::parse;
    use nova_sema::{analyze, hir::ExpressionKind};
    use nova_source::{SourceFile, SourceId};

    fn execute_text(text: &str) -> Result<Value, nova_diagnostics::Diagnostic> {
        let source = SourceFile::new(SourceId::new(0), "test.nv", text);
        let lexed = lex(&source);
        assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
        let parsed = parse(&source, &lexed.tokens);
        assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
        let analyzed = analyze(&parsed.program);
        assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
        execute(&analyzed.program)
    }

    #[test]
    fn executes_records_projection_and_function_passing() {
        let value = execute_text(
            "record Pair { left: Int, right: Int }\n\
             fn sum(pair: Pair) -> Int { pair.left + pair.right }\n\
             fn main() -> Int { sum(new Pair { right: 2, left: 40 }) }",
        )
        .expect("program executes");
        assert_eq!(value, Value::Int(42));
    }

    #[test]
    fn executes_recursive_enums_and_payload_matching() {
        let value = execute_text(
            "enum Nat { Zero, Succ(Nat) }\n\
             fn to_int(number: Nat) -> Int {\n\
                 match number {\n\
                     Nat::Zero => 0,\n\
                     Nat::Succ(previous) => 1 + to_int(previous),\n\
                 }\n\
             }\n\
             fn main() -> Int { to_int(Nat::Succ(Nat::Succ(Nat::Zero))) }",
        )
        .expect("recursive enum program should execute");
        assert_eq!(value, Value::Int(2));
    }

    #[test]
    fn match_evaluates_only_the_selected_arm_and_propagates_return() {
        let selected = execute_text(
            "enum Choice { Safe, Dangerous }\n\
             fn main() -> Int {\n\
                 match Choice::Safe {\n\
                     Choice::Safe => 42,\n\
                     Choice::Dangerous => 1 / 0,\n\
                 }\n\
             }",
        )
        .expect("unselected arm must not execute");
        assert_eq!(selected, Value::Int(42));

        let returned = execute_text(
            "enum Choice { Early, Later }\n\
             fn choose(value: Choice) -> Int {\n\
                 match value {\n\
                     Choice::Early => { return 7; },\n\
                     Choice::Later => 9,\n\
                 }\n\
             }\n\
             fn main() -> Int { choose(Choice::Early) }",
        )
        .expect("return should propagate through a selected match arm");
        assert_eq!(returned, Value::Int(7));

        let scrutinee_return = execute_text(
            "enum Choice { A, B }\n\
             fn main() -> Int {\n\
                 match { return 5; } { Choice::A => 1, Choice::B => 2, }\n\
             }",
        )
        .expect("return should propagate while evaluating a match scrutinee");
        assert_eq!(scrutinee_return, Value::Int(5));
    }

    #[test]
    fn malformed_non_exhaustive_match_hir_fails_closed() {
        let source = SourceFile::new(
            SourceId::new(0),
            "test.nv",
            "enum Choice { A, B } fn main() -> Int { match Choice::A { Choice::A => 1, Choice::B => 2, } }",
        );
        let lexed = lex(&source);
        let parsed = parse(&source, &lexed.tokens);
        let mut analyzed = analyze(&parsed.program);
        assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
        let tail = analyzed.program.functions[0]
            .body
            .tail
            .as_deref_mut()
            .expect("match tail");
        let ExpressionKind::Match { arms, .. } = &mut tail.kind else {
            panic!("expected match HIR");
        };
        arms.pop();

        let diagnostic = execute(&analyzed.program).expect_err("malformed HIR must fail");
        assert_eq!(diagnostic.code, "N4005");
    }

    #[test]
    fn record_initializers_preserve_source_evaluation_order() {
        let value = execute_text(
            "record Pair { left: Int, right: Int }\n\
             fn main() -> Int {\n\
                 var state = 0;\n\
                 let pair = new Pair {\n\
                     right: { state = state * 10 + 2; state },\n\
                     left: { state = state * 10 + 1; state },\n\
                 };\n\
                 pair.left;\n\
                 state\n\
             }",
        )
        .expect("program executes");
        assert_eq!(value, Value::Int(21));
    }

    #[test]
    fn executes_arithmetic_calls_recursion_and_if() {
        let value = execute_text(
            "fn factorial(n: Int) -> Int {\n\
                 if n == 0 { 1 } else { n * factorial(n - 1) }\n\
             }\n\
             fn main() -> Int { factorial(5) }",
        )
        .expect("program executes");
        assert_eq!(value, Value::Int(120));
    }

    #[test]
    fn executes_mutation_and_definite_assignment() {
        let value = execute_text(
            "fn main() -> Int { var value: Int; value = 40; value = value + 2; value }",
        )
        .expect("program executes");
        assert_eq!(value, Value::Int(42));
    }

    #[test]
    fn executes_while_loop_and_mutation() {
        let value = execute_text(
            "fn main() -> Int { var value = 0; while value < 5 { value = value + 1; } value }",
        )
        .expect("program executes");
        assert_eq!(value, Value::Int(5));
    }

    #[test]
    fn return_propagates_out_of_while_body() {
        let value = execute_text("fn main() -> Int { while true { return 7; } 0 }")
            .expect("program executes");
        assert_eq!(value, Value::Int(7));
    }

    #[test]
    fn bounds_nonterminating_loops() {
        let error = execute_text("fn main() -> Int { while true {} 0 }")
            .expect_err("nonterminating loop must fail closed");
        assert_eq!(error.code, "N4006");
    }

    #[test]
    fn propagates_return_through_nested_expressions() {
        let value =
            execute_text("fn main() -> Int { { return 7; }; 1 }").expect("program executes");
        assert_eq!(value, Value::Int(7));
    }

    #[test]
    fn boolean_operators_short_circuit_left_to_right() {
        let value = execute_text(
            "fn main() -> Int {\n\
                 var value = 0;\n\
                 false && { value = 1; true };\n\
                 true || { value = 2; false };\n\
                 value\n\
             }",
        )
        .expect("program executes");
        assert_eq!(value, Value::Int(0));
    }

    #[test]
    fn rejects_checked_integer_overflow() {
        let error = execute_text("fn main() -> Int { 9223372036854775807 + 1 }")
            .expect_err("overflow must fail");
        assert_eq!(error.code, "N4002");
    }

    #[test]
    fn rejects_zero_divisor() {
        for text in ["fn main() -> Int { 10 / 0 }", "fn main() -> Int { 10 % 0 }"] {
            let error = execute_text(text).expect_err("zero divisor must fail");
            assert_eq!(error.code, "N4003");
        }
    }

    #[test]
    fn requires_zero_argument_main() {
        let source = SourceFile::new(SourceId::new(0), "test.nv", "fn other() -> Int { 1 }");
        let lexed = lex(&source);
        let parsed = parse(&source, &lexed.tokens);
        let analyzed = analyze(&parsed.program);
        let error = execute(&analyzed.program).expect_err("missing main must fail");
        assert_eq!(error.code, "N4001");

        let source = SourceFile::new(SourceId::new(0), "test.nv", "fn main(x: Int) -> Int { x }");
        let lexed = lex(&source);
        let parsed = parse(&source, &lexed.tokens);
        let analyzed = analyze(&parsed.program);
        let error = execute(&analyzed.program).expect_err("parameterized main must fail");
        assert_eq!(error.code, "N4001");
    }

    #[test]
    fn bounds_recursive_execution() {
        let error = execute_text(
            "fn recurse() -> Int { recurse() }\n\
             fn main() -> Int { recurse() }",
        )
        .expect_err("unbounded recursion must fail closed");
        assert_eq!(error.code, "N4004");
    }
}
