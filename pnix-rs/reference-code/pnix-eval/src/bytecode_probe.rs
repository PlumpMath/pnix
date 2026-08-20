//! §5.5 bytecode probe (B-0, 2026-06-11) — MEASUREMENT ONLY.
//!
//! Design owner: `project-wiki/maps/host-bytecode-55-design-map.md`.
//! Measures the instruction-dispatch ceiling: the same dispatch-heavy
//! tree the St-0/Ex-0 probes used, hand-compiled ONCE into a flat
//! postfix instruction sequence executed by a pc-loop stack machine
//! (no per-visit AST match, no per-node frame push, no native
//! recursion — and deliberately no depth-guard charges, like the
//! earlier probes: this is the ceiling, the wired form pays guard
//! accounting). NOT native/JIT — instructions are plain enum data.
//!
//! Run with:
//!   cargo test -p pnix-eval --release --lib bytecode_probe -- --ignored --nocapture

#[cfg(test)]
mod tests {
  use crate::interpret;
  use crate::value::{Env, Value};
  use anyhow::{anyhow, Result};
  use pnix_core::lang::pnix::PnixExpr;
  use std::sync::Arc;

  #[derive(Clone, Copy, Debug)]
  enum OpK {
    Add,
    Sub,
    Mul,
    Lt,
    Eq,
  }

  #[derive(Debug)]
  enum Instr {
    /// Push a literal int.
    PushInt(i64),
    /// Look the name up in the env and push (forced binding — the
    /// probe env holds plain values).
    LoadVar(String),
    /// Pop attrset, push its `attr` field.
    SelectAttr(String),
    /// Pop rhs, pop lhs, push op result.
    Bin(OpK),
    /// Pop a bool; if false jump to target pc.
    JumpIfFalse(usize),
    /// Unconditional jump.
    Jump(usize),
  }

  /// Hand compiler over the probe subset — postfix code with
  /// branch targets resolved in one pass.
  fn compile(expr: &PnixExpr, code: &mut Vec<Instr>) -> Result<()> {
    match expr {
      PnixExpr::Int(n) => {
        code.push(Instr::PushInt(*n));
        Ok(())
      }
      PnixExpr::Var(name) => {
        code.push(Instr::LoadVar(name.clone()));
        Ok(())
      }
      PnixExpr::Select { base, attr } => {
        compile(base, code)?;
        code.push(Instr::SelectAttr(attr.clone()));
        Ok(())
      }
      PnixExpr::Binary { op, lhs, rhs } => {
        let opk = match op.as_ref() {
          "+" => OpK::Add,
          "-" => OpK::Sub,
          "*" => OpK::Mul,
          "<" => OpK::Lt,
          "==" => OpK::Eq,
          other => return Err(anyhow!("probe: unsupported op {}", other)),
        };
        compile(lhs, code)?;
        compile(rhs, code)?;
        code.push(Instr::Bin(opk));
        Ok(())
      }
      PnixExpr::If { cond, then_, else_ } => {
        compile(cond, code)?;
        let jif_at = code.len();
        code.push(Instr::JumpIfFalse(usize::MAX));
        compile(then_, code)?;
        let jmp_at = code.len();
        code.push(Instr::Jump(usize::MAX));
        let else_pc = code.len();
        compile(else_, code)?;
        let end_pc = code.len();
        let Instr::JumpIfFalse(t) = &mut code[jif_at] else {
          unreachable!()
        };
        *t = else_pc;
        let Instr::Jump(t) = &mut code[jmp_at] else {
          unreachable!()
        };
        *t = end_pc;
        Ok(())
      }
      other => Err(anyhow!("probe: unsupported form {:?}", other)),
    }
  }

  /// pc-loop stack machine. One native frame; operand stack on the
  /// heap. No guards (ceiling measurement).
  fn execute(code: &[Instr], env: &Env) -> Result<Value> {
    let mut stack: Vec<Value> = Vec::with_capacity(16);
    let mut pc = 0usize;
    while pc < code.len() {
      match &code[pc] {
        Instr::PushInt(n) => stack.push(Value::Int(*n)),
        Instr::LoadVar(name) => {
          let v = env
            .lookup(name)?
            .ok_or_else(|| anyhow!("undefined variable: {}", name))?;
          stack.push(v);
        }
        Instr::SelectAttr(attr) => {
          let base = stack.pop().expect("stack underflow");
          match base {
            Value::AttrSet(map) => stack.push(
              map
                .get(attr)
                .cloned()
                .ok_or_else(|| anyhow!("attribute '{}' not found", attr))?,
            ),
            _ => return Err(anyhow!("cannot select '{}' from non-attrset", attr)),
          }
        }
        Instr::Bin(opk) => {
          let r = stack.pop().expect("stack underflow");
          let l = stack.pop().expect("stack underflow");
          let out = match (&l, &r) {
            (Value::Int(a), Value::Int(b)) => match opk {
              OpK::Add => Value::Int(a + b),
              OpK::Sub => Value::Int(a - b),
              OpK::Mul => Value::Int(a * b),
              OpK::Lt => Value::Bool(a < b),
              OpK::Eq => Value::Bool(a == b),
            },
            _ => return Err(anyhow!("probe: non-int operands")),
          };
          stack.push(out);
        }
        Instr::JumpIfFalse(target) => {
          let c = stack.pop().expect("stack underflow");
          match c {
            Value::Bool(true) => {}
            Value::Bool(false) => {
              pc = *target;
              continue;
            }
            _ => return Err(anyhow!("probe: non-bool condition")),
          }
        }
        Instr::Jump(target) => {
          pc = *target;
          continue;
        }
      }
      pc += 1;
    }
    Ok(stack.pop().expect("empty result stack"))
  }

  /// Same dispatch-heavy source family as the St-0/Ex-0 probes.
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

  #[test]
  #[ignore = "bytecode probe — run explicitly with --ignored --nocapture"]
  fn bytecode_55_instruction_dispatch_ceiling_probe() {
    let source = build_source(11);
    let expr = interpret::parse_expr_arc_with_inline_cache(&source).expect("parse");
    let env = probe_env();

    let mut code = Vec::new();
    compile(&expr, &mut code).expect("compile");
    eprintln!("bytecode-55 probe: {} instructions", code.len());

    // Correctness gate first.
    let production = interpret::eval(expr.as_ref(), &env).expect("eval");
    let bytecode = execute(&code, &env).expect("execute");
    assert_eq!(production.to_json(), bytecode.to_json(), "semantic drift");

    const ROUNDS: usize = 7;
    const ITERS: usize = 40;
    let mut prod_times = Vec::new();
    let mut bc_times = Vec::new();
    for _ in 0..ROUNDS {
      let t0 = std::time::Instant::now();
      for _ in 0..ITERS {
        let _ = interpret::eval(expr.as_ref(), &env).unwrap();
      }
      prod_times.push(t0.elapsed());
      let t1 = std::time::Instant::now();
      for _ in 0..ITERS {
        let _ = execute(&code, &env).unwrap();
      }
      bc_times.push(t1.elapsed());
    }
    prod_times.sort();
    bc_times.sort();
    let p = prod_times[ROUNDS / 2];
    let b = bc_times[ROUNDS / 2];
    eprintln!(
      "bytecode-55 probe: production={:?} bytecode={:?} ratio={:.3}x (depth-11 tree, {} iters, median of {} rounds; B-0 gate: <1.2x => owner referral)",
      p,
      b,
      p.as_secs_f64() / b.as_secs_f64(),
      ITERS,
      ROUNDS
    );
  }
}
