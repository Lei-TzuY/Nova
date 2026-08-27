//! Resolved, typed high-level intermediate representation for Nova's semantic core.

use nova_parser::ast::{BinaryOperator, UnaryOperator};
use nova_source::Span;
use std::fmt;

/// Stable source-order identifier for one top-level record in a HIR program.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RecordId(usize);

impl RecordId {
    /// Creates an identifier from its source-order index among records.
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    /// Returns the source-order index among records.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

/// Nominal record identity carried by semantic types.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordType {
    /// Stable record identity.
    pub id: RecordId,
    /// Declared spelling used in diagnostics and debug output.
    pub name: String,
}

/// Semantic type assigned to a resolved Nova expression or binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Type {
    /// Bootstrap `Int` type.
    Int,
    /// Boolean type.
    Bool,
    /// Nominal user-defined record type.
    Record(RecordType),
    /// Internal type of a value-less block. There is no surface `Unit` type yet.
    Unit,
    /// Internal bottom type for expressions or blocks that cannot complete normally.
    Never,
    /// Callable function signature.
    Function(FunctionType),
    /// Error-recovery sentinel. It is never a successfully checked source type.
    Error,
}

impl Type {
    /// Reports whether this is the error-recovery sentinel.
    #[must_use]
    pub const fn is_error(&self) -> bool {
        matches!(self, Self::Error)
    }

    /// Reports whether control cannot continue after producing this type.
    #[must_use]
    pub const fn is_never(&self) -> bool {
        matches!(self, Self::Never)
    }
}

impl fmt::Display for Type {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int => formatter.write_str("Int"),
            Self::Bool => formatter.write_str("Bool"),
            Self::Record(record) => formatter.write_str(&record.name),
            Self::Unit => formatter.write_str("()"),
            Self::Never => formatter.write_str("!"),
            Self::Error => formatter.write_str("<error>"),
            Self::Function(signature) => {
                formatter.write_str("fn(")?;
                for (index, parameter) in signature.parameters.iter().enumerate() {
                    if index != 0 {
                        formatter.write_str(", ")?;
                    }
                    write!(formatter, "{parameter}")?;
                }
                write!(formatter, ") -> {}", signature.return_type)
            }
        }
    }
}

/// Fully resolved function type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionType {
    /// Parameter types in declaration order.
    pub parameters: Vec<Type>,
    /// Declared return type.
    pub return_type: Box<Type>,
}

/// Stable source-order identifier for one top-level function in a HIR program.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FunctionId(usize);

impl FunctionId {
    /// Creates an identifier from its source-order index.
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    /// Returns the source-order index.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

/// Stable analysis-order identifier for one local binding or parameter.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BindingId(usize);

impl BindingId {
    /// Creates an identifier from its analysis-order index.
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    /// Returns the analysis-order index.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

/// A complete semantically resolved source file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Program {
    /// Records in source order among record declarations.
    pub records: Vec<Record>,
    /// Functions in source order, including declarations diagnosed as duplicates.
    pub functions: Vec<Function>,
    /// Range covering the source file.
    pub span: Span,
}

/// A resolved nominal record declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Record {
    /// Stable source-order identity among records.
    pub id: RecordId,
    /// Declared spelling.
    pub name: String,
    /// Fields in declaration order.
    pub fields: Vec<RecordField>,
    /// Complete declaration range.
    pub span: Span,
}

/// One resolved record field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordField {
    /// Declared field spelling.
    pub name: String,
    /// Resolved field type.
    pub ty: Type,
    /// Complete field declaration range.
    pub span: Span,
}

/// A resolved top-level function.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Function {
    /// Stable source-order identity.
    pub id: FunctionId,
    /// Declared spelling.
    pub name: String,
    /// Parameters, each represented as a local binding.
    pub parameters: Vec<Binding>,
    /// Resolved declared return type.
    pub return_type: Type,
    /// Typed function body.
    pub body: Block,
    /// Complete declaration range.
    pub span: Span,
}

/// A resolved local binding or parameter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Binding {
    /// Stable analysis-order identity.
    pub id: BindingId,
    /// Declared spelling.
    pub name: String,
    /// Inferred or explicitly checked type.
    pub ty: Type,
    /// Whether this binding was introduced with `var`.
    pub mutable: bool,
    /// Range occupied by the binding name.
    pub span: Span,
}

/// A typed lexical block.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Block {
    /// Statements before the optional tail expression.
    pub statements: Vec<Statement>,
    /// Optional final value expression.
    pub tail: Option<Box<Expression>>,
    /// Result type, including internal `()` and `!` types.
    pub ty: Type,
    /// Complete range including braces.
    pub span: Span,
}

/// A typed statement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Statement {
    /// Statement form.
    pub kind: StatementKind,
    /// Complete statement range.
    pub span: Span,
}

/// Resolved statement forms.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StatementKind {
    /// Initialized local binding.
    Binding {
        /// Resolved binding metadata.
        binding: Binding,
        /// Typed initializer.
        initializer: Expression,
    },
    /// Mutable binding declared before its first assignment.
    UninitializedBinding(Binding),
    /// Assignment to a named binding.
    Assignment {
        /// Resolved assignment target, or `None` for an already-diagnosed target.
        target: Option<BindingId>,
        /// Typed replacement value.
        value: Expression,
    },
    /// Pre-test loop with a checked Boolean condition.
    While {
        /// Typed condition evaluated before every iteration.
        condition: Expression,
        /// Typed loop body.
        body: Block,
    },
    /// Explicit function return.
    Return(Expression),
    /// Expression whose value is discarded.
    Expression(Expression),
}

/// One resolved record initializer, preserving source evaluation order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordFieldValue {
    /// Zero-based destination slot in the record's declaration order.
    pub field_index: usize,
    /// Typed initializer expression, evaluated in source order.
    pub value: Expression,
}

/// A typed, resolved expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Expression {
    /// Resolved expression form.
    pub kind: ExpressionKind,
    /// Semantic result type.
    pub ty: Type,
    /// Exact source range.
    pub span: Span,
}

/// Resolved expression forms.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExpressionKind {
    /// Integer literal.
    Integer(i64),
    /// Boolean literal.
    Boolean(bool),
    /// Reference to a local binding or parameter.
    Binding(BindingId),
    /// Reference to a top-level function.
    Function(FunctionId),
    /// Nominal record construction with resolved destination slots.
    RecordLiteral {
        /// Resolved nominal record identity.
        record: RecordId,
        /// Typed field initializers in source evaluation order.
        fields: Vec<RecordFieldValue>,
    },
    /// Resolved record field projection.
    FieldAccess {
        /// Typed base expression.
        base: Box<Expression>,
        /// Nominal record identity expected at runtime.
        record: RecordId,
        /// Zero-based field slot in declaration order.
        field_index: usize,
    },
    /// Prefix operation.
    Unary {
        /// Parsed operator.
        operator: UnaryOperator,
        /// Typed operand.
        operand: Box<Expression>,
    },
    /// Infix operation.
    Binary {
        /// Parsed operator.
        operator: BinaryOperator,
        /// Typed left operand.
        left: Box<Expression>,
        /// Typed right operand.
        right: Box<Expression>,
    },
    /// Function invocation.
    Call {
        /// Typed callee expression.
        callee: Box<Expression>,
        /// Typed arguments in source order.
        arguments: Vec<Expression>,
    },
    /// Nested lexical block.
    Block(Block),
    /// Two-branch conditional.
    If {
        /// Boolean condition.
        condition: Box<Expression>,
        /// Branch selected by `true`.
        then_branch: Block,
        /// Branch selected by `false`.
        else_branch: Box<Expression>,
    },
    /// Placeholder for an expression already rejected by semantic analysis.
    Error,
}
