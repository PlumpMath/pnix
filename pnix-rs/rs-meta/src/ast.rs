//! AST for the Rust subset that rs-meta evaluates meta-circularly.
//!
//! rs-meta is the Rust meta-circular compiler/evaluator: written in Rust, it
//! evaluates Rust. The subset grows stage by stage toward the point where the
//! evaluator's own source falls inside it (true meta-circularity). This slice
//! covers functions, inherent `impl`, `let`, `if`/`else`, blocks, integer/bool
//! arithmetic, calls, recursion, `println!`, plus `struct` / `enum` / tuples /
//! `match`.

#[derive(Clone, Debug)]
pub struct Program {
    pub funcs: Vec<Func>,
    pub structs: Vec<StructDef>,
    pub enums: Vec<EnumDef>,
    pub impls: Vec<ImplBlock>,
    pub traits: Vec<TraitDef>,
    pub aliases: Vec<TypeAlias>,
    pub globals: Vec<Global>,
    /// Canonicalized top-level `use ...;` items, preserved for emission.
    pub uses: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct TypeAlias {
    pub name: String,
    pub ty: Type,
}

#[derive(Clone, Debug)]
pub struct Global {
    pub name: String,
    pub ty: Type,
    pub init: Expr,
}

#[derive(Clone, Debug)]
pub struct StructDef {
    pub name: String,
    /// Generic parameter names (`T`, `'a`), preserved for emission (E1c).
    pub generics: Vec<String>,
    pub fields: Vec<(String, Type)>,
    /// `#[derive(...)]` names, preserved for emission.
    pub derives: Vec<String>,
    /// A UNIT struct declared with `struct D;` (no braces). It has no fields and
    /// is constructed by the bare name `D` (not `D {}`), which is how rustc
    /// distinguishes it from an empty braced struct `struct D {}`.
    pub unit: bool,
    /// A TUPLE struct `struct P(i64, i64);` -- positional fields stored under
    /// names "0","1",... constructed by a call `P(a, b)` and accessed by `p.0`.
    pub tuple: bool,
}

#[derive(Clone, Debug)]
pub struct EnumDef {
    pub name: String,
    /// Generic parameter names, preserved for emission (E1c).
    pub generics: Vec<String>,
    pub variants: Vec<Variant>,
    /// `#[derive(...)]` names, preserved for emission.
    pub derives: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Variant {
    pub name: String,
    /// Empty = unit variant; otherwise a tuple variant `V(T, ...)`.
    pub fields: Vec<Type>,
    /// Struct-like variant fields `V { x: T, ... }`.
    pub named_fields: Vec<(String, Type)>,
}

#[derive(Clone, Debug)]
pub struct Func {
    pub name: String,
    /// Generic parameter names, preserved for emission (E1c).
    pub generics: Vec<String>,
    pub params: Vec<Param>,
    pub ret: Type,
    pub body: Block,
}

#[derive(Clone, Debug)]
pub struct TraitDecl {
    pub name: String,
    pub generics: Vec<String>,
    pub receiver: Option<ReceiverKind>,
    pub params: Vec<Param>,
    pub ret: Type,
}

#[derive(Clone, Debug)]
pub struct TraitDef {
    pub name: String,
    /// Default methods (those with a body) -- flattened onto implementing types.
    pub methods: Vec<Method>,
    /// Signature-only declarations `fn m(&self) -> T;` -- kept for emission so
    /// the implementing type's overrides remain valid trait members.
    pub decls: Vec<TraitDecl>,
}

#[derive(Clone, Debug)]
pub struct ImplBlock {
    /// Generic parameter names, preserved for emission (E1c).
    pub generics: Vec<String>,
    pub target: Type,
    /// For `impl Trait for Type`, the trait name (used to flatten the trait's
    /// default methods onto the type). `None` for inherent `impl Type`.
    pub trait_name: Option<String>,
    pub methods: Vec<Method>,
    /// Associated `const` items (`impl S { const N: i64 = 42; }`), resolved as
    /// `S::N`. Registered as globals under the qualified name.
    pub consts: Vec<Global>,
}

#[derive(Clone, Debug)]
pub struct Method {
    pub name: String,
    /// Generic parameter names, preserved for emission (E1c).
    pub generics: Vec<String>,
    pub receiver: Option<ReceiverKind>,
    pub params: Vec<Param>,
    pub ret: Type,
    pub body: Block,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ReceiverKind {
    Value,
    Ref,
    RefMut,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    I64,
    /// The type of an UNSUFFIXED integer literal (and literal-only arithmetic),
    /// typeck-internal: it coerces to any concrete integer, collapses to i64 at
    /// bindings/containers, and REJECTS method calls (rustc E0689 -- method
    /// resolution precedes literal fallback). The parser never produces it.
    IntLit,
    F64,
    I32,
    U32,
    U64,
    U8,
    Usize,
    Char,
    Bool,
    Unit,
    Never,
    Named(String),
    Generic { name: String, args: Vec<Type> },
    ImplTrait(String),
    Tuple(Vec<Type>),
    Slice(Box<Type>),
    /// Fixed-size array `[T; N]`. The size is kept as an expression so emission
    /// reproduces it faithfully; the interpreter models arrays as vecs and does
    /// not enforce the length.
    Array(Box<Type>, String),
    Ref { mutable: bool, inner: Box<Type> },
    /// Reference WITH an explicit lifetime (`&'a T`), produced only by the
    /// parser and preserved for emission (E1d). Typeck normalization
    /// (resolve_aliases) strips it to `Ref` before any comparison.
    RefLt { lifetime: String, mutable: bool, inner: Box<Type> },
    Closure { params: Vec<Type>, ret: Box<Type> },
}

#[derive(Clone, Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub tail: Option<Box<Expr>>,
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Let {
        name: String,
        mutable: bool,
        ty: Option<Type>,
        init: Expr,
    },
    LetPat {
        pat: Pattern,
        init: Expr,
    },
    LetElse {
        pat: Pattern,
        init: Expr,
        else_blk: Block,
    },
    Assign {
        target: Expr,
        value: Expr,
    },
    Expr(Expr),
    Return(Option<Expr>),
}

#[derive(Clone, Debug)]
pub enum Expr {
    Int(i64),
    /// Hex literal: i64-wrapped value (interpreter semantics) + source text
    /// (emission semantics — the u64-range bits survive rustc).
    IntHex(i64, String),
    /// Float literal kept as source text for byte-exact emit roundtrip.
    Float(String),
    Char(char),
    Str(String),
    Bool(bool),
    Var(String),
    Ref {
        mutable: bool,
        expr: Box<Expr>,
    },
    Unary {
        op: UnOp,
        rhs: Box<Expr>,
    },
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Cast {
        expr: Box<Expr>,
        ty: Type,
    },
    Try(Box<Expr>),
    Return(Option<Box<Expr>>),
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
    },
    Closure {
        params: Vec<ClosureParam>,
        ret: Option<Type>,
        body: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
    CallExpr {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    PathCall {
        type_name: String,
        item: String,
        args: Vec<Expr>,
    },
    MethodCall {
        receiver: Box<Expr>,
        name: String,
        type_args: Vec<Type>,
        args: Vec<Expr>,
    },
    If {
        cond: Box<Expr>,
        then_blk: Block,
        else_blk: Option<Block>,
    },
    Block(Block),
    Println {
        fmt: String,
        args: Vec<Expr>,
    },
    Print {
        fmt: String,
        args: Vec<Expr>,
    },
    Eprintln {
        fmt: String,
        args: Vec<Expr>,
    },
    Format {
        fmt: String,
        args: Vec<Expr>,
    },
    Write {
        newline: bool,
        target: Box<Expr>,
        fmt: String,
        args: Vec<Expr>,
    },
    Panic {
        name: String,
    },
    Assert {
        cond: Box<Expr>,
    },
    AssertEq {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Cfg {
        name: String,
    },
    Matches {
        expr: Box<Expr>,
        pat: Pattern,
        guard: Option<Box<Expr>>,
    },
    TupleLit(Vec<Expr>),
    VecLit(Vec<Expr>),
    VecRepeat {
        elem: Box<Expr>,
        count: Box<Expr>,
    },
    StructLit {
        name: String,
        fields: Vec<(String, Expr)>,
    },
    EnumCtor {
        enum_name: String,
        variant: String,
    },
    EnumStructLit {
        enum_name: String,
        variant: String,
        fields: Vec<(String, Expr)>,
    },
    Field {
        base: Box<Expr>,
        name: String,
    },
    TupleIndex {
        base: Box<Expr>,
        index: usize,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    Slice {
        base: Box<Expr>,
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        inclusive: bool,
    },
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        inclusive: bool,
    },
    Match {
        scrut: Box<Expr>,
        arms: Vec<Arm>,
    },
    While {
        cond: Box<Expr>,
        body: Block,
    },
    WhileLet {
        pat: Pattern,
        expr: Box<Expr>,
        body: Block,
    },
    Loop {
        body: Block,
    },
    For {
        var: String,
        start: Box<Expr>,
        end: Box<Expr>,
        inclusive: bool,
        body: Block,
    },
    ForEach {
        pat: Pattern,
        iter: Box<Expr>,
        body: Block,
    },
    Break {
        label: Option<String>,
        value: Option<Box<Expr>>,
    },
    Continue,
    /// A labelled block/loop `'name: <body>`. `break 'name [value]` unwinds to
    /// here. (Labelled `continue` is not modelled -- see docs.)
    Labeled {
        label: String,
        body: Box<Expr>,
    },
}

#[derive(Clone, Debug)]
pub struct ClosureParam {
    pub pat: Pattern,
    pub ty: Option<Type>,
}

#[derive(Clone, Debug)]
pub struct Arm {
    pub pat: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
}

#[derive(Clone, Debug)]
pub enum Pattern {
    Wild,
    Bind(String),
    Int(i64),
    Char(char),
    Str(String),
    Bool(bool),
    IntRange {
        start: i64,
        end: i64,
    },
    CharRange {
        start: char,
        end: char,
    },
    BindAt {
        name: String,
        sub: Box<Pattern>,
    },
    BindRef {
        name: String,
        mutable: bool,
    },
    Tuple(Vec<Pattern>),
    /// Slice pattern `[a, b, c]` / `[a, rest @ .., z]`. `rest` = None means an
    /// exact-length match; Some(None) is a bare `..`; Some(Some(name)) binds the
    /// middle to a Vec.
    Slice {
        prefix: Vec<Pattern>,
        rest: Option<Option<String>>,
        suffix: Vec<Pattern>,
    },
    Or(Vec<Pattern>),
    Ref {
        mutable: bool,
        sub: Box<Pattern>,
    },
    Struct {
        name: String,
        fields: Vec<(String, Pattern)>,
        rest: bool,
    },
    Enum {
        enum_name: String,
        variant: String,
        sub: Vec<Pattern>,
    },
    EnumStruct {
        enum_name: String,
        variant: String,
        fields: Vec<(String, Pattern)>,
        rest: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UnOp {
    Neg,
    Not,
    Deref,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    BitXor,
    BitAnd,
    BitOr,
    Shl,
    Shr,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}
