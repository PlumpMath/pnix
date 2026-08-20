//! Recursion depth guard: deep `eval()` recursion that would otherwise
//! overflow the Rust stack and abort the process must surface as a
//! catchable `Err` instead. See pnix-eval interpret::enter_eval.

use pnix_eval::eval_expr;

/// Helper: run `src` in a thread with a 64 MiB stack (matches the
/// pnixc-meta worker default) and return Ok(json) or Err(message).
fn run_with_stack(src: &str) -> Result<String, String> {
  let src = src.to_string();
  let h = std::thread::Builder::new()
    .stack_size(64 * 1024 * 1024)
    .spawn(move || match eval_expr(&src) {
      Ok(v) => Ok(v.to_json()),
      Err(e) => Err(format!("{e}")),
    })
    .map_err(|e| format!("spawn: {e}"))?;
  match h.join() {
    Ok(r) => r,
    Err(_) => Err("thread panicked / aborted".into()),
  }
}

/// Non-tail recursion (`1 + go (n - 1)`) on a depth larger than the
/// default 16,384 cap must return an Err instead of aborting.
#[test]
fn deep_non_tail_recursion_surfaces_as_err_not_abort() {
  // `go 50000` -- previously aborted the process on 64 MiB stack.
  // Now must return Err mentioning depth exceeded.
  let src = "let go = n: if n == 0 then 0 else 1 + go (n - 1); in go 50000";
  match run_with_stack(src) {
    Ok(out) => panic!("expected depth-exceeded Err, got Ok({})", out),
    Err(e) => assert!(
      e.contains("recursion depth exceeded"),
      "expected 'recursion depth exceeded' in error, got: {e}"
    ),
  }
}

/// Modest non-tail recursion (depth well under the cap) must still
/// evaluate normally -- the guard only fires when the cap is exceeded.
#[test]
fn modest_non_tail_recursion_still_evaluates() {
  // sum 1..100 = 5050 -- well within budget.
  let src = "let go = n: if n == 0 then 0 else n + go (n - 1); in go 100";
  let out = run_with_stack(src).expect("should evaluate");
  assert_eq!(out, "5050");
}

/// Tail recursion (no Rust-stack growth thanks to the trampoline) is
/// NOT affected by the cap. `f N` where N >> cap must succeed.
#[test]
fn tail_recursion_unaffected_by_depth_guard() {
  // Trampolined: Apply(Lambda) rewrites cur_expr without adding an
  // eval frame, so this completes regardless of cap.
  let src = "let f = n: if n == 50000 then n else f (n + 1); in f 0";
  let out = run_with_stack(src).expect("should evaluate");
  assert_eq!(out, "50000");
}

/// PNIX_EVAL_MAX_DEPTH env override changes the cap. We run this in
/// the same test process as the others, but EVAL_MAX_DEPTH is a
/// process-level OnceLock so we can only meaningfully assert the
/// default-cap behavior here (overrides are tested manually).
#[test]
fn depth_guard_error_mentions_override_env_var() {
  let src = "let go = n: if n == 0 then 0 else 1 + go (n - 1); in go 30000";
  match run_with_stack(src) {
    Ok(out) => panic!("expected depth-exceeded Err, got Ok({})", out),
    Err(e) => assert!(
      e.contains("PNIX_EVAL_MAX_DEPTH"),
      "error should mention env override, got: {e}"
    ),
  }
}
