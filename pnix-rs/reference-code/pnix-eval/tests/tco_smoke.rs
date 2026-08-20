//! Tail-call optimization smoke tests at increasing depths so we can
//! locate the threshold where the trampoline still blows the stack.

use pnix_eval::eval_expr;

fn run(n: i64) -> Result<String, String> {
  let src = format!("let f = n: if n == {n} then n else f (n + 1); in f 0");
  let h = std::thread::Builder::new()
    .stack_size(32 * 1024 * 1024)
    .spawn(move || match eval_expr(&src) {
      Ok(v) => Ok(v.to_json()),
      Err(e) => Err(format!("{e}")),
    })
    .map_err(|e| format!("spawn: {e}"))?;
  match h.join() {
    Ok(r) => r,
    Err(_) => Err("overflow".into()),
  }
}

#[test]
fn tco_100() {
  assert_eq!(run(100).unwrap(), "100");
}

#[test]
fn tco_1000() {
  assert_eq!(run(1000).unwrap(), "1000");
}

#[test]
fn tco_10000() {
  assert_eq!(run(10000).unwrap(), "10000");
}

#[test]
fn tco_100000() {
  assert_eq!(run(100000).unwrap(), "100000");
}
