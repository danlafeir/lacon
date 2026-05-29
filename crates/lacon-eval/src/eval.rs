use smol_str::SmolStr;
use std::collections::HashMap;

use lacon_parser::ast::*;

use crate::env::Env;
use crate::value::Value;

#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    #[error("undefined variable: {0}")]
    Undefined(SmolStr),

    #[error("type error: {0}")]
    TypeError(String),

    #[error("non-exhaustive match")]
    NonExhaustiveMatch,

    #[error("division by zero")]
    DivisionByZero,

    #[error("arity error: expected {expected} args, got {got}")]
    Arity { expected: usize, got: usize },
}

pub type EvalResult = Result<Value, EvalError>;

// ── Built-in functions ────────────────────────────────────────────────────────

fn builtin_env() -> Env {
    let mut env = Env::new();

    // print: print a value to stdout, return Unit
    env = env.extend(SmolStr::new("print"), Value::Closure {
        env: Env::new(),
        params: vec![SmolStr::new("x")],
        body: Box::new(Spanned::new(Expr::Var(SmolStr::new("__builtin_print__")), 0..0)),
    });

    // Basic list operations
    env = env.extend(SmolStr::new("map"), Value::Closure {
        env: Env::new(),
        params: vec![SmolStr::new("f"), SmolStr::new("xs")],
        body: Box::new(Spanned::new(Expr::Var(SmolStr::new("__builtin_map__")), 0..0)),
    });

    env = env.extend(SmolStr::new("filter"), Value::Closure {
        env: Env::new(),
        params: vec![SmolStr::new("f"), SmolStr::new("xs")],
        body: Box::new(Spanned::new(Expr::Var(SmolStr::new("__builtin_filter__")), 0..0)),
    });

    env = env.extend(SmolStr::new("fold"), Value::Closure {
        env: Env::new(),
        params: vec![SmolStr::new("init"), SmolStr::new("f"), SmolStr::new("xs")],
        body: Box::new(Spanned::new(Expr::Var(SmolStr::new("__builtin_fold__")), 0..0)),
    });

    env = env.extend(SmolStr::new("head"), Value::Closure {
        env: Env::new(),
        params: vec![SmolStr::new("xs")],
        body: Box::new(Spanned::new(Expr::Var(SmolStr::new("__builtin_head__")), 0..0)),
    });

    env = env.extend(SmolStr::new("tail"), Value::Closure {
        env: Env::new(),
        params: vec![SmolStr::new("xs")],
        body: Box::new(Spanned::new(Expr::Var(SmolStr::new("__builtin_tail__")), 0..0)),
    });

    env = env.extend(SmolStr::new("len"), Value::Closure {
        env: Env::new(),
        params: vec![SmolStr::new("xs")],
        body: Box::new(Spanned::new(Expr::Var(SmolStr::new("__builtin_len__")), 0..0)),
    });

    env
}

// ── Evaluator ─────────────────────────────────────────────────────────────────

pub struct Evaluator {
    global: Env,
}

impl Evaluator {
    pub fn new() -> Self {
        Self { global: builtin_env() }
    }

    /// Register a top-level definition in the global environment.
    /// `params` is the list of parameter names (types are ignored at runtime).
    pub fn define(&mut self, name: SmolStr, params: Vec<SmolStr>, body: Spanned<Expr>) {
        let closure = if params.is_empty() {
            // Zero-param: evaluate immediately (constant / main entry point)
            match self.eval(&self.global.clone(), &body) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("error evaluating {name}: {e}");
                    Value::Unit
                }
            }
        } else {
            Value::Closure {
                env: self.global.clone(),
                params,
                body: Box::new(body),
            }
        };
        self.global = self.global.extend(name, closure);
    }

    /// Evaluate an expression in a given environment.
    pub fn eval(&self, env: &Env, expr: &Spanned<Expr>) -> EvalResult {
        match &expr.node {
            Expr::Int(n)   => Ok(Value::Int(*n)),
            Expr::Float(n) => Ok(Value::Float(*n)),
            Expr::Str(s)   => Ok(Value::Str(s.clone())),
            Expr::Bool(b)  => Ok(Value::Bool(*b)),

            Expr::Var(name) => {
                // Check local env first, then global (enables mutual recursion between top-level defs)
                if let Some(v) = env.get(name.as_str()).or_else(|| self.global.get(name.as_str())) {
                    return Ok(v.clone());
                }
                Err(EvalError::Undefined(name.clone()))
            }

            Expr::Constructor(name) => Ok(Value::Constructor(name.clone(), vec![])),

            Expr::Neg(inner) => {
                match self.eval(env, inner)? {
                    Value::Int(n)   => Ok(Value::Int(-n)),
                    Value::Float(n) => Ok(Value::Float(-n)),
                    other => Err(EvalError::TypeError(format!("cannot negate {other}"))),
                }
            }

            Expr::Not(inner) => {
                match self.eval(env, inner)? {
                    Value::Bool(b) => Ok(Value::Bool(!b)),
                    other => Err(EvalError::TypeError(format!("cannot `not` {other}"))),
                }
            }

            Expr::BinOp { op, lhs, rhs } => {
                let l = self.eval(env, lhs)?;
                let r = self.eval(env, rhs)?;
                self.eval_binop(*op, l, r)
            }

            Expr::Lambda { params, body } => {
                Ok(Value::Closure {
                    env: env.clone(),
                    params: params.clone(),
                    body: body.clone(),
                })
            }

            Expr::App { func, arg } => {
                let fv = self.eval(env, func)?;
                let av = self.eval(env, arg)?;
                self.apply(fv, av)
            }

            Expr::Let { name, value, body } => {
                let v = self.eval(env, value)?;
                let env2 = env.extend(name.clone(), v);
                self.eval(&env2, body)
            }

            Expr::If { cond, then_, else_ } => {
                match self.eval(env, cond)? {
                    Value::Bool(true)  => self.eval(env, then_),
                    Value::Bool(false) => self.eval(env, else_),
                    other => Err(EvalError::TypeError(format!("if condition must be Bool, got {other}"))),
                }
            }

            Expr::Match { scrutinee, arms } => {
                let v = self.eval(env, scrutinee)?;
                for arm in arms {
                    if let Some(bindings) = match_pattern(&arm.pat, &v) {
                        let env2 = env.extend_many(bindings);
                        // Check guard if present
                        if let Some(guard) = &arm.guard {
                            match self.eval(&env2, guard)? {
                                Value::Bool(true)  => {}
                                Value::Bool(false) => continue,
                                other => return Err(EvalError::TypeError(
                                    format!("match guard must be Bool, got {other}")
                                )),
                            }
                        }
                        return self.eval(&env2, &arm.body);
                    }
                }
                Err(EvalError::NonExhaustiveMatch)
            }

            Expr::Pipe { lhs, rhs } => {
                // lhs |> rhs  ==>  rhs lhs
                let lv = self.eval(env, lhs)?;
                let fv = self.eval(env, rhs)?;
                self.apply(fv, lv)
            }

            Expr::List(items) => {
                let vals: Result<Vec<Value>, EvalError> = items.iter().map(|e| self.eval(env, e)).collect();
                Ok(Value::List(vals?))
            }

            Expr::Tuple(items) => {
                let vals: Result<Vec<_>, _> = items.iter().map(|e| self.eval(env, e)).collect();
                Ok(Value::Tuple(vals?))
            }

            Expr::Record(fields) => {
                let mut map = HashMap::new();
                for (k, v) in fields {
                    map.insert(k.clone(), self.eval(env, v)?);
                }
                Ok(Value::Record(map))
            }

            Expr::Field { record, field } => {
                match self.eval(env, record)? {
                    Value::Record(map) => map.get(field.as_str())
                        .cloned()
                        .ok_or_else(|| EvalError::TypeError(format!("no field `{field}`"))),
                    other => Err(EvalError::TypeError(format!("field access on non-record: {other}"))),
                }
            }

            Expr::RecordUpdate { base, fields } => {
                match self.eval(env, base)? {
                    Value::Record(mut map) => {
                        for (k, v) in fields {
                            map.insert(k.clone(), self.eval(env, v)?);
                        }
                        Ok(Value::Record(map))
                    }
                    other => Err(EvalError::TypeError(format!("record update on non-record: {other}"))),
                }
            }

            Expr::Bind { .. } => {
                Err(EvalError::TypeError("async bind `<-` used outside async context".to_string()))
            }

            Expr::Block(stmts) => {
                let mut result = Value::Unit;
                for stmt in stmts {
                    result = self.eval(env, stmt)?;
                }
                Ok(result)
            }
        }
    }

    fn apply(&self, func: Value, arg: Value) -> EvalResult {
        match func {
            Value::Closure { env, params, body } => {
                if params.is_empty() {
                    return Err(EvalError::Arity { expected: 0, got: 1 });
                }

                let name = params[0].clone();
                let env2 = env.extend(name.clone(), arg);

                if params.len() == 1 {
                    // Check for built-in sentinels
                    match body.node {
                        Expr::Var(ref sentinel) if sentinel.starts_with("__builtin_") => {
                            return self.call_builtin(sentinel.as_str(), &env2);
                        }
                        _ => {}
                    }
                    self.eval(&env2, &body)
                } else {
                    // Partial application
                    Ok(Value::Closure {
                        env: env2,
                        params: params[1..].to_vec(),
                        body,
                    })
                }
            }
            // Constructor application: add argument to payload
            Value::Constructor(name, mut args) => {
                args.push(arg);
                Ok(Value::Constructor(name, args))
            }
            other => Err(EvalError::TypeError(format!("cannot apply non-function: {other}"))),
        }
    }

    fn call_builtin(&self, sentinel: &str, env: &Env) -> EvalResult {
        match sentinel {
            "__builtin_print__" => {
                let x = env.get("x").cloned().unwrap_or(Value::Unit);
                println!("{x}");
                Ok(Value::Unit)
            }
            "__builtin_map__" => {
                let f  = env.get("f").cloned().ok_or_else(|| EvalError::Undefined(SmolStr::new("f")))?;
                let xs = env.get("xs").cloned().ok_or_else(|| EvalError::Undefined(SmolStr::new("xs")))?;
                match xs {
                    Value::List(items) => {
                        let mapped: Result<Vec<_>, _> = items.into_iter()
                            .map(|v| self.apply(f.clone(), v))
                            .collect();
                        Ok(Value::List(mapped?))
                    }
                    other => Err(EvalError::TypeError(format!("map: expected List, got {other}"))),
                }
            }
            "__builtin_filter__" => {
                let f  = env.get("f").cloned().ok_or_else(|| EvalError::Undefined(SmolStr::new("f")))?;
                let xs = env.get("xs").cloned().ok_or_else(|| EvalError::Undefined(SmolStr::new("xs")))?;
                match xs {
                    Value::List(items) => {
                        let mut out = Vec::new();
                        for v in items {
                            match self.apply(f.clone(), v.clone())? {
                                Value::Bool(true)  => out.push(v),
                                Value::Bool(false) => {}
                                other => return Err(EvalError::TypeError(
                                    format!("filter predicate must return Bool, got {other}")
                                )),
                            }
                        }
                        Ok(Value::List(out))
                    }
                    other => Err(EvalError::TypeError(format!("filter: expected List, got {other}"))),
                }
            }
            "__builtin_fold__" => {
                let init = env.get("init").cloned().ok_or_else(|| EvalError::Undefined(SmolStr::new("init")))?;
                let f    = env.get("f").cloned().ok_or_else(|| EvalError::Undefined(SmolStr::new("f")))?;
                let xs   = env.get("xs").cloned().ok_or_else(|| EvalError::Undefined(SmolStr::new("xs")))?;
                match xs {
                    Value::List(items) => {
                        let mut acc = init;
                        for v in items {
                            let partial = self.apply(f.clone(), acc)?;
                            acc = self.apply(partial, v)?;
                        }
                        Ok(acc)
                    }
                    other => Err(EvalError::TypeError(format!("fold: expected List, got {other}"))),
                }
            }
            "__builtin_head__" => {
                let xs = env.get("xs").cloned().ok_or_else(|| EvalError::Undefined(SmolStr::new("xs")))?;
                match xs {
                    Value::List(mut items) if !items.is_empty() => Ok(items.remove(0)),
                    Value::List(_) => Err(EvalError::TypeError("head: empty list".to_string())),
                    other => Err(EvalError::TypeError(format!("head: expected List, got {other}"))),
                }
            }
            "__builtin_tail__" => {
                let xs = env.get("xs").cloned().ok_or_else(|| EvalError::Undefined(SmolStr::new("xs")))?;
                match xs {
                    Value::List(mut items) if !items.is_empty() => {
                        items.remove(0);
                        Ok(Value::List(items))
                    }
                    Value::List(_) => Err(EvalError::TypeError("tail: empty list".to_string())),
                    other => Err(EvalError::TypeError(format!("tail: expected List, got {other}"))),
                }
            }
            "__builtin_len__" => {
                let xs = env.get("xs").cloned().ok_or_else(|| EvalError::Undefined(SmolStr::new("xs")))?;
                match xs {
                    Value::List(items) => Ok(Value::Int(items.len() as i64)),
                    other => Err(EvalError::TypeError(format!("len: expected List, got {other}"))),
                }
            }
            other => Err(EvalError::Undefined(SmolStr::new(other))),
        }
    }

    fn eval_binop(&self, op: BinOp, l: Value, r: Value) -> EvalResult {
        use BinOp::*;
        match (op, l, r) {
            // Int arithmetic
            (Add, Value::Int(a),   Value::Int(b))   => Ok(Value::Int(a + b)),
            (Sub, Value::Int(a),   Value::Int(b))   => Ok(Value::Int(a - b)),
            (Mul, Value::Int(a),   Value::Int(b))   => Ok(Value::Int(a * b)),
            (Div, Value::Int(a),   Value::Int(b))   => {
                if b == 0 { Err(EvalError::DivisionByZero) } else { Ok(Value::Int(a / b)) }
            }
            (Mod, Value::Int(a),   Value::Int(b))   => {
                if b == 0 { Err(EvalError::DivisionByZero) } else { Ok(Value::Int(a % b)) }
            }
            // Float arithmetic
            (Add, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Sub, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
            (Mul, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
            (Div, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
            // Mixed int+float (promote int)
            (Add, Value::Int(a),   Value::Float(b)) => Ok(Value::Float(a as f64 + b)),
            (Add, Value::Float(a), Value::Int(b))   => Ok(Value::Float(a + b as f64)),
            (Sub, Value::Int(a),   Value::Float(b)) => Ok(Value::Float(a as f64 - b)),
            (Sub, Value::Float(a), Value::Int(b))   => Ok(Value::Float(a - b as f64)),
            (Mul, Value::Int(a),   Value::Float(b)) => Ok(Value::Float(a as f64 * b)),
            (Mul, Value::Float(a), Value::Int(b))   => Ok(Value::Float(a * b as f64)),
            (Div, Value::Int(a),   Value::Float(b)) => Ok(Value::Float(a as f64 / b)),
            (Div, Value::Float(a), Value::Int(b))   => Ok(Value::Float(a / b as f64)),
            // String concatenation
            (Add, Value::Str(a),   Value::Str(b))   => {
                Ok(Value::Str(SmolStr::new(format!("{a}{b}"))))
            }
            // Comparisons (Int)
            (Eq,  Value::Int(a),   Value::Int(b))   => Ok(Value::Bool(a == b)),
            (Neq, Value::Int(a),   Value::Int(b))   => Ok(Value::Bool(a != b)),
            (Lt,  Value::Int(a),   Value::Int(b))   => Ok(Value::Bool(a < b)),
            (Gt,  Value::Int(a),   Value::Int(b))   => Ok(Value::Bool(a > b)),
            (Leq, Value::Int(a),   Value::Int(b))   => Ok(Value::Bool(a <= b)),
            (Geq, Value::Int(a),   Value::Int(b))   => Ok(Value::Bool(a >= b)),
            // Comparisons (Float)
            (Eq,  Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a == b)),
            (Neq, Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a != b)),
            (Lt,  Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a < b)),
            (Gt,  Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a > b)),
            (Leq, Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a <= b)),
            (Geq, Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a >= b)),
            // Comparisons (String)
            (Eq,  Value::Str(a),   Value::Str(b))   => Ok(Value::Bool(a == b)),
            (Neq, Value::Str(a),   Value::Str(b))   => Ok(Value::Bool(a != b)),
            // Bool
            (Eq,  Value::Bool(a),  Value::Bool(b))  => Ok(Value::Bool(a == b)),
            (Neq, Value::Bool(a),  Value::Bool(b))  => Ok(Value::Bool(a != b)),
            (And, Value::Bool(a),  Value::Bool(b))  => Ok(Value::Bool(a && b)),
            (Or,  Value::Bool(a),  Value::Bool(b))  => Ok(Value::Bool(a || b)),
            (op, l, r) => Err(EvalError::TypeError(
                format!("operator {op:?} not applicable to {l} and {r}")
            )),
        }
    }

    pub fn global_env(&self) -> &Env {
        &self.global
    }
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}

// ── Pattern matching ──────────────────────────────────────────────────────────

/// Try to match a value against a pattern.
/// Returns Some(bindings) on success, None on failure.
fn match_pattern(pat: &Pattern, val: &Value) -> Option<Vec<(SmolStr, Value)>> {
    match (pat, val) {
        (Pattern::Wildcard, _) => Some(vec![]),

        (Pattern::Var(name), v) => Some(vec![(name.clone(), v.clone())]),

        (Pattern::Int(n),   Value::Int(m))   if n == m => Some(vec![]),
        (Pattern::Float(a), Value::Float(b)) if a == b => Some(vec![]),
        (Pattern::Str(a),   Value::Str(b))   if a == b => Some(vec![]),
        (Pattern::Bool(a),  Value::Bool(b))  if a == b => Some(vec![]),

        (Pattern::Constructor(name, pats), Value::Constructor(vname, vals)) => {
            if name != vname || pats.len() != vals.len() {
                return None;
            }
            let mut bindings = Vec::new();
            for (p, v) in pats.iter().zip(vals.iter()) {
                bindings.extend(match_pattern(p, v)?);
            }
            Some(bindings)
        }

        (Pattern::Tuple(pats), Value::Tuple(vals)) => {
            if pats.len() != vals.len() { return None; }
            let mut bindings = Vec::new();
            for (p, v) in pats.iter().zip(vals.iter()) {
                bindings.extend(match_pattern(p, v)?);
            }
            Some(bindings)
        }

        (Pattern::Record(fields), Value::Record(map)) => {
            let mut bindings = Vec::new();
            for (field, pat) in fields {
                let v = map.get(field.as_str())?;
                bindings.extend(match_pattern(pat, v)?);
            }
            Some(bindings)
        }

        _ => None,
    }
}
