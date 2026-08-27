//! Resolved HIR nodes for the implemented single-file Nova subset.

use nova_source::Span;

/// Dense, deterministic identity for one declaration in a lowered program.
///
/// IDs are session-local and are not stable across source edits. All unique
/// top-level functions are allocated first in source order, followed by their
/// parameters and local bindings in deterministic traversal order.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SymbolId(u32);

impl SymbolId {
    pub(crate) const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the session-local integer representation.
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// A complete resolved source file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Program {
    /// Successfully lowered top-level functions in source order.
    pub functions: Vec<Function>,
    /// Dense declaration table indexed by [`SymbolId`].
    pub symbols: Vec<Symbol>,
    /// Range covering the complete source file.
    pub span: Span,
}

impl Program {
    /// Looks up declaration metadata by its resolved identity.
    #[must_use]
    pub fn symbol(&self, id: SymbolId) -> Option<&Symbol> {
        let index = usize::try_from(id.raw()).ok()?;
        self.symbols.get(index).filter(|symbol| symbol.id == id)
    }
}

/// Metadata for one resolved declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Symbol {
    /// Dense identity referenced by HIR nodes.
    pub id: SymbolId,
    /// Original declaration spelling.
    pub name: String,
    /// Declaration category.
    pub kind: SymbolKind,
    /// Exact range of the declared name.
    pub span: Span,
}

/// Kinds of value declarations in the implemented subset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SymbolKind {
    /// Top-level function declaration.
    Function,
    /// Function parameter.
    Parameter,
    /// Block-local initialized binding.
    Local(Mutability),
}

/// Whether a binding was introduced with `let` or `var`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mutability {
    /// A `let` binding.
    Immutable,
    /// A `var` binding.
    Mutable,
}

/// A resolved top-level function.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Function {
    /// Identity of the function declaration.
    pub symbol: SymbolId,
    /// Explicitly typed parameters.
    pub parameters: Vec<Parameter>,
    /// Explicit return type.
    pub return_type: TypeRef,
    /// Resolved function body.
    pub body: Block,
    /// Range from `fn` through the closing body brace.
    pub span: Span,
}

/// One resolved function parameter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Parameter {
    /// Identity of the parameter declaration.
    pub symbol: SymbolId,
    /// Resolved explicit parameter type.
    pub ty: TypeRef,
    /// Range covering the name, colon, and type.
    pub span: Span,
}

/// A built-in type reference in source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeRef {
    /// Resolved built-in type.
    pub kind: BuiltinType,
    /// Exact range of the written type name.
    pub span: Span,
}

/// Built-in type names recognized by the first semantic slice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuiltinType {
    /// Signed integer type whose full semantics remain provisional.
    Int,
    /// Boolean type.
    Bool,
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

/// A resolved semicolon-terminated statement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Statement {
    /// Statement form.
    pub kind: StatementKind,
    /// Range including the terminating semicolon.
    pub span: Span,
}

/// Implemented resolved statement forms.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StatementKind {
    /// Immutable or mutable initialized binding.
    Binding {
        /// Identity introduced after the initializer.
        symbol: SymbolId,
        /// Whether the binding was written with `let` or `var`.
        mutability: Mutability,
        /// Optional resolved type annotation.
        annotation: Option<TypeRef>,
        /// Required initializer.
        initializer: Expression,
    },
    /// Explicit function return.
    Return(Expression),
    /// Expression whose value is discarded by a semicolon.
    Expression(Expression),
}

/// A resolved expression and its complete source range.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Expression {
    /// Expression form.
    pub kind: ExpressionKind,
    /// Range occupied by the complete expression.
    pub span: Span,
}

/// Implemented resolved expression forms.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExpressionKind {
    /// Checked decimal integer value.
    Integer(i64),
    /// Boolean value.
    Boolean(bool),
    /// Reference to a resolved value declaration.
    Name(SymbolId),
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

/// Prefix operators owned by HIR rather than parser syntax.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnaryOperator {
    /// Numeric negation (`-`).
    Negate,
    /// Boolean negation (`!`).
    Not,
}

/// Infix operators owned by HIR rather than parser syntax.
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
