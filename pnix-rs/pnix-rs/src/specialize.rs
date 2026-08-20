//! px partial evaluation (specialize) for the pnix-rs lane (P8).
//!
//! Soundness-first strategy: a sub-expression is folded ONLY when it is closed
//! over the static environment (every free variable bound to a static data
//! value); folding then just runs the sacred runtime (`px_eval`) on it — no
//! second evaluator, so folded semantics are the runtime's semantics by
//! construction. Recursive `let` follows the A4 audit rule: a `let` either
//! folds as a whole (closed), or stays intact as a dynamic residual with a
//! `let-recursive-not-static` gap — partial folds are never emitted as a
//! sequential let, because that by itself would change pnix's recursive-let
//! meaning. Honesty over specialization strength.
//!
//! The residual is always re-parseable px (checked by `specialize-check`).

use crate::gate;
use crate::px;
use crate::sha256::sha256_hex;

pub const SPECIALIZE_SCHEMA: &str = "pnix-rs.specialize.v0";

pub struct SpecializeRecord {
    pub schema: &'static str,
    pub residual: String,
    pub fully_static: Option<String>,
    pub gaps: Vec<String>,
    pub witness: gate::Witness,
}

/// Free variables of a px expression (recursive-let aware; `builtins` is a
/// global, not a free variable).
pub fn px_free_vars(e: &px::PxExpr, out: &mut Vec<String>, bound: &mut Vec<String>) {
    match e {
        px::PxExpr::DeferredError(_) => {}
        px::PxExpr::Int(_) | px::PxExpr::Float(_) | px::PxExpr::Bool(_) | px::PxExpr::Null => {}
        px::PxExpr::With { scope, body } => {
            px_free_vars(scope, out, bound);
            // Anything under `with` may bind from the dynamic scope — treat
            // body vars conservatively as free (specialization stays sound).
            px_free_vars(body, out, bound);
        }
        px::PxExpr::Isolated { with_scope, body } => {
            // body evaluates in a reset (fresh) environment, but be as
            // conservative as With above rather than trying to prove body's
            // vars don't depend on the outer scope at all.
            if let Some(ws) = with_scope {
                px_free_vars(ws, out, bound);
            }
            px_free_vars(body, out, bound);
        }
        px::PxExpr::Str(parts) => {
            for part in parts {
                if let px::PxStrPart::Sub(sub) = part {
                    px_free_vars(sub, out, bound);
                }
            }
        }
        px::PxExpr::Var(name) => {
            if name != "builtins"
                && !bound.iter().any(|b| b == name)
                && !out.iter().any(|o| o == name)
            {
                out.push(name.clone());
            }
        }
        px::PxExpr::List(items) => {
            for item in items {
                px_free_vars(item, out, bound);
            }
        }
        px::PxExpr::Select { base, .. } => px_free_vars(base, out, bound),
        px::PxExpr::Lambda { param, body } => {
            bound.push(param.clone());
            px_free_vars(body, out, bound);
            bound.pop();
        }
        px::PxExpr::Apply { func, arg } => {
            px_free_vars(func, out, bound);
            px_free_vars(arg, out, bound);
        }
        px::PxExpr::If { cond, then_e, else_e } => {
            px_free_vars(cond, out, bound);
            px_free_vars(then_e, out, bound);
            px_free_vars(else_e, out, bound);
        }
        px::PxExpr::Binary { lhs, rhs, .. } => {
            px_free_vars(lhs, out, bound);
            px_free_vars(rhs, out, bound);
        }
        px::PxExpr::LetIn { bindings, body } => {
            // Recursive scope: every binding name is bound inside every
            // binding expression AND the body.
            let mut added = 0usize;
            for (name, _v) in bindings {
                bound.push(name.clone());
                added += 1;
            }
            for (_name, value) in bindings {
                px_free_vars(value, out, bound);
            }
            px_free_vars(body, out, bound);
            while added > 0 {
                bound.pop();
                added -= 1;
            }
        }
        px::PxExpr::Attrs(fields) => {
            for (_name, value) in fields {
                px_free_vars(value, out, bound);
            }
        }
    }
}

/// Embed a static data value back as a px expression (None for opaque leaves).
fn value_to_expr(v: &px::PxVal) -> Option<px::PxExpr> {
    match v {
        // force then embed the resolved value (None if the thunk is a cycle)
        px::PxVal::Thunk(_) => px::px_force(v).ok().and_then(|f| value_to_expr(&f)),
        px::PxVal::Bytes(_) => None,
        px::PxVal::Int(n) => Some(px::PxExpr::Int(*n)),
        px::PxVal::Float(f) => Some(px::PxExpr::Float(*f)),
        px::PxVal::Bool(b) => Some(px::PxExpr::Bool(*b)),
        px::PxVal::Null => Some(px::PxExpr::Null),
        px::PxVal::Str(s) => Some(px::PxExpr::Str(vec![px::PxStrPart::Lit(s.clone())])),
        px::PxVal::List(items) => {
            let mut out = Vec::new();
            for item in items.iter() {
                out.push(value_to_expr(item)?);
            }
            Some(px::PxExpr::List(out))
        }
        px::PxVal::Attrs(fields) => {
            let mut out = Vec::new();
            for (name, value) in fields.iter() {
                out.push((name.clone(), value_to_expr(value)?));
            }
            Some(px::PxExpr::Attrs(out))
        }
        px::PxVal::Closure { .. } | px::PxVal::Builtin { .. } => None,
        // Round-trips through the same `:path:`-marked Var the parser
        // already produces for a literal; re-evaluating it resolves back to
        // an equal PxVal::Path (px_normalize_path is idempotent).
        px::PxVal::Path(p) => Some(px::PxExpr::Var(format!(":path:{}", p))),
    }
}

fn closed_over(e: &px::PxExpr, senv: &[(String, px::PxVal)]) -> bool {
    let mut free = Vec::new();
    let mut bound = Vec::new();
    px_free_vars(e, &mut free, &mut bound);
    free.iter().all(|f| senv.iter().any(|(n, _)| n == f))
}

fn static_env_frames(senv: &[(String, px::PxVal)]) -> Vec<px::PxFrame> {
    let mut env = Vec::new();
    for (name, value) in senv {
        env.push(px::PxFrame::Bind {
            name: name.clone(),
            value: value.clone(),
        });
    }
    env
}

/// Specialize one expression. Folds closed sub-expressions via the sacred
/// runtime; recurses otherwise. Returns the residual expression.
fn spec(e: &px::PxExpr, senv: &[(String, px::PxVal)], gaps: &mut Vec<String>) -> px::PxExpr {
    if closed_over(e, senv) {
        let env = static_env_frames(senv);
        match px::px_eval(e, &env) {
            Ok(v) => match value_to_expr(&v) {
                Some(folded) => return folded,
                // Opaque result (e.g. a lambda): sound to keep the original
                // expression; not a gap, just not embeddable.
                None => return e.clone(),
            },
            Err(err) => {
                // Evaluation errors at specialization time stay residual so
                // runtime semantics (incl. error timing) are preserved.
                gaps.push(format!("closed-eval-error: {}", err));
                return e.clone();
            }
        }
    }
    match e {
        px::PxExpr::LetIn { bindings, body } => {
            // A4: not closed => the let does NOT partially fold. Binding
            // expressions and the body are specialized with the let names
            // masked out of the outer static env (rule 2: names of this frame
            // must never resolve to outer statics).
            let masked: Vec<(String, px::PxVal)> = senv
                .iter()
                .filter(|(n, _)| !bindings.iter().any(|(bn, _)| bn == n))
                .cloned()
                .collect();
            let mut free = Vec::new();
            let mut bound = Vec::new();
            px_free_vars(e, &mut free, &mut bound);
            let dynamic: Vec<String> = free
                .iter()
                .filter(|f| !senv.iter().any(|(n, _)| &n == f))
                .cloned()
                .collect();
            gaps.push(format!(
                "let-recursive-not-static: dynamic free [{}]",
                dynamic.join(" ")
            ));
            let mut new_bindings = Vec::new();
            for (name, value) in bindings {
                new_bindings.push((name.clone(), spec(value, &masked, gaps)));
            }
            px::PxExpr::LetIn {
                bindings: new_bindings,
                body: Box::new(spec(body, &masked, gaps)),
            }
        }
        px::PxExpr::Lambda { param, body } => {
            let masked: Vec<(String, px::PxVal)> = senv
                .iter()
                .filter(|(n, _)| n != param)
                .cloned()
                .collect();
            px::PxExpr::Lambda {
                param: param.clone(),
                body: std::rc::Rc::new(spec(body, &masked, gaps)),
            }
        }
        px::PxExpr::Str(parts) => {
            let mut out = Vec::new();
            for part in parts {
                match part {
                    px::PxStrPart::Lit(s) => out.push(px::PxStrPart::Lit(s.clone())),
                    px::PxStrPart::Sub(sub) => {
                        out.push(px::PxStrPart::Sub(spec(sub, senv, gaps)))
                    }
                }
            }
            px::PxExpr::Str(out)
        }
        px::PxExpr::List(items) => {
            px::PxExpr::List(items.iter().map(|i| spec(i, senv, gaps)).collect())
        }
        px::PxExpr::Select { base, name } => px::PxExpr::Select {
            base: Box::new(spec(base, senv, gaps)),
            name: name.clone(),
        },
        px::PxExpr::Apply { func, arg } => px::PxExpr::Apply {
            func: Box::new(spec(func, senv, gaps)),
            arg: Box::new(spec(arg, senv, gaps)),
        },
        px::PxExpr::If { cond, then_e, else_e } => px::PxExpr::If {
            cond: Box::new(spec(cond, senv, gaps)),
            then_e: Box::new(spec(then_e, senv, gaps)),
            else_e: Box::new(spec(else_e, senv, gaps)),
        },
        px::PxExpr::Binary { op, lhs, rhs } => px::PxExpr::Binary {
            op: op.clone(),
            lhs: Box::new(spec(lhs, senv, gaps)),
            rhs: Box::new(spec(rhs, senv, gaps)),
        },
        px::PxExpr::Attrs(fields) => px::PxExpr::Attrs(
            fields
                .iter()
                .map(|(n, v)| (n.clone(), spec(v, senv, gaps)))
                .collect(),
        ),
        other => other.clone(),
    }
}

pub fn specialize(
    source: &str,
    static_bindings: &[(String, px::PxVal)],
) -> Result<SpecializeRecord, String> {
    let ast = px::px_parse(source)?;
    let mut gaps = Vec::new();
    let residual_ast = spec(&ast, static_bindings, &mut gaps);
    let residual = px::px_emit(&residual_ast);

    let fully_static = match &residual_ast {
        px::PxExpr::Int(_)
        | px::PxExpr::Bool(_)
        | px::PxExpr::Str(_)
        | px::PxExpr::List(_)
        | px::PxExpr::Attrs(_) => {
            let env = Vec::new();
            match px::px_eval(&residual_ast, &env) {
                Ok(v) => Some(px::px_print(&v)),
                Err(_) => None,
            }
        }
        _ => None,
    };

    let loss_status = if gaps.is_empty() { "lossless" } else { "held" };
    let mut sorted_gaps = gaps.clone();
    sorted_gaps.sort();
    let witness = gate::Witness {
        direction: String::from("specialize"),
        source_lang: String::from("px"),
        target_lang: String::from("px"),
        input_kind: String::from("source"),
        output_kind: String::from("residual-source"),
        loss_status: String::from(loss_status),
        effect_class: String::from("pure"),
        capability_required: String::from("-"),
        in_hash: sha256_hex(source.as_bytes()),
        out_hash: sha256_hex(residual.as_bytes()),
        env_hash: sha256_hex(format!("gaps={}", sorted_gaps.join(";")).as_bytes()),
        status: String::from("ok"),
        loss: format!("{} gaps", gaps.len()),
    };
    Ok(SpecializeRecord {
        schema: SPECIALIZE_SCHEMA,
        residual,
        fully_static,
        gaps,
        witness,
    })
}

pub fn render(r: &SpecializeRecord) -> String {
    let mut out = String::new();
    out.push_str(&format!("schema {}\n", r.schema));
    out.push_str(&format!("residual {}\n", r.residual));
    match &r.fully_static {
        Some(v) => out.push_str(&format!("fully_static {}\n", v)),
        None => out.push_str("fully_static -\n"),
    }
    out.push_str(&format!("gaps [{}]\n", r.gaps.join("; ")));
    out.push_str(&gate::render_witness(&r.witness));
    out
}
