# lacon — Language TODO

Things to iron out before lacon is a fully-fledged language. Roughly ordered by dependency.

## Language Design

- [ ] **String interpolation** — `"hello {name}, 2+2={2+2}"` inside double-quoted strings
- [ ] **Nested generics disambiguation** — `List<List<Int>>` currently tokenises `>>` as the compose operator; needs a fix (split token in type context, or drop `>>` in favour of a `compose` stdlib fn)
- [ ] **Type aliases** — `type UserId = Int`, useful for readability without full ADT overhead
- [ ] **Mutually recursive types** — `type Tree<a> = Leaf | Node<a, Tree<a>>`; type declarations must be allowed to reference each other
- [ ] **Variance** — decide whether generic types are covariant, contravariant, or invariant (matters once the type checker lands)
- [ ] **`Unit` value** — currently a constructor; decide if it should be a built-in keyword/value instead
- [ ] **Lambdas with typed params** — `\(x: Int) -> x + 1`; currently untyped (inferred from context)
- [ ] **Multi-binding let** — `let x = 1, y = 2 in x + y` or `let { x = 1, y = 2 } in ...`
- [ ] **Record creation and update syntax** — finalise `{ name = "dan", age = 35 }` and `{ rec | age = 36 }`
- [ ] **Record patterns** — `match user | { name, age } => ...`

## Type System

- [ ] **Hindley-Milner type checker** — infer types for all expressions; unify at call sites
- [ ] **Type error messages** — source-location–pointing errors via `ariadne` (what was expected vs. what was found)
- [ ] **Exhaustiveness checking** — warn/error when a `match` doesn't cover all constructors
- [ ] **Async type boundary enforcement** — calling `async fn` from a pure context should be a compile error
- [ ] **Polymorphic functions** — `fn identity(x: a) -> a` should work without explicit `forall`; decide whether explicit type params (`fn<a>`) are ever required
- [ ] **ADT constructor types** — `Ok :: a -> Result<a, e>` inferred from the `type` declaration; used for type-checking match arms

## Standard Library

- [ ] **`std.list`** — `map`, `filter`, `fold`, `sort`, `group`, `zip`, `head`, `tail`, `len`, `range`, `filterMap`, `flatMap`
- [ ] **`std.string`** — `split`, `join`, `trim`, `toUpper`, `toLower`, `contains`, `startsWith`, `replace`
- [ ] **`std.maybe`** — `Maybe<a>`, `fromMaybe`, `mapMaybe`
- [ ] **`std.result`** — `Result<a, e>`, `mapOk`, `mapErr`, `andThen`, `fromResult`
- [ ] **`std.json`** — `encode`, `decode` backed by `serde_json`
- [ ] **`http.client`** — `get`, `post`, `put`, `delete` returning `Async<Result<Response, String>>`
- [ ] **`std.io`** — `readFile`, `writeFile`, `readLine`, `print`, `printErr`

## Async / IO

- [ ] **`async fn` evaluation** — currently `async fn` parses but evaluates like a pure fn; needs a `tokio` executor
- [ ] **`<-` bind in async blocks** — sequencing async operations with automatic error short-circuiting
- [ ] **`Async.sequence`** — `List<Async<a>> -> Async<List<a>>` for parallel fan-out
- [ ] **`Async.parallel`** — run multiple async operations concurrently, collect results

## Runtime

- [ ] **Tail-call optimisation** — deep recursion (`factorial 1_000_000`) currently stack-overflows; needs trampolining or explicit TCO
- [ ] **Arena allocator** — replace `Rc<Value>` with `bumpalo` arenas for pure-function call trees
- [ ] **Module loader** — `use std.list` resolves to a `.lc` file on disk or an embedded stdlib module
- [ ] **Runtime error locations** — panics and `EvalError` should report the source span of the failing expression

## Modules

- [ ] **`use` declarations** — `use std.list`, `use http.client as http`, `use std.io (readFile)`
- [ ] **Module resolution** — file path → module name (`src/http/client.lc` → `http.client`)
- [ ] **Private definitions** — `_prefixed` names are not exported

## Tooling

- [ ] **`lacon check`** — type-check only, no evaluation
- [ ] **`lacon fmt`** — canonical formatter (consistent indentation, spacing)
- [ ] **REPL** — `lacon repl` for interactive evaluation
- [ ] **`--print-ast`** / **`--print-types`** debug flags
- [ ] **VSCode extension** — syntax highlighting grammar for `.lc` files
- [ ] **`lacon new`** — scaffold a new project with a `main.lc` and `Cargo`-style manifest

## Open Design Questions

- [ ] How should lacon handle **numeric tower** — automatic Int→Float promotion, or explicit `toFloat(n)`?
- [ ] Should **`match` arms** support binding with `as` — `| Just x as v => ...`?
- [ ] Should there be a **`do` block** shorthand for chaining `Result` / `Maybe` without going async?
- [ ] How are **circular / self-referential data structures** prevented in a pure language?
- [ ] What is the **FFI story** — calling Rust or C from lacon?
