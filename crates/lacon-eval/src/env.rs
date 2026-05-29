use smol_str::SmolStr;
use std::collections::HashMap;
use std::rc::Rc;

use crate::value::Value;

/// Persistent (copy-on-write) environment mapping names to values.
/// Clone is O(1) — the underlying map is reference-counted.
#[derive(Clone, Default)]
pub struct Env(Rc<HashMap<SmolStr, Value>>);

impl Env {
    pub fn new() -> Self {
        Self(Rc::new(HashMap::new()))
    }

    /// Look up a name.
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.0.get(name)
    }

    /// Extend with a new binding, returning a new Env.
    pub fn extend(&self, name: SmolStr, value: Value) -> Self {
        let mut map = (*self.0).clone();
        map.insert(name, value);
        Self(Rc::new(map))
    }

    /// Extend with multiple bindings at once.
    pub fn extend_many(&self, bindings: impl IntoIterator<Item = (SmolStr, Value)>) -> Self {
        let mut map = (*self.0).clone();
        for (k, v) in bindings {
            map.insert(k, v);
        }
        Self(Rc::new(map))
    }
}
