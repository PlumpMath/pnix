//! Regression cover for import-cycle detection.
//!
//! Without this, `a.px` importing `b.px` importing `a.px` recurses
//! through `eval_file_at_path` until the Rust call stack overflows
//! — even with a 32 MB test thread. The fix adds a thread-local
//! `IMPORT_FILE_STACK` plus an RAII `ImportFileGuard` whose
//! `push_checked` returns an `import cycle` error when the same
//! canonical path is already on the stack.

use pnix_eval::eval_file;
use std::path::Path;

fn cycle_dir() -> &'static str {
  // Build the import cycle fixtures the first time this test runs.
  static ONCE: std::sync::Once = std::sync::Once::new();
  ONCE.call_once(|| {
    let dir = "/tmp/pnix-import-cycle-fixtures";
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(
      format!("{dir}/a.px"),
      "let b = import ./b.px; in { from-a = 1; via-b = b.from-b; }",
    )
    .unwrap();
    std::fs::write(
      format!("{dir}/b.px"),
      "let a = import ./a.px; in { from-b = 2; via-a = a.from-a; }",
    )
    .unwrap();
    std::fs::write(format!("{dir}/run.px"), "import ./a.px").unwrap();
    std::fs::write(format!("{dir}/self.px"), "import ./self.px").unwrap();
    std::fs::write(format!("{dir}/leaf.px"), "{ x = 42; }").unwrap();
    std::fs::write(format!("{dir}/use-leaf.px"), "(import ./leaf.px).x").unwrap();
  });
  "/tmp/pnix-import-cycle-fixtures"
}

#[test]
fn two_file_cycle_returns_error_not_overflow() {
  let dir = cycle_dir();
  let r = eval_file(&Path::new(dir).join("run.px"));
  let err = r.expect_err("expected cycle error");
  let msg = format!("{err}");
  assert!(
    msg.contains("import cycle"),
    "expected `import cycle` in error, got: {msg}"
  );
  // Error message should include the chain so a developer can see
  // which file re-entered the cycle.
  assert!(msg.contains("a.px"));
  assert!(msg.contains("b.px"));
}

#[test]
fn self_import_is_cycle() {
  let dir = cycle_dir();
  let r = eval_file(&Path::new(dir).join("self.px"));
  let err = r.expect_err("self-import should be a cycle");
  assert!(format!("{err}").contains("import cycle"));
}

#[test]
fn unrelated_imports_still_work() {
  // The cycle guard pops on success, so independent imports keep
  // working without false-positives.
  let dir = cycle_dir();
  let v = eval_file(&Path::new(dir).join("use-leaf.px")).unwrap();
  assert_eq!(v.to_json(), "42");
}

#[test]
fn diamond_import_is_not_a_cycle() {
  // a depends on c, b depends on c, root depends on both.
  // c is imported twice but those imports are sequential, not
  // re-entrant, so the guard pops between them and the import
  // succeeds.
  let dir = cycle_dir();
  std::fs::write(format!("{dir}/diamond-c.px"), "{ leaf = 7; }").unwrap();
  std::fs::write(
    format!("{dir}/diamond-a.px"),
    "(import ./diamond-c.px).leaf + 1",
  )
  .unwrap();
  std::fs::write(
    format!("{dir}/diamond-b.px"),
    "(import ./diamond-c.px).leaf + 2",
  )
  .unwrap();
  std::fs::write(
    format!("{dir}/diamond-root.px"),
    "(import ./diamond-a.px) + (import ./diamond-b.px)",
  )
  .unwrap();
  let v = eval_file(&Path::new(dir).join("diamond-root.px")).unwrap();
  // (7 + 1) + (7 + 2) = 17
  assert_eq!(v.to_json(), "17");
}
