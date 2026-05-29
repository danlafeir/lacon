pub mod layout;
pub mod token;

pub use token::Token;

use logos::Logos;

/// Lex a source string into a token stream with layout tokens applied.
/// Returns `Ok(tokens)` or `Err(vec_of_error_spans)`.
pub fn lex(source: &str) -> Result<Vec<(Token, std::ops::Range<usize>)>, Vec<std::ops::Range<usize>>> {
    let mut tokens = Vec::new();
    let mut errors = Vec::new();

    for (result, span) in Token::lexer(source).spanned() {
        match result {
            Ok(tok) => tokens.push((tok, span)),
            Err(_)  => errors.push(span),
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(layout::apply_layout(tokens))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_integers() {
        let tokens = lex("42 0 100").unwrap();
        let toks: Vec<_> = tokens.into_iter().map(|(t, _)| t).collect();
        assert_eq!(toks, vec![Token::Int(42), Token::Int(0), Token::Int(100)]);
    }

    #[test]
    fn lex_simple_def() {
        let src = "add x y = x + y";
        let tokens = lex(src).unwrap();
        let toks: Vec<_> = tokens.into_iter().map(|(t, _)| t).collect();
        use Token::*;
        use smol_str::SmolStr;
        assert_eq!(toks, vec![
            Ident(SmolStr::new("add")),
            Ident(SmolStr::new("x")),
            Ident(SmolStr::new("y")),
            Eq,
            Ident(SmolStr::new("x")),
            Plus,
            Ident(SmolStr::new("y")),
        ]);
    }

    #[test]
    fn lex_keywords() {
        let src = "match let in if then else type use async when";
        let tokens = lex(src).unwrap();
        let toks: Vec<_> = tokens.into_iter().map(|(t, _)| t).collect();
        use Token::*;
        assert_eq!(toks, vec![Match, Let, In, If, Then, Else, Type, Use, Async, When]);
    }
}
