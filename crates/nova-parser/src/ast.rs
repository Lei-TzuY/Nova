//! Parsed syntax tree for the implemented Nova subset.

use nova_source::Span;

/// A complete source file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Program {
    /// Top-level record declarations in source order among records.
    pub records: Vec<Record>,
    /// Top-level enum declarations in source order among enums.
    pub enums: Vec<Enum>,
    /// Top-level functions in source order among functions.
    pub functions: Vec<Function>,
    /// Range covering the complete source file.
    pub span: Span,
}

/// A top-level nominal enum declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Enum {
    /// Declared enum name.
    pub name: Name,
    /// Variants in declaration order.
    pub variants: Vec<EnumVariant>,
    /// Range from `enum` through the closing brace.
    pub span: Span,
}

/// One enum variant with zero or one payload type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumVariant {
    /// Variant name.
    pub name: Name,
    /// Optional single payload type.
    pub payload: Option<TypeRef>,
    /// Complete variant range.
    pub span: Span,
}

/// A top-level nominal record declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Record {
    /// Declared record name.
    pub name: Name,
    /// Fields in declaration order.
    pub fields: Vec<RecordField>,
    /// Range from `record` through the closing brace.
    pub span: Span,
}

/// One record field declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordField {
    /// Field name.
    pub name: Name,
    /// Explicit field type.
    pub ty: TypeRef,
    /// Range covering the field name, colon, and type.
    pub span: Span,
}

/// A top-level function declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Function {
    /// Declared function name.
    pub name: Name,
    /// Explicitly typed parameters.
    pub parameters: Vec<Parameter>,
    /// Explicit return type.
    pub return_type: TypeRef,
    /// Function body.
    pub body: Block,
    /// Range from `fn` through the closing body brace.
    pub span: Span,
}

/// One function parameter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Parameter {
    /// Parameter binding name.
    pub name: Name,
    /// Explicit parameter type.
    pub ty: TypeRef,
    /// Range covering the name, colon, and type.
    pub span: Span,
}

/// An identifier with its spelling and exact range.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Name {
    /// Identifier spelling.
    pub text: String,
    /// Range occupied by the spelling.
    pub span: Span,
}

/// A parsed type name. Type meaning is assigned only in later phases.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeRef {
    /// Unresolved type name.
    pub name: Name,
    /// Range occupied by the type reference.
    pub span: Span,
}

/// A value-producing lexical block.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Block {
    /// Statements before the optional tail expression.
    pub statements: Vec<Statement>,
    /// Optional final expression without a semicolon.
    pub tail: Option<Box<Expression>>,
    /// Range including both braces.
    pub span: Span,
}

/// A statement in a lexical block.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Statement {
    /// Statement form.
    pub kind: StatementKind,
    /// Complete range occupied by the statement.
    pub span: Span,
}

/// Implemented statement forms.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StatementKind {
    /// Immutable or mutable initialized binding.
    Binding {
        /// `true` for `var`, `false` for `let`.
        mutable: bool,
        /// Bound name.
        name: Name,
        /// Optional explicit type annotation.
        annotation: Option<TypeRef>,
        /// Required initializer.
        initializer: Expression,
    },
    /// Mutable binding declared with a type but initialized later.
    UninitializedBinding {
        /// Bound name.
        name: Name,
        /// Required explicit type annotation.
        annotation: TypeRef,
    },
    /// Assignment to a named binding.
    Assignment {
        /// Name selected as the assignment target.
        target: Name,
        /// New value expression.
        value: Expression,
    },
    /// Pre-test loop whose body may execute zero or more times.
    While {
        /// Boolean loop condition.
        condition: Expression,
        /// Loop body.
        body: Block,
    },
    /// Explicit function return.
    Return(Expression),
    /// Expression whose value is discarded by a semicolon.
    Expression(Expression),
}

/// One named initializer in a record literal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordLiteralField {
    /// Field name written by the caller.
    pub name: Name,
    /// Initializer expression.
    pub value: Expression,
    /// Complete `name: expression` range.
    pub span: Span,
}

/// A nominal enum-variant pattern.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumPattern {
    /// Enum type qualifier.
    pub enumeration: Name,
    /// Selected variant.
    pub variant: Name,
    /// Optional immutable binding for the variant payload.
    pub binding: Option<Name>,
    /// Complete pattern range.
    pub span: Span,
}

/// One arm in a `match` expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchArm {
    /// Variant pattern selecting this arm.
    pub pattern: EnumPattern,
    /// Value produced by the arm.
    pub value: Expression,
    /// Complete arm range, excluding a trailing comma.
    pub span: Span,
}

/// A parsed expression and its complete source range.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Expression {
    /// Expression form.
    pub kind: ExpressionKind,
    /// Range occupied by the complete expression.
    pub span: Span,
}

/// Implemented expression forms.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExpressionKind {
    /// Checked decimal integer value.
    Integer(i64),
    /// Boolean value.
    Boolean(bool),
    /// Unresolved name reference.
    Name(Name),
    /// Nominal record construction with named fields.
    RecordLiteral {
        /// Record type name.
        name: Name,
        /// Written fields in source order.
        fields: Vec<RecordLiteralField>,
    },
    /// Nominal enum variant construction.
    EnumConstructor {
        /// Enum type qualifier.
        enumeration: Name,
        /// Selected variant.
        variant: Name,
        /// Optional single payload expression.
        payload: Option<Box<Expression>>,
    },
    /// Field projection from a record value.
    FieldAccess {
        /// Base expression.
        base: Box<Expression>,
        /// Selected field name.
        field: Name,
    },
    /// Prefix operation.
    Unary {
        /// Prefix operator.
        operator: UnaryOperator,
        /// Operand.
        operand: Box<Expression>,
    },
    /// Binary operation.
    Binary {
        /// Infix operator.
        operator: BinaryOperator,
        /// Left operand.
        left: Box<Expression>,
        /// Right operand.
        right: Box<Expression>,
    },
    /// Function or callable expression invocation.
    Call {
        /// Expression producing the callee.
        callee: Box<Expression>,
        /// Arguments in source order.
        arguments: Vec<Expression>,
    },
    /// Nested block expression.
    Block(Block),
    /// Required-two-branch conditional expression.
    If {
        /// Condition expression.
        condition: Box<Expression>,
        /// Branch selected by `true`.
        then_branch: Block,
        /// Block or nested `if` selected by `false`.
        else_branch: Box<Expression>,
    },
    /// Exhaustively checked nominal enum matching.
    Match {
        /// Value inspected exactly once before arm selection.
        scrutinee: Box<Expression>,
        /// Arms in written source order.
        arms: Vec<MatchArm>,
    },
}

/// Prefix operators.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnaryOperator {
    /// Numeric negation (`-`).
    Negate,
    /// Boolean negation (`!`).
    Not,
}

/// Infix operators in increasing precedence groups.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryOperator {
    /// Boolean disjunction.
    Or,
    /// Boolean conjunction.
    And,
    /// Equality.
    Equal,
    /// Inequality.
    NotEqual,
    /// Less-than comparison.
    Less,
    /// Less-than-or-equal comparison.
    LessEqual,
    /// Greater-than comparison.
    Greater,
    /// Greater-than-or-equal comparison.
    GreaterEqual,
    /// Addition.
    Add,
    /// Subtraction.
    Subtract,
    /// Multiplication.
    Multiply,
    /// Division.
    Divide,
    /// Remainder.
    Remainder,
}
