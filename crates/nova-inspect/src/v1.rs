//! Data model for semantic-inspection schema version 1.

use serde::Serialize;

/// Stable schema family name carried by every document.
pub const SCHEMA_NAME: &str = "nova.semantic-inspection";

/// Numeric version of the schema in this module.
pub const SCHEMA_VERSION: u32 = 1;

/// One complete semantic-inspection document.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Document {
    /// Schema family, always [`SCHEMA_NAME`].
    pub schema: String,
    /// Schema version, always [`SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Compiler component that produced this document.
    pub producer: Producer,
    /// The single source accepted by the bootstrap pipeline.
    pub source: Source,
    /// Stable semantic facts for the accepted program.
    pub program: Program,
}

/// Identity of the document producer.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Producer {
    /// Tool name.
    pub name: String,
    /// Bootstrap toolchain version.
    pub version: String,
}

/// Source metadata. Source contents are intentionally not copied into the document.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Source {
    /// Document-local source identity.
    pub id: String,
    /// Display name supplied to the compiler, normally a path.
    pub name: String,
    /// Validated UTF-8 byte length.
    pub byte_length: usize,
}

/// A half-open UTF-8 byte span in one document-local source.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Span {
    /// Document-local source identity.
    pub source: String,
    /// Inclusive byte start.
    pub start: usize,
    /// Exclusive byte end.
    pub end: usize,
}

/// Semantic facts for one source file.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Program {
    /// Complete source range.
    pub span: Span,
    /// Interned semantic types in deterministic order.
    pub types: Vec<Type>,
    /// Nominal record declarations.
    pub records: Vec<Record>,
    /// Nominal enum declarations.
    pub enums: Vec<Enum>,
    /// Function declarations.
    pub functions: Vec<Function>,
    /// Parameters, locals, and match payload bindings.
    pub bindings: Vec<Binding>,
    /// Typed lexical blocks.
    pub blocks: Vec<Block>,
    /// Resolved statements.
    pub statements: Vec<Statement>,
    /// Typed expressions in deterministic semantic traversal order.
    pub expressions: Vec<Expression>,
    /// Exhaustive-match facts.
    pub matches: Vec<Match>,
}

/// One interned semantic type.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Type {
    /// Document-local type identity.
    pub id: String,
    /// Type category.
    pub kind: TypeKind,
    /// Deterministic human-readable spelling.
    pub display: String,
    /// Nominal declaration identity, when applicable.
    pub declaration: Option<String>,
    /// Function parameter type identities, otherwise empty.
    pub parameters: Vec<String>,
    /// Function return type identity, when applicable.
    pub return_type: Option<String>,
}

/// Categories represented in the v1 type table.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeKind {
    /// Surface `Int`.
    Int,
    /// Surface `UInt`; emitted only by schema v7 and later.
    #[serde(rename = "uint")]
    UInt,
    /// Surface `Bool`.
    Bool,
    /// Surface `String`; emitted only by schema v4 and later.
    String,
    /// Nominal record.
    Record,
    /// Nominal enum.
    Enum,
    /// Surface `Unit` type.
    Unit,
    /// Surface `!`, encoded as the non-continuing `never` type kind.
    Never,
    /// Callable signature.
    Function,
}

/// One nominal record declaration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Record {
    /// Document-local declaration identity.
    pub id: String,
    /// Declared name.
    pub name: String,
    /// Nominal type identity.
    pub type_id: String,
    /// Complete declaration range.
    pub span: Span,
    /// Fields in declaration order.
    pub fields: Vec<RecordField>,
}

/// One declared record field.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RecordField {
    /// Identity derived from the record and declaration-order slot.
    pub id: String,
    /// Declared field name.
    pub name: String,
    /// Resolved field type.
    pub type_id: String,
    /// Complete field declaration range.
    pub span: Span,
}

/// One nominal enum declaration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Enum {
    /// Document-local declaration identity.
    pub id: String,
    /// Declared name.
    pub name: String,
    /// Nominal type identity.
    pub type_id: String,
    /// Complete declaration range.
    pub span: Span,
    /// Variants in declaration order.
    pub variants: Vec<EnumVariant>,
}

/// One declared enum variant.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EnumVariant {
    /// Identity derived from the enum and declaration-order slot.
    pub id: String,
    /// Declared variant name.
    pub name: String,
    /// Optional resolved payload type.
    pub payload_type: Option<String>,
    /// Complete variant declaration range.
    pub span: Span,
}

/// One top-level function declaration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Function {
    /// Document-local function identity.
    pub id: String,
    /// Declared function name.
    pub name: String,
    /// Interned callable type.
    pub type_id: String,
    /// Declared return type.
    pub return_type: String,
    /// Parameter binding identities in declaration order.
    pub parameters: Vec<String>,
    /// Function-body block identity.
    pub body: String,
    /// Complete declaration range.
    pub span: Span,
}

/// One local value definition.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Binding {
    /// Document-local binding identity.
    pub id: String,
    /// Declared name.
    pub name: String,
    /// How the binding enters scope.
    pub role: BindingRole,
    /// Enclosing function identity.
    pub owner: String,
    /// Lexical scope identity.
    pub scope: String,
    /// Resolved binding type.
    pub type_id: String,
    /// Whether assignment is permitted.
    pub mutable: bool,
    /// Exact name range.
    pub span: Span,
}

/// Binding introduction categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingRole {
    /// Function parameter.
    Parameter,
    /// Initialized or delayed local declaration.
    Local,
    /// Immutable enum payload bound by one match arm.
    MatchPayload,
}

/// One typed lexical block.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Block {
    /// Document-local block identity.
    pub id: String,
    /// Enclosing function identity.
    pub owner: String,
    /// Semantic result type.
    pub type_id: String,
    /// Complete block range.
    pub span: Span,
    /// Statement identities in source order.
    pub statements: Vec<String>,
    /// Optional tail-expression identity.
    pub tail_expression: Option<String>,
}

/// One resolved statement.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Statement {
    /// Document-local statement identity.
    pub id: String,
    /// Enclosing function identity.
    pub owner: String,
    /// Lexical block containing the statement.
    pub block: String,
    /// Statement category.
    pub kind: StatementKind,
    /// Binding introduced by this statement, when present.
    pub binding: Option<String>,
    /// Resolved assignment target, when present.
    pub target: Option<String>,
    /// Direct expression inputs in deterministic semantic traversal order.
    pub expressions: Vec<String>,
    /// Direct nested blocks in deterministic semantic traversal order.
    pub blocks: Vec<String>,
    /// Complete statement range.
    pub span: Span,
}

/// Statement categories represented by v1.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StatementKind {
    /// Initialized `let` or `var` declaration.
    InitializedBinding,
    /// Typed `var` declaration initialized later.
    UninitializedBinding,
    /// Assignment to a mutable binding.
    Assignment,
    /// Pre-test loop.
    While,
    /// Exit the nearest enclosing loop.
    Break,
    /// Start the nearest enclosing loop's next iteration.
    Continue,
    /// Explicit return.
    Return,
    /// Value discarded by a semicolon.
    Expression,
}

/// One typed expression fact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Expression {
    /// Document-local expression identity.
    pub id: String,
    /// Enclosing function identity.
    pub owner: String,
    /// Expression category.
    pub kind: ExpressionKind,
    /// Semantic result type.
    pub type_id: String,
    /// Resolved semantic target, when the expression selects one.
    pub target: Option<String>,
    /// Stable surface operator spelling, when applicable.
    pub operator: Option<String>,
    /// Direct expression children in deterministic semantic traversal order.
    pub children: Vec<String>,
    /// Direct lexical block children in deterministic semantic traversal order.
    pub blocks: Vec<String>,
    /// Resolved record-field inputs in written evaluation order, otherwise empty.
    pub field_initializers: Vec<RecordFieldInitializer>,
    /// Exact expression range.
    pub span: Span,
}

/// One resolved field/value edge in a record construction expression.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RecordFieldInitializer {
    /// Selected declaration-order record field.
    pub field: String,
    /// Expression evaluated to initialize that field.
    pub value: String,
}

/// Expression categories represented by v1.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpressionKind {
    /// Integer literal.
    Integer,
    /// Unsigned integer literal; emitted only by schema v7 and later.
    UnsignedInteger,
    /// String literal; emitted only by schema v4 and later.
    String,
    /// Boolean literal.
    Boolean,
    /// Unit literal.
    Unit,
    /// Anonymous closure expression; emitted only by schema v5 and later.
    Closure,
    /// Resolved local or parameter reference.
    BindingReference,
    /// Resolved top-level function reference.
    FunctionReference,
    /// Nominal record construction.
    RecordConstruction,
    /// Nominal enum variant construction.
    EnumConstruction,
    /// Resolved record field projection.
    FieldAccess,
    /// Prefix operation.
    Unary,
    /// Infix operation.
    Binary,
    /// Explicit checked conversion between numeric families; emitted only by schema v7
    /// and later.
    NumericConversion,
    /// Function invocation.
    Call,
    /// Nested lexical block expression.
    Block,
    /// Two-branch conditional.
    If,
    /// Exhaustive nominal enum match.
    Match,
}

/// One accepted exhaustive enum match.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Match {
    /// Document-local match identity.
    pub id: String,
    /// Enclosing function identity.
    pub owner: String,
    /// Expression fact representing this match.
    pub expression: String,
    /// Matched nominal enum declaration.
    pub enumeration: String,
    /// Always true in v1 because rejected programs produce no document.
    pub exhaustive: bool,
    /// Scrutinee expression identity.
    pub scrutinee: String,
    /// Resolved scrutinee type, including internal `never` when applicable.
    pub scrutinee_type: String,
    /// Arms in written source order.
    pub arms: Vec<MatchArm>,
    /// Complete match-expression range.
    pub span: Span,
}

/// One resolved arm of an exhaustive match.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MatchArm {
    /// Identity derived from the match and written arm index.
    pub id: String,
    /// Resolved enum variant identity.
    pub variant: String,
    /// Optional payload binding identity.
    pub binding: Option<String>,
    /// Arm value expression identity.
    pub value: String,
    /// Resolved arm result type.
    pub result_type: String,
    /// Complete arm range.
    pub span: Span,
}
