use logos::Logos;
use smol_str::SmolStr;

#[derive(Logos, Debug, Clone, PartialEq, Eq, Hash)]
#[logos(skip r"[ \t\r]+")]
pub enum Token {
    // --- Literals ---
    #[regex(r"[0-9]+", |lex| lex.slice().parse::<i64>().ok())]
    Int(i64),

    // Store float as bits so Token can derive Hash + Eq (f64 doesn't impl those)
    #[regex(r"[0-9]+\.[0-9]+", |lex| lex.slice().parse::<f64>().map(|f| f.to_bits()).ok())]
    Float(u64),

    #[regex(r#""[^"]*""#, |lex| {
        let s = lex.slice();
        Some(SmolStr::new(&s[1..s.len()-1]))
    })]
    Str(SmolStr),

    #[token("true")]
    True,

    #[token("false")]
    False,

    // --- Keywords (must appear before Ident) ---
    #[token("fn")]
    Fn,

    #[token("async")]
    Async,

    #[token("match")]
    Match,

    #[token("let")]
    Let,

    #[token("in")]
    In,

    #[token("if")]
    If,

    #[token("then")]
    Then,

    #[token("else")]
    Else,

    #[token("type")]
    Type,

    #[token("use")]
    Use,

    #[token("when")]
    When,

    // --- Identifiers ---
    #[regex(r"[a-z_][a-zA-Z0-9_]*", |lex| SmolStr::new(lex.slice()))]
    Ident(SmolStr),

    #[regex(r"[A-Z][a-zA-Z0-9_]*", |lex| SmolStr::new(lex.slice()))]
    TypeIdent(SmolStr),

    // --- Operators (longer tokens before shorter to avoid prefix ambiguity) ---
    #[token("|>")]
    PipeRight,

    #[token("<|")]
    PipeLeft,

    #[token(">>")]
    Compose,

    #[token("::")]
    DoubleColon,

    #[token("->")]
    Arrow,

    #[token("<-")]
    BindArrow,

    #[token("=>")]
    FatArrow,

    #[token("==")]
    EqEq,

    #[token("!=")]
    Neq,

    #[token("<=")]
    Leq,

    #[token(">=")]
    Geq,

    #[token("&&")]
    And,

    #[token("||")]
    Or,

    #[token("..")]
    DotDot,

    #[token(".")]
    Dot,

    #[token("=")]
    Eq,

    #[token("+")]
    Plus,

    #[token("-")]
    Minus,

    #[token("*")]
    Star,

    #[token("/")]
    Slash,

    #[token("%")]
    Percent,

    #[token("<")]
    Lt,

    #[token(">")]
    Gt,

    #[token("!")]
    Bang,

    #[token("\\")]
    BackSlash,

    #[token("|")]
    Pipe,

    #[token(",")]
    Comma,

    #[token(":")]
    Colon,

    // --- Delimiters ---
    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[token("[")]
    LBrack,

    #[token("]")]
    RBrack,

    #[token("{")]
    LBrace,

    #[token("}")]
    RBrace,

    // --- Layout (produced by post-processor) ---
    Newline,
    Indent,
    Dedent,

    // --- Raw newline consumed by layout pass ---
    #[token("\n")]
    RawNewline,

    // --- Comments (skip) ---
    #[regex(r"--[^\n]*")]
    Comment,
}

impl Token {
    pub fn float_value(bits: u64) -> f64 {
        f64::from_bits(bits)
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Int(n)       => write!(f, "{n}"),
            Token::Float(b)     => write!(f, "{}", f64::from_bits(*b)),
            Token::Str(s)       => write!(f, "\"{s}\""),
            Token::True         => write!(f, "true"),
            Token::False        => write!(f, "false"),
            Token::Fn           => write!(f, "fn"),
            Token::Async        => write!(f, "async"),
            Token::Match        => write!(f, "match"),
            Token::Let          => write!(f, "let"),
            Token::In           => write!(f, "in"),
            Token::If           => write!(f, "if"),
            Token::Then         => write!(f, "then"),
            Token::Else         => write!(f, "else"),
            Token::Type         => write!(f, "type"),
            Token::Use          => write!(f, "use"),
            Token::When         => write!(f, "when"),
            Token::Ident(s)     => write!(f, "{s}"),
            Token::TypeIdent(s) => write!(f, "{s}"),
            Token::PipeRight    => write!(f, "|>"),
            Token::PipeLeft     => write!(f, "<|"),
            Token::Compose      => write!(f, ">>"),
            Token::DoubleColon  => write!(f, "::"),
            Token::Arrow        => write!(f, "->"),
            Token::BindArrow    => write!(f, "<-"),
            Token::FatArrow     => write!(f, "=>"),
            Token::EqEq         => write!(f, "=="),
            Token::Neq          => write!(f, "!="),
            Token::Leq          => write!(f, "<="),
            Token::Geq          => write!(f, ">="),
            Token::And          => write!(f, "&&"),
            Token::Or           => write!(f, "||"),
            Token::DotDot       => write!(f, ".."),
            Token::Dot          => write!(f, "."),
            Token::Eq           => write!(f, "="),
            Token::Plus         => write!(f, "+"),
            Token::Minus        => write!(f, "-"),
            Token::Star         => write!(f, "*"),
            Token::Slash        => write!(f, "/"),
            Token::Percent      => write!(f, "%"),
            Token::Lt           => write!(f, "<"),
            Token::Gt           => write!(f, ">"),
            Token::Bang         => write!(f, "!"),
            Token::BackSlash    => write!(f, "\\"),
            Token::Pipe         => write!(f, "|"),
            Token::Comma        => write!(f, ","),
            Token::Colon        => write!(f, ":"),
            Token::LParen       => write!(f, "("),
            Token::RParen       => write!(f, ")"),
            Token::LBrack       => write!(f, "["),
            Token::RBrack       => write!(f, "]"),
            Token::LBrace       => write!(f, "{{"),
            Token::RBrace       => write!(f, "}}"),
            Token::Newline      => write!(f, "<newline>"),
            Token::Indent       => write!(f, "<indent>"),
            Token::Dedent       => write!(f, "<dedent>"),
            Token::RawNewline   => write!(f, "<raw-newline>"),
            Token::Comment      => write!(f, "<comment>"),
        }
    }
}
