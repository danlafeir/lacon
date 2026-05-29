use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::error::Simple;
use lacon_lexer::Token;

/// Render parse errors to stderr using ariadne.
pub fn report_errors(source: &str, filename: &str, errors: &[Simple<Token>]) {
    for e in errors {
        let span = e.span();
        let msg = match e.reason() {
            chumsky::error::SimpleReason::Unexpected => {
                let found = e.found()
                    .map(|t| format!("`{t}`"))
                    .unwrap_or_else(|| "end of input".to_string());
                let expected: Vec<String> = e.expected()
                    .filter_map(|t| t.as_ref().map(|t| format!("`{t}`")))
                    .collect();
                if expected.is_empty() {
                    format!("unexpected {found}")
                } else {
                    format!("unexpected {found}, expected {}", expected.join(", "))
                }
            }
            chumsky::error::SimpleReason::Unclosed { span: _, delimiter } => {
                format!("unclosed delimiter `{delimiter}`")
            }
            chumsky::error::SimpleReason::Custom(msg) => msg.clone(),
        };

        Report::build(ReportKind::Error, filename, span.start)
            .with_message(msg.clone())
            .with_label(
                Label::new((filename, span))
                    .with_message(msg)
                    .with_color(Color::Red),
            )
            .finish()
            .print((filename, Source::from(source)))
            .unwrap();
    }
}
