//! §4.1.7 staging probe (2026-06-11) — MEASUREMENT ONLY, not wired.
//!
//! SICP 4.1.7 separates syntactic analysis from execution: analyze the
//! AST once into a tree of execution closures, then execute many times
//! without re-matching the AST per visit. Before committing to that
//! design for the host evaluator (a large slice with trampoline-TCO and
//! Rule-1 constraints), this probe measures the actual ceiling: on a
//! dispatch-dominated workload (deep Binary/If tree over two env
//! bindings, no lambdas/thunks), how much faster is a pre-analyzed
//! closure tree than the production `eval` path, using the SAME
//! `Env`/`Value` machinery on both sides?
//!
//! Everything here is `#[cfg(test)]` and `#[ignore]` — zero impact on
//! the production evaluator. Run with:
//!   cargo test -p pnix-eval --lib staging_probe -- --ignored --nocapture

#[cfg(test)]
mod tests {
  use crate::interpret;
  use crate::value::{Env, Value};
  use anyhow::{anyhow, Result};
  use pnix_core::lang::pnix::PnixExpr;
  use std::sync::Arc;

  type Compiled = Box<dyn Fn(&Env) -> Result<Value>>;

  /// Minimal 4.1.7 analyzer over the probe subset. Mirrors the
  /// production semantics for the covered forms (int arithmetic,
  /// comparisons, If on bools, Var lookup) — the probe asserts value
  /// equality against the production eval before timing.
  fn analyze(expr: &PnixExpr) -> Result<Compiled> {
    match expr {
      PnixExpr::Int(n) => {
        let n = *n;
        Ok(Box::new(move |_env| Ok(Value::Int(n))))
      }
      PnixExpr::Var(name) => {
        let name = name.clone();
        Ok(Box::new(move |env| {
          env
            .lookup(&name)?
            .ok_or_else(|| anyhow!("undefined variable: {}", name))
        }))
      }
      PnixExpr::Binary { op, lhs, rhs } => {
        let l = analyze(lhs)?;
        let r = analyze(rhs)?;
        let op: String = op.to_string();
        Ok(Box::new(move |env| {
          let lv = l(env)?;
          let rv = r(env)?;
          match (&lv, &rv) {
            (Value::Int(a), Value::Int(b)) => match op.as_str() {
              "+" => Ok(Value::Int(a + b)),
              "-" => Ok(Value::Int(a - b)),
              "*" => Ok(Value::Int(a * b)),
              "<" => Ok(Value::Bool(a < b)),
              "==" => Ok(Value::Bool(a == b)),
              other => Err(anyhow!("probe: unsupported op {}", other)),
            },
            _ => Err(anyhow!("probe: non-int operands")),
          }
        }))
      }
      PnixExpr::If { cond, then_, else_ } => {
        let c = analyze(cond)?;
        let t = analyze(then_)?;
        let e = analyze(else_)?;
        Ok(Box::new(move |env| match c(env)? {
          Value::Bool(true) => t(env),
          Value::Bool(false) => e(env),
          _ => Err(anyhow!("probe: non-bool condition")),
        }))
      }
      other => Err(anyhow!("probe: unsupported form {:?}", other)),
    }
  }

  /// Build a dispatch-heavy source: a balanced tree of Binary/If nodes
  /// over env vars `a` / `b`. depth 12 → ~2^12 leaves ≈ 12k AST nodes.
  fn build_source(depth: usize) -> String {
    if depth == 0 {
      return "(a + b)".to_string();
    }
    let sub = build_source(depth - 1);
    if depth % 3 == 0 {
      format!("(if ({sub}) < ({sub}) then ({sub}) + 1 else ({sub}) * 1)")
    } else if depth % 2 == 0 {
      format!("(({sub}) + ({sub}))")
    } else {
      format!("(({sub}) * 1 - ({sub}) * 0)")
    }
  }

  #[test]
  #[ignore = "staging probe — run explicitly with --ignored --nocapture"]
  fn staging_417_dispatch_ceiling_probe() {
    let source = build_source(11);
    let expr = interpret::parse_expr_arc_with_inline_cache(&source).expect("parse");

    let mut env = Env::new();
    env.bind("a".to_string(), Value::Int(3));
    env.bind("b".to_string(), Value::Int(4));

    // Correctness gate first: analyzed result == production result.
    let compiled = analyze(&expr).expect("analyze");
    let production = interpret::eval(expr.as_ref(), &env).expect("eval");
    let staged = compiled(&env).expect("compiled");
    assert_eq!(production.to_json(), staged.to_json(), "semantic drift");

    // Interleaved timing, median of rounds.
    const ROUNDS: usize = 7;
    const ITERS: usize = 40;
    let mut prod_times = Vec::new();
    let mut staged_times = Vec::new();
    for _ in 0..ROUNDS {
      let t0 = std::time::Instant::now();
      for _ in 0..ITERS {
        let _ = interpret::eval(expr.as_ref(), &env).unwrap();
      }
      prod_times.push(t0.elapsed());
      let t1 = std::time::Instant::now();
      for _ in 0..ITERS {
        let _ = compiled(&env).unwrap();
      }
      staged_times.push(t1.elapsed());
    }
    prod_times.sort();
    staged_times.sort();
    let p = prod_times[ROUNDS / 2];
    let s = staged_times[ROUNDS / 2];
    eprintln!(
      "staging-417 probe: production={:?} staged={:?} ratio={:.3}x (depth-11 tree, {} iters, median of {} rounds)",
      p,
      s,
      p.as_secs_f64() / s.as_secs_f64(),
      ITERS,
      ROUNDS
    );
  }

  /// The TCO constraint probe: a deep right-leaning Binary chain. The
  /// analyzed closure tree recurses NATIVELY per node — this measures
  /// how deep it can go relative to the production trampoline/guarded
  /// recursion before the Rust stack becomes the limit. Run on a big
  /// stack thread to find the closure tree's depth cost; documents the
  /// design constraint that a real staging slice must keep the
  /// trampoline (or CEK-style continuations) for the recursive forms.
  #[test]
  #[ignore = "staging probe — run explicitly with --ignored --nocapture"]
  fn staging_417_deep_chain_native_recursion_constraint() {
    let depth = 20_000usize;
    let handle = std::thread::Builder::new()
      .stack_size(512 * 1024 * 1024)
      .spawn(move || {
        let mut source = String::with_capacity(depth * 4 + 16);
        source.push_str("a");
        for _ in 0..depth {
          source.push_str(" + 1");
        }
        // Parse inside the big-stack thread — the recursive-descent
        // parser is itself depth-bound by the chain length.
        let expr = interpret::parse_expr_arc_with_inline_cache(&source).expect("parse");
        let mut env = Env::new();
        env.bind("a".to_string(), Value::Int(0));
        let compiled = analyze(&expr).expect("analyze");
        // Production may legitimately refuse via the EVAL_MAX_DEPTH
        // guard — that asymmetry IS the finding being documented.
        let production = interpret::eval(expr.as_ref(), &env);
        let staged = compiled(&env).expect("compiled deep chain");
        match production {
          Ok(p) => {
            assert_eq!(p.to_json(), staged.to_json());
            eprintln!(
              "staging-417 deep-chain: depth {} — both paths OK on 512MiB stack",
              depth
            );
          }
          Err(e) => eprintln!(
            "staging-417 deep-chain: depth {} — production refused via depth guard ({e}); staged closure tree evaluated natively to {} — a wired slice MUST re-impose the guard/trampoline boundary",
            depth,
            staged.to_json()
          ),
        }
      })
      .expect("spawn");
    handle.join().expect("deep chain thread");
  }

  /// Arc-pinned compiled-cache shape check: holding the Arc alongside
  /// the compiled form pins the allocation, so pointer-keyed reuse is
  /// ABA-safe (the deep_force visited-set lesson). This is the shape a
  /// wired slice would use to attach compiled bodies to shared ASTs
  /// without touching the pub `Value::Thunk` payload (Rule 1).
  #[test]
  #[ignore = "staging probe — run explicitly with --ignored --nocapture"]
  fn staging_417_arc_pinned_cache_shape() {
    let source = build_source(6);
    let expr: Arc<PnixExpr> = interpret::parse_expr_arc_with_inline_cache(&source).expect("parse");
    let mut cache: std::collections::HashMap<usize, (Arc<PnixExpr>, Compiled)> =
      std::collections::HashMap::default();
    let key = Arc::as_ptr(&expr) as usize;
    let compiled = analyze(&expr).expect("analyze");
    cache.insert(key, (expr.clone(), compiled));
    // Reuse hit: same Arc → same key, allocation pinned by the cache.
    let hit = cache.get(&(Arc::as_ptr(&expr) as usize)).is_some();
    assert!(hit, "pinned pointer key must hit while the Arc lives");
    eprintln!(
      "staging-417 cache shape: Arc-pinned pointer key OK ({} entries)",
      cache.len()
    );
  }
}
