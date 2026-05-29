use chumsky::prelude::*;
use smol_str::SmolStr;

use lacon_lexer::Token;
use crate::ast::*;

pub type ParseError = Simple<Token>;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn tok(t: Token) -> impl Parser<Token, Token, Error = ParseError> + Clone {
    just(t)
}

fn ident_parser() -> impl Parser<Token, SmolStr, Error = ParseError> + Clone {
    select! { Token::Ident(s) => s }
}

fn type_ident_parser() -> impl Parser<Token, SmolStr, Error = ParseError> + Clone {
    select! { Token::TypeIdent(s) => s }
}

// ── Type expressions ──────────────────────────────────────────────────────────

/// Parse a type expression.
/// - Atomic:  Int  String  Bool  Float  Unit  a  (type variable)
/// - Applied: List(Int)  Result(User, String)  Maybe(a)
/// - Tuple:   (Int, String)
/// - Function: a -> b  (only when parenthesised in param position)
pub fn type_expr_parser() -> impl Parser<Token, TypeExpr, Error = ParseError> + Clone {
    recursive(|ty| {
        // Atomic: named type or type variable (both ident and TypeIdent)
        let named_lower = ident_parser().map(TypeExpr::Named);
        let named_upper = type_ident_parser().map(TypeExpr::Named);

        // Applied: TypeName<arg, arg, ...>
        let applied = type_ident_parser()
            .then(
                ty.clone()
                    .separated_by(tok(Token::Comma))
                    .at_least(1)
                    .collect::<Vec<_>>()
                    .delimited_by(tok(Token::Lt), tok(Token::Gt))
            )
            .map(|(name, args)| {
                args.into_iter().fold(TypeExpr::Named(name), |acc, arg| {
                    TypeExpr::App(Box::new(acc), Box::new(arg))
                })
            });

        // Tuple: (T, T, ...)  — two or more types
        let tuple = ty.clone()
            .separated_by(tok(Token::Comma))
            .at_least(2)
            .collect::<Vec<_>>()
            .delimited_by(tok(Token::LParen), tok(Token::RParen))
            .map(TypeExpr::Tuple);

        // Parenthesised single type (also used to wrap function types)
        let paren = ty.clone()
            .delimited_by(tok(Token::LParen), tok(Token::RParen));

        // Atom: try applied (TypeIdent + parens) before bare TypeIdent
        let atom = applied.or(tuple).or(paren).or(named_upper).or(named_lower);

        // Function type: atom -> atom -> ...  (right-associative)
        atom.clone()
            .then(
                tok(Token::Arrow)
                    .ignore_then(ty.clone())
                    .repeated()
                    .collect::<Vec<_>>()
            )
            .map(|(head, rest)| {
                // right-fold: a -> b -> c  becomes  Fun(a, Fun(b, c))
                if rest.is_empty() {
                    head
                } else {
                    let mut all = vec![head];
                    all.extend(rest);
                    all.into_iter().rev().reduce(|b, a| TypeExpr::Fun(Box::new(a), Box::new(b))).unwrap()
                }
            })
    })
}

// ── Patterns ──────────────────────────────────────────────────────────────────

fn pattern_parser() -> impl Parser<Token, Pattern, Error = ParseError> + Clone {
    recursive(|pat| {
        let wildcard = select! { Token::Ident(s) if s == "_" => Pattern::Wildcard };
        let var      = select! { Token::Ident(s) if s != "_" => Pattern::Var(s) };
        let int_lit  = select! { Token::Int(n)   => Pattern::Int(n) };
        let float_lit= select! { Token::Float(bits) => Pattern::Float(f64::from_bits(bits)) };
        let str_lit  = select! { Token::Str(s)   => Pattern::Str(s) };
        let bool_lit = tok(Token::True).map(|_| Pattern::Bool(true))
            .or(tok(Token::False).map(|_| Pattern::Bool(false)));

        let atom = wildcard.or(bool_lit).or(int_lit).or(float_lit).or(str_lit).or(var);

        // Constructor: TypeIdent payload*
        let ctor = type_ident_parser()
            .then(
                // payload patterns inside parens: Ok(x) or bare: Ok
                pat.clone()
                    .separated_by(tok(Token::Comma))
                    .collect::<Vec<_>>()
                    .delimited_by(tok(Token::LParen), tok(Token::RParen))
                    .or(atom.clone().map(|p| vec![p]))
                    .or_not()
                    .map(|opt| opt.unwrap_or_default())
            )
            .map(|(name, args)| Pattern::Constructor(name, args));

        // Tuple: (p, p, ...)
        let tuple = pat.clone()
            .separated_by(tok(Token::Comma))
            .at_least(2)
            .collect::<Vec<_>>()
            .delimited_by(tok(Token::LParen), tok(Token::RParen))
            .map(Pattern::Tuple);

        let paren = pat.clone()
            .delimited_by(tok(Token::LParen), tok(Token::RParen));

        tuple.or(paren).or(ctor).or(atom)
    })
}

// ── Expressions ───────────────────────────────────────────────────────────────

pub fn expr_parser() -> impl Parser<Token, Spanned<Expr>, Error = ParseError> + Clone {
    recursive(|expr| {
        // Literals
        let int_lit = select! { Token::Int(n) => n }
            .map_with_span(|n, span| Spanned::new(Expr::Int(n), span));
        let float_lit = select! { Token::Float(bits) => f64::from_bits(bits) }
            .map_with_span(|n, span| Spanned::new(Expr::Float(n), span));
        let str_lit = select! { Token::Str(s) => s }
            .map_with_span(|s, span| Spanned::new(Expr::Str(s), span));
        let bool_true = tok(Token::True)
            .map_with_span(|_, span| Spanned::new(Expr::Bool(true), span));
        let bool_false = tok(Token::False)
            .map_with_span(|_, span| Spanned::new(Expr::Bool(false), span));
        let var = select! { Token::Ident(s) => s }
            .map_with_span(|s, span| Spanned::new(Expr::Var(s), span));
        let ctor = select! { Token::TypeIdent(s) => s }
            .map_with_span(|s, span| Spanned::new(Expr::Constructor(s), span));

        // List literal: [a, b, c]
        let list = expr.clone()
            .separated_by(tok(Token::Comma))
            .collect::<Vec<_>>()
            .delimited_by(tok(Token::LBrack), tok(Token::RBrack))
            .map_with_span(|items, span| {
                Spanned::new(Expr::List(items.into_iter().map(Box::new).collect()), span)
            });

        // Parenthesised expr / tuple: (a) or (a, b)
        let paren = expr.clone()
            .separated_by(tok(Token::Comma))
            .at_least(1)
            .collect::<Vec<_>>()
            .delimited_by(tok(Token::LParen), tok(Token::RParen))
            .map_with_span(|mut items, span| {
                if items.len() == 1 {
                    items.remove(0)
                } else {
                    Spanned::new(Expr::Tuple(items.into_iter().map(Box::new).collect()), span)
                }
            });

        // Lambda: \x y -> body
        let lambda = tok(Token::BackSlash)
            .ignore_then(ident_parser().repeated().at_least(1).collect::<Vec<_>>())
            .then_ignore(tok(Token::Arrow))
            .then(expr.clone())
            .map_with_span(|(params, body), span| {
                Spanned::new(Expr::Lambda { params, body: Box::new(body) }, span)
            });

        // Let: let x = val in body
        let let_expr = tok(Token::Let)
            .ignore_then(ident_parser())
            .then_ignore(tok(Token::Eq))
            .then(expr.clone())
            .then_ignore(tok(Token::In))
            .then(expr.clone())
            .map_with_span(|((name, value), body), span| {
                Spanned::new(Expr::Let { name, value: Box::new(value), body: Box::new(body) }, span)
            });

        // If-then-else
        let if_expr = tok(Token::If)
            .ignore_then(expr.clone())
            .then_ignore(tok(Token::Then))
            .then(expr.clone())
            .then_ignore(tok(Token::Else))
            .then(expr.clone())
            .map_with_span(|((cond, then_), else_), span| {
                Spanned::new(Expr::If {
                    cond:  Box::new(cond),
                    then_: Box::new(then_),
                    else_: Box::new(else_),
                }, span)
            });

        // Match arm: | pat (when guard)? => body
        let match_arm = tok(Token::Pipe)
            .ignore_then(pattern_parser())
            .then(tok(Token::When).ignore_then(expr.clone()).or_not())
            .then_ignore(tok(Token::FatArrow))
            .then(expr.clone())
            .map(|((pat, guard), body)| MatchArm {
                pat,
                guard: guard.map(Box::new),
                body: Box::new(body),
            });

        // Match: match scrutinee { arms } or indented arms
        let nl = tok(Token::Newline).ignored();
        let arm_with_nl = nl.clone().repeated().ignore_then(match_arm.clone());

        let match_braced = tok(Token::Match)
            .ignore_then(expr.clone())
            .then(
                arm_with_nl.clone()
                    .repeated()
                    .at_least(1)
                    .collect::<Vec<_>>()
                    .delimited_by(
                        tok(Token::LBrace).then_ignore(nl.clone().or_not()),
                        nl.clone().or_not().ignore_then(tok(Token::RBrace)),
                    )
            )
            .map_with_span(|(scrutinee, arms), span| {
                Spanned::new(Expr::Match { scrutinee: Box::new(scrutinee), arms }, span)
            });

        // Also allow indented match (no braces) for backwards compat
        let match_indented = tok(Token::Match)
            .ignore_then(expr.clone())
            .then_ignore(nl.clone().or_not())
            .then_ignore(tok(Token::Indent).or_not())
            .then(arm_with_nl.repeated().at_least(1).collect::<Vec<_>>())
            .then_ignore(nl.clone().or_not())
            .then_ignore(tok(Token::Dedent).or_not())
            .map_with_span(|(scrutinee, arms), span| {
                Spanned::new(Expr::Match { scrutinee: Box::new(scrutinee), arms }, span)
            });

        let match_expr = match_braced.or(match_indented);

        // Atom
        let atom = int_lit
            .or(float_lit)
            .or(str_lit)
            .or(bool_true)
            .or(bool_false)
            .or(lambda)
            .or(let_expr)
            .or(if_expr)
            .or(match_expr)
            .or(list)
            .or(paren)
            .or(ctor)
            .or(var);

        // Field access: atom(.field)*
        let field_access = atom
            .then(
                tok(Token::Dot)
                    .ignore_then(ident_parser())
                    .repeated()
                    .collect::<Vec<_>>()
            )
            .map(|(base, fields)| {
                fields.into_iter().fold(base, |acc, field| {
                    let span = acc.span.clone();
                    Spanned::new(Expr::Field { record: Box::new(acc), field }, span)
                })
            });

        // Function application: f a b  (left-associative juxtaposition)
        // f(a, b)  is parsed as  App(f, Tuple(a,b))  which we expand to  App(App(f, a), b)
        // This makes parenthesised multi-arg calls behave like curried application.
        let app = field_access.clone()
            .then(field_access.repeated().collect::<Vec<_>>())
            .map(|(func, args)| {
                args.into_iter().fold(func, |f, arg| {
                    match arg.node {
                        // Expand tuple arg into multiple curried applications
                        Expr::Tuple(items) => items.into_iter().fold(f, |f, item| {
                            let span = f.span.start..item.span.end;
                            Spanned::new(Expr::App { func: Box::new(f), arg: item }, span)
                        }),
                        _ => {
                            let span = f.span.start..arg.span.end;
                            Spanned::new(Expr::App { func: Box::new(f), arg: Box::new(arg) }, span)
                        }
                    }
                })
            });

        // Unary negation
        let unary_neg = tok(Token::Minus)
            .ignore_then(app.clone())
            .map_with_span(|e, span| Spanned::new(Expr::Neg(Box::new(e)), span));

        let unary = unary_neg.or(app);

        // Binary operators, lowest precedence first
        macro_rules! binop_level {
            ($lower:expr, $ops:expr) => {{
                $lower.clone()
                    .then(
                        $ops.then($lower.clone())
                            .repeated()
                            .collect::<Vec<_>>()
                    )
                    .map(|(lhs, rhs)| {
                        rhs.into_iter().fold(lhs, |l, (op, r)| {
                            let span = l.span.start..r.span.end;
                            Spanned::new(Expr::BinOp { op, lhs: Box::new(l), rhs: Box::new(r) }, span)
                        })
                    })
            }};
        }

        let mul_ops = tok(Token::Star).map(|_| BinOp::Mul)
            .or(tok(Token::Slash).map(|_| BinOp::Div))
            .or(tok(Token::Percent).map(|_| BinOp::Mod));
        let mul = binop_level!(unary, mul_ops);

        let add_ops = tok(Token::Plus).map(|_| BinOp::Add)
            .or(tok(Token::Minus).map(|_| BinOp::Sub));
        let add = binop_level!(mul, add_ops);

        let cmp_ops = tok(Token::EqEq).map(|_| BinOp::Eq)
            .or(tok(Token::Neq).map(|_| BinOp::Neq))
            .or(tok(Token::Leq).map(|_| BinOp::Leq))
            .or(tok(Token::Geq).map(|_| BinOp::Geq))
            .or(tok(Token::Lt).map(|_| BinOp::Lt))
            .or(tok(Token::Gt).map(|_| BinOp::Gt));
        let cmp = binop_level!(add, cmp_ops);

        let and_op = tok(Token::And).map(|_| BinOp::And);
        let and_expr = binop_level!(cmp, and_op);

        let or_op = tok(Token::Or).map(|_| BinOp::Or);
        let or_expr = binop_level!(and_expr, or_op);

        // Pipe: |> (left-associative)
        let pipe = or_expr.clone()
            .then(
                tok(Token::PipeRight)
                    .ignore_then(or_expr.clone())
                    .repeated()
                    .collect::<Vec<_>>()
            )
            .map(|(lhs, rhss)| {
                rhss.into_iter().fold(lhs, |l, r| {
                    let span = l.span.start..r.span.end;
                    Spanned::new(Expr::Pipe { lhs: Box::new(l), rhs: Box::new(r) }, span)
                })
            });

        pipe
    })
}

// ── Top-level ─────────────────────────────────────────────────────────────────

/// Parse a single typed parameter: name: Type
fn param_parser() -> impl Parser<Token, (SmolStr, TypeExpr), Error = ParseError> + Clone {
    ident_parser()
        .then_ignore(tok(Token::Colon))
        .then(type_expr_parser())
}

/// Parse the function body: { expr } or { expr\nexpr\n... }
/// Each statement is an expression optionally followed by whitespace/newline tokens.
/// The last expression is the return value; earlier ones are evaluated for side effects.
fn body_parser() -> impl Parser<Token, Spanned<Expr>, Error = ParseError> + Clone {
    let ws = tok(Token::Newline)
        .or(tok(Token::Indent))
        .or(tok(Token::Dedent))
        .repeated()
        .ignored();

    // Each statement: parse an expression, then consume any trailing whitespace
    let stmt = expr_parser().then_ignore(ws.clone());

    stmt.repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .delimited_by(
            tok(Token::LBrace).then_ignore(ws.clone()),
            tok(Token::RBrace),
        )
        .map_with_span(|mut stmts, span| {
            if stmts.len() == 1 {
                stmts.remove(0)
            } else {
                Spanned::new(Expr::Block(stmts.into_iter().map(Box::new).collect()), span)
            }
        })
}

pub fn top_level_parser() -> impl Parser<Token, Vec<Spanned<TopLevel>>, Error = ParseError> + Clone {
    let skip_nl = tok(Token::Newline)
        .or(tok(Token::Dedent))
        .repeated()
        .ignored();

    // fn name(p: T, ...) -> ReturnType { body }
    // async fn name(p: T, ...) -> ReturnType { body }
    let def = tok(Token::Async).or_not()
        .then_ignore(tok(Token::Fn))
        .then(ident_parser())
        .then(
            param_parser()
                .separated_by(tok(Token::Comma))
                .collect::<Vec<_>>()
                .delimited_by(tok(Token::LParen), tok(Token::RParen))
        )
        .then_ignore(tok(Token::Arrow))
        .then(type_expr_parser())
        .then(body_parser())
        .map_with_span(|((((is_async, name), params), return_type), body), span| {
            Spanned::new(TopLevel::Def {
                name,
                params,
                return_type,
                body: Box::new(body),
                is_async: is_async.is_some(),
            }, span)
        });

    def.padded_by(skip_nl)
        .repeated()
        .collect::<Vec<_>>()
        .then_ignore(end())
}

// ── Public entry points ───────────────────────────────────────────────────────

pub fn parse_module(tokens: Vec<(Token, std::ops::Range<usize>)>) -> (Option<Module>, Vec<ParseError>) {
    let eoi = 0..usize::MAX;
    let stream = chumsky::Stream::from_iter(eoi, tokens.into_iter());
    let (items, errors) = top_level_parser().parse_recovery(stream);
    (items.map(|items| Module { items }), errors)
}

pub fn parse_expr(tokens: Vec<(Token, std::ops::Range<usize>)>) -> (Option<Spanned<Expr>>, Vec<ParseError>) {
    let eoi = 0..usize::MAX;
    let stream = chumsky::Stream::from_iter(eoi, tokens.into_iter());
    expr_parser().then_ignore(end()).parse_recovery(stream)
}
