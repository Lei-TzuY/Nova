//! Resolved, typed high-level intermediate representation for Nova's semantic core.

use nova_parser::ast::{BinaryOperator, UnaryOperator};
use nova_source::Span;
use std::fmt;

/// Stable compiler-session identity for one Nova module.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ModuleId(u32);

impl ModuleId {
    /// Identity assigned to the bootstrap compiler's implicit root module.
    pub const ROOT: Self = Self(0);

    /// Creates an identity from its compiler-session integer.
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the compiler-session integer representation.
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

macro_rules! module_scoped_id {
    ($(#[$attribute:meta])* $name:ident) => {
        $(#[$attribute])*
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name {
            module: ModuleId,
            index: usize,
        }

        impl $name {
            /// Creates an identity in the implicit root module.
            #[must_use]
            pub const fn new(index: usize) -> Self {
                Self::in_module(ModuleId::ROOT, index)
            }

            /// Creates an identity qualified by its owning module.
            #[must_use]
            pub const fn in_module(module: ModuleId, index: usize) -> Self {
                Self { module, index }
            }

            /// Returns the owning module identity.
            #[must_use]
            pub const fn module(self) -> ModuleId {
                self.module
            }

            /// Returns the declaration-order or traversal-order index within the module.
            #[must_use]
            pub const fn index(self) -> usize {
                self.index
            }
        }
    };
}

module_scoped_id!(
    /// Stable source-order identifier for one top-level record in a HIR module.
    RecordId
);

/// Nominal record identity carried by semantic types.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordType {
    /// Stable record identity.
    pub id: RecordId,
    /// Declared spelling used in diagnostics and debug output.
    pub name: String,
}

module_scoped_id!(
    /// Stable source-order identifier for one top-level enum in a HIR module.
    EnumId
);

/// Nominal enum identity carried by semantic types.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumType {
    /// Stable enum identity.
    pub id: EnumId,
    /// Declared spelling used in diagnostics and debug output.
    pub name: String,
}

/// Semantic type assigned to a resolved Nova expression or binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Type {
    /// Bootstrap `Int` type.
    Int,
    /// Unsigned 64-bit integer type.
    UInt,
    /// Boolean type.
    Bool,
    /// Immutable UTF-8 string value.
    String,
    /// Nominal user-defined record type.
    Record(RecordType),
    /// Nominal user-defined enum type.
    Enum(EnumType),
    /// Unit type, produced by `()` and value-less blocks.
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
            Self::UInt => formatter.write_str("UInt"),
            Self::Bool => formatter.write_str("Bool"),
            Self::String => formatter.write_str("String"),
            Self::Record(record) => formatter.write_str(&record.name),
            Self::Enum(enumeration) => formatter.write_str(&enumeration.name),
            Self::Unit => formatter.write_str("Unit"),
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

module_scoped_id!(
    /// Stable source-order identifier for one top-level function in a HIR module.
    FunctionId
);

module_scoped_id!(
    /// Stable semantic-traversal identifier for one anonymous closure expression in a module.
    ClosureId
);

module_scoped_id!(
    /// Stable analysis-order identifier for one local binding or parameter in a module.
    BindingId
);

/// One analyzed module and its complete source range.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Module {
    /// Stable compiler-session module identity.
    pub id: ModuleId,
    /// Range covering the module's source file.
    pub span: Span,
}

/// A complete semantically resolved source file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Program {
    /// Module that owns every declaration and local identity in this program.
    pub module: Module,
    /// Records in source order among record declarations.
    pub records: Vec<Record>,
    /// Enums in source order among enum declarations.
    pub enums: Vec<Enum>,
    /// Functions in source order, including declarations diagnosed as duplicates.
    pub functions: Vec<Function>,
    /// Range covering the source file.
    pub span: Span,
}

/// A resolved nominal enum declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Enum {
    /// Stable source-order identity among enums.
    pub id: EnumId,
    /// Declared spelling.
    pub name: String,
    /// Variants in declaration order.
    pub variants: Vec<EnumVariant>,
    /// Complete declaration range.
    pub span: Span,
}

/// One resolved enum variant.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumVariant {
    /// Declared variant spelling.
    pub name: String,
    /// Optional single payload type.
    pub payload: Option<Type>,
    /// Complete variant declaration range.
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

/// How one lexical binding enters a closure environment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptureMode {
    /// Copy the current value when the closure expression is evaluated.
    ByValue,
    /// Share one mutable runtime cell with the enclosing binding and closure aliases.
    ByReference,
}

/// One lexical binding captured when a closure is created.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Capture {
    /// Resolved declaration identity and metadata.
    pub reference: BindingReference,
    /// Resolved value type exposed through the closure environment.
    pub ty: Type,
    /// Environment transfer mode selected by semantic analysis.
    pub mode: CaptureMode,
    /// First lexical use that caused this capture, used for deterministic ordering and diagnostics.
    pub first_use: Span,
}

/// A typed anonymous callable embedded in its creating expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Closure {
    /// Stable semantic-traversal identity.
    pub id: ClosureId,
    /// Explicitly typed parameters in source order.
    pub parameters: Vec<Binding>,
    /// Explicit resolved return type.
    pub return_type: Type,
    /// Environment inputs in first-lexical-use order.
    pub captures: Vec<Capture>,
    /// Body evaluated at the closure call boundary.
    pub body: Block,
    /// Complete anonymous-function expression range.
    pub span: Span,
}

impl Closure {
    /// Returns the structural callable type exposed by this closure.
    #[must_use]
    pub fn function_type(&self) -> FunctionType {
        FunctionType {
            parameters: self
                .parameters
                .iter()
                .map(|parameter| parameter.ty.clone())
                .collect(),
            return_type: Box::new(self.return_type.clone()),
        }
    }
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

/// A resolved local/parameter reference paired with declaration metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingReference {
    /// Stable analysis-order binding identity.
    pub binding: BindingId,
    /// Source-resolved declaration spelling.
    pub binding_name: String,
    /// Span of the declaration name that resolution selected.
    pub declaration_span: Span,
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
        target: Option<BindingReference>,
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
    /// Exit the nearest lexically enclosing loop.
    Break,
    /// Start the next iteration of the nearest lexically enclosing loop.
    Continue,
    /// Explicit function return; `None` preserves a source-level bare `return;`.
    Return(Option<Expression>),
    /// Expression whose value is discarded.
    Expression(Expression),
}

/// One resolved record initializer, preserving source evaluation order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordFieldValue {
    /// Resolved field spelling paired with the destination slot.
    pub field_name: String,
    /// Zero-based destination slot in the record's declaration order.
    pub field_index: usize,
    /// Typed initializer expression, evaluated in source order.
    pub value: Expression,
}

/// One resolved arm in an exhaustive enum match.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchArm {
    /// Resolved variant spelling paired with the declaration-order slot.
    pub variant_name: String,
    /// Zero-based variant slot in declaration order.
    pub variant_index: usize,
    /// Optional immutable payload binding.
    pub binding: Option<Binding>,
    /// Whether a payload-bearing variant explicitly discards its payload with `_`.
    pub payload_discarded: bool,
    /// Typed arm value.
    pub value: Expression,
    /// Complete arm range.
    pub span: Span,
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
    /// Unsigned integer value introduced by the UInt numeric surface.
    Unsigned(u64),
    /// Decoded UTF-8 string literal.
    String(String),
    /// Boolean literal.
    Boolean(bool),
    /// Unit literal.
    Unit,
    /// Anonymous function plus its resolved lexical capture contract.
    Closure(Box<Closure>),
    /// Reference to a local binding or parameter.
    Binding(BindingReference),
    /// Reference to a top-level function.
    Function {
        /// Stable source-order function identity.
        function: FunctionId,
        /// Source-resolved function spelling paired with the stable identity.
        function_name: String,
    },
    /// Nominal record construction with resolved destination slots.
    RecordLiteral {
        /// Resolved nominal record identity.
        record: RecordId,
        /// Typed field initializers in source evaluation order.
        fields: Vec<RecordFieldValue>,
    },
    /// Nominal enum variant construction.
    EnumConstructor {
        /// Resolved nominal enum identity.
        enumeration: EnumId,
        /// Resolved variant spelling paired with the declaration-order slot.
        variant_name: String,
        /// Zero-based variant slot in declaration order.
        variant_index: usize,
        /// Optional typed payload expression.
        payload: Option<Box<Expression>>,
    },
    /// Resolved record field projection.
    FieldAccess {
        /// Typed base expression.
        base: Box<Expression>,
        /// Nominal record identity expected at runtime.
        record: RecordId,
        /// Resolved field spelling paired with the declaration-order slot.
        field_name: String,
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
    /// Checked explicit conversion from signed Int to UInt.
    IntToUInt {
        /// Operand evaluated exactly once.
        operand: Box<Expression>,
    },
    /// Checked explicit conversion from UInt to signed Int.
    UIntToInt {
        /// Operand evaluated exactly once.
        operand: Box<Expression>,
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
    /// Exhaustive nominal enum match.
    Match {
        /// Typed scrutinee evaluated exactly once.
        scrutinee: Box<Expression>,
        /// Resolved nominal enum identity.
        enumeration: EnumId,
        /// Arms in written source order.
        arms: Vec<MatchArm>,
    },
    /// Placeholder for an expression already rejected by semantic analysis.
    Error,
}
