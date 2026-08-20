use pnix_eval::eval_expr;
#[test]
fn check() {
  println!("1: {:?}", eval_expr(r#"({a={b=1;};}.x or "fallback")"#));
  println!("2: {:?}", eval_expr(r#"let bs = {b=1;}; in bs.bar or "x""#));
  println!(
    "3: {:?}",
    eval_expr(r#"({a={b=1;};}.x or {a={b=1;};}.a.b)"#)
  );
  println!(
    "4: {:?}",
    eval_expr(r#"({a={b=1;};}.x or {b=2;}.bar or "xyzzy")"#)
  );
  println!("5: {:?}", eval_expr(r#"(123).bla or null.foo or "xyzzy""#));
}
