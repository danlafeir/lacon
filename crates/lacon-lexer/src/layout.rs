use crate::token::Token;

/// Converts a flat token stream (with RawNewline) into a layout-aware stream
/// (Newline, Indent, Dedent) using Python-style indentation rules.
///
/// Column of a token = token_span.start - line_start_byte_offset.
/// Since logos skips horizontal whitespace, the first token on a line starts
/// immediately after the whitespace logos consumed; its span.start gives us the
/// absolute byte offset, and we subtract the byte offset of the start of the
/// line (= end of the preceding RawNewline token) to get the column.
pub fn apply_layout(tokens: Vec<(Token, std::ops::Range<usize>)>) -> Vec<(Token, std::ops::Range<usize>)> {
    let mut out = Vec::new();
    let mut indent_stack: Vec<usize> = vec![0];

    // byte offset where the current line begins (right after last newline)
    let mut line_start: usize = 0;

    let mut i = 0;
    while i < tokens.len() {
        let (tok, span) = tokens[i].clone();

        match tok {
            Token::RawNewline => {
                // The line just ended. Peek ahead to find the next meaningful
                // token (skipping blank lines and comment-only lines).
                let newline_end = span.end;
                let mut j = i + 1;
                let mut next_line_start = newline_end;

                loop {
                    if j >= tokens.len() {
                        break;
                    }
                    match &tokens[j].0 {
                        Token::RawNewline => {
                            next_line_start = tokens[j].1.end;
                            j += 1;
                        }
                        Token::Comment => {
                            // skip comment tokens on their own line
                            j += 1;
                            // consume the following newline too, if present
                            if j < tokens.len() && matches!(tokens[j].0, Token::RawNewline) {
                                next_line_start = tokens[j].1.end;
                                j += 1;
                            }
                        }
                        _ => break,
                    }
                }

                // Column of next real token
                let next_col = if j < tokens.len() {
                    tokens[j].1.start.saturating_sub(next_line_start)
                } else {
                    0
                };

                // Emit Newline for the line that just finished
                out.push((Token::Newline, span.clone()));

                let top = *indent_stack.last().unwrap();
                if next_col > top {
                    indent_stack.push(next_col);
                    out.push((Token::Indent, span.clone()));
                } else {
                    while *indent_stack.last().unwrap() > next_col {
                        indent_stack.pop();
                        out.push((Token::Dedent, span.clone()));
                    }
                }

                line_start = next_line_start;
                i += 1;
            }
            Token::Comment => {
                i += 1;
            }
            _ => {
                out.push((tok, span));
                i += 1;
            }
        }
    }

    // Close any open indentation blocks at EOF
    let eof_span = if let Some((_, s)) = tokens.last() {
        s.end..s.end
    } else {
        0..0
    };
    while indent_stack.len() > 1 {
        indent_stack.pop();
        out.push((Token::Dedent, eof_span.clone()));
    }

    let _ = line_start; // used above, suppress warning if not
    out
}
