//! SES-Compartment-style guest isolation for px evaluation (P10).
//!
//! A Compartment is an evaluation scope with its OWN persistent environment
//! (bindings accumulate, REPL-style) and its OWN module table (modules are
//! registered as px sources and materialize lazily as bindings on first
//! reference), while the pure intrinsics (`builtins`) are shared with every
//! other compartment through the runtime's global fallback — exactly the SES
//! shape: own globalThis + own module system, shared frozen primordials.
//!
//! No new evaluator: every evaluation goes through the one sacred runtime
//! (`px::px_eval`). Compartments are isolation BOOKKEEPING, not a second VM.

use crate::px;
use crate::specialize::px_free_vars;

pub struct Compartment {
    env: Vec<px::PxFrame>,
    /// (name, px source, materialized)
    modules: Vec<(String, String, bool)>,
    pub materialize_count: usize,
}

impl Compartment {
    pub fn new() -> Compartment {
        Compartment {
            env: Vec::new(),
            modules: Vec::new(),
            materialize_count: 0,
        }
    }

    /// Evaluate and persistently bind (REPL-style accumulation).
    pub fn define(&mut self, name: &str, px_source: &str) -> Result<(), String> {
        let value = self.eval_value(px_source)?;
        self.env.push(px::PxFrame::Bind {
            name: String::from(name),
            value,
        });
        Ok(())
    }

    /// Register a module source; it materializes lazily on first reference.
    pub fn register_module(&mut self, name: &str, px_source: &str) {
        self.modules
            .push((String::from(name), String::from(px_source), false));
    }

    fn materialize_needed(&mut self, source_ast: &px::PxExpr) -> Result<(), String> {
        let mut free = Vec::new();
        let mut bound = Vec::new();
        px_free_vars(source_ast, &mut free, &mut bound);
        // Names already bound in this compartment don't need modules.
        let unbound: Vec<String> = free
            .into_iter()
            .filter(|f| {
                !self.env.iter().any(|frame| match frame {
                    px::PxFrame::Bind { name, .. } => name == f,
                    px::PxFrame::Rec(bindings, _cache) => bindings.iter().any(|(n, _)| n == f),
                    px::PxFrame::With(_) => false,
                })
            })
            .collect();
        for want in unbound {
            let mut found: Option<(usize, String)> = None;
            for (i, (name, source, materialized)) in self.modules.iter().enumerate() {
                if *name == want && !*materialized {
                    found = Some((i, source.clone()));
                }
            }
            if let Some((i, source)) = found {
                // Modules may themselves reference registered modules.
                let value = self.eval_value(&source)?;
                self.env.push(px::PxFrame::Bind {
                    name: want.clone(),
                    value,
                });
                self.modules[i].2 = true;
                self.materialize_count += 1;
            }
        }
        Ok(())
    }

    fn eval_value(&mut self, source: &str) -> Result<px::PxVal, String> {
        let ast = px::px_parse(source)?;
        self.materialize_needed(&ast)?;
        px::px_eval(&ast, &self.env)
    }

    /// Evaluate a source in this compartment; canonical print of the value.
    pub fn eval(&mut self, source: &str) -> Result<String, String> {
        let value = self.eval_value(source)?;
        Ok(px::px_print(&value))
    }
}
