use smol_str::SmolStr;
use std::ops::Range;

pub type Span = Range<usize>;

/// A node carrying a value and its source span.
#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }
}

pub type SpExpr = Box<Spanned<Expr>>;

// ── Expressions ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Expr {
    /// Integer literal
    Int(i64),
    /// Float literal
    Float(f64),
    /// String literal
    Str(SmolStr),
    /// Boolean literal
    Bool(bool),
    /// Variable reference
    Var(SmolStr),
    /// Constructor (starts with uppercase)
    Constructor(SmolStr),

    /// Binary operation: left op right
    BinOp {
        op:  BinOp,
        lhs: SpExpr,
        rhs: SpExpr,
    },

    /// Unary negation
    Neg(SpExpr),

    /// Not
    Not(SpExpr),

    /// Function application (left-associative juxtaposition)
    App {
        func: SpExpr,
        arg:  SpExpr,
    },

    /// Lambda: \x -> body
    Lambda {
        params: Vec<SmolStr>,
        body:   SpExpr,
    },

    /// Let binding: let x = val in body
    Let {
        name: SmolStr,
        value: SpExpr,
        body:  SpExpr,
    },

    /// Match expression
    Match {
        scrutinee: SpExpr,
        arms:      Vec<MatchArm>,
    },

    /// if-then-else (sugar for match on bool)
    If {
        cond:  SpExpr,
        then_: SpExpr,
        else_: SpExpr,
    },

    /// Pipeline: lhs |> rhs
    Pipe {
        lhs: SpExpr,
        rhs: SpExpr,
    },

    /// List literal [a, b, c]
    List(Vec<SpExpr>),

    /// Tuple (a, b)
    Tuple(Vec<SpExpr>),

    /// Record literal { name = val, ... }
    Record(Vec<(SmolStr, SpExpr)>),

    /// Record field access: expr.field
    Field {
        record: SpExpr,
        field:  SmolStr,
    },

    /// Record update: { base | field = val }
    RecordUpdate {
        base:   SpExpr,
        fields: Vec<(SmolStr, SpExpr)>,
    },

    /// Async bind:  name <- expr  (only valid inside async function body)
    Bind {
        name:  SmolStr,
        value: SpExpr,
        rest:  SpExpr,
    },

    /// Sequence of expressions; all but the last are evaluated for side effects.
    /// The last expression is the block's return value.
    Block(Vec<SpExpr>),
}

// ── Binary operators ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
    Eq, Neq, Lt, Gt, Leq, Geq,
    And, Or,
    Compose,   // >>
}

// ── Patterns ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Pattern {
    /// Wildcard _
    Wildcard,
    /// Variable binding
    Var(SmolStr),
    /// Integer literal
    Int(i64),
    /// Float literal
    Float(f64),
    /// String literal
    Str(SmolStr),
    /// Bool literal
    Bool(bool),
    /// Constructor with optional payload patterns
    Constructor(SmolStr, Vec<Pattern>),
    /// Tuple pattern
    Tuple(Vec<Pattern>),
    /// Record pattern { field, ... }
    Record(Vec<(SmolStr, Pattern)>),
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pat:   Pattern,
    pub guard: Option<SpExpr>,
    pub body:  SpExpr,
}

// ── Top-level definitions ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum TopLevel {
    /// Function definition: fn name(p: T, ...) -> R { body }
    Def {
        name:        SmolStr,
        params:      Vec<(SmolStr, TypeExpr)>,  // (name, type) pairs
        return_type: TypeExpr,
        body:        SpExpr,
        is_async:    bool,
    },
    /// Type declaration
    TypeDecl {
        name:   SmolStr,
        params: Vec<SmolStr>,
        kind:   TypeKind,
    },
    /// Use/import declaration
    Use {
        path:  Vec<SmolStr>,
        alias: Option<SmolStr>,
    },
}

#[derive(Debug, Clone)]
pub enum TypeKind {
    /// Sum type (tagged union)
    Sum(Vec<Variant>),
    /// Record/product type
    Record(Vec<(SmolStr, TypeExpr)>),
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub name:    SmolStr,
    pub payload: Vec<TypeExpr>,
}

// ── Type expressions ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum TypeExpr {
    /// Named type: Int, String, a, etc.
    Named(SmolStr),
    /// Type application: List Int, Maybe String
    App(Box<TypeExpr>, Box<TypeExpr>),
    /// Function type: a -> b
    Fun(Box<TypeExpr>, Box<TypeExpr>),
    /// Async type: Async a
    Async(Box<TypeExpr>),
    /// Tuple type: (a, b)
    Tuple(Vec<TypeExpr>),
    /// Record type: { name: String, age: Int }
    Record(Vec<(SmolStr, TypeExpr)>),
}

// ── Module ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Module {
    pub items: Vec<Spanned<TopLevel>>,
}
