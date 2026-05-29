use smol_str::SmolStr;
use std::collections::HashMap;
use std::fmt;

use lacon_parser::ast::{Expr, Spanned};
use crate::env::Env;

#[derive(Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(SmolStr),
    List(Vec<Value>),
    Tuple(Vec<Value>),
    Record(HashMap<SmolStr, Value>),
    /// Tagged constructor: name + payload values
    Constructor(SmolStr, Vec<Value>),
    /// Closure: captured environment, parameter names, body expression
    Closure {
        env:    Env,
        params: Vec<SmolStr>,
        body:   Box<Spanned<Expr>>,
    },
    /// Unit / empty value (returned by print, etc.)
    Unit,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n)     => write!(f, "{n}"),
            Value::Float(n)   => write!(f, "{n}"),
            Value::Bool(b)    => write!(f, "{b}"),
            Value::Str(s)     => write!(f, "{s}"),
            Value::Unit       => write!(f, "()"),
            Value::List(items) => {
                write!(f, "[")?;
                for (i, v) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{v}")?;
                }
                write!(f, "]")
            }
            Value::Tuple(items) => {
                write!(f, "(")?;
                for (i, v) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{v}")?;
                }
                write!(f, ")")
            }
            Value::Record(fields) => {
                write!(f, "{{ ")?;
                let mut pairs: Vec<_> = fields.iter().collect();
                pairs.sort_by_key(|(k, _)| k.as_str());
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{k} = {v}")?;
                }
                write!(f, " }}")
            }
            Value::Constructor(name, args) => {
                if args.is_empty() {
                    write!(f, "{name}")
                } else {
                    write!(f, "({name}")?;
                    for a in args {
                        write!(f, " {a}")?;
                    }
                    write!(f, ")")
                }
            }
            Value::Closure { .. } => write!(f, "<fn>"),
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}
