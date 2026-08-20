//! UnifiedExpr → FxCoreExpr 변환 (Clojure용)
//!
//! pnix-old의 lang_clojure/lower.rs에서 마이그레이션.
//!
//! lang_pnix의 lower 모듈을 재사용
//!
//! ## 헌법 준수 (P0-1)
//!
//! 순수 구조 변환만, 값 계산 없음

use crate::fx::core_expr::FxCoreExpr;
use crate::lang::clojure_error::ClojureError;
use crate::lang::pnix::lower_to_fx_core as pnix_lower;
use crate::lang::pnix::UnifiedExpr;

/// UnifiedExpr를 FxCoreExpr로 변환 (Clojure 전용)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn lower_clj_to_fx_core(expr: &UnifiedExpr) -> Result<FxCoreExpr, ClojureError> {
  // lang_pnix의 lowering 로직 재사용
  pnix_lower(expr).map_err(|e| ClojureError::Lowering(e.to_string()))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::fx::meaning_op::MeaningOpId;
  use crate::lang::clojure::parse::parse_clj_expr;

  fn flatten_apply_chain(expr: &FxCoreExpr) -> Option<(String, usize)> {
    match expr {
      FxCoreExpr::Var(func) => Some((func.clone(), 0)),
      FxCoreExpr::Derived { meta, args } if meta.op == MeaningOpId::Apply && args.len() == 2 => {
        let (func, argc) = flatten_apply_chain(&args[0])?;
        Some((func, argc + 1))
      }
      _ => None,
    }
  }

  fn contains_var_name(expr: &FxCoreExpr, name: &str) -> bool {
    match expr {
      FxCoreExpr::Var(v) => v == name,
      FxCoreExpr::Unary { arg, .. } => contains_var_name(arg, name),
      FxCoreExpr::Binary { lhs, rhs, .. } => {
        contains_var_name(lhs, name) || contains_var_name(rhs, name)
      }
      FxCoreExpr::Derived { args, .. } | FxCoreExpr::Construct { args, .. } => {
        args.iter().any(|arg| contains_var_name(arg, name))
      }
      FxCoreExpr::If { cond, then_, else_ } => {
        contains_var_name(cond, name)
          || contains_var_name(then_, name)
          || contains_var_name(else_, name)
      }
      FxCoreExpr::Let { value, body, .. } => {
        contains_var_name(value, name) || contains_var_name(body, name)
      }
      FxCoreExpr::Lambda { body, .. } => contains_var_name(body, name),
      FxCoreExpr::Select { expr, .. } => contains_var_name(expr, name),
      FxCoreExpr::List(items) => items.iter().any(|item| contains_var_name(item, name)),
      FxCoreExpr::AttrSet(pairs) => pairs
        .iter()
        .any(|(_, value)| contains_var_name(value, name)),
      FxCoreExpr::Interop { .. }
      | FxCoreExpr::ConstInt(_)
      | FxCoreExpr::ConstFloat(_)
      | FxCoreExpr::ConstBool(_)
      | FxCoreExpr::ConstString(_)
      | FxCoreExpr::ParamSysTime
      | FxCoreExpr::ParamDeltaTime
      | FxCoreExpr::SignalVar(_)
      | FxCoreExpr::Throw { .. } => false,
    }
  }

  fn contains_meaning_op(expr: &FxCoreExpr, op: MeaningOpId) -> bool {
    match expr {
      FxCoreExpr::Unary { meta, arg } => meta.op == op || contains_meaning_op(arg, op),
      FxCoreExpr::Binary { meta, lhs, rhs } => {
        meta.op == op || contains_meaning_op(lhs, op) || contains_meaning_op(rhs, op)
      }
      FxCoreExpr::Derived { meta, args } => {
        meta.op == op || args.iter().any(|arg| contains_meaning_op(arg, op))
      }
      FxCoreExpr::If { cond, then_, else_ } => {
        contains_meaning_op(cond, op)
          || contains_meaning_op(then_, op)
          || contains_meaning_op(else_, op)
      }
      FxCoreExpr::Let { value, body, .. } => {
        contains_meaning_op(value, op) || contains_meaning_op(body, op)
      }
      FxCoreExpr::Lambda { body, .. } => contains_meaning_op(body, op),
      FxCoreExpr::Select { expr, .. } => contains_meaning_op(expr, op),
      FxCoreExpr::List(items) => items.iter().any(|item| contains_meaning_op(item, op)),
      FxCoreExpr::AttrSet(pairs) => pairs
        .iter()
        .any(|(_, value)| contains_meaning_op(value, op)),
      FxCoreExpr::Construct { args, .. } => args.iter().any(|arg| contains_meaning_op(arg, op)),
      FxCoreExpr::Interop { meta, .. } => meta.op == op,
      FxCoreExpr::ConstInt(_)
      | FxCoreExpr::ConstFloat(_)
      | FxCoreExpr::ConstBool(_)
      | FxCoreExpr::ConstString(_)
      | FxCoreExpr::ParamSysTime
      | FxCoreExpr::ParamDeltaTime
      | FxCoreExpr::SignalVar(_)
      | FxCoreExpr::Var(_)
      | FxCoreExpr::Throw { .. } => false,
    }
  }

  #[test]
  fn lower_sequence_basic_calls_to_apply_chain() {
    let cases = [
      ("(seq [1 2])", "seq", 1usize),
      ("(first [1 2])", "first", 1usize),
      ("(rest [1 2])", "rest", 1usize),
      ("(next [1 2])", "next", 1usize),
      ("(nth [1 2 3] 1)", "nth", 2usize),
      ("(nth [1 2 3] 99 :nf)", "nth", 3usize),
      ("(last [1 2 3])", "last", 1usize),
      ("(butlast [1 2 3])", "butlast", 1usize),
      ("(take 2 [1 2 3])", "take", 2usize),
      ("(drop 1 [1 2 3])", "drop", 2usize),
      ("(cons 0 [1 2 3])", "cons", 2usize),
      ("(conj [1] 2 3)", "conj", 3usize),
      ("(into [] [1 2])", "into", 2usize),
      ("(into [] [1 2] [3])", "into", 3usize),
      ("(vec (list 1 2))", "vec", 1usize),
      ("(set [1 2 2])", "set", 1usize),
    ];

    for (source, expected_func, expected_arity) in cases {
      let parsed = parse_clj_expr(source).unwrap();
      let lowered = lower_clj_to_fx_core(&parsed).unwrap();
      let (func, argc) = flatten_apply_chain(&lowered)
        .unwrap_or_else(|| panic!("expected Apply chain for {} but got {:?}", source, lowered));
      assert_eq!(func, expected_func, "source={}", source);
      assert_eq!(argc, expected_arity, "source={}", source);
    }
  }

  #[test]
  fn lower_sequence_variadic_zero_args_calls() {
    let cases = [("(concat)", "concat"), ("(list)", "list")];
    for (source, expected_func) in cases {
      let parsed = parse_clj_expr(source).unwrap();
      let lowered = lower_clj_to_fx_core(&parsed).unwrap();
      let (func, argc) = flatten_apply_chain(&lowered)
        .unwrap_or_else(|| panic!("expected Apply chain for {} but got {:?}", source, lowered));
      assert_eq!(func, expected_func, "source={}", source);
      assert_eq!(argc, 0, "source={}", source);
    }
  }

  #[test]
  fn lower_sequence_higher_order_calls_to_apply_chain() {
    let cases = [
      ("(map inc [1 2 3])", "map", 2usize),
      ("(map + [1 2] [3 4])", "map", 3usize),
      ("(mapv inc [1 2 3])", "mapv", 2usize),
      ("(filter odd? [1 2 3])", "filter", 2usize),
      ("(remove odd? [1 2 3])", "remove", 2usize),
      ("(keep identity [1 nil 3])", "keep", 2usize),
      ("(reduce + [1 2 3])", "reduce", 2usize),
      ("(reduce + 0 [1 2 3])", "reduce", 3usize),
      ("(reduce-kv + 0 {:a 1})", "reduce-kv", 3usize),
      ("(some odd? [1 2 3])", "some", 2usize),
      ("(every? odd? [1 3 5])", "every?", 2usize),
      ("(not-any? odd? [2 4 6])", "not-any?", 2usize),
      ("(not-every? odd? [1 2 3])", "not-every?", 2usize),
    ];

    for (source, expected_func, expected_arity) in cases {
      let parsed = parse_clj_expr(source).unwrap();
      let lowered = lower_clj_to_fx_core(&parsed).unwrap();
      let (func, argc) = flatten_apply_chain(&lowered)
        .unwrap_or_else(|| panic!("expected Apply chain for {} but got {:?}", source, lowered));
      assert_eq!(func, expected_func, "source={}", source);
      assert_eq!(argc, expected_arity, "source={}", source);
    }
  }

  #[test]
  fn lower_function_binding_helper_calls_to_apply_chain() {
    let cases = [
      ("(apply + [1 2 3])", "apply", 2usize),
      ("(partial + 1 2)", "partial", 3usize),
      ("(comp inc str)", "comp", 2usize),
      ("(comp)", "comp", 0usize),
      ("(juxt inc dec)", "juxt", 2usize),
      ("(identity 1)", "identity", 1usize),
      ("(constantly 7)", "constantly", 1usize),
    ];

    for (source, expected_func, expected_arity) in cases {
      let parsed = parse_clj_expr(source).unwrap();
      let lowered = lower_clj_to_fx_core(&parsed).unwrap();
      let (func, argc) = flatten_apply_chain(&lowered)
        .unwrap_or_else(|| panic!("expected Apply chain for {} but got {:?}", source, lowered));
      assert_eq!(func, expected_func, "source={}", source);
      assert_eq!(argc, expected_arity, "source={}", source);
    }
  }

  #[test]
  fn lower_fn_defn_letfn_and_destructuring_shapes() {
    let fn_expr = parse_clj_expr("(fn [x] (+ x 1))").unwrap();
    let fn_lowered = lower_clj_to_fx_core(&fn_expr).unwrap();
    assert!(matches!(fn_lowered, FxCoreExpr::Lambda { .. }));

    let defn_expr = parse_clj_expr("(defn add1 [x] (+ x 1))").unwrap();
    let defn_lowered = lower_clj_to_fx_core(&defn_expr).unwrap();
    assert!(matches!(
      defn_lowered,
      FxCoreExpr::Let {
        name,
        value,
        body
      } if name == "add1"
        && matches!(*value, FxCoreExpr::Lambda { .. })
        && matches!(*body, FxCoreExpr::Var(ref n) if n == "add1")
    ));

    let letfn_expr = parse_clj_expr("(letfn [(inc1 [x] (+ x 1))] (inc1 2))").unwrap();
    let letfn_lowered = lower_clj_to_fx_core(&letfn_expr).unwrap();
    assert!(matches!(letfn_lowered, FxCoreExpr::Let { name, .. } if name == "inc1"));

    let let_vec_expr = parse_clj_expr("(let [[x y & rest] [1 2 3 4]] (+ x y))").unwrap();
    let let_vec_lowered = lower_clj_to_fx_core(&let_vec_expr).unwrap();
    assert!(contains_var_name(&let_vec_lowered, "nth"));
    assert!(contains_var_name(&let_vec_lowered, "drop"));

    let let_map_expr = parse_clj_expr("(let [{:keys [a b] :as m} {:a 1 :b 2}] (+ a b))").unwrap();
    let let_map_lowered = lower_clj_to_fx_core(&let_map_expr).unwrap();
    assert!(contains_meaning_op(&let_map_lowered, MeaningOpId::GetAttr));
  }
}
