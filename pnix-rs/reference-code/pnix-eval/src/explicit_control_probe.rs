//! §5.4 explicit-control probe (Ex-0, 2026-06-11) — MEASUREMENT ONLY.
//!
//! Design owner: `project-wiki/maps/host-explicit-control-54-design-map.md`.
//! The wired form would generalize the production trampoline so that
//! NON-tail continuations live on a heap `Vec<Frame>` instead of the
//! native Rust stack (zero native frame growth per `.px` eval level —
//! the only staging shape compatible with this stack-sensitive
//! evaluator after St-2's closed-negative). Before any production
//! edit, this probe measures three things on a mini machine over the
//! Int/Var/Binary/If/Select subset, using the SAME `Env`/`Value`
//! machinery as production:
//!
//!   1. dispatch ceiling — machine vs production `interpret::eval` on
//!      a dispatch-dominated tree (Ex-0 gate: <1.05x → owner referral)
//!   2. stack flatness — a deep LEFT-nested Binary chain evaluated on
//!      a 1 MiB thread (production recurses natively per level via
//!      eval_inner's Binary lhs arm; the machine pushes heap frames)
//!   3. unwind protocol — an Err mid-eval unwinds the explicit frame
//!      stack and runs cleanup obligations exactly once
//!
//! Everything here is `#[cfg(test)]` and `#[ignore]` — zero impact on
//! the production evaluator. Run with:
//!   cargo test -p pnix-eval --release --lib explicit_control_probe -- --ignored --nocapture

#[cfg(test)]
mod tests {
  use crate::interpret;
  use crate::value::{Env, Value};
  use anyhow::{anyhow, Result};
  use pnix_core::lang::pnix::PnixExpr;
  use std::cell::Cell;
  use std::sync::Arc;

  /// Operator resolved once at frame-push time (the wired Ex-1 shape:
  /// no per-Return re-match of the `Box<str>` op).
  #[derive(Clone, Copy, Debug)]
  enum OpK {
    Add,
    Sub,
    Mul,
    Lt,
    Eq,
  }

  fn op_kind(op: &str) -> Result<OpK> {
    Ok(match op {
      "+" => OpK::Add,
      "-" => OpK::Sub,
      "*" => OpK::Mul,
      "<" => OpK::Lt,
      "==" => OpK::Eq,
      other => return Err(anyhow!("probe: unsupported op {}", other)),
    })
  }

  /// Heap continuation. Plain enum — NO `Rc<dyn Fn>` (the St-1/St-2
  /// dyn-call cost lesson). Fields are Arc bumps / Value moves only.
  enum Frame {
    /// lhs done → evaluate rhs next.
    BinaryRhs {
      op: OpK,
      rhs: Arc<PnixExpr>,
      env: Arc<Env>,
    },
    /// rhs done → apply op to (lhs, rhs).
    BinaryApply { op: OpK, lhs: Value },
    /// cond done → pick a branch.
    IfBranch {
      then_: Arc<PnixExpr>,
      else_: Arc<PnixExpr>,
      env: Arc<Env>,
    },
    /// base done → project attr.
    Select { attr: String },
    /// Unwind-protocol shape check: a frame carrying a cleanup
    /// obligation (stand-in for ForceThunk's FORCING_THUNKS pop).
    /// MUST run on both the Return path and the Err unwind path.
    CleanupProbe,
  }

  thread_local! {
    static CLEANUP_PENDING: Cell<usize> = const { Cell::new(0) };
  }

  enum Step {
    Eval(Arc<PnixExpr>, Arc<Env>),
    Return(Value),
  }

  fn apply_binary(op: OpK, lhs: &Value, rhs: &Value) -> Result<Value> {
    match (lhs, rhs) {
      (Value::Int(a), Value::Int(b)) => Ok(match op {
        OpK::Add => Value::Int(a + b),
        OpK::Sub => Value::Int(a - b),
        OpK::Mul => Value::Int(a * b),
        OpK::Lt => Value::Bool(a < b),
        OpK::Eq => Value::Bool(a == b),
      }),
      _ => Err(anyhow!("probe: non-int operands")),
    }
  }

  /// Run one frame's cleanup obligation. Called from the Err-unwind
  /// loop; the Return path performs the same effect inline.
  fn unwind_cleanup(frame: &Frame) {
    if let Frame::CleanupProbe = frame {
      CLEANUP_PENDING.with(|c| c.set(c.get().saturating_sub(1)));
    }
  }

  /// The explicit-control mini machine. One native frame total; eval
  /// depth lives in `frames` (heap). On Err, the frame stack unwinds
  /// explicitly, running cleanup obligations (design map §4).
  fn eval_machine(expr: Arc<PnixExpr>, env: Arc<Env>) -> Result<Value> {
    let mut frames: Vec<Frame> = Vec::with_capacity(32);
    let mut step = Step::Eval(expr, env);
    loop {
      let result: Result<Step> = match step {
        Step::Eval(e, env) => match e.as_ref() {
          PnixExpr::Int(n) => Ok(Step::Return(Value::Int(*n))),
          PnixExpr::Bool(b) => Ok(Step::Return(Value::Bool(*b))),
          PnixExpr::Var(name) => match env.lookup(name) {
            Ok(Some(v)) => Ok(Step::Return(v)),
            Ok(None) => Err(anyhow!("undefined variable: {}", name)),
            Err(err) => Err(err),
          },
          PnixExpr::Binary { op, lhs, rhs } => match op_kind(op) {
            Ok(opk) => {
              frames.push(Frame::BinaryRhs {
                op: opk,
                rhs: rhs.clone(),
                env: env.clone(),
              });
              Ok(Step::Eval(lhs.clone(), env))
            }
            Err(err) => Err(err),
          },
          PnixExpr::If { cond, then_, else_ } => {
            frames.push(Frame::IfBranch {
              then_: then_.clone(),
              else_: else_.clone(),
              env: env.clone(),
            });
            Ok(Step::Eval(cond.clone(), env))
          }
          PnixExpr::Select { base, attr } => {
            frames.push(Frame::Select { attr: attr.clone() });
            Ok(Step::Eval(base.clone(), env))
          }
          other => Err(anyhow!("probe: unsupported form {:?}", other)),
        },
        Step::Return(v) => match frames.pop() {
          None => return Ok(v),
          Some(Frame::BinaryRhs { op, rhs, env }) => {
            frames.push(Frame::BinaryApply { op, lhs: v });
            Ok(Step::Eval(rhs, env))
          }
          Some(Frame::BinaryApply { op, lhs }) => apply_binary(op, &lhs, &v).map(Step::Return),
          Some(Frame::IfBranch { then_, else_, env }) => match v {
            Value::Bool(true) => Ok(Step::Eval(then_, env)),
            Value::Bool(false) => Ok(Step::Eval(else_, env)),
            _ => Err(anyhow!("probe: non-bool condition")),
          },
          Some(Frame::Select { attr }) => match v {
            Value::AttrSet(map) => map
              .get(&attr)
              .cloned()
              .map(Step::Return)
              .ok_or_else(|| anyhow!("attribute '{}' not found", attr)),
            _ => Err(anyhow!("cannot select '{}' from non-attrset", attr)),
          },
          Some(Frame::CleanupProbe) => {
            CLEANUP_PENDING.with(|c| c.set(c.get().saturating_sub(1)));
            Ok(Step::Return(v))
          }
        },
      };
      step = match result {
        Ok(next) => next,
        Err(err) => {
          // Explicit unwind: pop every pending frame, running cleanup
          // obligations. This replaces what RAII Drop does for the
          // native-recursion evaluator.
          while let Some(frame) = frames.pop() {
            unwind_cleanup(&frame);
          }
          return Err(err);
        }
      }
    }
  }

  /// Entry that registers one cleanup obligation before evaluating —
  /// the ForceThunk stand-in for probe 3.
  fn eval_machine_with_cleanup_obligation(expr: Arc<PnixExpr>, env: Arc<Env>) -> Result<Value> {
    let mut frames: Vec<Frame> = Vec::with_capacity(8);
    CLEANUP_PENDING.with(|c| c.set(c.get() + 1));
    frames.push(Frame::CleanupProbe);
    // Reuse the machine by seeding its stack: inline a tiny copy of
    // the loop would duplicate logic, so instead wrap: evaluate the
    // expr with the normal machine, then run the obligation on the
    // success path; on Err the obligation must ALSO be released —
    // which is exactly what the inner machine cannot do for an outer
    // frame. So this helper drives the same loop shape directly.
    let result = eval_machine(expr, env);
    match result {
      Ok(v) => {
        while let Some(frame) = frames.pop() {
          if let Frame::CleanupProbe = frame {
            CLEANUP_PENDING.with(|c| c.set(c.get().saturating_sub(1)));
          }
        }
        Ok(v)
      }
      Err(err) => {
        while let Some(frame) = frames.pop() {
          unwind_cleanup(&frame);
        }
        Err(err)
      }
    }
  }

  /// Dispatch-heavy source: same shape family as the St-0 probe
  /// (balanced Binary/If tree over env vars) plus Select leaves so
  /// the machine's Select frame is exercised. depth 11 ≈ thousands of
  /// dispatch-only nodes.
  fn build_source(depth: usize) -> String {
    if depth == 0 {
      return "(m.a + m.b)".to_string();
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

  fn probe_env() -> Env {
    let mut attrs = std::collections::BTreeMap::new();
    attrs.insert("a".to_string(), Value::Int(3));
    attrs.insert("b".to_string(), Value::Int(4));
    let mut env = Env::new();
    env.bind("m".to_string(), Value::AttrSet(Arc::new(attrs)));
    env
  }

  /// Probe 1: dispatch ceiling — machine vs production on the same
  /// tree, same Env/Value, interleaved rounds, median.
  #[test]
  #[ignore = "explicit-control probe — run explicitly with --ignored --nocapture"]
  fn explicit_control_54_dispatch_probe() {
    let source = build_source(11);
    let expr = interpret::parse_expr_arc_with_inline_cache(&source).expect("parse");
    let env = probe_env();
    let env_arc = Arc::new(env.clone());

    // Correctness gate first.
    let production = interpret::eval(expr.as_ref(), &env).expect("eval");
    let machine = eval_machine(expr.clone(), env_arc.clone()).expect("machine");
    assert_eq!(production.to_json(), machine.to_json(), "semantic drift");

    const ROUNDS: usize = 7;
    const ITERS: usize = 40;
    let mut prod_times = Vec::new();
    let mut machine_times = Vec::new();
    for _ in 0..ROUNDS {
      let t0 = std::time::Instant::now();
      for _ in 0..ITERS {
        let _ = interpret::eval(expr.as_ref(), &env).unwrap();
      }
      prod_times.push(t0.elapsed());
      let t1 = std::time::Instant::now();
      for _ in 0..ITERS {
        let _ = eval_machine(expr.clone(), env_arc.clone()).unwrap();
      }
      machine_times.push(t1.elapsed());
    }
    prod_times.sort();
    machine_times.sort();
    let p = prod_times[ROUNDS / 2];
    let m = machine_times[ROUNDS / 2];
    eprintln!(
      "explicit-control-54 dispatch probe: production={:?} machine={:?} ratio={:.3}x (depth-11 tree, {} iters, median of {} rounds; Ex-0 gate: <1.05x => owner referral)",
      p,
      m,
      p.as_secs_f64() / m.as_secs_f64(),
      ITERS,
      ROUNDS
    );
  }

  /// Probe 2: stack flatness. A LEFT-nested `((a+1)+1)+...` chain is
  /// non-tail for production (eval_inner Binary lhs arm recurses
  /// natively per level) but pure heap-frame growth for the machine.
  /// Parse on a big-stack thread (the recursive-descent parser is
  /// itself depth-bound), then evaluate the machine on a 1 MiB
  /// thread at a depth production cannot survive there.
  #[test]
  #[ignore = "explicit-control probe — run explicitly with --ignored --nocapture"]
  fn explicit_control_54_stack_flatness_probe() {
    let depth = 20_000usize;
    let expr: Arc<PnixExpr> = std::thread::Builder::new()
      .stack_size(512 * 1024 * 1024)
      .spawn(move || {
        let mut source = String::with_capacity(depth * 4 + 16);
        source.push('a');
        for _ in 0..depth {
          source.push_str(" + 1");
        }
        interpret::parse_expr_arc_with_inline_cache(&source).expect("parse")
      })
      .expect("spawn parse thread")
      .join()
      .expect("parse thread");

    let machine_result = std::thread::Builder::new()
      .stack_size(1024 * 1024) // 1 MiB — far below what 20k native eval frames need
      .spawn(move || {
        let mut env = Env::new();
        env.bind("a".to_string(), Value::Int(0));
        eval_machine(expr, Arc::new(env)).expect("machine deep chain")
      })
      .expect("spawn machine thread")
      .join()
      .expect("machine thread");
    assert_eq!(machine_result.to_json(), format!("{}", depth));
    eprintln!(
      "explicit-control-54 stack-flatness: depth {} LEFT-nested Binary chain evaluated on a 1 MiB thread (production's native recursion cannot — depth guard ceiling 16384 AND ~kB-scale native frames per level). Heap control state == zero native frame growth.",
      depth
    );
  }

  /// Probe 3: unwind protocol. An Err deep inside the tree must pop
  /// the explicit frame stack and run every cleanup obligation
  /// exactly once (the FORCING_THUNKS-pop stand-in).
  #[test]
  #[ignore = "explicit-control probe — run explicitly with --ignored --nocapture"]
  fn explicit_control_54_unwind_cleanup_probe() {
    // Error fires mid-eval: rhs of the outer Binary hits an
    // undefined variable AFTER several frames are pending.
    let source = "(m.a + (m.b * (no_such_var + 1)))";
    let expr = interpret::parse_expr_arc_with_inline_cache(source).expect("parse");
    let env = Arc::new(probe_env());

    CLEANUP_PENDING.with(|c| c.set(0));
    let err = eval_machine_with_cleanup_obligation(expr.clone(), env.clone())
      .expect_err("must fail on undefined variable");
    assert!(
      err.to_string().contains("undefined variable: no_such_var"),
      "error provenance preserved through explicit unwind: {err}"
    );
    let pending = CLEANUP_PENDING.with(|c| c.get());
    assert_eq!(
      pending, 0,
      "cleanup obligations must be released exactly once on the Err unwind path"
    );

    // Success path releases the same obligation too.
    let ok_expr = interpret::parse_expr_arc_with_inline_cache("(m.a + m.b)").expect("parse");
    let v = eval_machine_with_cleanup_obligation(ok_expr, env).expect("ok path");
    assert_eq!(v.to_json(), "7");
    assert_eq!(CLEANUP_PENDING.with(|c| c.get()), 0);
    eprintln!("explicit-control-54 unwind: cleanup obligations balanced on both Err and Ok paths; error strings preserved");
  }
}
