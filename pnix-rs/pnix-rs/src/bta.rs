//! Offline binding-time analysis (tower m8, analysis facet).
//!
//! The pnix-hy tower uses BTA to PREDICT the interpretive collapse rather than
//! to drive the online specializer (Jones 1985: the static/dynamic division is
//! the prerequisite naive unfolding lacks). This lane does the same: a
//! monovariant, memo-free structural classification of a px expression against
//! a set of DYNAMIC variable names, conservative (unknown constructs are
//! Dynamic). Its value is the CROSS-CHECK — BTA's prediction "this if-condition
//! is Static" must agree with what the actual specializer (mix.px) does (folds
//! the if). Agreement is gated; the 3rd Futamura projection stays held (its
//! self-application polyvariance is semantic, not something offline BTA
//! bounds — that needs generalization points, a research horizon).

use crate::px;

#[derive(Clone, Copy, PartialEq)]
pub enum Bt {
    Static,
    Dynamic,
}

fn join(a: Bt, b: Bt) -> Bt {
    if a == Bt::Dynamic || b == Bt::Dynamic {
        Bt::Dynamic
    } else {
        Bt::Static
    }
}

pub struct BtaResult {
    pub whole: Bt,
    /// Binding-time of every `if` condition, in pre-order (for cross-check).
    pub if_conds: Vec<Bt>,
    pub static_count: usize,
    pub dynamic_count: usize,
}

struct Analyzer {
    if_conds: Vec<Bt>,
    static_count: usize,
    dynamic_count: usize,
}

impl Analyzer {
    fn tally(&mut self, bt: Bt) -> Bt {
        match bt {
            Bt::Static => self.static_count += 1,
            Bt::Dynamic => self.dynamic_count += 1,
        }
        bt
    }

    fn bt(&mut self, e: &px::PxExpr, env: &Vec<(String, Bt)>) -> Bt {
        let raw = self.bt_inner(e, env);
        self.tally(raw)
    }

    fn bt_inner(&mut self, e: &px::PxExpr, env: &Vec<(String, Bt)>) -> Bt {
        match e {
            px::PxExpr::DeferredError(_) => Bt::Static,
            px::PxExpr::Int(_) | px::PxExpr::Float(_) | px::PxExpr::Bool(_) | px::PxExpr::Null => Bt::Static,
            px::PxExpr::With { scope, body } => join(self.bt(scope, env), self.bt(body, env)),
            px::PxExpr::Str(parts) => {
                let mut acc = Bt::Static;
                for p in parts {
                    if let px::PxStrPart::Sub(sub) = p {
                        acc = join(acc, self.bt(sub, env));
                    }
                }
                acc
            }
            px::PxExpr::Var(name) => {
                if name == "builtins" {
                    return Bt::Static;
                }
                for (n, bt) in env.iter().rev() {
                    if n == name {
                        return *bt;
                    }
                }
                // Free / declared-dynamic variables are Dynamic (conservative).
                Bt::Dynamic
            }
            px::PxExpr::List(items) => {
                let mut acc = Bt::Static;
                for it in items {
                    acc = join(acc, self.bt(it, env));
                }
                acc
            }
            px::PxExpr::Select { base, .. } => self.bt(base, env),
            px::PxExpr::Lambda { body, param } => {
                // A lambda value is Static (closure); analyze the body with the
                // param Dynamic (its argument binding-time is unknown offline —
                // conservative, matches monovariant BTA).
                let mut env2 = env.clone();
                env2.push((param.clone(), Bt::Dynamic));
                let _ = self.bt(body, &env2);
                Bt::Static
            }
            px::PxExpr::Apply { func, arg } => {
                let f = self.bt(func, env);
                let a = self.bt(arg, env);
                join(f, a)
            }
            px::PxExpr::If { cond, then_e, else_e } => {
                let c = self.bt(cond, env);
                self.if_conds.push(c);
                let t = self.bt(then_e, env);
                let el = self.bt(else_e, env);
                // Static cond => the if collapses to one branch; result bt is
                // the join of the branches (conservative without knowing which).
                join(c, join(t, el))
            }
            px::PxExpr::Binary { lhs, rhs, .. } => {
                let l = self.bt(lhs, env);
                let r = self.bt(rhs, env);
                join(l, r)
            }
            px::PxExpr::LetIn { bindings, body } => {
                // Recursive scope: every binding name visible in every binding
                // and the body. Bindings analyzed at their own binding-time
                // (lambda => Static value; else the value's bt).
                let mut env2 = env.clone();
                for (n, _) in bindings {
                    env2.push((n.clone(), Bt::Dynamic));
                }
                let mut env3 = env.clone();
                for (n, v) in bindings {
                    let vb = self.bt(v, &env2);
                    env3.push((n.clone(), vb));
                }
                self.bt(body, &env3)
            }
            px::PxExpr::Attrs(fields) => {
                let mut acc = Bt::Static;
                for (_n, v) in fields {
                    acc = join(acc, self.bt(v, env));
                }
                acc
            }
        }
    }
}

/// Classify a px source against a set of DYNAMIC top-level variable names.
pub fn analyze(source: &str, dynamic_vars: &[String]) -> Result<BtaResult, String> {
    let ast = px::px_parse(source)?;
    let mut env: Vec<(String, Bt)> = Vec::new();
    for name in dynamic_vars {
        env.push((name.clone(), Bt::Dynamic));
    }
    let mut a = Analyzer {
        if_conds: Vec::new(),
        static_count: 0,
        dynamic_count: 0,
    };
    let whole = a.bt(&ast, &env);
    Ok(BtaResult {
        whole,
        if_conds: a.if_conds,
        static_count: a.static_count,
        dynamic_count: a.dynamic_count,
    })
}
