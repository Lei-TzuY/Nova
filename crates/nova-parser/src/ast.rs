//! Parsed syntax tree for the implemented Nova subset.

use nova_source::Span;

/// A complete source file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Program {
    /// Top-level functions in source order.
    pub functions: Vec<Function>,
    /// Range covering the complete source file.
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
    /// Semicolon-terminated statements.
    pub statements: Vec<Statement>,
    /// Optional final expression without a semicolon.
    pub tail: Option<Box<Expression>>,
    /// Range including both braces.
    pub span: Span,
}

/// A semicolon-terminated statement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Statement {
    /// Statement form.
    pub kind: StatementKind,
    /// Range including the terminating semicolon.
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
    /// Assignment to a named binding.
    Assignment {
        /// Name selected as the assignment target.
        target: Name,
        /// New value expression.
        value: Expression,
    },
    /// Explicit function return.
    Return(Expression),
    /// Expression whose value is discarded by a semicolon.
    Expression(Expression),
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
