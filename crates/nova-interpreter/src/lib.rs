//! Deterministic bootstrap interpreter for semantically accepted Nova HIR.

use nova_diagnostics::Diagnostic;
use nova_int_semantics as int_semantics;
use nova_int_semantics::IntArithmeticError;
use nova_parser::ast::{BinaryOperator, UnaryOperator};
use nova_sema::equality_rules::matching_equality_types;
use nova_sema::hir::{
    BindingId, BindingReference, Block, EnumId, Expression, ExpressionKind, Function, FunctionId,
    Program, RecordId, Statement, StatementKind, Type,
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
    /// Immutable UTF-8 string value.
    String(String),
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
    /// Unit value produced by `()` or a value-less block.
    Unit,
}

impl fmt::Display for Value {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(value) => write!(formatter, "{value}"),
            Self::Bool(value) => write!(formatter, "{value}"),
            Self::String(value) => formatter.write_str(value),
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

struct RuntimeSlot {
    ty: Type,
    mutable: bool,
    binding_name: String,
    declaration_span: nova_source::Span,
    value: Option<Value>,
}

type Frame = BTreeMap<BindingId, RuntimeSlot>;

enum Flow {
    Value(Value),
    Return(Value),
    Break,
    Continue,
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
                "`nova run` requires a top-level zero-argument `main` returning `Int`, `Bool`, `String`, or `Unit`",
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
        if !matches!(
            main.return_type,
            Type::Int | Type::Bool | Type::String | Type::Unit
        ) {
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
        if function.id != function_id {
            return Err(self.invariant(
                function.span,
                "function declaration index does not match its resolved identity",
            ));
        }
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
        for (index, (parameter, argument)) in function.parameters.iter().zip(&arguments).enumerate()
        {
            if !self.value_conforms_to_type(argument, &parameter.ty) {
                return Err(self.invariant(
                    function.span,
                    format!(
                        "argument {index} for parameter `{}` of function `{}` does not conform to declared runtime type {}",
                        parameter.name, function.name, parameter.ty
                    ),
                ));
            }
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
            self.bind_runtime_slot(&mut frame, parameter, Some(argument), function.span)?;
        }

        self.call_depth += 1;
        let result = self.eval_function(&function, &mut frame);
        self.call_depth -= 1;
        let value = result?;
        if !self.value_conforms_to_type(&value, &function.return_type) {
            return Err(self.invariant(
                function.span,
                format!(
                    "function `{}` returned a runtime value that does not conform to declared type {}",
                    function.name, function.return_type
                ),
            ));
        }
        Ok(value)
    }

    fn eval_function(
        &mut self,
        function: &Function,
        frame: &mut Frame,
    ) -> Result<Value, Diagnostic> {
        match self.eval_block(&function.body, frame)? {
            Flow::Value(value) | Flow::Return(value) => Ok(value),
            Flow::Break | Flow::Continue => Err(self.invariant(
                function.span,
                "loop control escaped the function that owns its lexical loop",
            )),
        }
    }

    fn eval_block(&mut self, block: &Block, frame: &mut Frame) -> Result<Flow, Diagnostic> {
        let flow = self.eval_block_unchecked(block, frame)?;
        if let Flow::Value(value) = &flow {
            if !self.value_conforms_to_type(value, &block.ty) {
                return Err(self.invariant(
                    block.span,
                    format!(
                        "block produced a runtime value that does not conform to HIR type {}",
                        block.ty
                    ),
                ));
            }
        }
        Ok(flow)
    }

    fn eval_block_unchecked(
        &mut self,
        block: &Block,
        frame: &mut Frame,
    ) -> Result<Flow, Diagnostic> {
        for statement in &block.statements {
            if let Some(flow) = self.eval_statement(statement, frame)? {
                return Ok(flow);
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
    ) -> Result<Option<Flow>, Diagnostic> {
        self.step(statement.span)?;
        match &statement.kind {
            StatementKind::Binding {
                binding,
                initializer,
            } => match self.eval_expression(initializer, frame)? {
                Flow::Value(value) => {
                    self.bind_runtime_slot(frame, binding, Some(value), binding.span)?;
                    Ok(None)
                }
                flow => Ok(Some(flow)),
            },
            StatementKind::UninitializedBinding(binding) => {
                if !binding.mutable {
                    return Err(self.invariant(
                        binding.span,
                        "semantically accepted uninitialized binding is not mutable",
                    ));
                }
                self.bind_runtime_slot(frame, binding, None, binding.span)?;
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
                        self.validate_binding_reference(frame, target, statement.span)?;
                        let slot = frame
                            .get_mut(&target.binding)
                            .expect("validated assignment target must have a runtime slot");
                        if !slot.mutable {
                            return Err(self.invariant(
                                statement.span,
                                format!(
                                    "assignment target {} resolved to an immutable runtime slot",
                                    target.binding.index()
                                ),
                            ));
                        }
                        if !self.value_conforms_to_type(&value, &slot.ty) {
                            return Err(self.invariant(
                                statement.span,
                                format!(
                                    "assignment target {} received a runtime value that does not conform to slot type {}",
                                    target.binding.index(), slot.ty
                                ),
                            ));
                        }
                        slot.value = Some(value);
                        Ok(None)
                    }
                    flow => Ok(Some(flow)),
                }
            }
            StatementKind::While { condition, body } => loop {
                let condition = match self.eval_expression(condition, frame)? {
                    Flow::Value(value) => value,
                    Flow::Return(value) => return Ok(Some(Flow::Return(value))),
                    Flow::Break | Flow::Continue => {
                        return Err(self.invariant(
                            condition.span,
                            "loop control reached a `while` condition outside the loop-body control scope",
                        ));
                    }
                };
                match condition {
                    Value::Bool(false) => return Ok(None),
                    Value::Bool(true) => match self.eval_block(body, frame)? {
                        Flow::Value(_) | Flow::Continue => {}
                        Flow::Break => return Ok(None),
                        Flow::Return(value) => return Ok(Some(Flow::Return(value))),
                    },
                    _ => {
                        return Err(self.invariant(
                            statement.span,
                            "semantically accepted `while` condition was not Bool",
                        ));
                    }
                }
            },
            StatementKind::Break => Ok(Some(Flow::Break)),
            StatementKind::Continue => Ok(Some(Flow::Continue)),
            StatementKind::Return(expression) => {
                let Some(expression) = expression else {
                    return Ok(Some(Flow::Return(Value::Unit)));
                };
                match self.eval_expression(expression, frame)? {
                    Flow::Value(value) | Flow::Return(value) => Ok(Some(Flow::Return(value))),
                    Flow::Break => Ok(Some(Flow::Break)),
                    Flow::Continue => Ok(Some(Flow::Continue)),
                }
            }
            StatementKind::Expression(expression) => {
                match self.eval_expression(expression, frame)? {
                    Flow::Value(_) => Ok(None),
                    flow => Ok(Some(flow)),
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
        let flow = self.eval_expression_unchecked(expression, frame)?;
        if let Flow::Value(value) = &flow {
            if !self.value_conforms_to_type(value, &expression.ty) {
                return Err(self.invariant(
                    expression.span,
                    format!(
                        "expression produced a runtime value that does not conform to HIR type {}",
                        expression.ty
                    ),
                ));
            }
        }
        Ok(flow)
    }

    fn eval_expression_unchecked(
        &mut self,
        expression: &Expression,
        frame: &mut Frame,
    ) -> Result<Flow, Diagnostic> {
        match &expression.kind {
            ExpressionKind::Integer(value) => Ok(Flow::Value(Value::Int(*value))),
            ExpressionKind::String(value) => Ok(Flow::Value(Value::String(value.clone()))),
            ExpressionKind::Boolean(value) => Ok(Flow::Value(Value::Bool(*value))),
            ExpressionKind::Unit => Ok(Flow::Value(Value::Unit)),
            ExpressionKind::Binding(reference) => {
                self.validate_binding_reference(frame, reference, expression.span)?;
                let slot = frame
                    .get(&reference.binding)
                    .expect("validated binding reference must have a runtime slot");
                if expression.ty != slot.ty {
                    return Err(self.invariant(
                        expression.span,
                        format!(
                            "binding {} expression type {} does not match runtime slot type {}",
                            reference.binding.index(),
                            expression.ty,
                            slot.ty
                        ),
                    ));
                }
                let Some(value) = slot.value.as_ref() else {
                    return Err(self.invariant(
                        expression.span,
                        format!(
                            "binding {} reached runtime before initialization",
                            reference.binding.index()
                        ),
                    ));
                };
                if !self.value_conforms_to_type(value, &slot.ty) {
                    return Err(self.invariant(
                        expression.span,
                        format!(
                            "binding {} stored a runtime value that does not conform to slot type {}",
                            reference.binding.index(), slot.ty
                        ),
                    ));
                }
                Ok(Flow::Value(value.clone()))
            }
            ExpressionKind::Function {
                function,
                function_name,
            } => {
                self.resolved_function_reference(*function, function_name, expression.span)?;
                Ok(Flow::Value(Value::Function(*function)))
            }
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
                        flow => return Ok(flow),
                    };
                    let field_definition = self.resolved_record_field(
                        *record,
                        field.field_index,
                        &field.field_name,
                        field.value.span,
                    )?;
                    if !self.value_conforms_to_type(&value, &field_definition.ty) {
                        return Err(self.invariant(
                            field.value.span,
                            format!(
                                "record field `{}` of `{}` received a runtime value that does not conform to declared type {}",
                                field_definition.name, definition.name, field_definition.ty
                            ),
                        ));
                    }
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
                variant_name,
                variant_index,
                payload,
            } => {
                let payload = if let Some(payload) = payload {
                    match self.eval_expression(payload, frame)? {
                        Flow::Value(value) => Some(Box::new(value)),
                        flow => return Ok(flow),
                    }
                } else {
                    None
                };
                let (definition, variant) = self.resolved_enum_variant(
                    *enumeration,
                    *variant_index,
                    variant_name,
                    expression.span,
                )?;
                if variant.payload.is_some() != payload.is_some() {
                    return Err(self.invariant(
                        expression.span,
                        "resolved enum constructor payload arity does not match its variant",
                    ));
                }
                if let (Some(payload_type), Some(payload_value)) =
                    (&variant.payload, payload.as_deref())
                {
                    if !self.value_conforms_to_type(payload_value, payload_type) {
                        return Err(self.invariant(
                            expression.span,
                            format!(
                                "enum variant `{}` of `{}` received a runtime payload that does not conform to declared type {}",
                                variant.name, definition.name, payload_type
                            ),
                        ));
                    }
                }
                Ok(Flow::Value(Value::Enum {
                    enumeration: *enumeration,
                    variant_index: *variant_index,
                    payload,
                }))
            }
            ExpressionKind::FieldAccess {
                base,
                record,
                field_name,
                field_index,
            } => {
                let base = match self.eval_expression(base, frame)? {
                    Flow::Value(value) => value,
                    flow => return Ok(flow),
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
                let field_definition =
                    self.resolved_record_field(*record, *field_index, field_name, expression.span)?;
                if expression.ty != field_definition.ty {
                    return Err(self.invariant(
                        expression.span,
                        format!(
                            "field access result type {} does not match resolved field `{}` type {}",
                            expression.ty, field_name, field_definition.ty
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
                    flow => return Ok(flow),
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
                    flow => return Ok(flow),
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
                        flow => return Ok(flow),
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
                    flow => return Ok(flow),
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
                let scrutinee = match self.eval_expression(scrutinee, frame)? {
                    Flow::Value(value) => value,
                    flow => return Ok(flow),
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
                    let (_, declared) = self.resolved_enum_variant(
                        *enumeration,
                        arm.variant_index,
                        &arm.variant_name,
                        arm.span,
                    )?;
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
                    let payload_shape_valid = matches!(
                        (
                            declared.payload.is_some(),
                            arm.binding.is_some(),
                            arm.payload_discarded,
                        ),
                        (true, true, false) | (true, false, true) | (false, false, false)
                    );
                    if !payload_shape_valid {
                        return Err(self.invariant(
                            arm.span,
                            "resolved match payload mode does not match its variant",
                        ));
                    }
                }

                let Some(arm) = arms.iter().find(|arm| arm.variant_index == variant_index) else {
                    return Err(self.invariant(
                        expression.span,
                        "exhaustive match has no arm for the runtime variant",
                    ));
                };
                match (&arm.binding, arm.payload_discarded, payload) {
                    (Some(binding), false, Some(payload)) => {
                        self.bind_runtime_slot(frame, binding, Some(*payload), arm.span)?;
                    }
                    (None, true, Some(_)) | (None, false, None) => {}
                    _ => {
                        return Err(self.invariant(
                            arm.span,
                            "runtime enum payload mode does not match the selected arm",
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
            (UnaryOperator::Negate, Value::Int(value)) => {
                self.int_result(int_semantics::negate(value), expression)
            }
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
        if matches!(operator, BinaryOperator::Equal | BinaryOperator::NotEqual)
            && !left.ty.is_never()
            && !right.ty.is_never()
            && !self.equality_types_match(&left.ty, &right.ty)
        {
            return Err(self.invariant(
                expression.span,
                format!(
                    "equality operator received HIR operand types outside the semantic equality contract: {} and {}",
                    left.ty, right.ty
                ),
            ));
        }

        let left = match self.eval_expression(left, frame)? {
            Flow::Value(value) => value,
            flow => return Ok(flow),
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
            flow => return Ok(flow),
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
            (BinaryOperator::Add, Value::Int(left), Value::Int(right)) => {
                self.int_result(int_semantics::add(left, right), expression)
            }
            (BinaryOperator::Subtract, Value::Int(left), Value::Int(right)) => {
                self.int_result(int_semantics::subtract(left, right), expression)
            }
            (BinaryOperator::Multiply, Value::Int(left), Value::Int(right)) => {
                self.int_result(int_semantics::multiply(left, right), expression)
            }
            (BinaryOperator::Divide, Value::Int(left), Value::Int(right)) => {
                self.int_result(int_semantics::divide(left, right), expression)
            }
            (BinaryOperator::Remainder, Value::Int(left), Value::Int(right)) => {
                self.int_result(int_semantics::remainder(left, right), expression)
            }
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
            (BinaryOperator::Equal, Value::String(left), Value::String(right)) => {
                Ok(Value::Bool(left == right))
            }
            (BinaryOperator::NotEqual, Value::String(left), Value::String(right)) => {
                Ok(Value::Bool(left != right))
            }
            (BinaryOperator::Equal, Value::Unit, Value::Unit) => Ok(Value::Bool(true)),
            (BinaryOperator::NotEqual, Value::Unit, Value::Unit) => Ok(Value::Bool(false)),
            (BinaryOperator::Equal, Value::Function(left), Value::Function(right)) => {
                if !self.function_signatures_match(left, right) {
                    return Err(self.invariant(
                        expression.span,
                        "function equality received runtime functions with different signatures",
                    ));
                }
                Ok(Value::Bool(left == right))
            }
            (BinaryOperator::NotEqual, Value::Function(left), Value::Function(right)) => {
                if !self.function_signatures_match(left, right) {
                    return Err(self.invariant(
                        expression.span,
                        "function equality received runtime functions with different signatures",
                    ));
                }
                Ok(Value::Bool(left != right))
            }
            (
                BinaryOperator::Equal,
                Value::Enum {
                    enumeration: left_enum,
                    variant_index: left_variant,
                    payload: None,
                },
                Value::Enum {
                    enumeration: right_enum,
                    variant_index: right_variant,
                    payload: None,
                },
            ) if left_enum == right_enum => Ok(Value::Bool(left_variant == right_variant)),
            (
                BinaryOperator::NotEqual,
                Value::Enum {
                    enumeration: left_enum,
                    variant_index: left_variant,
                    payload: None,
                },
                Value::Enum {
                    enumeration: right_enum,
                    variant_index: right_variant,
                    payload: None,
                },
            ) if left_enum == right_enum => Ok(Value::Bool(left_variant != right_variant)),
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

    fn validate_binding_reference(
        &self,
        frame: &Frame,
        reference: &BindingReference,
        span: nova_source::Span,
    ) -> Result<(), Diagnostic> {
        let Some(slot) = frame.get(&reference.binding) else {
            return Err(self.invariant(
                span,
                format!(
                    "resolved binding {} is absent from the frame",
                    reference.binding.index()
                ),
            ));
        };
        if slot.binding_name != reference.binding_name
            || slot.declaration_span != reference.declaration_span
        {
            return Err(self.invariant(
                span,
                format!(
                    "resolved binding reference `{}` does not match declaration identity for binding {}",
                    reference.binding_name,
                    reference.binding.index()
                ),
            ));
        }
        Ok(())
    }

    fn bind_runtime_slot(
        &self,
        frame: &mut Frame,
        binding: &nova_sema::hir::Binding,
        value: Option<Value>,
        span: nova_source::Span,
    ) -> Result<(), Diagnostic> {
        if let Some(value) = value.as_ref() {
            if !self.value_conforms_to_type(value, &binding.ty) {
                return Err(self.invariant(
                    span,
                    format!(
                        "binding `{}` received a runtime value that does not conform to declared type {}",
                        binding.name, binding.ty
                    ),
                ));
            }
        }
        if let Some(slot) = frame.get_mut(&binding.id) {
            if slot.ty != binding.ty
                || slot.mutable != binding.mutable
                || slot.binding_name != binding.name
                || slot.declaration_span != binding.span
            {
                return Err(self.invariant(
                    span,
                    format!(
                        "binding id {} was reused with runtime slot metadata that does not match its resolved binding",
                        binding.id.index()
                    ),
                ));
            }
            slot.value = value;
            return Ok(());
        }
        frame.insert(
            binding.id,
            RuntimeSlot {
                ty: binding.ty.clone(),
                mutable: binding.mutable,
                binding_name: binding.name.clone(),
                declaration_span: binding.span,
                value,
            },
        );
        Ok(())
    }

    fn resolved_function_reference(
        &self,
        function: FunctionId,
        function_name: &str,
        span: nova_source::Span,
    ) -> Result<&Function, Diagnostic> {
        let Some(definition) = self.program.functions.get(function.index()) else {
            return Err(self.invariant(
                span,
                format!(
                    "resolved function id {} is outside the program",
                    function.index()
                ),
            ));
        };
        if definition.id != function {
            return Err(self.invariant(
                span,
                "function declaration index does not match its resolved identity",
            ));
        }
        if definition.name != function_name {
            return Err(self.invariant(
                span,
                format!(
                    "resolved function `{function_name}` does not match declaration id {} (`{}`)",
                    function.index(),
                    definition.name
                ),
            ));
        }
        Ok(definition)
    }

    fn resolved_enum_variant(
        &self,
        enumeration: EnumId,
        variant_index: usize,
        variant_name: &str,
        span: nova_source::Span,
    ) -> Result<(&nova_sema::hir::Enum, &nova_sema::hir::EnumVariant), Diagnostic> {
        let Some(definition) = self.program.enums.get(enumeration.index()) else {
            return Err(self.invariant(
                span,
                format!(
                    "resolved enum id {} is outside the program",
                    enumeration.index()
                ),
            ));
        };
        if definition.id != enumeration {
            return Err(self.invariant(
                span,
                "enum declaration index does not match its resolved identity",
            ));
        }
        let Some(variant) = definition.variants.get(variant_index) else {
            return Err(self.invariant(
                span,
                format!(
                    "variant slot {variant_index} is outside enum `{}`",
                    definition.name
                ),
            ));
        };
        if variant.name != variant_name {
            return Err(self.invariant(
                span,
                format!(
                    "resolved enum variant `{variant_name}` does not match declaration slot {variant_index} (`{}`)",
                    variant.name
                ),
            ));
        }
        Ok((definition, variant))
    }

    fn resolved_record_field(
        &self,
        record: RecordId,
        field_index: usize,
        field_name: &str,
        span: nova_source::Span,
    ) -> Result<&nova_sema::hir::RecordField, Diagnostic> {
        let Some(definition) = self.program.records.get(record.index()) else {
            return Err(self.invariant(
                span,
                format!(
                    "resolved record id {} is outside the program",
                    record.index()
                ),
            ));
        };
        if definition.id != record {
            return Err(self.invariant(
                span,
                "record declaration index does not match its resolved identity",
            ));
        }
        let Some(field) = definition.fields.get(field_index) else {
            return Err(self.invariant(
                span,
                format!(
                    "resolved field `{field_name}` targets slot {field_index} outside record `{}`",
                    definition.name
                ),
            ));
        };
        if field.name != field_name {
            return Err(self.invariant(
                span,
                format!(
                    "resolved field `{field_name}` targets slot {field_index}, declared as `{}` in record `{}`",
                    field.name, definition.name
                ),
            ));
        }
        Ok(field)
    }

    fn equality_types_match(&self, left: &Type, right: &Type) -> bool {
        matching_equality_types(left, right, |enum_id| {
            self.program
                .enums
                .get(enum_id.index())
                .is_some_and(|definition| {
                    definition.id == enum_id
                        && definition
                            .variants
                            .iter()
                            .all(|variant| variant.payload.is_none())
                })
        })
    }

    fn function_signatures_match(&self, left: FunctionId, right: FunctionId) -> bool {
        let Some(left_function) = self.program.functions.get(left.index()) else {
            return false;
        };
        let Some(right_function) = self.program.functions.get(right.index()) else {
            return false;
        };
        left_function.id == left
            && right_function.id == right
            && left_function.parameters.len() == right_function.parameters.len()
            && left_function
                .parameters
                .iter()
                .zip(&right_function.parameters)
                .all(|(left_parameter, right_parameter)| left_parameter.ty == right_parameter.ty)
            && left_function.return_type == right_function.return_type
    }

    fn type_is_runtime_valid(&self, ty: &Type) -> bool {
        match ty {
            Type::Int | Type::Bool | Type::String | Type::Unit => true,
            Type::Record(record) => {
                self.program
                    .records
                    .get(record.id.index())
                    .is_some_and(|definition| {
                        definition.id == record.id && definition.name == record.name
                    })
            }
            Type::Enum(enumeration) => {
                self.program
                    .enums
                    .get(enumeration.id.index())
                    .is_some_and(|definition| {
                        definition.id == enumeration.id && definition.name == enumeration.name
                    })
            }
            Type::Function(signature) => {
                signature
                    .parameters
                    .iter()
                    .all(|parameter| self.type_is_runtime_valid(parameter))
                    && self.type_is_runtime_valid(&signature.return_type)
            }
            Type::Never | Type::Error => false,
        }
    }

    fn value_conforms_to_type(&self, value: &Value, ty: &Type) -> bool {
        if !self.type_is_runtime_valid(ty) {
            return false;
        }
        match (value, ty) {
            (Value::Int(_), Type::Int)
            | (Value::Bool(_), Type::Bool)
            | (Value::String(_), Type::String)
            | (Value::Unit, Type::Unit) => true,
            (Value::Record { record, fields }, Type::Record(expected))
                if *record == expected.id =>
            {
                let Some(definition) = self.program.records.get(record.index()) else {
                    return false;
                };
                definition.id == *record
                    && fields.len() == definition.fields.len()
                    && fields
                        .iter()
                        .zip(&definition.fields)
                        .all(|(value, field)| self.value_conforms_to_type(value, &field.ty))
            }
            (
                Value::Enum {
                    enumeration,
                    variant_index,
                    payload,
                },
                Type::Enum(expected),
            ) if *enumeration == expected.id => {
                let Some(definition) = self.program.enums.get(enumeration.index()) else {
                    return false;
                };
                if definition.id != *enumeration {
                    return false;
                }
                let Some(variant) = definition.variants.get(*variant_index) else {
                    return false;
                };
                match (&variant.payload, payload.as_deref()) {
                    (None, None) => true,
                    (Some(payload_type), Some(payload_value)) => {
                        self.value_conforms_to_type(payload_value, payload_type)
                    }
                    _ => false,
                }
            }
            (Value::Function(id), Type::Function(expected)) => {
                let Some(function) = self.program.functions.get(id.index()) else {
                    return false;
                };
                function.id == *id
                    && function.parameters.len() == expected.parameters.len()
                    && function
                        .parameters
                        .iter()
                        .zip(&expected.parameters)
                        .all(|(parameter, expected_type)| &parameter.ty == expected_type)
                    && &function.return_type == expected.return_type.as_ref()
            }
            _ => false,
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

    fn int_result(
        &self,
        result: Result<i64, IntArithmeticError>,
        expression: &Expression,
    ) -> Result<Value, Diagnostic> {
        match result {
            Ok(value) => Ok(Value::Int(value)),
            Err(IntArithmeticError::Overflow) => Err(self.overflow(expression)),
            Err(IntArithmeticError::ZeroDivisor) => Err(self.zero_divisor(expression)),
        }
    }

    fn zero_divisor(&self, expression: &Expression) -> Diagnostic {
        Diagnostic::error("N4003", "division by zero")
            .with_primary(expression.span, "zero divisor is not executable")
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
    use nova_sema::{
        analyze,
        hir::{ExpressionKind, Type},
    };
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
    fn executes_utf8_strings_through_aggregates_and_equality() {
        let value = execute_text(
            r#"
record Message { text: String }
enum MaybeMessage { None, Some(Message) }
fn main() -> String {
    let selected = match MaybeMessage::Some(new Message { text: "Nova 🦀\n" }) {
        MaybeMessage::None => "empty",
        MaybeMessage::Some(message) => message.text,
    };
    if selected != "empty" { selected } else { "unreachable" }
}
"#,
        )
        .expect("string program executes");
        assert_eq!(value, Value::String("Nova 🦀\n".to_owned()));

        let equal = execute_text(r#"fn main() -> Bool { "same" == "same" }"#)
            .expect("string equality executes");
        assert_eq!(equal, Value::Bool(true));

        let returned = execute_text(
            r#"fn main() -> String { "evaluated first" == { return "returned"; } }"#,
        )
        .expect("non-continuing equality operand propagates return");
        assert_eq!(returned, Value::String("returned".to_owned()));
    }

    #[test]
    fn malformed_string_literal_type_fails_the_runtime_boundary() {
        let source = SourceFile::new(
            SourceId::new(0),
            "test.nv",
            r#"fn main() -> String { "value" }"#,
        );
        let lexed = lex(&source);
        let parsed = parse(&source, &lexed.tokens);
        let mut analyzed = analyze(&parsed.program);
        assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
        analyzed.program.functions[0]
            .body
            .tail
            .as_deref_mut()
            .expect("string tail")
            .ty = Type::Bool;

        let diagnostic = execute(&analyzed.program).expect_err("malformed HIR must fail closed");
        assert_eq!(diagnostic.code, "N4005");
        assert!(
            diagnostic.labels[0]
                .message
                .contains("does not conform to HIR type Bool")
        );
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
    fn executes_break_continue_and_nested_loop_targets() {
        let value = execute_text(
            "fn main() -> Int {\n\
                 var index = 0;\n\
                 var sum = 0;\n\
                 while index < 8 {\n\
                     index = index + 1;\n\
                     if index == 3 { continue; } else { 0 };\n\
                     if index == 6 { break; } else { 0 };\n\
                     sum = sum + index;\n\
                 }\n\
                 var outer = 0;\n\
                 while outer < 3 {\n\
                     outer = outer + 1;\n\
                     while true { sum = sum + 10; break; }\n\
                 }\n\
                 sum\n\
             }",
        )
        .expect("loop control should execute");
        assert_eq!(value, Value::Int(42));
    }

    #[test]
    fn continue_propagates_through_selected_match_arm() {
        let value = execute_text(
            "enum Step { Skip, Add(Int) }\n\
             fn main() -> Int {\n\
                 var index = 0;\n\
                 var sum = 0;\n\
                 while index < 4 {\n\
                     index = index + 1;\n\
                     let step = if index == 2 { Step::Skip } else { Step::Add(index) };\n\
                     match step {\n\
                         Step::Skip => { continue; },\n\
                         Step::Add(value) => { sum = sum + value; 0 },\n\
                     };\n\
                 }\n\
                 sum\n\
             }",
        )
        .expect("continue should propagate through the selected match arm");
        assert_eq!(value, Value::Int(8));
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
        for text in [
            "fn one() -> Int { 1 } fn main() -> Int { 9223372036854775807 + one() }",
            "fn minimum() -> Int { -9223372036854775808 } fn main() -> Int { -minimum() }",
            "fn minimum() -> Int { -9223372036854775808 } fn minus_one() -> Int { -1 } fn main() -> Int { minimum() / minus_one() }",
            "fn minimum() -> Int { -9223372036854775808 } fn minus_one() -> Int { -1 } fn main() -> Int { minimum() % minus_one() }",
        ] {
            let error = execute_text(text).expect_err("dynamic overflow must fail at runtime");
            assert_eq!(error.code, "N4002");
        }
    }

    #[test]
    fn rejects_zero_divisor() {
        for text in [
            "fn zero() -> Int { 0 } fn main() -> Int { 10 / zero() }",
            "fn zero() -> Int { 0 } fn main() -> Int { 10 % zero() }",
        ] {
            let error = execute_text(text).expect_err("dynamic zero divisor must fail at runtime");
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
