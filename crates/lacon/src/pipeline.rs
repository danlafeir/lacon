use std::fs;
use std::process;

use lacon_eval::{Evaluator, Value};
use lacon_lexer::lex;
use lacon_parser::{ast::TopLevel, parse_module};
use lacon_parser::error::report_errors;

/// Lex → parse → evaluate a lacon source file.
pub fn run_file(path: &str) {
    let source = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("error: cannot read '{path}': {e}");
        process::exit(1);
    });

    let tokens = lex(&source).unwrap_or_else(|spans| {
        for span in &spans {
            eprintln!("lexer error at {}..{}", span.start, span.end);
        }
        process::exit(1);
    });

    let (module, errors) = parse_module(tokens);

    if !errors.is_empty() {
        report_errors(&source, path, &errors);
        if module.is_none() {
            process::exit(1);
        }
    }

    let module = match module {
        Some(m) => m,
        None    => process::exit(1),
    };

    // Register all top-level definitions.
    // Types on parameters are erased — only names are passed to the evaluator.
    let mut eval = Evaluator::new();
    let mut has_main = false;

    for item in module.items {
        match item.node {
            TopLevel::Def { name, params, body, .. } => {
                if name == "main" { has_main = true; }
                // Strip type annotations; evaluator only needs the param names
                let param_names = params.into_iter().map(|(n, _)| n).collect();
                eval.define(name, param_names, *body);
            }
            TopLevel::TypeDecl { .. } | TopLevel::Use { .. } => {}
        }
    }

    if !has_main {
        eprintln!("error: no `main` defined");
        process::exit(1);
    }

    // `main` is zero-arg, so define() already evaluated it. Inspect the stored value
    // to determine the exit outcome.
    match eval.global_env().get("main") {
        Some(Value::Constructor(tag, args)) => {
            match (tag.as_str(), args.as_slice()) {
                // Ok(Unit) — success
                ("Ok", [Value::Constructor(inner, inner_args)])
                    if inner == "Unit" && inner_args.is_empty() =>
                {
                    process::exit(0);
                }
                // Ok(_) — success, print the inner value
                ("Ok", [val]) => {
                    println!("{val}");
                    process::exit(0);
                }
                // Err(msg) — print to stderr, exit 1
                ("Err", [Value::Str(msg)]) => {
                    eprintln!("error: {msg}");
                    process::exit(1);
                }
                ("Err", [val]) => {
                    eprintln!("error: {val}");
                    process::exit(1);
                }
                _ => {
                    // Non-Result return — just exit 0
                    process::exit(0);
                }
            }
        }
        Some(Value::Unit) | None => {
            process::exit(0);
        }
        Some(other) => {
            // main returned a plain value — print it
            println!("{other}");
            process::exit(0);
        }
    }
}
