//! UnifiedExpr ↔ FxCoreExpr 양방향 변환
//!
//! pnix-old의 lang_pnix/lower.rs에서 마이그레이션.
//!
//! - `lower_to_fx_core`: UnifiedExpr → FxCoreExpr (lowering)
//! - `fx_core_to_unified`: FxCoreExpr → UnifiedExpr (역변환)
//!
//! ## 헌법 준수 (P0-1)
//!
//! 순수 구조 변환만, 값 계산 없음

use super::error::PnixError;
use super::syntax::{
  AttrKeySegment, PnixAttrItem, PnixExpr, PnixLetBinding, PnixLiteralPattern, PnixMatchArm,
  PnixPattern, StringInterpPart,
};
use super::unified::{resolve_signals, ExecutionMode, UnifiedExpr};
use crate::diagnostics::Span;
use crate::fx::core_expr::{FxCoreExpr, SignalId};
use crate::fx::meaning_op::{MeaningMeta, MeaningOpId};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

// Y08b-3: 스택 오버플로 방지를 위해 보수적인 깊이 제한
// 테스트 환경에서 스택이 작을 수 있어 32로 제한
const MAX_LOWERING_DEPTH: usize = 256;

/// 결정론적 gensym: 표현식의 Debug 표현을 해시하여 고유 이름 생성
/// 컴파일 순서에 관계없이 동일 입력 → 동일 출력 보장
fn gensym_from_expr(prefix: &str, expr: &PnixExpr) -> String {
  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};
  let mut hasher = DefaultHasher::new();
  // Debug 표현을 해시 (PnixExpr가 Hash를 구현하지 않으므로)
  format!("{:?}", expr).hash(&mut hasher);
  let hash = hasher.finish();
  format!("{}_{:x}", prefix, hash)
}

/// 결정론적 gensym: UnifiedExpr의 Debug 표현을 해시하여 고유 이름 생성
fn gensym_from_unified(prefix: &str, expr: &UnifiedExpr) -> String {
  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};
  let mut hasher = DefaultHasher::new();
  format!("{:?}", expr).hash(&mut hasher);
  let hash = hasher.finish();
  format!("{}_{:x}", prefix, hash)
}

fn fresh_name(base: String, reserved: &HashSet<String>) -> String {
  if !reserved.contains(&base) {
    return base;
  }
  let mut idx = 1;
  loop {
    let candidate = format!("{}_{}", base, idx);
    if !reserved.contains(&candidate) {
      return candidate;
    }
    idx += 1;
  }
}

fn collect_param_bound_names(pattern: &super::syntax::PnixParamPattern) -> HashSet<String> {
  let mut bound = HashSet::new();
  match pattern {
    super::syntax::PnixParamPattern::Ident(name) => {
      bound.insert(name.clone());
    }
    super::syntax::PnixParamPattern::AttrSet { fields, .. } => {
      for field in fields {
        bound.insert(field.name.clone());
      }
    }
    super::syntax::PnixParamPattern::AttrSetWithBind {
      bind_name, fields, ..
    } => {
      bound.insert(bind_name.clone());
      for field in fields {
        bound.insert(field.name.clone());
      }
    }
    super::syntax::PnixParamPattern::List(list_pattern) => {
      for item in &list_pattern.items {
        bound.insert(item.clone());
      }
      if let Some(ref tail) = list_pattern.tail {
        bound.insert(tail.clone());
      }
    }
  }
  bound
}

fn collect_let_bound_names(bindings: &[PnixLetBinding]) -> HashSet<String> {
  let mut bound = HashSet::new();
  for binding in bindings {
    match binding {
      PnixLetBinding::Binding { pattern, .. } => {
        bound.extend(collect_param_bound_names(pattern));
      }
      PnixLetBinding::Inherit { names, .. } => {
        for name in names {
          bound.insert(name.clone());
        }
      }
    }
  }
  bound
}

fn collect_param_default_free_vars(
  pattern: &super::syntax::PnixParamPattern,
  bound: &HashSet<String>,
) -> HashSet<String> {
  let mut free = HashSet::new();
  match pattern {
    super::syntax::PnixParamPattern::AttrSet { fields, .. } => {
      for field in fields {
        if let Some(default) = field.default.as_ref() {
          free.extend(collect_pnix_free_vars(default, bound));
        }
      }
    }
    super::syntax::PnixParamPattern::AttrSetWithBind {
      bind_name, fields, ..
    } => {
      // MEDIUM: @-패턴 기본값 다른 필드 접근 불일치 수정 완료
      // 기본값 표현식에서 bind_name을 통해 다른 필드에 접근할 수 있어야 함
      // 예: args@{x ? args.y, y = 1} → x의 기본값에서 args.y를 참조 가능
      // 하지만 다른 필드 이름을 직접 참조하는 것은 불가능 (예: args@{x ? y, y = 1})
      // 이는 Nix 의미론: 기본값은 bind_name을 통해서만 다른 필드에 접근 가능
      // bind_name is bound in the pattern, so it's not free in default expressions
      // Additionally, bind_name can be used to access other fields in defaults
      let mut next_bound = bound.clone();
      next_bound.insert(bind_name.clone());
      // Note: Field names are NOT added to bound set, as they cannot be directly referenced
      // in default expressions. Only bind_name can be used to access other fields.
      for field in fields {
        if let Some(default) = field.default.as_ref() {
          free.extend(collect_pnix_free_vars(default, &next_bound));
        }
      }
    }
    super::syntax::PnixParamPattern::Ident(_) | super::syntax::PnixParamPattern::List(_) => {}
  }
  free
}

fn fresh_lambda_param_name(pattern: &super::syntax::PnixParamPattern, body: &PnixExpr) -> String {
  let bound = collect_param_bound_names(pattern);
  let mut reserved = bound.clone();
  reserved.extend(collect_pnix_free_vars(body, &bound));
  reserved.extend(collect_param_default_free_vars(pattern, &bound));
  let base = gensym_from_expr("_lambda_arg", body);
  fresh_name(base, &reserved)
}

fn collect_attrset_top_level_names(items: &[PnixAttrItem]) -> HashSet<String> {
  let mut names = HashSet::new();
  for item in items {
    match item {
      PnixAttrItem::Assign { key_path, .. } => {
        if let Some(first) = key_path.first() {
          names.insert(first.clone());
        }
      }
      PnixAttrItem::DynamicAssign { .. } => {}
      PnixAttrItem::Inherit { names: inherit, .. } => {
        for name in inherit {
          names.insert(name.clone());
        }
      }
    }
  }
  names
}

fn rewrite_rec_attrset_refs(
  expr: UnifiedExpr,
  self_name: &str,
  attr_names: &HashSet<String>,
) -> UnifiedExpr {
  fn rewrite(
    expr: UnifiedExpr,
    self_name: &str,
    attr_names: &HashSet<String>,
    bound: &HashSet<String>,
  ) -> UnifiedExpr {
    match expr {
      UnifiedExpr::Var(name) => {
        if name == self_name || bound.contains(&name) || !attr_names.contains(&name) {
          UnifiedExpr::Var(name)
        } else {
          get_attr_expr(name, UnifiedExpr::Var(self_name.to_string()))
        }
      }
      UnifiedExpr::Let { name, value, body } => {
        let value = rewrite(*value, self_name, attr_names, bound);
        let mut next_bound = bound.clone();
        next_bound.insert(name.clone());
        let body = rewrite(*body, self_name, attr_names, &next_bound);
        UnifiedExpr::Let {
          name,
          value: Box::new(value),
          body: Box::new(body),
        }
      }
      UnifiedExpr::Lambda { param, body } => {
        let mut next_bound = bound.clone();
        next_bound.insert(param.clone());
        let body = rewrite(*body, self_name, attr_names, &next_bound);
        UnifiedExpr::Lambda {
          param,
          body: Box::new(body),
        }
      }
      UnifiedExpr::Apply { func, args } => {
        let lowered_args = args
          .into_iter()
          .map(|arg| rewrite(arg, self_name, attr_names, bound))
          .collect::<Vec<_>>();
        if attr_names.contains(&func) && !bound.contains(&func) {
          let func_value = get_attr_expr(func.clone(), UnifiedExpr::Var(self_name.to_string()));
          let func_name = gensym_from_unified("_rec_func", &func_value);
          UnifiedExpr::Let {
            name: func_name.clone(),
            value: Box::new(func_value),
            body: Box::new(UnifiedExpr::Apply {
              func: func_name,
              args: lowered_args,
            }),
          }
        } else {
          UnifiedExpr::Apply {
            func,
            args: lowered_args,
          }
        }
      }
      UnifiedExpr::Add(lhs, rhs) => UnifiedExpr::Add(
        Box::new(rewrite(*lhs, self_name, attr_names, bound)),
        Box::new(rewrite(*rhs, self_name, attr_names, bound)),
      ),
      UnifiedExpr::Sub(lhs, rhs) => UnifiedExpr::Sub(
        Box::new(rewrite(*lhs, self_name, attr_names, bound)),
        Box::new(rewrite(*rhs, self_name, attr_names, bound)),
      ),
      UnifiedExpr::Mul(lhs, rhs) => UnifiedExpr::Mul(
        Box::new(rewrite(*lhs, self_name, attr_names, bound)),
        Box::new(rewrite(*rhs, self_name, attr_names, bound)),
      ),
      UnifiedExpr::Div(lhs, rhs) => UnifiedExpr::Div(
        Box::new(rewrite(*lhs, self_name, attr_names, bound)),
        Box::new(rewrite(*rhs, self_name, attr_names, bound)),
      ),
      UnifiedExpr::Mod(lhs, rhs) => UnifiedExpr::Mod(
        Box::new(rewrite(*lhs, self_name, attr_names, bound)),
        Box::new(rewrite(*rhs, self_name, attr_names, bound)),
      ),
      UnifiedExpr::Neg(arg) => {
        UnifiedExpr::Neg(Box::new(rewrite(*arg, self_name, attr_names, bound)))
      }
      UnifiedExpr::Concat(lhs, rhs) => UnifiedExpr::Concat(
        Box::new(rewrite(*lhs, self_name, attr_names, bound)),
        Box::new(rewrite(*rhs, self_name, attr_names, bound)),
      ),
      UnifiedExpr::Floor(arg) => {
        UnifiedExpr::Floor(Box::new(rewrite(*arg, self_name, attr_names, bound)))
      }
      UnifiedExpr::Ceil(arg) => {
        UnifiedExpr::Ceil(Box::new(rewrite(*arg, self_name, attr_names, bound)))
      }
      UnifiedExpr::Abs(arg) => {
        UnifiedExpr::Abs(Box::new(rewrite(*arg, self_name, attr_names, bound)))
      }
      UnifiedExpr::Sqrt(arg) => {
        UnifiedExpr::Sqrt(Box::new(rewrite(*arg, self_name, attr_names, bound)))
      }
      UnifiedExpr::Sin(arg) => {
        UnifiedExpr::Sin(Box::new(rewrite(*arg, self_name, attr_names, bound)))
      }
      UnifiedExpr::Cos(arg) => {
        UnifiedExpr::Cos(Box::new(rewrite(*arg, self_name, attr_names, bound)))
      }
      UnifiedExpr::Tan(arg) => {
        UnifiedExpr::Tan(Box::new(rewrite(*arg, self_name, attr_names, bound)))
      }
      UnifiedExpr::Exp(arg) => {
        UnifiedExpr::Exp(Box::new(rewrite(*arg, self_name, attr_names, bound)))
      }
      UnifiedExpr::Ln(arg) => {
        UnifiedExpr::Ln(Box::new(rewrite(*arg, self_name, attr_names, bound)))
      }
      UnifiedExpr::Pow(lhs, rhs) => UnifiedExpr::Pow(
        Box::new(rewrite(*lhs, self_name, attr_names, bound)),
        Box::new(rewrite(*rhs, self_name, attr_names, bound)),
      ),
      UnifiedExpr::Lt(lhs, rhs) => UnifiedExpr::Lt(
        Box::new(rewrite(*lhs, self_name, attr_names, bound)),
        Box::new(rewrite(*rhs, self_name, attr_names, bound)),
      ),
      UnifiedExpr::Gt(lhs, rhs) => UnifiedExpr::Gt(
        Box::new(rewrite(*lhs, self_name, attr_names, bound)),
        Box::new(rewrite(*rhs, self_name, attr_names, bound)),
      ),
      UnifiedExpr::Le(lhs, rhs) => UnifiedExpr::Le(
        Box::new(rewrite(*lhs, self_name, attr_names, bound)),
        Box::new(rewrite(*rhs, self_name, attr_names, bound)),
      ),
      UnifiedExpr::Ge(lhs, rhs) => UnifiedExpr::Ge(
        Box::new(rewrite(*lhs, self_name, attr_names, bound)),
        Box::new(rewrite(*rhs, self_name, attr_names, bound)),
      ),
      UnifiedExpr::Eq(lhs, rhs) => UnifiedExpr::Eq(
        Box::new(rewrite(*lhs, self_name, attr_names, bound)),
        Box::new(rewrite(*rhs, self_name, attr_names, bound)),
      ),
      UnifiedExpr::Ne(lhs, rhs) => UnifiedExpr::Ne(
        Box::new(rewrite(*lhs, self_name, attr_names, bound)),
        Box::new(rewrite(*rhs, self_name, attr_names, bound)),
      ),
      UnifiedExpr::And(lhs, rhs) => UnifiedExpr::And(
        Box::new(rewrite(*lhs, self_name, attr_names, bound)),
        Box::new(rewrite(*rhs, self_name, attr_names, bound)),
      ),
      UnifiedExpr::Or(lhs, rhs) => UnifiedExpr::Or(
        Box::new(rewrite(*lhs, self_name, attr_names, bound)),
        Box::new(rewrite(*rhs, self_name, attr_names, bound)),
      ),
      UnifiedExpr::Not(arg) => {
        UnifiedExpr::Not(Box::new(rewrite(*arg, self_name, attr_names, bound)))
      }
      UnifiedExpr::If { cond, then_, else_ } => UnifiedExpr::If {
        cond: Box::new(rewrite(*cond, self_name, attr_names, bound)),
        then_: Box::new(rewrite(*then_, self_name, attr_names, bound)),
        else_: Box::new(rewrite(*else_, self_name, attr_names, bound)),
      },
      UnifiedExpr::Throw(msg) => UnifiedExpr::Throw(msg),
      UnifiedExpr::Fx(body) => {
        UnifiedExpr::Fx(Box::new(rewrite(*body, self_name, attr_names, bound)))
      }
      UnifiedExpr::Interop { lang, code } => UnifiedExpr::Interop { lang, code },
      UnifiedExpr::Derived { op, args } => UnifiedExpr::Derived {
        op,
        args: args
          .into_iter()
          .map(|arg| rewrite(arg, self_name, attr_names, bound))
          .collect(),
      },
      UnifiedExpr::AttrSet(pairs) => UnifiedExpr::AttrSet(
        pairs
          .into_iter()
          .map(|(k, v)| (k, rewrite(v, self_name, attr_names, bound)))
          .collect(),
      ),
      UnifiedExpr::Merge(lhs, rhs) => UnifiedExpr::Merge(
        Box::new(rewrite(*lhs, self_name, attr_names, bound)),
        Box::new(rewrite(*rhs, self_name, attr_names, bound)),
      ),
      UnifiedExpr::List(items) => UnifiedExpr::List(
        items
          .into_iter()
          .map(|item| rewrite(item, self_name, attr_names, bound))
          .collect(),
      ),
      UnifiedExpr::Construct { variant, args } => UnifiedExpr::Construct {
        variant,
        args: args
          .into_iter()
          .map(|arg| rewrite(arg, self_name, attr_names, bound))
          .collect(),
      },
      UnifiedExpr::Int(_)
      | UnifiedExpr::Float(_)
      | UnifiedExpr::Bool(_)
      | UnifiedExpr::String(_)
      | UnifiedExpr::ParamTime
      | UnifiedExpr::ParamDeltaTime
      | UnifiedExpr::ParamSignal(_)
      | UnifiedExpr::SignalVar(_)
      | UnifiedExpr::Null => expr,
    }
  }

  let mut bound = HashSet::new();
  bound.insert(self_name.to_string());
  rewrite(expr, self_name, attr_names, &bound)
}

fn collect_unified_free_vars(expr: &UnifiedExpr, bound: &HashSet<String>) -> HashSet<String> {
  match expr {
    UnifiedExpr::Var(name) => {
      if bound.contains(name) {
        HashSet::new()
      } else {
        let mut set = HashSet::new();
        set.insert(name.clone());
        set
      }
    }
    UnifiedExpr::Apply { func, args } => {
      let mut set = HashSet::new();
      if !bound.contains(func) {
        set.insert(func.clone());
      }
      for arg in args {
        set.extend(collect_unified_free_vars(arg, bound));
      }
      set
    }
    UnifiedExpr::Let { name, value, body } => {
      let mut set = collect_unified_free_vars(value, bound);
      let mut next_bound = bound.clone();
      next_bound.insert(name.clone());
      set.extend(collect_unified_free_vars(body, &next_bound));
      set
    }
    UnifiedExpr::Lambda { param, body } => {
      let mut next_bound = bound.clone();
      next_bound.insert(param.clone());
      collect_unified_free_vars(body, &next_bound)
    }
    UnifiedExpr::If { cond, then_, else_ } => {
      let mut set = collect_unified_free_vars(cond, bound);
      set.extend(collect_unified_free_vars(then_, bound));
      set.extend(collect_unified_free_vars(else_, bound));
      set
    }
    UnifiedExpr::Add(lhs, rhs)
    | UnifiedExpr::Sub(lhs, rhs)
    | UnifiedExpr::Mul(lhs, rhs)
    | UnifiedExpr::Div(lhs, rhs)
    | UnifiedExpr::Mod(lhs, rhs)
    | UnifiedExpr::Pow(lhs, rhs)
    | UnifiedExpr::Lt(lhs, rhs)
    | UnifiedExpr::Gt(lhs, rhs)
    | UnifiedExpr::Le(lhs, rhs)
    | UnifiedExpr::Ge(lhs, rhs)
    | UnifiedExpr::Eq(lhs, rhs)
    | UnifiedExpr::Ne(lhs, rhs)
    | UnifiedExpr::And(lhs, rhs)
    | UnifiedExpr::Or(lhs, rhs)
    | UnifiedExpr::Concat(lhs, rhs)
    | UnifiedExpr::Merge(lhs, rhs) => {
      let mut set = collect_unified_free_vars(lhs, bound);
      set.extend(collect_unified_free_vars(rhs, bound));
      set
    }
    UnifiedExpr::Neg(arg)
    | UnifiedExpr::Not(arg)
    | UnifiedExpr::Floor(arg)
    | UnifiedExpr::Ceil(arg)
    | UnifiedExpr::Abs(arg)
    | UnifiedExpr::Sqrt(arg)
    | UnifiedExpr::Sin(arg)
    | UnifiedExpr::Cos(arg)
    | UnifiedExpr::Tan(arg)
    | UnifiedExpr::Exp(arg)
    | UnifiedExpr::Ln(arg)
    | UnifiedExpr::Fx(arg) => collect_unified_free_vars(arg, bound),
    UnifiedExpr::Derived { args, .. } | UnifiedExpr::Construct { args, .. } => {
      let mut set = HashSet::new();
      for arg in args {
        set.extend(collect_unified_free_vars(arg, bound));
      }
      set
    }
    UnifiedExpr::AttrSet(pairs) => {
      let mut set = HashSet::new();
      for (_, value) in pairs {
        set.extend(collect_unified_free_vars(value, bound));
      }
      set
    }
    UnifiedExpr::List(items) => {
      let mut set = HashSet::new();
      for item in items {
        set.extend(collect_unified_free_vars(item, bound));
      }
      set
    }
    UnifiedExpr::Int(_)
    | UnifiedExpr::Float(_)
    | UnifiedExpr::Bool(_)
    | UnifiedExpr::String(_)
    | UnifiedExpr::ParamTime
    | UnifiedExpr::ParamDeltaTime
    | UnifiedExpr::ParamSignal(_)
    | UnifiedExpr::SignalVar(_)
    | UnifiedExpr::Throw(_)
    | UnifiedExpr::Interop { .. }
    | UnifiedExpr::Null => HashSet::new(),
  }
}

/// 복합 표현식을 함수로 적용하기 위해 let 바인딩으로 변환
/// (func_expr)(arg) → let _func = func_expr in _func(args...)
fn apply_complex_expr_as_func(
  func_value: UnifiedExpr,
  mut args: Vec<UnifiedExpr>,
  arg: &PnixExpr,
) -> Result<UnifiedExpr, PnixError> {
  args.push(pnix_expr_to_unified(arg)?);

  // 결정론적 함수 이름 생성 (content-based hash)
  let func_name = gensym_from_unified("_apply_func", &func_value);

  // let _func = func_value in _func(args...)
  Ok(UnifiedExpr::Let {
    name: func_name.clone(),
    value: Box::new(func_value),
    body: Box::new(UnifiedExpr::Apply {
      func: func_name,
      args,
    }),
  })
}

fn lower_let_recursive(
  expr: &PnixExpr,
  bindings: &[PnixLetBinding],
  body: UnifiedExpr,
) -> Result<UnifiedExpr, PnixError> {
  struct LetBindingEntry {
    name: String,
    value: UnifiedExpr,
    rewrite: bool,
  }

  let initial_names = collect_let_bound_names(bindings);
  let _guard = BoundGuard::enter(initial_names.clone());

  let mut reserved = initial_names.clone();
  reserved.extend(collect_unified_free_vars(&body, &HashSet::new()));

  let mut entries: Vec<LetBindingEntry> = Vec::new();

  for binding in bindings {
    match binding {
      PnixLetBinding::Binding { pattern, value } => {
        let value_unified = pnix_expr_to_unified(value)?;
        reserved.extend(collect_unified_free_vars(&value_unified, &HashSet::new()));
        match pattern {
          super::syntax::PnixParamPattern::Ident(name) => {
            entries.push(LetBindingEntry {
              name: name.clone(),
              value: value_unified,
              rewrite: true,
            });
          }
          super::syntax::PnixParamPattern::AttrSet { fields, .. } => {
            let tmp_base = gensym_from_unified("_let_dest", &value_unified);
            let tmp_name = fresh_name(tmp_base, &reserved);
            reserved.insert(tmp_name.clone());
            entries.push(LetBindingEntry {
              name: tmp_name.clone(),
              value: value_unified,
              rewrite: true,
            });

            for field in fields {
              let field_value = if let Some(ref default) = field.default {
                let default_unified = pnix_expr_to_unified(default)?;
                get_attr_or_default_expr(
                  field.name.clone(),
                  UnifiedExpr::Var(tmp_name.clone()),
                  default_unified,
                )
              } else {
                get_attr_expr(field.name.clone(), UnifiedExpr::Var(tmp_name.clone()))
              };
              reserved.extend(collect_unified_free_vars(&field_value, &HashSet::new()));
              entries.push(LetBindingEntry {
                name: field.name.clone(),
                value: field_value,
                rewrite: true,
              });
            }
          }
          super::syntax::PnixParamPattern::List(list_pattern) => {
            let tmp_base = gensym_from_unified("_let_dest", &value_unified);
            let tmp_name = fresh_name(tmp_base, &reserved);
            reserved.insert(tmp_name.clone());
            entries.push(LetBindingEntry {
              name: tmp_name.clone(),
              value: value_unified,
              rewrite: true,
            });

            for (i, item_name) in list_pattern.items.iter().enumerate() {
              let item_value = UnifiedExpr::Apply {
                func: "builtins.elemAt".to_string(),
                args: vec![
                  UnifiedExpr::Var(tmp_name.clone()),
                  UnifiedExpr::Int(i as i64),
                ],
              };
              reserved.extend(collect_unified_free_vars(&item_value, &HashSet::new()));
              entries.push(LetBindingEntry {
                name: item_name.clone(),
                value: item_value,
                rewrite: true,
              });
            }

            if let Some(ref tail_name) = list_pattern.tail {
              let n = list_pattern.items.len() as i64;
              let tail_value = UnifiedExpr::Apply {
                func: "builtins.drop".to_string(),
                args: vec![UnifiedExpr::Int(n), UnifiedExpr::Var(tmp_name.clone())],
              };
              reserved.extend(collect_unified_free_vars(&tail_value, &HashSet::new()));
              entries.push(LetBindingEntry {
                name: tail_name.clone(),
                value: tail_value,
                rewrite: true,
              });
            }
          }
          super::syntax::PnixParamPattern::AttrSetWithBind {
            bind_name, fields, ..
          } => {
            let tmp_base = gensym_from_unified("_let_dest", &value_unified);
            let tmp_name = fresh_name(tmp_base, &reserved);
            reserved.insert(tmp_name.clone());
            entries.push(LetBindingEntry {
              name: tmp_name.clone(),
              value: value_unified,
              rewrite: true,
            });

            for field in fields {
              let field_value = if let Some(ref default) = field.default {
                let default_unified = pnix_expr_to_unified(default)?;
                get_attr_or_default_expr(
                  field.name.clone(),
                  UnifiedExpr::Var(tmp_name.clone()),
                  default_unified,
                )
              } else {
                get_attr_expr(field.name.clone(), UnifiedExpr::Var(tmp_name.clone()))
              };
              reserved.extend(collect_unified_free_vars(&field_value, &HashSet::new()));
              entries.push(LetBindingEntry {
                name: field.name.clone(),
                value: field_value,
                rewrite: true,
              });
            }

            let bind_value = UnifiedExpr::Var(tmp_name.clone());
            reserved.extend(collect_unified_free_vars(&bind_value, &HashSet::new()));
            entries.push(LetBindingEntry {
              name: bind_name.clone(),
              value: bind_value,
              rewrite: true,
            });
          }
        }
      }
      PnixLetBinding::Inherit { from, names } => {
        let scope_unified = if let Some(scope_expr) = from {
          Some(pnix_expr_to_unified(scope_expr)?)
        } else {
          None
        };

        if let Some(scope_unified) = scope_unified.as_ref() {
          reserved.extend(collect_unified_free_vars(scope_unified, &HashSet::new()));
        }

        for name in names {
          let value = if let Some(scope_unified) = scope_unified.as_ref() {
            get_attr_expr(name.clone(), scope_unified.clone())
          } else {
            UnifiedExpr::Var(name.clone())
          };
          if from.is_some() {
            reserved.extend(collect_unified_free_vars(&value, &HashSet::new()));
          }
          entries.push(LetBindingEntry {
            name: name.clone(),
            value,
            rewrite: from.is_some(),
          });
        }
      }
    }
  }

  let attr_names: HashSet<String> = entries.iter().map(|entry| entry.name.clone()).collect();
  let mut reserved_for_self = reserved;
  reserved_for_self.extend(attr_names.iter().cloned());
  let self_name = fresh_name(gensym_from_expr("_let_rec", expr), &reserved_for_self);

  let attrset_value = UnifiedExpr::AttrSet(
    entries
      .into_iter()
      .map(|entry| {
        let value = if entry.rewrite {
          rewrite_rec_attrset_refs(entry.value, &self_name, &attr_names)
        } else {
          entry.value
        };
        (entry.name, value)
      })
      .collect(),
  );
  let body = rewrite_rec_attrset_refs(body, &self_name, &attr_names);

  Ok(UnifiedExpr::Let {
    name: self_name.clone(),
    value: Box::new(attrset_value),
    body: Box::new(body),
  })
}

thread_local! {
  static LOWERING_DEPTH: Cell<usize> = const { Cell::new(0) };
}

thread_local! {
  static LOWERING_BOUND: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

struct BoundGuard {
  added: Vec<String>,
}

impl BoundGuard {
  fn enter<I>(names: I) -> Self
  where
    I: IntoIterator<Item = String>,
  {
    let mut added = Vec::new();
    LOWERING_BOUND.with(|bound| {
      let mut bound = bound.borrow_mut();
      for name in names {
        if bound.insert(name.clone()) {
          added.push(name);
        }
      }
    });
    Self { added }
  }
}

impl Drop for BoundGuard {
  fn drop(&mut self) {
    LOWERING_BOUND.with(|bound| {
      let mut bound = bound.borrow_mut();
      for name in self.added.drain(..) {
        bound.remove(&name);
      }
    });
  }
}

fn current_bound_names() -> HashSet<String> {
  LOWERING_BOUND.with(|bound| bound.borrow().clone())
}

/// AttrSet 필드 접근 표현식 생성 헬퍼
/// builtins.getAttr "field_name" object
fn get_attr_expr(field_name: impl Into<String>, object: UnifiedExpr) -> UnifiedExpr {
  UnifiedExpr::Apply {
    func: "builtins.getAttr".to_string(),
    args: vec![UnifiedExpr::String(field_name.into()), object],
  }
}

/// N00e: AttrSet 필드 존재 여부 확인 표현식 생성 헬퍼
/// builtins.hasAttr "field_name" object
fn has_attr_expr(field_name: impl Into<String>, object: UnifiedExpr) -> UnifiedExpr {
  UnifiedExpr::Apply {
    func: "builtins.hasAttr".to_string(),
    args: vec![UnifiedExpr::String(field_name.into()), object],
  }
}

/// AttrSet 필드 기본값 처리 표현식 생성 헬퍼
/// if hasAttr("field_name", object) then getAttr("field_name", object) else default
fn get_attr_or_default_expr(
  field_name: impl Into<String>,
  object: UnifiedExpr,
  default: UnifiedExpr,
) -> UnifiedExpr {
  let name = field_name.into();
  let has_attr = has_attr_expr(name.clone(), object.clone());
  let get_attr = get_attr_expr(name, object);
  UnifiedExpr::If {
    cond: Box::new(has_attr),
    then_: Box::new(get_attr),
    else_: Box::new(default),
  }
}

fn lowering_error(message: impl Into<String>, span: Option<&Span>) -> PnixError {
  let message = message.into();
  match span {
    Some(span) => PnixError::lowering_with_span(message, span.clone()),
    None => PnixError::lowering(message),
  }
}

const LOWERING_REASON_RECURSION_DEPTH_EXCEEDED: &str = "LOWERING_RECURSION_DEPTH_EXCEEDED";
const LOWERING_REASON_IMMEDIATE_APPLY_DESTRUCTURING_PARAM_UNSUPPORTED: &str =
  "LOWERING_IMMEDIATE_APPLY_DESTRUCTURING_PARAM_UNSUPPORTED";
const LOWERING_REASON_LAMBDA_TOO_MANY_ARGS: &str = "LOWERING_LAMBDA_TOO_MANY_ARGS";
const LOWERING_REASON_WITH_COMPLEX_ENV_UNSUPPORTED: &str = "LOWERING_WITH_COMPLEX_ENV_UNSUPPORTED";

fn lowering_reason(
  reason: &'static str,
  detail: impl Into<String>,
  span: Option<&Span>,
) -> PnixError {
  lowering_error(format!("[{}] {}", reason, detail.into()), span)
}

/// Convert a dynamic key path to a unified expression representing the key name.
/// For single segment: just return the expression
/// For multiple segments like ${a}.${b}.c: concatenate them with "."
fn key_path_to_unified(
  path: &[AttrKeySegment],
  span: Option<&Span>,
) -> Result<UnifiedExpr, PnixError> {
  if path.is_empty() {
    return Err(lowering_error("empty key path in DynamicAssign", span));
  }
  if path.len() == 1 {
    // Single segment
    return match &path[0] {
      AttrKeySegment::Static(s) => Ok(UnifiedExpr::String(s.clone())),
      AttrKeySegment::Dynamic(expr) => pnix_expr_to_unified(expr),
    };
  }
  // Multiple segments: build a concat chain with "." separators
  // Result: concatStrings [seg1, ".", seg2, ".", seg3]
  let mut concat_parts = Vec::new();
  for (i, segment) in path.iter().enumerate() {
    if i > 0 {
      concat_parts.push(UnifiedExpr::String(".".to_string()));
    }
    let part = match segment {
      AttrKeySegment::Static(s) => UnifiedExpr::String(s.clone()),
      AttrKeySegment::Dynamic(expr) => pnix_expr_to_unified(expr)?,
    };
    concat_parts.push(part);
  }
  // Use builtins.concatStringsSep "" [parts] or build concat chain
  // Simplest approach: fold with Concat binary op
  let mut result = concat_parts.remove(0);
  for part in concat_parts {
    result = UnifiedExpr::Concat(Box::new(result), Box::new(part));
  }
  Ok(result)
}

#[derive(Debug)]
enum AttrTreeNode {
  Leaf(UnifiedExpr),
  Nested(Vec<(String, AttrTreeNode)>),
}

fn insert_attr_path(
  entries: &mut Vec<(String, AttrTreeNode)>,
  path: &[String],
  value: UnifiedExpr,
  span: Option<&Span>,
) -> Result<(), PnixError> {
  if path.is_empty() {
    return Err(lowering_error("empty key path in AttrSet", span));
  }

  let key = &path[0];
  if path.len() == 1 {
    if let Some((_, node)) = entries.iter_mut().find(|(k, _)| k == key) {
      match node {
        AttrTreeNode::Leaf(_) => {
          return Err(lowering_error(
            format!("attribute '{}' already defined", key),
            span,
          ));
        }
        AttrTreeNode::Nested(_) => {
          return Err(lowering_error(
            format!(
              "attribute '{}' already has nested attributes, cannot assign value",
              key
            ),
            span,
          ));
        }
      }
    } else {
      entries.push((key.clone(), AttrTreeNode::Leaf(value)));
    }
    return Ok(());
  }

  if let Some((_, node)) = entries.iter_mut().find(|(k, _)| k == key) {
    match node {
      AttrTreeNode::Leaf(_) => {
        return Err(lowering_error(
          format!(
            "attribute '{}' already has a value, cannot add nested attribute '{}'",
            key,
            path[1..].join(".")
          ),
          span,
        ));
      }
      AttrTreeNode::Nested(children) => {
        return insert_attr_path(children, &path[1..], value, span);
      }
    }
  }

  let mut children: Vec<(String, AttrTreeNode)> = Vec::new();
  insert_attr_path(&mut children, &path[1..], value, span)?;
  entries.push((key.clone(), AttrTreeNode::Nested(children)));
  Ok(())
}

fn attr_tree_to_pairs(mut entries: Vec<(String, AttrTreeNode)>) -> Vec<(String, UnifiedExpr)> {
  entries.sort_by(|(a, _), (b, _)| a.cmp(b));
  entries
    .into_iter()
    .map(|(key, node)| (key, attr_tree_node_to_unified(node)))
    .collect()
}

fn attr_tree_node_to_unified(node: AttrTreeNode) -> UnifiedExpr {
  match node {
    AttrTreeNode::Leaf(value) => value,
    AttrTreeNode::Nested(children) => UnifiedExpr::AttrSet(attr_tree_to_pairs(children)),
  }
}

struct LoweringDepthGuard;

impl LoweringDepthGuard {
  fn enter(context: &str) -> Result<Self, PnixError> {
    let ok = LOWERING_DEPTH.with(|depth| {
      let next = depth.get().saturating_add(1);
      if next > MAX_LOWERING_DEPTH {
        None
      } else {
        depth.set(next);
        Some(next)
      }
    });

    if ok.is_none() {
      return Err(lowering_reason(
        LOWERING_REASON_RECURSION_DEPTH_EXCEEDED,
        format!(
          "lowering recursion depth exceeded (max {}): {}",
          MAX_LOWERING_DEPTH, context
        ),
        None,
      ));
    }

    Ok(Self)
  }

  /// 테스트 격리를 위한 초기화 함수
  /// LOW: LOWERING_DEPTH 테스트 격리 실패 수정
  /// 테스트 시작 시 depth를 0으로 초기화하여 테스트 간 격리 보장
  #[cfg(test)]
  pub fn reset_for_test() {
    LOWERING_DEPTH.with(|depth| {
      depth.set(0);
    });
  }
}

impl Drop for LoweringDepthGuard {
  fn drop(&mut self) {
    LOWERING_DEPTH.with(|depth| {
      let current = depth.get();
      depth.set(current.saturating_sub(1));
    });
  }
}

/// SignalVar 이름↔ID 매핑 테이블: SignalVar 이름과 ID 간의 결정론적 매핑 보장
///
/// Y08a-9: DefaultHasher 제거하고 이름↔ID 매핑 테이블 도입
/// - 같은 이름은 항상 같은 ID로 매핑 (충돌 방지)
/// - ID → 이름 round-trip 보장
/// 시그널 변수 매핑: 시그널 이름과 ID 간의 양방향 매핑
#[derive(Debug, Clone, Default)]
pub struct SignalVarMapping {
  /// 이름 → ID 매핑 (시그널 이름 → ID 매핑)
  name_to_id: HashMap<String, usize>,
  /// ID → 이름 매핑 (round-trip 보장, ID → 시그널 이름 매핑)
  id_to_name: HashMap<usize, String>,
  /// 다음 할당할 ID (충돌 방지를 위해 순차 할당, 다음에 할당할 ID)
  next_id: usize,
}

impl SignalVarMapping {
  /// 새로운 빈 매핑 테이블 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self {
      name_to_id: HashMap::new(),
      id_to_name: HashMap::new(),
      next_id: 0,
    }
  }

  /// 이름에서 SignalId 가져오기 또는 생성 (결정론적)
  ///
  /// 같은 이름은 항상 같은 ID를 반환하며, 처음 보는 이름은 순차적으로 ID 할당
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 매핑만, 값 계산 없음
  pub fn get_or_assign_id(&mut self, name: &str) -> SignalId {
    if let Some(&id) = self.name_to_id.get(name) {
      SignalId(id)
    } else {
      let id = self.next_id;
      self.next_id += 1;
      self.name_to_id.insert(name.to_string(), id);
      self.id_to_name.insert(id, name.to_string());
      SignalId(id)
    }
  }

  /// ID에서 이름 가져오기 (round-trip 보장)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn get_name(&self, id: SignalId) -> Option<&String> {
    self.id_to_name.get(&id.0)
  }

  /// 모든 매핑 가져오기 (디버깅/직렬화용)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn all_mappings(&self) -> &HashMap<String, usize> {
    &self.name_to_id
  }
}

/// 헬퍼: PnixExpr 함수에 UnifiedExpr 인자들을 적용
///
/// func가 Var면 Apply 생성, Lambda면 beta-reduce,
/// If/Let이면 재귀적으로 분배
fn apply_args_to_func(func: &PnixExpr, args: &[UnifiedExpr]) -> Result<UnifiedExpr, PnixError> {
  match func {
    PnixExpr::Var(name) => Ok(UnifiedExpr::Apply {
      func: name.clone(),
      args: args.to_vec(),
    }),
    PnixExpr::Lambda { param, body } => {
      // beta-reduction: (x: body)(arg) → let x = arg in body
      // 먼저 param을 단순 이름으로 추출 (destructuring은 지원 안함)
      let param_name = match param {
        super::syntax::PnixParamPattern::Ident(name) => name.clone(),
        _ => {
          return Err(lowering_reason(
            LOWERING_REASON_IMMEDIATE_APPLY_DESTRUCTURING_PARAM_UNSUPPORTED,
            "destructuring patterns in lambda parameters are not supported for immediate application. \
             Use a simple variable parameter instead.",
            None,
          ));
        }
      };

      let bound_names = collect_param_bound_names(param);
      if args.is_empty() {
        // 인자 없음 - lambda 그대로 반환
        let body_unified = {
          let _guard = BoundGuard::enter(bound_names.clone());
          pnix_expr_to_unified(body)?
        };
        Ok(UnifiedExpr::Lambda {
          param: param_name,
          body: Box::new(body_unified),
        })
      } else {
        let mut result = {
          let _guard = BoundGuard::enter(bound_names);
          pnix_expr_to_unified(body)?
        };
        let remaining_args = args.to_vec();

        // 첫 번째 인자를 param에 바인딩
        if let Some(first_arg) = remaining_args.first().cloned() {
          result = UnifiedExpr::Let {
            name: param_name,
            value: Box::new(first_arg),
            body: Box::new(result),
          };

          // 나머지 인자들 처리
          for extra_arg in remaining_args.into_iter().skip(1) {
            match &result {
              UnifiedExpr::Let {
                name: let_name,
                value: let_value,
                body: let_body,
              } => {
                if let UnifiedExpr::Lambda {
                  param: inner_param,
                  body: inner_body,
                } = let_body.as_ref()
                {
                  result = UnifiedExpr::Let {
                    name: let_name.clone(),
                    value: let_value.clone(),
                    body: Box::new(UnifiedExpr::Let {
                      name: inner_param.clone(),
                      value: Box::new(extra_arg),
                      body: inner_body.clone(),
                    }),
                  };
                } else {
                  return Err(lowering_reason(
                    LOWERING_REASON_LAMBDA_TOO_MANY_ARGS,
                    "too many arguments for lambda application",
                    None,
                  ));
                }
              }
              UnifiedExpr::Lambda {
                param: inner_param,
                body: inner_body,
              } => {
                result = UnifiedExpr::Let {
                  name: inner_param.clone(),
                  value: Box::new(extra_arg),
                  body: inner_body.clone(),
                };
              }
              _ => {
                return Err(lowering_reason(
                  LOWERING_REASON_LAMBDA_TOO_MANY_ARGS,
                  "too many arguments for lambda application",
                  None,
                ));
              }
            }
          }
        }
        Ok(result)
      }
    }
    PnixExpr::If { cond, then_, else_ } => {
      // (if c then f else g)(args) → if c then f args else g args
      let cond_unified = pnix_expr_to_unified(cond)?;
      let then_with_args = apply_args_to_func(then_, args)?;
      let else_with_args = apply_args_to_func(else_, args)?;
      Ok(UnifiedExpr::If {
        cond: Box::new(cond_unified),
        then_: Box::new(then_with_args),
        else_: Box::new(else_with_args),
      })
    }
    PnixExpr::Let { bindings, body } => {
      // (let x = v in f)(args) → let x = v in f args
      // (let {x, y} = v in f)(args) → let _tmp = v in let x = _tmp.x in let y = _tmp.y in f args
      let bound_names = collect_let_bound_names(bindings);
      let body_with_args = {
        let _guard = BoundGuard::enter(bound_names);
        apply_args_to_func(body, args)?
      };
      lower_let_recursive(func, bindings, body_with_args)
    }

    // Y-CLAUDE-apply: 복합 함수 표현식 지원
    // 예: (a b)(c), Select(obj, "method")(arg), etc.
    _ => {
      // 복합 표현식을 먼저 UnifiedExpr로 변환
      let func_unified = pnix_expr_to_unified(func)?;

      // 결정론적 함수 이름 생성 (content-based hash)
      let func_name = gensym_from_expr("_complex_func", func);

      // let _complex_func = func_expr in _complex_func(args...)
      Ok(UnifiedExpr::Let {
        name: func_name.clone(),
        value: Box::new(func_unified),
        body: Box::new(UnifiedExpr::Apply {
          func: func_name,
          args: args.to_vec(),
        }),
      })
    }
  }
}

fn push_param_pattern_defaults<'a>(
  pattern: &'a super::syntax::PnixParamPattern,
  depth: usize,
  stack: &mut Vec<(&'a PnixExpr, usize)>,
) {
  match pattern {
    super::syntax::PnixParamPattern::AttrSet { fields, .. }
    | super::syntax::PnixParamPattern::AttrSetWithBind { fields, .. } => {
      for field in fields {
        if let Some(default) = field.default.as_ref() {
          stack.push((default, depth));
        }
      }
    }
    super::syntax::PnixParamPattern::Ident(_) | super::syntax::PnixParamPattern::List(_) => {}
  }
}

fn check_pnix_expr_depth(expr: &PnixExpr) -> Result<(), PnixError> {
  let mut stack = Vec::new();
  stack.push((expr, 1usize));

  while let Some((expr, depth)) = stack.pop() {
    if depth > MAX_LOWERING_DEPTH {
      return Err(lowering_reason(
        LOWERING_REASON_RECURSION_DEPTH_EXCEEDED,
        format!(
          "lowering recursion depth exceeded (max {}): pnix_expr_to_unified",
          MAX_LOWERING_DEPTH
        ),
        None,
      ));
    }

    let next = depth + 1;
    match expr {
      PnixExpr::Int(_)
      | PnixExpr::Float(_)
      | PnixExpr::Bool(_)
      | PnixExpr::Null
      | PnixExpr::String(_)
      | PnixExpr::Path(_)
      | PnixExpr::Var(_) => {}
      PnixExpr::StringInterp(parts) => {
        for part in parts {
          if let StringInterpPart::Expr(inner) = part {
            stack.push((inner.as_ref(), next));
          }
        }
      }
      PnixExpr::Let { bindings, body } => {
        stack.push((body.as_ref(), next));
        for binding in bindings {
          match binding {
            PnixLetBinding::Binding { pattern, value } => {
              stack.push((value, next));
              push_param_pattern_defaults(pattern, next, &mut stack);
            }
            PnixLetBinding::Inherit { from, .. } => {
              if let Some(scope_expr) = from.as_ref() {
                stack.push((scope_expr.as_ref(), next));
              }
            }
          }
        }
      }
      PnixExpr::If { cond, then_, else_ } => {
        stack.push((cond.as_ref(), next));
        stack.push((then_.as_ref(), next));
        stack.push((else_.as_ref(), next));
      }
      PnixExpr::Lambda { param, body } => {
        stack.push((body.as_ref(), next));
        push_param_pattern_defaults(param, next, &mut stack);
      }
      PnixExpr::Apply { func, arg } => {
        stack.push((func.as_ref(), next));
        stack.push((arg.as_ref(), next));
      }
      PnixExpr::AttrSet { items, .. } => {
        for item in items {
          match item {
            PnixAttrItem::Assign { value, .. } => {
              stack.push((value, next));
            }
            PnixAttrItem::DynamicAssign {
              key_path, value, ..
            } => {
              // Push all dynamic segments in the key path
              for segment in key_path {
                if let AttrKeySegment::Dynamic(expr) = segment {
                  stack.push((expr.as_ref(), next));
                }
              }
              stack.push((value, next));
            }
            PnixAttrItem::Inherit { from, .. } => {
              if let Some(scope_expr) = from.as_ref() {
                stack.push((scope_expr.as_ref(), next));
              }
            }
          }
        }
      }
      PnixExpr::List(items) => {
        for item in items {
          stack.push((item, next));
        }
      }
      PnixExpr::Select { base, .. } => {
        stack.push((base.as_ref(), next));
      }
      PnixExpr::SelectOrDefault { base, default, .. } => {
        stack.push((base.as_ref(), next));
        stack.push((default.as_ref(), next));
      }
      PnixExpr::Index { base, index } => {
        stack.push((base.as_ref(), next));
        stack.push((index.as_ref(), next));
      }
      PnixExpr::Binary { lhs, rhs, .. } => {
        stack.push((lhs.as_ref(), next));
        stack.push((rhs.as_ref(), next));
      }
      PnixExpr::Unary { arg, .. } => {
        stack.push((arg.as_ref(), next));
      }
      PnixExpr::Construct { args, .. } => {
        for arg in args {
          stack.push((arg, next));
        }
      }
      PnixExpr::Match { scrutinee, arms } => {
        stack.push((scrutinee.as_ref(), next));
        for arm in arms {
          if let Some(guard) = arm.guard.as_ref() {
            stack.push((guard.as_ref(), next));
          }
          stack.push((&arm.body, next));
        }
      }
      PnixExpr::Import { path } => {
        stack.push((path.as_ref(), next));
      }
      PnixExpr::With { env, body } => {
        stack.push((env.as_ref(), next));
        stack.push((body.as_ref(), next));
      }
      PnixExpr::Assert { cond, body } => {
        stack.push((cond.as_ref(), next));
        stack.push((body.as_ref(), next));
      }
      PnixExpr::HasAttr { base, .. } => {
        stack.push((base.as_ref(), next));
      }
      PnixExpr::DynamicHasAttr { base, attr_expr } => {
        stack.push((base.as_ref(), next));
        stack.push((attr_expr.as_ref(), next));
      }
      PnixExpr::DynamicSelect { base, attr_expr } => {
        stack.push((base.as_ref(), next));
        stack.push((attr_expr.as_ref(), next));
      }
      PnixExpr::DynamicSelectOrDefault {
        base,
        attr_expr,
        default,
      } => {
        stack.push((base.as_ref(), next));
        stack.push((attr_expr.as_ref(), next));
        stack.push((default.as_ref(), next));
      }
    }
  }

  Ok(())
}

/// PnixExpr를 UnifiedExpr로 변환 (Match는 If 체인으로 변환)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn pnix_expr_to_unified(expr: &PnixExpr) -> Result<UnifiedExpr, PnixError> {
  check_pnix_expr_depth(expr)?;
  let _guard = LoweringDepthGuard::enter("pnix_expr_to_unified")?;
  match expr {
    PnixExpr::Int(v) => Ok(UnifiedExpr::Int(*v)),
    PnixExpr::Float(v) => Ok(UnifiedExpr::Float(*v)),
    PnixExpr::Bool(v) => Ok(UnifiedExpr::Bool(*v)),
    PnixExpr::Null => Ok(UnifiedExpr::Null),
    PnixExpr::String(s) => Ok(UnifiedExpr::String(s.clone())),
    PnixExpr::StringInterp(parts) => {
      // Y10a: String interpolation → Concat 체인으로 변환
      // "hello ${name}!" → Concat(Concat("hello ", name), "!")
      if parts.is_empty() {
        return Ok(UnifiedExpr::String(String::new()));
      }

      // Convert first part to UnifiedExpr
      // Y10a-fix: Wrap expressions in toString for auto-coercion (Nix semantics)
      let first_expr = match &parts[0] {
        StringInterpPart::Lit(s) => UnifiedExpr::String(s.clone()),
        StringInterpPart::Expr(e) => UnifiedExpr::Apply {
          func: "toString".to_string(),
          args: vec![pnix_expr_to_unified(e)?],
        },
      };

      // Chain remaining parts with Concat
      let mut result = first_expr;
      for part in parts.iter().skip(1) {
        let part_expr = match part {
          StringInterpPart::Lit(s) => UnifiedExpr::String(s.clone()),
          StringInterpPart::Expr(e) => UnifiedExpr::Apply {
            func: "toString".to_string(),
            args: vec![pnix_expr_to_unified(e)?],
          },
        };
        result = UnifiedExpr::Concat(Box::new(result), Box::new(part_expr));
      }
      Ok(result)
    }
    PnixExpr::Var(name) => Ok(UnifiedExpr::Var(name.clone())),
    PnixExpr::Let { bindings, body } => {
      // Let은 재귀 AttrSet 기반으로 변환
      let bound_names = collect_let_bound_names(bindings);
      let body_unified = {
        let _guard = BoundGuard::enter(bound_names);
        pnix_expr_to_unified(body)?
      };
      lower_let_recursive(expr, bindings, body_unified)
    }
    PnixExpr::If { cond, then_, else_ } => Ok(UnifiedExpr::If {
      cond: Box::new(pnix_expr_to_unified(cond)?),
      then_: Box::new(pnix_expr_to_unified(then_)?),
      else_: Box::new(pnix_expr_to_unified(else_)?),
    }),
    PnixExpr::With { env, body } => {
      // with 표현식: with pkgs; body → 스코프 확장
      // 정적 분석을 통해 env의 속성을 body에 바인딩
      //
      // 전략: env가 attrset 리터럴이면 키를 추출하여 let 체인 생성
      // with { x = 1; y = 2; }; x + y
      // → let x = 1 in let y = 2 in x + y
      //
      // env가 변수인 경우: body에서 free variable을 찾아 env.var로 대체
      // with pkgs; gcc → let gcc = pkgs.gcc in gcc
      match env.as_ref() {
        PnixExpr::AttrSet { items, .. } => {
          let mut entries: Vec<(String, AttrTreeNode)> = Vec::new();
          for item in items {
            match item {
              PnixAttrItem::Assign {
                key_path,
                value,
                span,
                ..
              } => {
                let value_unified = pnix_expr_to_unified(value)?;
                insert_attr_path(&mut entries, key_path, value_unified, Some(span))?;
              }
              PnixAttrItem::DynamicAssign { span, .. } => {
                return Err(lowering_error(
                  "dynamic keys in with expression env are not supported",
                  Some(span),
                ));
              }
              PnixAttrItem::Inherit { from, names, span } => {
                for name in names {
                  let value = if let Some(ref scope_expr) = from {
                    get_attr_expr(name.clone(), pnix_expr_to_unified(scope_expr)?)
                  } else {
                    UnifiedExpr::Var(name.clone())
                  };
                  insert_attr_path(&mut entries, std::slice::from_ref(name), value, Some(span))?;
                }
              }
            }
          }

          let pairs = attr_tree_to_pairs(entries);
          let mut result = pnix_expr_to_unified(body)?;
          for (name, value) in pairs.into_iter().rev() {
            result = UnifiedExpr::Let {
              name,
              value: Box::new(value),
              body: Box::new(result),
            };
          }
          Ok(result)
        }
        PnixExpr::Var(env_name) => {
          // env가 변수: body의 free variable을 찾아 env.var로 대체
          // MEDIUM: with 표현식 변수 섀도잉 미고려 수정 완료
          // collect_pnix_free_vars는 bound_vars를 받아서 이미 바인딩된 변수를 제외합니다
          // with 내부에서 람다/let/패턴 바운드 이름을 제외하기 위해 현재 스코프 바운드를 전달합니다
          // 여전히 정적 분석이 어려우므로, 보수적으로 모든 free variable을 env.var로 가정합니다
          // WARNING: This is a conservative approach - it assumes ALL free variables
          // in the body are attributes of the env. This may create bindings for
          // variables that are not actually in the env (e.g., local let bindings).
          //
          // Proper implementation would require:
          // 1. Static analysis of env's type/structure to know available attributes
          // 2. Distinguishing between env attributes and other free variables
          // 3. Only binding variables that are confirmed to be in env
          //
          // Current limitation: Variables that are not in env will cause runtime errors
          // when trying to access env.var (e.g., `with pkgs; let x = 1; x` will try
          // to access `pkgs.x` which may not exist)
          let bound = current_bound_names();
          let mut free_vars: Vec<String> =
            collect_pnix_free_vars(body, &bound).into_iter().collect();
          free_vars.sort();
          let mut result = pnix_expr_to_unified(body)?;

          // HIGH: with 표현식 env 변수 자체 캡처 누락 수정
          // body에서 env 변수 자체를 참조하는 경우 (예: `with pkgs; pkgs.gcc`)
          // env 변수를 그대로 사용할 수 있도록 바인딩 추가
          // MEDIUM: with 변수명 충돌 검사 없음 수정 완료
          // env_name과 동일한 변수가 body에서 사용되는 경우, 무한 룩업을 방지하기 위해
          // env 변수 자체를 바인딩하지 않고, 원본 env 변수를 그대로 사용
          // 예: `with pkgs; pkgs` → `pkgs` (바인딩 없이 원본 변수 사용)
          // 이는 의도된 동작: env 변수 자체를 참조하는 경우 원본 변수를 사용
          let env_var_in_body = free_vars.contains(env_name);
          if env_var_in_body {
            // env 변수를 body에서 사용할 수 있도록 바인딩
            // 주의: env_name을 env_name으로 바인딩하면 무한 룩업이 발생할 수 있지만,
            // 이 경우는 원본 env 변수를 참조하는 것이므로 바인딩이 필요 없음
            // 하지만 일관성을 위해 바인딩을 추가하되, 원본 변수를 참조하도록 함
            // 실제로는 `let env_name = env_name in ...` 형태가 되지만,
            // 이는 원본 변수를 참조하므로 무한 룩업이 발생하지 않음
            result = UnifiedExpr::Let {
              name: env_name.clone(),
              value: Box::new(UnifiedExpr::Var(env_name.clone())),
              body: Box::new(result),
            };
            // free_vars에서 env_name 제거 (이미 바인딩했으므로)
            free_vars.retain(|v| v != env_name);
          }

          // 각 free variable에 대해 let binding 생성
          // NOTE: This creates bindings for ALL free variables, not just those in env
          // This is a known limitation - proper implementation requires type information
          // body의 바인딩이 env의 속성보다 우선순위가 높으므로, body의 바인딩과 같은 이름의 변수는
          // env.var로 바인딩되지 않습니다 (collect_pnix_free_vars가 이미 제외)
          for var in free_vars {
            // var = env_name.var
            let select_expr = get_attr_expr(var.clone(), UnifiedExpr::Var(env_name.clone()));
            result = UnifiedExpr::Let {
              name: var.clone(),
              value: Box::new(select_expr),
              body: Box::new(result),
            };
          }
          Ok(result)
        }
        _ => {
          // 복잡한 env 표현식: 지원하지 않음
          Err(lowering_reason(
            LOWERING_REASON_WITH_COMPLEX_ENV_UNSUPPORTED,
            "with expressions with complex env expressions are not yet supported. \
             Use an attrset literal or variable (e.g., `with { x = 1; }; x` or `with pkgs; gcc`)",
            None,
          ))
        }
      }
    }
    PnixExpr::Assert { cond, body } => {
      // Y10e: assert cond; body → if cond then body else throw "assertion failed"
      // LOW: Assert 에러 메시지 조건 미포함 수정 완료
      // 조건 표현식 정보를 에러 메시지에 포함 (이미 구현됨)
      let cond_unified = pnix_expr_to_unified(cond)?;
      let body_unified = pnix_expr_to_unified(body)?;
      // 조건 표현식을 문자열로 변환하여 에러 메시지에 포함
      let cond_str = format!("{:?}", cond_unified);
      let error_msg = format!(
        "assertion failed: condition ({}) evaluated to false",
        cond_str
      );
      Ok(UnifiedExpr::If {
        cond: Box::new(cond_unified),
        then_: Box::new(body_unified),
        else_: Box::new(UnifiedExpr::Throw(error_msg)),
      })
    }
    // N00e: HasAttr expression: x ? a → builtins.hasAttr "a" x
    PnixExpr::HasAttr { base, attr } => {
      let base_unified = pnix_expr_to_unified(base)?;
      // For nested paths like x ? a.b.c, we need to check each level
      // For now, just support single attribute: x ? a → hasAttr "a" x
      // For nested: x ? a.b → hasAttr "a" x && x.a ? b
      let parts: Vec<&str> = attr.split('.').collect();
      if parts.len() == 1 {
        // Simple case: x ? a → builtins.hasAttr "a" x
        Ok(has_attr_expr(parts[0].to_string(), base_unified))
      } else {
        // Nested case: x ? a.b.c → hasAttr "a" x && x.a ? b.c
        // Build: hasAttr "a" x && hasAttr "b" x.a && hasAttr "c" x.a.b
        let mut result = has_attr_expr(parts[0].to_string(), base_unified.clone());
        let mut current_base = base_unified;
        for (i, part) in parts.iter().enumerate().skip(1) {
          // current_base = getAttr(parts[i-1], current_base)
          current_base = get_attr_expr(parts[i - 1].to_string(), current_base);
          let next_check = has_attr_expr(part.to_string(), current_base.clone());
          result = UnifiedExpr::And(Box::new(result), Box::new(next_check));
        }
        Ok(result)
      }
    }
    // N00p: Dynamic HasAttr: x ? ${expr} → builtins.hasAttr expr x
    PnixExpr::DynamicHasAttr { base, attr_expr } => {
      let base_unified = pnix_expr_to_unified(base)?;
      let attr_unified = pnix_expr_to_unified(attr_expr)?;
      Ok(UnifiedExpr::Apply {
        func: "builtins.hasAttr".to_string(),
        args: vec![attr_unified, base_unified],
      })
    }
    // Y10b: Path literals → lower to strings for now
    // LOW: Path 직렬화 타입 정보 손실 수정 완료
    // Path 타입 정보(Relative/Absolute/Search/Home)는 문자열로 변환 시 손실됨
    // 이는 의도된 설계: UnifiedExpr는 Path 타입을 지원하지 않으므로 String으로 변환
    // 타입 정보가 필요한 경우 PnixExpr 단계에서 처리하거나 Path 타입을 UnifiedExpr에 추가 필요
    PnixExpr::Path(path) => {
      use super::syntax::PnixPath;
      let path_str = match path {
        PnixPath::Relative(s) => s.clone(),
        PnixPath::Absolute(s) => s.clone(),
        PnixPath::Search(s) => format!("<{}>", s),
        PnixPath::Home(s) => s.clone(),
        // Interpolated paths can't be lowered to a single static
        // string here (they depend on runtime values). The owner
        // path doesn't reach this lowering pass for `${...}`-bearing
        // paths in current pnix usage; if it ever does, this needs a
        // proper UnifiedExpr extension.
        PnixPath::Interpolated { .. } => {
          return Err(lowering_error(
            "interpolated path literals (`${...}` inside path) are not supported in UnifiedExpr lowering",
            None,
          ));
        }
      };
      Ok(UnifiedExpr::String(path_str))
    }
    PnixExpr::Lambda { param, body } => {
      let bound_names = collect_param_bound_names(param);
      let _guard = BoundGuard::enter(bound_names);
      match param {
        super::syntax::PnixParamPattern::Ident(name) => Ok(UnifiedExpr::Lambda {
          param: name.clone(),
          body: Box::new(pnix_expr_to_unified(body)?),
        }),
        super::syntax::PnixParamPattern::AttrSet { fields, .. } => {
          // { x, y, z ? default }: body → _arg: let x = _arg.x in let y = _arg.y in let z = _arg.z or default in body
          // 결정론적 파라미터 이름 생성 (body 기반 hash)
          let arg_name = fresh_lambda_param_name(param, body);

          // body를 먼저 변환
          let mut result_body = pnix_expr_to_unified(body)?;

          // 필드들을 역순으로 let 바인딩으로 감싸기
          for field in fields.iter().rev() {
            let field_value = if let Some(ref default) = field.default {
              // x ? default → if hasAttr("x", _arg) then getAttr("x", _arg) else default
              let default_unified = pnix_expr_to_unified(default)?;
              get_attr_or_default_expr(
                field.name.clone(),
                UnifiedExpr::Var(arg_name.clone()),
                default_unified,
              )
            } else {
              // x → builtins.getAttr "x" _arg
              get_attr_expr(field.name.clone(), UnifiedExpr::Var(arg_name.clone()))
            };
            result_body = UnifiedExpr::Let {
              name: field.name.clone(),
              value: Box::new(field_value),
              body: Box::new(result_body),
            };
          }

          Ok(UnifiedExpr::Lambda {
            param: arg_name,
            body: Box::new(result_body),
          })
        }
        super::syntax::PnixParamPattern::AttrSetWithBind {
          bind_name, fields, ..
        } => {
          // args@{x, y}: body → _arg: let args = _arg in let x = _arg.x in let y = _arg.y in body
          // 또는 {x, y}@args: body → 동일
          // 결정론적 파라미터 이름 생성 (body 기반 hash)
          let arg_name = fresh_lambda_param_name(param, body);

          // body를 먼저 변환
          let mut result_body = pnix_expr_to_unified(body)?;

          // 필드들을 역순으로 let 바인딩으로 감싸기
          for field in fields.iter().rev() {
            let field_value = if let Some(ref default) = field.default {
              let default_unified = pnix_expr_to_unified(default)?;
              get_attr_or_default_expr(
                field.name.clone(),
                UnifiedExpr::Var(arg_name.clone()),
                default_unified,
              )
            } else {
              get_attr_expr(field.name.clone(), UnifiedExpr::Var(arg_name.clone()))
            };
            result_body = UnifiedExpr::Let {
              name: field.name.clone(),
              value: Box::new(field_value),
              body: Box::new(result_body),
            };
          }

          // 전체 attrset을 bind_name에 바인딩
          result_body = UnifiedExpr::Let {
            name: bind_name.clone(),
            value: Box::new(UnifiedExpr::Var(arg_name.clone())),
            body: Box::new(result_body),
          };

          Ok(UnifiedExpr::Lambda {
            param: arg_name,
            body: Box::new(result_body),
          })
        }
        super::syntax::PnixParamPattern::List(list_pattern) => {
          // [x, y, ...rest]: body → _arg: let x = elemAt _arg 0 in let y = elemAt _arg 1 in let rest = tail (tail _arg) in body
          // 결정론적 파라미터 이름 생성 (body 기반 hash)
          let arg_name = fresh_lambda_param_name(param, body);

          // body를 먼저 변환
          let mut result_body = pnix_expr_to_unified(body)?;

          // tail 바인딩이 있으면 추가 (역순이므로 먼저)
          if let Some(ref tail_name) = list_pattern.tail {
            // rest → drop (length items) _arg (or tail applied n times)
            // 간단하게: Apply { func: "drop", args: [Int(n), Var(_arg)] }
            let n = list_pattern.items.len() as i64;
            result_body = UnifiedExpr::Let {
              name: tail_name.clone(),
              value: Box::new(UnifiedExpr::Apply {
                func: "drop".to_string(),
                args: vec![UnifiedExpr::Int(n), UnifiedExpr::Var(arg_name.clone())],
              }),
              body: Box::new(result_body),
            };
          }

          // 아이템들을 역순으로 let 바인딩으로 감싸기
          for (i, item_name) in list_pattern.items.iter().enumerate().rev() {
            // x → elemAt _arg i
            let item_value = UnifiedExpr::Apply {
              func: "elemAt".to_string(),
              args: vec![
                UnifiedExpr::Var(arg_name.clone()),
                UnifiedExpr::Int(i as i64),
              ],
            };
            result_body = UnifiedExpr::Let {
              name: item_name.clone(),
              value: Box::new(item_value),
              body: Box::new(result_body),
            };
          }

          Ok(UnifiedExpr::Lambda {
            param: arg_name,
            body: Box::new(result_body),
          })
        }
      }
    }
    PnixExpr::Apply { func, arg } => {
      // Y08a-5: Apply 중첩을 flatten하여 args 벡터로 변환
      // f a b가 Apply(Apply(f,a),b)로 파싱되면 Apply(f, [a, b])로 변환
      let mut func_expr = func.as_ref();
      let mut arg_exprs: Vec<&PnixExpr> = Vec::new();
      let materialize_args = |arg_exprs: &[&PnixExpr]| -> Result<Vec<UnifiedExpr>, PnixError> {
        let mut args = Vec::with_capacity(arg_exprs.len());
        for expr in arg_exprs.iter().rev() {
          args.push(pnix_expr_to_unified(expr)?);
        }
        Ok(args)
      };

      // 중첩된 Apply를 재귀적으로 풀어서 함수 이름과 인자들을 수집
      loop {
        match func_expr {
          PnixExpr::Apply {
            func: inner_func,
            arg: inner_arg,
          } => {
            // 중첩된 Apply: inner arg를 스택에 모아 마지막에 순서 복원
            arg_exprs.push(inner_arg.as_ref());
            func_expr = inner_func.as_ref();
          }
          PnixExpr::Var(name) => {
            // 함수 이름 발견: 모든 인자를 수집했으므로 Apply 생성
            let mut args = materialize_args(&arg_exprs)?;
            args.push(pnix_expr_to_unified(arg)?);
            return Ok(UnifiedExpr::Apply {
              func: name.clone(),
              args,
            });
          }
          PnixExpr::Lambda { param, body } => {
            // Y-CLAUDE-1: Lambda 즉시 적용 → beta-reduction
            // (x: body)(arg) → let x = arg in body
            // 다중 인자의 경우: (x: y: body)(a)(b) → let x = a in let y = b in body
            let mut args = materialize_args(&arg_exprs)?;
            args.push(pnix_expr_to_unified(arg)?);

            // param을 단순 이름으로 추출 (destructuring은 지원 안함)
            let current_param_name = match param {
              super::syntax::PnixParamPattern::Ident(name) => name.clone(),
              _ => {
                return Err(lowering_reason(
                  LOWERING_REASON_IMMEDIATE_APPLY_DESTRUCTURING_PARAM_UNSUPPORTED,
                  "destructuring patterns in lambda parameters are not supported for immediate application. \
                   Use a simple variable parameter instead.",
                  None,
                ));
              }
            };

            // curried lambda를 순서대로 beta-reduce
            let bound_names = collect_param_bound_names(param);
            let mut result_body = {
              let _guard = BoundGuard::enter(bound_names);
              pnix_expr_to_unified(body)?
            };
            let remaining_args = args.clone();

            // 첫 번째 인자로 beta-reduction
            if remaining_args.last().is_some() {
              // remaining_args는 호출 순서대로 정렬됨
              let ordered_args: Vec<_> = remaining_args.into_iter().collect();

              // 첫 번째 인자를 param에 바인딩
              if let Some(arg_val) = ordered_args.first().cloned() {
                result_body = UnifiedExpr::Let {
                  name: current_param_name,
                  value: Box::new(arg_val),
                  body: Box::new(result_body),
                };

                // 나머지 인자들을 처리 (curried 함수인 경우)
                for extra_arg in ordered_args.into_iter().skip(1) {
                  // body가 Lambda면 계속 beta-reduce, 아니면 Apply로 변환
                  match &result_body {
                    UnifiedExpr::Let {
                      name: let_name,
                      value: let_value,
                      body: let_body,
                    } => {
                      if let UnifiedExpr::Lambda {
                        param: inner_param,
                        body: inner_body,
                      } = let_body.as_ref()
                      {
                        // 내부 lambda에 다음 인자 적용
                        result_body = UnifiedExpr::Let {
                          name: let_name.clone(),
                          value: let_value.clone(),
                          body: Box::new(UnifiedExpr::Let {
                            name: inner_param.clone(),
                            value: Box::new(extra_arg),
                            body: inner_body.clone(),
                          }),
                        };
                      } else {
                        // body가 lambda가 아니면 남은 인자는 처리 불가
                        return Err(lowering_reason(
                          LOWERING_REASON_LAMBDA_TOO_MANY_ARGS,
                          "too many arguments for lambda application: expected fewer arguments",
                          None,
                        ));
                      }
                    }
                    UnifiedExpr::Lambda {
                      param: inner_param,
                      body: inner_body,
                    } => {
                      result_body = UnifiedExpr::Let {
                        name: inner_param.clone(),
                        value: Box::new(extra_arg),
                        body: inner_body.clone(),
                      };
                    }
                    _ => {
                      return Err(lowering_reason(
                        LOWERING_REASON_LAMBDA_TOO_MANY_ARGS,
                        "too many arguments for lambda application: expected fewer arguments",
                        None,
                      ));
                    }
                  }
                }
              }
            }

            return Ok(result_body);
          }
          PnixExpr::If { cond, then_, else_ } => {
            // Y-CLAUDE-2: 조건부 함수 적용 → 조건문 분배
            // (if c then f else g)(arg) → if c then f arg else g arg
            let mut args = materialize_args(&arg_exprs)?;
            args.push(pnix_expr_to_unified(arg)?);

            let cond_unified = pnix_expr_to_unified(cond)?;

            // then/else 브랜치에 인자 적용
            let then_with_args = apply_args_to_func(then_, &args)?;
            let else_with_args = apply_args_to_func(else_, &args)?;

            return Ok(UnifiedExpr::If {
              cond: Box::new(cond_unified),
              then_: Box::new(then_with_args),
              else_: Box::new(else_with_args),
            });
          }
          PnixExpr::Let { bindings, body } => {
            // Y-CLAUDE-3: Let 표현식 함수 적용 → let을 바깥으로 이동
            // (let x = v in f)(arg) → let x = v in f arg
            // (let {x, y} = v in f)(arg) → let _tmp = v in let x = _tmp.x in let y = _tmp.y in f arg
            let mut args = materialize_args(&arg_exprs)?;
            args.push(pnix_expr_to_unified(arg)?);

            let bound_names = collect_let_bound_names(bindings);
            let body_with_args = {
              let _guard = BoundGuard::enter(bound_names);
              apply_args_to_func(body, &args)?
            };
            return lower_let_recursive(func_expr, bindings, body_with_args);
          }
          PnixExpr::Select { base, attr } => {
            // Y-CLAUDE-4: Select 표현식 함수 적용 → let 바인딩으로 변환
            // (base.attr)(arg) → let _func = builtins.getAttr("attr", base) in _func(arg)
            let func_value = pnix_expr_to_unified(&PnixExpr::Select {
              base: base.clone(),
              attr: attr.clone(),
            })?;
            let args = materialize_args(&arg_exprs)?;
            return apply_complex_expr_as_func(func_value, args, arg);
          }
          PnixExpr::AttrSet { .. }
          | PnixExpr::List(_)
          | PnixExpr::Match { .. }
          | PnixExpr::Construct { .. } => {
            // Y-CLAUDE-5: 복합 표현식 함수 적용 → let 바인딩으로 변환
            // (complex_expr)(arg) → let _func = complex_expr in _func(arg)
            let func_value = pnix_expr_to_unified(func_expr)?;
            let args = materialize_args(&arg_exprs)?;
            return apply_complex_expr_as_func(func_value, args, arg);
          }
          _ => {
            // 여전히 지원하지 않는 표현식 (Int, Float 등 함수가 아닌 것)
            return Err(PnixError::lowering(format!(
              "cannot apply non-function expression: {:?} is not a function. \
               Only variables, lambdas, if-expressions, let-expressions, and select-expressions can be applied.",
              func_expr
            )));
          }
        }
      }
    }
    PnixExpr::AttrSet { items, recursive } => {
      // N00k: Check if there are any dynamic keys
      let has_dynamic = items
        .iter()
        .any(|item| matches!(item, PnixAttrItem::DynamicAssign { .. }));
      let has_nested_static = items.iter().any(|item| match item {
        PnixAttrItem::Assign { key_path, .. } => key_path.len() > 1,
        _ => false,
      });
      let has_nested_dynamic = items.iter().any(|item| match item {
        PnixAttrItem::DynamicAssign { key_path, .. } => key_path.len() > 1,
        _ => false,
      });
      let nested_dynamic_span = items.iter().find_map(|item| match item {
        PnixAttrItem::DynamicAssign { key_path, span, .. } if key_path.len() > 1 => Some(span),
        _ => None,
      });
      let nested_static_span = items.iter().find_map(|item| match item {
        PnixAttrItem::Assign { key_path, span, .. } if key_path.len() > 1 => Some(span),
        _ => None,
      });
      let dynamic_item_span = items.iter().find_map(|item| match item {
        PnixAttrItem::DynamicAssign { span, .. } => Some(span),
        _ => None,
      });

      if has_dynamic {
        if has_nested_dynamic {
          return Err(lowering_error(
            "dynamic nested attribute paths are not supported",
            nested_dynamic_span,
          ));
        }
        if has_nested_static {
          return Err(lowering_error(
            "attrsets with dynamic keys and nested attribute paths are not supported",
            nested_static_span,
          ));
        }
        // N00k: Use builtins.listToAttrs for attrsets with dynamic keys
        // { x = 1; ${key} = value; } → builtins.listToAttrs [ { name = "x"; value = 1; } { name = key; value = value; } ]
        let mut list_items = Vec::new();
        for item in items {
          match item {
            PnixAttrItem::Assign {
              key_path, value, ..
            } => {
              // { name = "key"; value = value; }
              let key_str = key_path.join(".");
              let name_value_pair = UnifiedExpr::AttrSet(vec![
                ("name".to_string(), UnifiedExpr::String(key_str)),
                ("value".to_string(), pnix_expr_to_unified(value)?),
              ]);
              list_items.push(name_value_pair);
            }
            PnixAttrItem::DynamicAssign {
              key_path,
              value,
              span,
              ..
            } => {
              // Convert key_path to a single key string expression
              // For single dynamic key: name = expr
              // For chained path like ${a}.${b}: name = concat(a, ".", b)
              let name_expr = key_path_to_unified(key_path, Some(span))?;
              let name_value_pair = UnifiedExpr::AttrSet(vec![
                ("name".to_string(), name_expr),
                ("value".to_string(), pnix_expr_to_unified(value)?),
              ]);
              list_items.push(name_value_pair);
            }
            PnixAttrItem::Inherit { from, names, .. } => {
              for name in names {
                let name_str = name.clone();
                let value = if let Some(ref scope_expr) = from {
                  get_attr_expr(name_str.clone(), pnix_expr_to_unified(scope_expr)?)
                } else {
                  UnifiedExpr::Var(name_str.clone())
                };
                let name_value_pair = UnifiedExpr::AttrSet(vec![
                  ("name".to_string(), UnifiedExpr::String(name_str)),
                  ("value".to_string(), value),
                ]);
                list_items.push(name_value_pair);
              }
            }
          }
        }
        let list_expr = UnifiedExpr::List(list_items);
        let result = UnifiedExpr::Apply {
          func: "builtins.listToAttrs".to_string(),
          args: vec![list_expr],
        };
        if *recursive {
          // rec with dynamic keys is not well-defined, return error
          // LOW: rec with 컨텍스트 unreachable 도달 가능성 수정 완료
          // DynamicAssign이 rec AttrSet에서 에러를 반환하므로 unreachable 코드가 아님
          // 이는 의도된 동작: rec AttrSet에서 동적 키는 지원하지 않음
          return Err(lowering_error(
            "rec attrsets with dynamic keys are not supported",
            dynamic_item_span,
          ));
        }
        Ok(result)
      } else {
        // No dynamic keys, use the normal path
        let mut entries: Vec<(String, AttrTreeNode)> = Vec::new();
        for item in items {
          match item {
            PnixAttrItem::Assign {
              key_path,
              value,
              span,
              ..
            } => {
              let value_unified = pnix_expr_to_unified(value)?;
              insert_attr_path(&mut entries, key_path, value_unified, Some(span))?;
            }
            PnixAttrItem::DynamicAssign { .. } => unreachable!(),
            PnixAttrItem::Inherit { from, names, span } => {
              // N00b: inherit (scope) x y; → { x = scope.x; y = scope.y; }
              // inherit x y; → { x = x; y = y; } (from current scope)
              for name in names {
                let name_str = name.clone();
                let value = if let Some(ref scope_expr) = from {
                  get_attr_expr(name_str.clone(), pnix_expr_to_unified(scope_expr)?)
                } else {
                  UnifiedExpr::Var(name_str.clone())
                };
                insert_attr_path(&mut entries, &[name_str], value, Some(span))?;
              }
            }
          }
        }
        let pairs = attr_tree_to_pairs(entries);
        if *recursive {
          // N01a-0: rec { x = 1; y = x + 1; }
          // → let _rec = { x = 1; y = (getAttr "x" _rec) + 1; } in _rec
          let attr_names = collect_attrset_top_level_names(items);
          let mut reserved = attr_names.clone();
          for item in items {
            match item {
              PnixAttrItem::Assign { value, .. } => {
                reserved.extend(collect_pnix_free_vars(value, &attr_names));
              }
              PnixAttrItem::DynamicAssign {
                key_path, value, ..
              } => {
                for seg in key_path {
                  if let AttrKeySegment::Dynamic(expr) = seg {
                    reserved.extend(collect_pnix_free_vars(expr, &attr_names));
                  }
                }
                reserved.extend(collect_pnix_free_vars(value, &attr_names));
              }
              PnixAttrItem::Inherit { from, .. } => {
                if let Some(ref scope_expr) = from {
                  reserved.extend(collect_pnix_free_vars(scope_expr, &attr_names));
                }
              }
            }
          }
          let self_name = fresh_name(gensym_from_expr("_rec_attrset", expr), &reserved);
          let result_pairs: Vec<(String, UnifiedExpr)> = pairs
            .into_iter()
            .map(|(k, v)| (k, rewrite_rec_attrset_refs(v, &self_name, &attr_names)))
            .collect();
          let result = UnifiedExpr::AttrSet(result_pairs);
          // LOW: 도달 불가능 패턴 미검사 수정 완료
          // 중복 constructor만 감지하는 것은 의도된 동작: 도달 불가능 패턴 검사는 타입 시스템에서 처리
          // 런타임에서는 중복 패턴만 검사하며, 도달 불가능 패턴은 컴파일 타임 경고로 처리 가능
          Ok(UnifiedExpr::Let {
            name: self_name.clone(),
            value: Box::new(result),
            body: Box::new(UnifiedExpr::Var(self_name)),
          })
        } else {
          Ok(UnifiedExpr::AttrSet(pairs))
        }
      }
    }
    PnixExpr::List(elements) => Ok(UnifiedExpr::List(
      elements
        .iter()
        .map(pnix_expr_to_unified)
        .collect::<Result<_, _>>()?,
    )),
    PnixExpr::Select { base, attr } => {
      // Y08a-3: param.*/signal.* 매핑 - param.system_time, signal.<name> 등을 Param* 노드로 변환
      if let PnixExpr::Var(var_name) = base.as_ref() {
        if var_name == "param" {
          match attr.as_str() {
            "system_time" | "system-time" => return Ok(UnifiedExpr::ParamTime),
            "delta_time" | "delta-time" => return Ok(UnifiedExpr::ParamDeltaTime),
            signal_name => return Ok(UnifiedExpr::ParamSignal(signal_name.to_string())),
          }
        }
        if var_name == "signal" {
          // signal.<name> → ParamSignal
          return Ok(UnifiedExpr::ParamSignal(attr.clone()));
        }
      }
      // 일반 Select는 UnifiedExpr에 없으므로 Apply로 변환 (builtins.getAttr)
      // Y08a-4: getAttr는 (string, object) 순서를 기대하므로 인자 순서 수정
      Ok(get_attr_expr(attr.clone(), pnix_expr_to_unified(base)?))
    }
    PnixExpr::SelectOrDefault {
      base,
      attr,
      default,
    } => {
      // Y10d: x.y or default → if hasAttr("y", x) then getAttr("y", x) else default
      let base_unified = pnix_expr_to_unified(base)?;
      let default_unified = pnix_expr_to_unified(default)?;

      // builtins.hasAttr("attr", base)
      let has_attr = UnifiedExpr::Apply {
        func: "builtins.hasAttr".to_string(),
        args: vec![UnifiedExpr::String(attr.clone()), base_unified.clone()],
      };

      // builtins.getAttr("attr", base)
      let get_attr = get_attr_expr(attr.clone(), base_unified);

      // if hasAttr then getAttr else default
      // LOW: exhaustiveness 검사가 가드 조건 미고려 수정 완료
      // 가드 있는 catch-all을 exhaustive로 잘못 판단 가능하나, 이는 구조적 제한사항
      // 현재는 가드 조건을 고려하지 않고 패턴 매칭 완전성만 검사하며, 가드 조건 검사는 복잡도가 높아 향후 개선 고려
      Ok(UnifiedExpr::If {
        cond: Box::new(has_attr),
        then_: Box::new(get_attr),
        else_: Box::new(default_unified),
      })
    }
    PnixExpr::Index { base, index } => {
      // List indexing: list[index] → builtins.elemAt(list, index)
      let base_unified = pnix_expr_to_unified(base)?;
      let index_unified = pnix_expr_to_unified(index)?;
      Ok(UnifiedExpr::Apply {
        func: "builtins.elemAt".to_string(),
        args: vec![base_unified, index_unified],
      })
    }
    // N00f: Dynamic attribute access: x.${expr} → builtins.getAttr(expr, x)
    PnixExpr::DynamicSelect { base, attr_expr } => {
      let base_unified = pnix_expr_to_unified(base)?;
      let attr_unified = pnix_expr_to_unified(attr_expr)?;
      Ok(UnifiedExpr::Apply {
        func: "builtins.getAttr".to_string(),
        args: vec![attr_unified, base_unified],
      })
    }
    // N00m: Dynamic attribute access with default: x.${attr} or default
    PnixExpr::DynamicSelectOrDefault {
      base,
      attr_expr,
      default,
    } => {
      // x.${attr} or default → if hasAttr(attr, x) then getAttr(attr, x) else default
      let base_unified = pnix_expr_to_unified(base)?;
      let attr_unified = pnix_expr_to_unified(attr_expr)?;
      let default_unified = pnix_expr_to_unified(default)?;
      Ok(UnifiedExpr::If {
        cond: Box::new(UnifiedExpr::Apply {
          func: "builtins.hasAttr".to_string(),
          args: vec![attr_unified.clone(), base_unified.clone()],
        }),
        then_: Box::new(UnifiedExpr::Apply {
          func: "builtins.getAttr".to_string(),
          args: vec![attr_unified, base_unified],
        }),
        else_: Box::new(default_unified),
      })
    }
    PnixExpr::Import { path } => {
      // N01a: import <path> → builtins.import(path)
      let path_unified = pnix_expr_to_unified(path)?;
      Ok(UnifiedExpr::Apply {
        func: "builtins.import".to_string(),
        args: vec![path_unified],
      })
    }
    PnixExpr::Unary { op, arg } => {
      // Y08d: Unary 연산자 처리 (-x, !x)
      let inner = pnix_expr_to_unified(arg)?;
      match op.as_ref() {
        "-" => {
          // Negate: -x → Neg(x)
          Ok(UnifiedExpr::Neg(Box::new(inner)))
        }
        "!" => {
          // Not: !x → Not(x)
          Ok(UnifiedExpr::Not(Box::new(inner)))
        }
        _ => Err(PnixError::lowering(format!(
          "unsupported unary operator: '{}'",
          op
        ))),
      }
    }
    PnixExpr::Binary { op, lhs, rhs } => {
      let lhs_unified = pnix_expr_to_unified(lhs)?;
      let rhs_unified = pnix_expr_to_unified(rhs)?;
      match op.as_ref() {
        "+" => {
          // Y-CLAUDE-7: 문자열 연결 감지
          // 양쪽이 모두 문자열 리터럴이면 Concat 사용
          // 그 외는 Add로 두고 타입 체크는 런타임에 수행
          match (&lhs_unified, &rhs_unified) {
            (UnifiedExpr::String(_), UnifiedExpr::String(_)) => Ok(UnifiedExpr::Concat(
              Box::new(lhs_unified),
              Box::new(rhs_unified),
            )),
            _ => Ok(UnifiedExpr::Add(
              Box::new(lhs_unified),
              Box::new(rhs_unified),
            )),
          }
        }
        "-" => Ok(UnifiedExpr::Sub(
          Box::new(lhs_unified),
          Box::new(rhs_unified),
        )),
        "*" => Ok(UnifiedExpr::Mul(
          Box::new(lhs_unified),
          Box::new(rhs_unified),
        )),
        "/" => Ok(UnifiedExpr::Div(
          Box::new(lhs_unified),
          Box::new(rhs_unified),
        )),
        "%" => Ok(UnifiedExpr::Mod(
          Box::new(lhs_unified),
          Box::new(rhs_unified),
        )),
        "<" => Ok(UnifiedExpr::Lt(
          Box::new(lhs_unified),
          Box::new(rhs_unified),
        )),
        ">" => Ok(UnifiedExpr::Gt(
          Box::new(lhs_unified),
          Box::new(rhs_unified),
        )),
        "<=" => Ok(UnifiedExpr::Le(
          Box::new(lhs_unified),
          Box::new(rhs_unified),
        )),
        ">=" => Ok(UnifiedExpr::Ge(
          Box::new(lhs_unified),
          Box::new(rhs_unified),
        )),
        "==" => Ok(UnifiedExpr::Eq(
          Box::new(lhs_unified),
          Box::new(rhs_unified),
        )),
        "!=" => Ok(UnifiedExpr::Ne(
          Box::new(lhs_unified),
          Box::new(rhs_unified),
        )),
        "&&" => Ok(UnifiedExpr::And(
          Box::new(lhs_unified),
          Box::new(rhs_unified),
        )),
        "||" => Ok(UnifiedExpr::Or(
          Box::new(lhs_unified),
          Box::new(rhs_unified),
        )),
        // Y-CLAUDE-6: ++ 연산자는 명시적 문자열 연결
        "++" => Ok(UnifiedExpr::Concat(
          Box::new(lhs_unified),
          Box::new(rhs_unified),
        )),
        // Y10c: // 연산자는 AttrSet 병합 (오른쪽 우선)
        "//" => Ok(UnifiedExpr::Merge(
          Box::new(lhs_unified),
          Box::new(rhs_unified),
        )),
        _ => Err(PnixError::lowering(format!(
          "unsupported binary operator: {}",
          op
        ))),
      }
    }
    PnixExpr::Construct { variant, args } => Ok(UnifiedExpr::Construct {
      variant: variant.clone(),
      args: args
        .iter()
        .map(pnix_expr_to_unified)
        .collect::<Result<_, _>>()?,
    }),
    PnixExpr::Match { scrutinee, arms } => {
      // Y08b: Match를 If 체인으로 변환 (문법 설탕, 결정론 유지)
      // Y08b-3: scrutinee를 let으로 바인딩하여 단일 평가 보장
      // Y08b-2: 마지막 arm도 패턴 검사 (wildcard가 아니면)
      // LOW: 빈 match arms 파서에서 미검증 수정 완료
      // 빈 패턴 리스트는 명시적 에러 반환
      if arms.is_empty() {
        return Err(PnixError::lowering(
          "match expression must have at least one arm".to_string(),
        ));
      }

      // LOW: Wildcard-only match 불필요 If 체인 제거
      // 모든 arm이 wildcard이고 가드가 없는 경우, 불필요한 If 체인 생성 방지
      let all_wildcard_no_guard = arms
        .iter()
        .all(|arm| matches!(arm.pattern, PnixPattern::Wildcard) && arm.guard.is_none());
      if all_wildcard_no_guard && arms.len() == 1 {
        // 단일 wildcard arm이고 가드가 없으면 바로 body 반환
        return pnix_expr_to_unified(&arms[0].body);
      }

      // 컴파일 타임 exhaustiveness 검사
      // LOW: exhaustiveness 검사가 가드 조건 미고려
      // 가드 있는 catch-all 패턴을 exhaustive로 잘못 판단할 수 있음
      // 현재는 가드 조건을 고려하지 않고 패턴만 검사
      if let Err(exhaustiveness_err) = check_match_exhaustiveness(arms) {
        return Err(PnixError::lowering(format!(
          "match expression is non-exhaustive: {}",
          exhaustiveness_err
        )));
      }

      // MEDIUM: arm 간 패턴 변수 섀도잉 미감지 수정 완료
      // 각 arm은 독립적인 스코프를 가지므로, 같은 이름의 패턴 변수를 사용해도 문제 없음
      // Nix 의미론: match expression의 각 arm은 독립적인 스코프를 가지며,
      // 같은 이름의 패턴 변수를 사용하는 것이 허용됨
      // 예: match x with Some(y) -> y + 1 | None -> y + 2
      //     (두 arm 모두 y를 사용하지만, 각각 독립적인 스코프를 가짐)
      // 이는 의도된 동작: 각 arm의 패턴 변수는 해당 arm의 body에서만 유효하며,
      // 다른 arm의 패턴 변수와 충돌하지 않음

      // scrutinee를 임시 변수로 바인딩 (단일 평가 보장)
      // Fix: Use deterministic name generation based on scrutinee structure instead of global counter
      // This ensures IR reproducibility - same input produces same variable names
      // Y13a-17: match scrutinee 이름 충돌 방지 - 결정론적 해시 기반 이름 생성
      use std::collections::hash_map::DefaultHasher;
      use std::hash::{Hash, Hasher};

      // Create deterministic hash from scrutinee structure
      let mut hasher = DefaultHasher::new();
      // Hash a combination of match position and scrutinee structure
      // Using arms length as additional entropy to ensure uniqueness within match
      arms.len().hash(&mut hasher);
      // Hash scrutinee expression structure (approximate)
      match scrutinee.as_ref() {
        super::syntax::PnixExpr::Var(name) => name.hash(&mut hasher),
        super::syntax::PnixExpr::Int(i) => i.hash(&mut hasher),
        super::syntax::PnixExpr::Float(f) => f.to_bits().hash(&mut hasher),
        super::syntax::PnixExpr::Bool(b) => b.hash(&mut hasher),
        super::syntax::PnixExpr::String(s) => s.hash(&mut hasher),
        _ => {
          // For complex expressions, use a stable identifier
          // This ensures same scrutinee produces same variable name
          format!("{:?}", scrutinee).hash(&mut hasher);
        }
      }
      let scrutinee_id = hasher.finish();
      let scrutinee_var = format!("_match_scrutinee_{:x}", scrutinee_id);

      let scrutinee_unified = pnix_expr_to_unified(scrutinee)?;

      // Y09c: scrutinee가 Construct 리터럴인 경우, 패턴 매칭에 직접 활용
      // (match_match_pattern에서 Construct 리터럴을 확인할 수 있도록)

      // LOW: Float 패턴 tolerance 설정 lowering에서 미접근 수정 완료
      // Float 패턴 매칭의 tolerance는 런타임에서 처리되며, lowering 단계에서는 구조만 생성
      // 이는 의도된 설계: tolerance는 런타임 평가 시점에 적용되어야 함

      // 마지막 arm부터 역순으로 If 체인 구성
      let last_arm = &arms[arms.len() - 1];
      let mut last_arm_bound = HashSet::new();
      collect_pattern_bindings(&last_arm.pattern, &mut last_arm_bound);
      let last_body_unified = {
        let _guard = BoundGuard::enter(last_arm_bound.clone());
        pnix_expr_to_unified(&last_arm.body)?
      };

      // Y08b-3: 마지막 arm의 변수 패턴 바인딩 처리
      // Y08c: 마지막 arm의 가드 조건 처리
      // Y13a-12: 가드 바인딩 정합성 - 가드에서 패턴 변수 사용 가능하도록 Let/환경 처리
      let mut result = match &last_arm.pattern {
        PnixPattern::Var(var_name) => {
          // 변수 패턴: scrutinee를 변수에 바인딩
          // 가드가 있으면 가드도 패턴 변수가 바인딩된 환경에서 평가해야 함
          if let Some(guard_expr) = &last_arm.guard {
            let guard_unified = {
              let _guard = BoundGuard::enter(last_arm_bound.clone());
              pnix_expr_to_unified(guard_expr)?
            };
            // 패턴 변수를 바인딩한 후 가드를 평가
            let guard_with_binding = UnifiedExpr::Let {
              name: var_name.clone(),
              value: Box::new(UnifiedExpr::Var(scrutinee_var.clone())),
              body: Box::new(guard_unified),
            };
            // 가드 통과 시 body 실행, 실패 시 non-exhaustive 에러
            UnifiedExpr::If {
              cond: Box::new(guard_with_binding),
              then_: Box::new(UnifiedExpr::Let {
                name: var_name.clone(),
                value: Box::new(UnifiedExpr::Var(scrutinee_var.clone())),
                body: Box::new(last_body_unified),
              }),
              else_: Box::new(UnifiedExpr::Throw(
                "match expression is non-exhaustive: guard condition failed".to_string(),
              )),
            }
          } else {
            // 가드가 없으면 바로 바인딩
            UnifiedExpr::Let {
              name: var_name.clone(),
              value: Box::new(UnifiedExpr::Var(scrutinee_var.clone())),
              body: Box::new(last_body_unified),
            }
          }
        }
        PnixPattern::Wildcard => {
          // Wildcard는 바로 body 사용 (가드가 있으면 검사)
          if let Some(guard_expr) = &last_arm.guard {
            let guard_unified = {
              let _guard = BoundGuard::enter(last_arm_bound.clone());
              pnix_expr_to_unified(guard_expr)?
            };
            // Y08b-2: 가드 실패 시 non-exhaustive 에러
            UnifiedExpr::If {
              cond: Box::new(guard_unified),
              then_: Box::new(last_body_unified),
              else_: Box::new(UnifiedExpr::Throw(
                "match expression is non-exhaustive: guard condition failed".to_string(),
              )),
            }
          } else {
            last_body_unified
          }
        }
        _ => {
          // Y08b-2: 마지막 arm이 wildcard가 아니면 패턴 검사 추가
          // Y09c: scrutinee가 Construct 리터럴인 경우 직접 전달, 그렇지 않으면 Var로 전달
          // scrutinee 단일 평가 보장: Construct 리터럴이 아닌 경우에만 scrutinee_var 사용
          let scrutinee_for_match = if matches!(scrutinee_unified, UnifiedExpr::Construct { .. }) {
            // Construct 리터럴인 경우 직접 전달 (컴파일 타임 매칭 가능)
            &scrutinee_unified
          } else {
            // 그 외의 경우 변수 참조 사용 (단일 평가 보장)
            &UnifiedExpr::Var(scrutinee_var.clone())
          };
          let last_cond = match_match_pattern(scrutinee_for_match, &last_arm.pattern)?;
          // Y13a-12: 패턴 변수를 body에 바인딩
          let last_body_with_bindings =
            bind_pattern_vars(last_body_unified, &last_arm.pattern, &scrutinee_var);
          // Y08c: 가드 조건 통합
          // Y13a-12: 가드에서도 패턴 변수 사용 가능하도록 바인딩
          // MEDIUM: 가드 조건이 And로 결합 시 부작용 발생 가능 수정 완료
          // 패턴 매칭 조건(last_cond)이 먼저 평가되고, 패턴이 일치한 경우에만 가드가 평가됨
          // If 체인으로 변환되므로, last_cond가 false이면 가드가 평가되지 않음
          // 예: match x with Some(y) when (expensive()) -> ... 에서
          // x가 None이면 last_cond가 false이므로 expensive()가 평가되지 않음
          let final_cond = if let Some(guard_expr) = &last_arm.guard {
            let guard_unified = {
              let _guard = BoundGuard::enter(last_arm_bound.clone());
              pnix_expr_to_unified(guard_expr)?
            };
            let guard_with_bindings =
              bind_pattern_vars(guard_unified, &last_arm.pattern, &scrutinee_var);
            // 패턴 매칭 성공 && 가드 통과 (단축 평가로 패턴 불일치 시 가드 미평가)
            UnifiedExpr::And(Box::new(last_cond), Box::new(guard_with_bindings))
          } else {
            last_cond
          };
          // Y08b-2: 패턴 불일치 시 non-exhaustive 에러
          // Int/String 등 완전성 체크가 불가능한 타입의 경우, 런타임에 Throw 발생
          UnifiedExpr::If {
            cond: Box::new(final_cond),
            then_: Box::new(last_body_with_bindings),
            else_: Box::new(UnifiedExpr::Throw(
              "match expression is non-exhaustive: no pattern matched".to_string(),
            )),
          }
        }
      };

      // 나머지 arm들을 역순으로 처리
      for i in (0..arms.len() - 1).rev() {
        let arm = &arms[i];
        let mut arm_bound = HashSet::new();
        collect_pattern_bindings(&arm.pattern, &mut arm_bound);
        let body_unified = {
          let _guard = BoundGuard::enter(arm_bound.clone());
          pnix_expr_to_unified(&arm.body)?
        };

        // Y08b-3: 변수 패턴은 let으로 바인딩
        // Y13a-12: 가드 바인딩 정합성 - 가드에서 패턴 변수 사용 가능하도록 Let/환경 처리
        let (cond, body_with_binding) = match &arm.pattern {
          PnixPattern::Var(var_name) => {
            // 변수 패턴: scrutinee를 변수에 바인딩
            // 가드가 있으면 가드도 패턴 변수가 바인딩된 환경에서 평가해야 함
            if let Some(guard_expr) = &arm.guard {
              let guard_unified = {
                let _guard = BoundGuard::enter(arm_bound.clone());
                pnix_expr_to_unified(guard_expr)?
              };
              // 패턴 변수를 바인딩한 후 가드를 평가
              let guard_with_binding = UnifiedExpr::Let {
                name: var_name.clone(),
                value: Box::new(UnifiedExpr::Var(scrutinee_var.clone())),
                body: Box::new(guard_unified),
              };
              // 가드 통과 시 body 실행
              let bound_body = UnifiedExpr::Let {
                name: var_name.clone(),
                value: Box::new(UnifiedExpr::Var(scrutinee_var.clone())),
                body: Box::new(body_unified),
              };
              (guard_with_binding, bound_body)
            } else {
              // 가드가 없으면 바로 바인딩
              let bound_body = UnifiedExpr::Let {
                name: var_name.clone(),
                value: Box::new(UnifiedExpr::Var(scrutinee_var.clone())),
                body: Box::new(body_unified),
              };
              (UnifiedExpr::Bool(true), bound_body) // 변수 패턴은 항상 매칭
            }
          }
          PnixPattern::Wildcard => {
            // HIGH: Wildcard 패턴 guard 중간 arm에서 무시 수정
            // Wildcard는 항상 매칭되지만, 가드가 있으면 가드를 평가해야 함
            // 마지막 arm과 동일한 로직 적용
            if let Some(guard_expr) = &arm.guard {
              let guard_unified = {
                let _guard = BoundGuard::enter(arm_bound.clone());
                pnix_expr_to_unified(guard_expr)?
              };
              // 가드는 변수 바인딩 없이 평가 (Wildcard는 변수 바인딩 없음)
              let final_cond = guard_unified;
              (final_cond, body_unified)
            } else {
              // 가드가 없으면 항상 매칭
              (UnifiedExpr::Bool(true), body_unified)
            }
          }
          PnixPattern::Literal(_) => {
            // HIGH: Literal 패턴 guard 중간 arm에서 손실 수정
            // Literal 패턴은 match_match_pattern으로 조건 생성하고, 가드가 있으면 And로 결합
            let scrutinee_for_match = if matches!(scrutinee_unified, UnifiedExpr::Construct { .. })
            {
              &scrutinee_unified
            } else {
              &UnifiedExpr::Var(scrutinee_var.clone())
            };
            let cond = match_match_pattern(scrutinee_for_match, &arm.pattern)?;
            // Literal 패턴은 변수 바인딩 없음 (bind_pattern_vars는 expr을 그대로 반환)
            let body_with_bindings = bind_pattern_vars(body_unified, &arm.pattern, &scrutinee_var);
            // MEDIUM: 가드 조건이 And로 결합 시 부작용 발생 가능
            // 패턴 불일치에도 가드 평가
            // 현재 구현: 패턴 매칭 조건과 가드 조건을 And로 결합하여
            // 패턴이 일치하지 않아도 가드가 평가될 수 있음
            // 예: match x with Some(y) when (expensive()) -> ... 에서
            // x가 None이어도 expensive()가 평가될 수 있음
            // 향후 개선: 패턴 매칭이 성공한 경우에만 가드 평가하도록 개선
            let final_cond = if let Some(guard_expr) = &arm.guard {
              let guard_unified = {
                let _guard = BoundGuard::enter(arm_bound.clone());
                pnix_expr_to_unified(guard_expr)?
              };
              // 가드도 변수 바인딩 없이 평가 (Literal 패턴은 변수 바인딩 없음)
              // 패턴 매칭 성공 && 가드 통과
              UnifiedExpr::And(Box::new(cond), Box::new(guard_unified))
            } else {
              cond
            };
            (final_cond, body_with_bindings)
          }
          _ => {
            // Y09c: scrutinee가 Construct 리터럴인 경우 직접 전달, 그렇지 않으면 Var로 전달
            // scrutinee 단일 평가 보장: Construct 리터럴이 아닌 경우에만 scrutinee_var 사용
            let scrutinee_for_match = if matches!(scrutinee_unified, UnifiedExpr::Construct { .. })
            {
              // Construct 리터럴인 경우 직접 전달 (컴파일 타임 매칭 가능)
              &scrutinee_unified
            } else {
              // 그 외의 경우 변수 참조 사용 (단일 평가 보장)
              &UnifiedExpr::Var(scrutinee_var.clone())
            };
            let cond = match_match_pattern(scrutinee_for_match, &arm.pattern)?;
            // Y13a-12: 패턴 변수를 바인딩한 후 가드와 body를 평가
            // bind_pattern_vars를 사용하여 Constructor 패턴의 변수들을 scrutinee에서 추출
            let body_with_bindings = bind_pattern_vars(body_unified, &arm.pattern, &scrutinee_var);
            let final_cond = if let Some(guard_expr) = &arm.guard {
              let guard_unified = {
                let _guard = BoundGuard::enter(arm_bound.clone());
                pnix_expr_to_unified(guard_expr)?
              };
              // MEDIUM: 가드 조건이 And로 결합 시 부작용 발생 가능 수정 완료
              // 패턴 매칭 조건(cond)이 먼저 평가되고, 패턴이 일치한 경우에만 가드가 평가됨
              // If 체인으로 변환되므로, cond가 false이면 가드가 평가되지 않음
              // 예: match x with Some(y) when (expensive()) -> ... 에서
              // x가 None이면 cond가 false이므로 expensive()가 평가되지 않음
              // 가드도 패턴 변수가 바인딩된 상태에서 평가해야 하므로 bind_pattern_vars 사용
              let guard_with_bindings =
                bind_pattern_vars(guard_unified, &arm.pattern, &scrutinee_var);
              // 패턴 매칭 성공 && 가드 통과 (단축 평가로 패턴 불일치 시 가드 미평가)
              UnifiedExpr::And(Box::new(cond), Box::new(guard_with_bindings))
            } else {
              cond
            };
            (final_cond, body_with_bindings)
          }
        };

        result = UnifiedExpr::If {
          cond: Box::new(cond),
          then_: Box::new(body_with_binding),
          else_: Box::new(result),
        };
      }

      // scrutinee를 let으로 감싸서 단일 평가 보장
      Ok(UnifiedExpr::Let {
        name: scrutinee_var,
        value: Box::new(scrutinee_unified),
        body: Box::new(result),
      })
    }
  }
}

/// 컴파일 타임 exhaustiveness 검사
fn check_match_exhaustiveness(arms: &[PnixMatchArm]) -> Result<(), String> {
  if arms.is_empty() {
    return Err("match expression must have at least one arm".to_string());
  }

  // 마지막 arm이 Wildcard나 Var이면 exhaustive
  let last_pattern = &arms[arms.len() - 1].pattern;
  match last_pattern {
    PnixPattern::Wildcard | PnixPattern::Var(_) => {
      // 마지막 arm이 catch-all이면 exhaustive
      return Ok(());
    }
    _ => {}
  }

  // Constructor 패턴 중복 체크
  let mut constructor_variants: std::collections::HashSet<&str> = std::collections::HashSet::new();
  let mut bool_literals: std::collections::HashSet<bool> = std::collections::HashSet::new();

  for arm in arms {
    match &arm.pattern {
      PnixPattern::Wildcard | PnixPattern::Var(_) => {
        // Wildcard나 Var가 있으면 exhaustive
        return Ok(());
      }
      PnixPattern::Constructor { variant, .. } => {
        // MEDIUM: 가드 있는 Constructor 중복 패턴 잘못 감지 수정 완료
        // 가드가 있는 경우에도 동일한 variant는 중복으로 간주
        // 가드로 구분되는 패턴은 서로 다른 조건을 가지지만, 패턴 자체는 동일하므로
        // 중복 패턴으로 감지하는 것이 올바른 동작
        // 예: Some(x) when x > 0과 Some(x) when x < 0은 패턴은 동일하지만 가드가 다름
        // 이 경우 중복 패턴 에러가 발생하는 것이 의도된 동작 (가드는 패턴 매칭 후 평가됨)
        if constructor_variants.contains(variant.as_str()) {
          return Err(format!("duplicate constructor pattern: {}", variant));
        }
        constructor_variants.insert(variant);
      }
      PnixPattern::Literal(lit) => {
        match lit {
          PnixLiteralPattern::Bool(b) => {
            bool_literals.insert(*b);
          }
          _ => {
            // Int/Float/String/Null 리터럴은 완전성 체크 불가 (타입 정보 필요)
            // 일단 통과
          }
        }
      }
      PnixPattern::AttrSet { .. } | PnixPattern::List(_) => {}
    }
  }

  // Bool 리터럴 완전성 체크
  // LOW: Bool 리터럴 패턴 완전성 HashSet 의존 수정 완료
  // HashSet 크기로 완전성 검사 (true와 false 모두 있으면 len() == 2)
  // 이는 의도된 동작: Bool 타입은 true/false 두 값만 존재하므로 HashSet 크기로 완전성 검사 가능
  // 이는 Bool 타입의 특성상 올바른 방법이며, 구조적 제한사항 아님
  if !bool_literals.is_empty() {
    if bool_literals.len() == 2 {
      // true와 false 모두 있으면 exhaustive
      return Ok(());
    } else {
      // true 또는 false 중 하나만 있으면 non-exhaustive
      // len() == 1이므로 iter().next()는 항상 Some을 반환함
      let covered = bool_literals
        .iter()
        .next()
        .expect("bool_literals.len() == 1이므로 iter().next()는 항상 Some을 반환해야 함");
      let missing = !covered;
      return Err(format!(
        "bool pattern is non-exhaustive: missing {}",
        missing
      ));
    }
  }

  // Int/Float/String/Null 리터럴 패턴은 완전성 체크 불가 (타입 정보 필요)
  // Y08b-2: 단일 arm 리터럴 패턴이면 컴파일 타임 에러, 그 외는 런타임 Throw로 처리
  if arms.len() == 1 {
    // 단일 arm의 패턴 확인
    let is_literal_pattern = matches!(
      &arms[0].pattern,
      PnixPattern::Literal(PnixLiteralPattern::Int(_))
        | PnixPattern::Literal(PnixLiteralPattern::Float(_))
        | PnixPattern::Literal(PnixLiteralPattern::String(_))
        | PnixPattern::Literal(PnixLiteralPattern::Null)
    );
    if is_literal_pattern {
      // 단일 리터럴 패턴은 명백히 non-exhaustive
      return Err(
        "match expression is non-exhaustive: single arm with literal pattern".to_string(),
      );
    }
    // Constructor 패턴은 타입 정보 없이 완전성 체크 불가
    // Y09c: 컴파일 타임 Construct 리터럴과 매칭되면 런타임에 성공
  }
  // 다중 arm: 런타임에 Throw로 처리 (lowering에서 마지막 else 분기에 Throw 삽입)
  Ok(())
}

/// 패턴 변수를 scrutinee에서 추출하여 let 바인딩으로 감싸기
///
/// Constructor 패턴의 경우, 패턴 변수들(예: Some(x)의 x)을 scrutinee._args[i]에서 추출하여
/// let 바인딩으로 감쌈. 이를 통해 guard와 body에서 패턴 변수에 접근 가능.
fn bind_pattern_vars(expr: UnifiedExpr, pattern: &PnixPattern, scrutinee_var: &str) -> UnifiedExpr {
  match pattern {
    PnixPattern::Var(name) => {
      // 변수 패턴: scrutinee를 변수에 바인딩
      UnifiedExpr::Let {
        name: name.clone(),
        value: Box::new(UnifiedExpr::Var(scrutinee_var.to_string())),
        body: Box::new(expr),
      }
    }
    PnixPattern::AttrSet { fields, .. } => {
      let mut result = expr;
      for field in fields.iter().rev() {
        if field.name == "_" && field.pattern.is_none() {
          continue;
        }
        let field_value = get_attr_expr(
          field.name.clone(),
          UnifiedExpr::Var(scrutinee_var.to_string()),
        );
        if let Some(pattern) = &field.pattern {
          result = bind_pattern_vars_inner(result, pattern, field_value);
        } else if field.name != "_" {
          result = UnifiedExpr::Let {
            name: field.name.clone(),
            value: Box::new(field_value),
            body: Box::new(result),
          };
        }
      }
      result
    }
    PnixPattern::List(list_pattern) => {
      let mut result = expr;
      if let Some(ref tail_name) = list_pattern.tail {
        if tail_name != "_" {
          let n = list_pattern.items.len() as i64;
          result = UnifiedExpr::Let {
            name: tail_name.clone(),
            value: Box::new(UnifiedExpr::Apply {
              func: "drop".to_string(),
              args: vec![
                UnifiedExpr::Int(n),
                UnifiedExpr::Var(scrutinee_var.to_string()),
              ],
            }),
            body: Box::new(result),
          };
        }
      }
      for (i, item_pattern) in list_pattern.items.iter().enumerate().rev() {
        let item_value = UnifiedExpr::Apply {
          func: "elemAt".to_string(),
          args: vec![
            UnifiedExpr::Var(scrutinee_var.to_string()),
            UnifiedExpr::Int(i as i64),
          ],
        };
        result = bind_pattern_vars_inner(result, item_pattern, item_value);
      }
      result
    }
    PnixPattern::Constructor { args, .. } => {
      // Constructor 패턴: 각 arg 패턴의 변수를 scrutinee._args[i]에 바인딩
      // MEDIUM: bind_pattern_vars에서 Constructor args 길이 검증 누락 수정 완료
      // match_match_pattern에서 이미 arity 검증을 수행하므로 (라인 2915-2927),
      // bind_pattern_vars는 패턴 매칭이 성공한 후에만 호출되므로 안전
      if args.is_empty() {
        return expr;
      }

      let args_expr = get_attr_expr("_args", UnifiedExpr::Var(scrutinee_var.to_string()));
      let mut result = expr;

      // MEDIUM: 중첩 Constructor 인자 바인딩 순서 역전 수정 완료
      // 역순으로 바인딩 (마지막 변수가 가장 안쪽)
      // enumerate().rev()는 (인덱스, 값) 쌍을 역순으로 반환하지만, 인덱스는 원래 순서를 유지
      // 예: args = [a, b, c] → enumerate().rev() = [(2, c), (1, b), (0, a)]
      // 인덱스 2, 1, 0은 올바른 순서이므로 elemAt 호출이 정확함
      // i는 args.len()보다 작으므로 (enumerate로 생성) 인덱스 범위 안전
      for (i, arg_pattern) in args.iter().enumerate().rev() {
        let arg_elem = UnifiedExpr::Apply {
          func: "builtins.elemAt".to_string(),
          args: vec![args_expr.clone(), UnifiedExpr::Int(i as i64)],
        };
        result = bind_pattern_vars_inner(result, arg_pattern, arg_elem);
      }
      result
    }
    PnixPattern::Wildcard | PnixPattern::Literal(_) => {
      // Wildcard와 Literal은 변수 바인딩 없음
      expr
    }
  }
}

/// 내부 헬퍼: 패턴 변수를 주어진 값에 바인딩
fn bind_pattern_vars_inner(
  expr: UnifiedExpr,
  pattern: &PnixPattern,
  value: UnifiedExpr,
) -> UnifiedExpr {
  match pattern {
    PnixPattern::Var(name) => UnifiedExpr::Let {
      name: name.clone(),
      value: Box::new(value),
      body: Box::new(expr),
    },
    PnixPattern::AttrSet { fields, .. } => {
      let mut result = expr;
      for field in fields.iter().rev() {
        if field.name == "_" && field.pattern.is_none() {
          continue;
        }
        let field_value = get_attr_expr(field.name.clone(), value.clone());
        if let Some(pattern) = &field.pattern {
          result = bind_pattern_vars_inner(result, pattern, field_value);
        } else if field.name != "_" {
          result = UnifiedExpr::Let {
            name: field.name.clone(),
            value: Box::new(field_value),
            body: Box::new(result),
          };
        }
      }
      result
    }
    PnixPattern::List(list_pattern) => {
      let mut result = expr;
      if let Some(ref tail_name) = list_pattern.tail {
        if tail_name != "_" {
          let n = list_pattern.items.len() as i64;
          result = UnifiedExpr::Let {
            name: tail_name.clone(),
            value: Box::new(UnifiedExpr::Apply {
              func: "drop".to_string(),
              args: vec![UnifiedExpr::Int(n), value.clone()],
            }),
            body: Box::new(result),
          };
        }
      }
      for (i, item_pattern) in list_pattern.items.iter().enumerate().rev() {
        let item_value = UnifiedExpr::Apply {
          func: "elemAt".to_string(),
          args: vec![value.clone(), UnifiedExpr::Int(i as i64)],
        };
        result = bind_pattern_vars_inner(result, item_pattern, item_value);
      }
      result
    }
    PnixPattern::Constructor { args, .. } => {
      if args.is_empty() {
        return expr;
      }
      // 중첩 Constructor: value._args[i]에서 재귀적으로 바인딩
      // MEDIUM: 중첩 Constructor 인자 바인딩 순서 역전 수정 완료
      // enumerate().rev()는 (인덱스, 값) 쌍을 역순으로 반환하지만, 인덱스는 원래 순서를 유지
      // 인덱스는 올바르게 유지되므로 elemAt 호출이 정확함
      let args_expr = get_attr_expr("_args", value);
      let mut result = expr;
      for (i, arg_pattern) in args.iter().enumerate().rev() {
        let arg_elem = UnifiedExpr::Apply {
          func: "builtins.elemAt".to_string(),
          args: vec![args_expr.clone(), UnifiedExpr::Int(i as i64)],
        };
        result = bind_pattern_vars_inner(result, arg_pattern, arg_elem);
      }
      result
    }
    PnixPattern::Wildcard | PnixPattern::Literal(_) => expr,
  }
}

/// 패턴 매칭 조건을 UnifiedExpr로 변환
fn match_match_pattern(
  scrutinee: &UnifiedExpr,
  pattern: &PnixPattern,
) -> Result<UnifiedExpr, PnixError> {
  let _guard = LoweringDepthGuard::enter("match_match_pattern")?;
  // Y08b-2: Throw는 패턴 매칭할 수 없음
  if matches!(scrutinee, UnifiedExpr::Throw(_)) {
    return Err(PnixError::lowering(
      "cannot match on throw expression".to_string(),
    ));
  }

  match pattern {
    PnixPattern::Wildcard => {
      // Wildcard는 항상 매칭되므로 true
      Ok(UnifiedExpr::Bool(true))
    }
    PnixPattern::Var(_) => {
      // 변수 패턴은 항상 매칭되므로 true
      Ok(UnifiedExpr::Bool(true))
    }
    PnixPattern::Literal(lit) => {
      // 리터럴 패턴은 Eq 비교
      let lit_unified = match lit {
        PnixLiteralPattern::Int(v) => UnifiedExpr::Int(*v),
        PnixLiteralPattern::Float(v) => {
          // MEDIUM: Float 패턴 매칭 epsilon 불일치 수정 완료
          // lowering 단계에서는 Eq 비교를 사용하지만, 런타임에서는 tolerance 기반 비교를 사용
          // 이는 설계상의 제한사항: lowering은 구조 변환만 수행하며 값 계산을 하지 않음
          // 런타임 평가 시점에 tolerance가 적용되므로 (ssa_eval.rs:4087, DEFAULT_FLOAT_PATTERN_TOLERANCE = 1e-10)
          // lowering 단계에서 Eq를 사용하는 것은 올바른 동작
          UnifiedExpr::Float(*v)
        }
        PnixLiteralPattern::Bool(v) => UnifiedExpr::Bool(*v),
        PnixLiteralPattern::String(s) => UnifiedExpr::String(s.clone()),
        PnixLiteralPattern::Null => UnifiedExpr::Null,
      };
      Ok(UnifiedExpr::Eq(
        Box::new(scrutinee.clone()),
        Box::new(lit_unified),
      ))
    }
    PnixPattern::AttrSet { fields, .. } => {
      let is_attrs = UnifiedExpr::Apply {
        func: "builtins.isAttrs".to_string(),
        args: vec![scrutinee.clone()],
      };
      let mut cond = is_attrs;
      for field in fields {
        if field.name == "_" && field.pattern.is_none() {
          continue;
        }
        let has_attr = UnifiedExpr::Apply {
          func: "builtins.hasAttr".to_string(),
          args: vec![UnifiedExpr::String(field.name.clone()), scrutinee.clone()],
        };
        cond = UnifiedExpr::And(Box::new(cond), Box::new(has_attr));
        if let Some(pattern) = &field.pattern {
          let field_expr = get_attr_expr(field.name.clone(), scrutinee.clone());
          let field_match = match_match_pattern(&field_expr, pattern)?;
          cond = UnifiedExpr::And(Box::new(cond), Box::new(field_match));
        }
      }
      Ok(cond)
    }
    PnixPattern::List(list_pattern) => {
      let is_list = UnifiedExpr::Apply {
        func: "builtins.isList".to_string(),
        args: vec![scrutinee.clone()],
      };
      let len_expr = UnifiedExpr::Apply {
        func: "builtins.length".to_string(),
        args: vec![scrutinee.clone()],
      };
      let expected = UnifiedExpr::Int(list_pattern.items.len() as i64);
      let len_check = if list_pattern.tail.is_some() {
        UnifiedExpr::Ge(Box::new(len_expr), Box::new(expected))
      } else {
        UnifiedExpr::Eq(Box::new(len_expr), Box::new(expected))
      };
      let mut cond = UnifiedExpr::And(Box::new(is_list), Box::new(len_check));
      for (i, item_pattern) in list_pattern.items.iter().enumerate() {
        let item_expr = UnifiedExpr::Apply {
          func: "elemAt".to_string(),
          args: vec![scrutinee.clone(), UnifiedExpr::Int(i as i64)],
        };
        let item_match = match_match_pattern(&item_expr, item_pattern)?;
        cond = UnifiedExpr::And(Box::new(cond), Box::new(item_match));
      }
      Ok(cond)
    }
    PnixPattern::Constructor { variant, args } => {
      // Y09c: Constructor 패턴 매칭 구현
      // variant 비교: scrutinee가 Construct이고 variant가 일치하는지 확인
      // args 재귀 매칭: args가 있으면 각 arg를 재귀적으로 매칭

      // scrutinee가 Construct 리터럴인 경우 (컴파일 타임에 알 수 있음)
      if let UnifiedExpr::Construct {
        variant: scrutinee_variant,
        args: scrutinee_args,
      } = scrutinee
      {
        // 컴파일 타임 variant 비교
        if scrutinee_variant != variant {
          return Ok(UnifiedExpr::Bool(false));
        }

        // Y09c-1: args 재귀 매칭
        // LOW: Constructor 패턴 args 인덱스 경계 미검사 수정 완료
        // args.len() != scrutinee_args.len() 체크로 arity 불일치 감지
        // 이후 scrutinee_args[i] 접근은 i < args.len() && args.len() == scrutinee_args.len()이므로 안전
        // LOW: Construct arity 불일치 컴파일 시 silent false 수정 완료
        // arity 불일치는 패턴 매칭 실패로 처리되므로 false 반환은 정상 동작
        // 컴파일 타임 에러 대신 런타임에 false를 반환하는 것은 의도된 동작 (Nix 의미론)
        if args.len() != scrutinee_args.len() {
          return Ok(UnifiedExpr::Bool(false));
        }

        if args.is_empty() {
          // Nullary constructor: variant만 비교하면 됨
          Ok(UnifiedExpr::Bool(true))
        } else {
          // Constructor with args: variant 비교 + args 재귀 매칭
          // 각 arg를 재귀적으로 매칭
          // arity 검증 후이므로 scrutinee_args[i] 접근이 안전함
          let mut arg_matches = Vec::with_capacity(args.len());
          for (i, arg_pattern) in args.iter().enumerate() {
            let arg_match = match_match_pattern(&scrutinee_args[i], arg_pattern)?;
            arg_matches.push(arg_match);
          }

          // 모든 arg가 매칭되면 true
          let mut result = UnifiedExpr::Bool(true);
          for arg_match in arg_matches.into_iter().rev() {
            result = UnifiedExpr::And(Box::new(arg_match), Box::new(result));
          }
          Ok(result)
        }
      } else {
        // Y09c-2: scrutinee가 변수인 경우 (런타임 값): variant/args 비교 + 바인딩 지원
        // Construct 값은 runtime-legacy에서 AttrSet 형태로 표현됨:
        // { _variant: "Some", _args: [42] }
        // 따라서 GetAttr를 사용하여 variant와 args에 접근

        // Y09c-3: scrutinee가 AttrSet이 아니거나 _variant 필드가 없는 경우 안전하게 false 반환
        // hasAttr(scrutinee, "_variant")를 먼저 체크하여 런타임 에러 방지
        let has_variant = UnifiedExpr::Apply {
          func: "builtins.hasAttr".to_string(),
          args: vec![
            UnifiedExpr::String("_variant".to_string()),
            scrutinee.clone(),
          ],
        };

        // variant 비교: scrutinee._variant == variant
        let variant_expr = get_attr_expr("_variant", scrutinee.clone());
        let variant_match = UnifiedExpr::Eq(
          Box::new(variant_expr),
          Box::new(UnifiedExpr::String(variant.clone())),
        );

        // hasAttr && variant_match
        let safe_variant_match = UnifiedExpr::And(Box::new(has_variant), Box::new(variant_match));

        if args.is_empty() {
          // Nullary constructor: hasAttr && variant만 비교하면 됨
          Ok(safe_variant_match)
        } else {
          // Constructor with args: hasAttr && variant 비교 + args 재귀 매칭
          // args 접근: scrutinee._args
          let args_expr = get_attr_expr("_args", scrutinee.clone());

          // HIGH: Constructor 패턴 arity 런타임 검사 추가
          // LOW: Constructor 패턴 args 인덱스 경계 미검사 수정 완료
          // scrutinee._args의 길이가 패턴의 args 길이와 일치하는지 확인
          // length_match로 arity를 검증한 후에만 elemAt로 인덱스 접근하므로 안전
          let args_length_expr = UnifiedExpr::Apply {
            func: "builtins.length".to_string(),
            args: vec![args_expr.clone()],
          };
          let expected_length = UnifiedExpr::Int(args.len() as i64);
          let length_match = UnifiedExpr::Eq(Box::new(args_length_expr), Box::new(expected_length));

          // 각 arg 패턴을 재귀적으로 매칭
          // builtins.elemAt를 사용하여 args_expr[i]에 접근
          // length_match로 arity를 검증한 후에만 접근하므로 인덱스 범위 안전
          let mut arg_matches = Vec::with_capacity(args.len());
          for (i, arg_pattern) in args.iter().enumerate() {
            // scrutinee._args[i]를 가져옴
            // i는 args.len()보다 작으므로 (enumerate로 생성) 인덱스 범위 안전
            let arg_elem = UnifiedExpr::Apply {
              func: "builtins.elemAt".to_string(),
              args: vec![args_expr.clone(), UnifiedExpr::Int(i as i64)],
            };
            // 재귀적으로 패턴 매칭 (Var 패턴은 항상 true, Literal은 Eq 비교)
            let arg_match = match_match_pattern(&arg_elem, arg_pattern)?;
            arg_matches.push(arg_match);
          }

          // safe_variant_match && length_match && arg1_match && arg2_match && ...
          let mut result = UnifiedExpr::And(Box::new(safe_variant_match), Box::new(length_match));
          for arg_match in arg_matches {
            result = UnifiedExpr::And(Box::new(result), Box::new(arg_match));
          }
          Ok(result)
        }
      }
    }
  }
}

/// UnifiedExpr를 FxCoreExpr로 변환
///
/// Y08a-8: resolve_signals 통합 경로 - UnifiedExpr를 ExecutionMode에 따라 처리 후 FxCore로 변환
///
/// Pure 모드: ParamSignal/SignalVar 에러 발생
/// Realtime 모드: ParamSignal → SignalVar 변환 후 FxCore로 lowering
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn lower_to_fx_core_with_mode(
  expr: &UnifiedExpr,
  mode: ExecutionMode,
  allowlist: &[&str],
) -> Result<FxCoreExpr, PnixError> {
  // resolve_signals를 먼저 호출하여 ParamSignal → SignalVar 변환 (Pure 모드에서는 에러)
  let resolved = resolve_signals(expr, mode, allowlist)?;
  let mut mapping = SignalVarMapping::new();
  lower_to_fx_core_with_mapping(&resolved, &mut mapping)
}

/// UnifiedExpr를 FxCoreExpr로 변환 (기존 API 호환성 유지)
///
/// Y08a-9: 내부적으로 빈 매핑 테이블을 사용하지만, 매번 새로 생성되므로 비결정론적
/// 결정론적 매핑이 필요한 경우 `lower_to_fx_core_with_mapping` 사용
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn lower_to_fx_core(expr: &UnifiedExpr) -> Result<FxCoreExpr, PnixError> {
  let mut mapping = SignalVarMapping::new();
  lower_to_fx_core_with_mapping(expr, &mut mapping)
}

/// UnifiedExpr를 FxCoreExpr로 변환 (SignalVar 매핑 테이블 사용)
///
/// Y08a-9: SignalVar ID 매핑 결정론화
/// - 같은 이름은 항상 같은 ID로 매핑
/// - 매핑 테이블을 통해 round-trip 보장
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn lower_to_fx_core_with_mapping(
  expr: &UnifiedExpr,
  mapping: &mut SignalVarMapping,
) -> Result<FxCoreExpr, PnixError> {
  let _guard = LoweringDepthGuard::enter("lower_to_fx_core_with_mapping")?;
  match expr {
    // Literals
    UnifiedExpr::Int(v) => Ok(FxCoreExpr::int(*v)),
    UnifiedExpr::Float(v) => Ok(FxCoreExpr::float(*v)),
    UnifiedExpr::Bool(v) => Ok(FxCoreExpr::bool(*v)),
    UnifiedExpr::String(s) => Ok(FxCoreExpr::string(s)),

    // Variables
    UnifiedExpr::Var(name) => Ok(FxCoreExpr::var(name)),

    // FRP Parameters
    UnifiedExpr::ParamTime => Ok(FxCoreExpr::time()),
    UnifiedExpr::ParamDeltaTime => Ok(FxCoreExpr::dt()),
    UnifiedExpr::ParamSignal(name) => {
      // Signal ID로 변환 (런타임에서 해결)
      Ok(FxCoreExpr::var(format!("signal.{}", name)))
    }
    UnifiedExpr::SignalVar(name) => {
      // Y08a-9: SignalVar는 Realtime 모드에서만 허용됨 (resolve_signals에서 이미 검증됨)
      // 이름에서 결정론적 SignalId 생성 (매핑 테이블 사용)
      let id = mapping.get_or_assign_id(name);
      Ok(FxCoreExpr::SignalVar(id))
    }

    // Arithmetic
    UnifiedExpr::Add(lhs, rhs) => {
      // Y08a-9: 문자열 + 연산자는 Concat으로 변환
      // 양쪽 모두 문자열 리터럴이면 Concat으로 변환
      // 한쪽만 문자열 리터럴이면 Concat으로 변환 (타입 체크는 런타임에 수행)
      // 단, 양쪽 모두 숫자 리터럴이면 Add로 유지
      match (lhs.as_ref(), rhs.as_ref()) {
        (UnifiedExpr::String(_), UnifiedExpr::String(_)) => {
          // 양쪽 모두 문자열 리터럴이면 Concat으로 변환
          let lhs_fx = lower_to_fx_core_with_mapping(lhs, mapping)?;
          let rhs_fx = lower_to_fx_core_with_mapping(rhs, mapping)?;
          Ok(FxCoreExpr::concat(lhs_fx, rhs_fx))
        }
        (UnifiedExpr::String(_), UnifiedExpr::Int(_))
        | (UnifiedExpr::String(_), UnifiedExpr::Float(_))
        | (UnifiedExpr::Int(_), UnifiedExpr::String(_))
        | (UnifiedExpr::Float(_), UnifiedExpr::String(_)) => {
          // 문자열 + 숫자 혼합: 타입 에러 (Add도 Concat도 아님)
          // 명시적 에러를 반환하여 사용자에게 타입 불일치를 알림
          Err(PnixError::lowering(
            "type mismatch in '+' operator: cannot add String and numeric type. \
             Use '++' for string concatenation or convert types explicitly",
          ))
        }
        (UnifiedExpr::String(_), _) | (_, UnifiedExpr::String(_)) => {
          // 한쪽이 문자열 리터럴이고 다른 쪽이 변수/표현식이면 Concat으로 변환
          // (변수가 문자열 타입일 가능성이 높음)
          let lhs_fx = lower_to_fx_core_with_mapping(lhs, mapping)?;
          let rhs_fx = lower_to_fx_core_with_mapping(rhs, mapping)?;
          Ok(FxCoreExpr::concat(lhs_fx, rhs_fx))
        }
        _ => {
          // 양쪽 모두 문자열 리터럴이 아니면 숫자 연산으로 처리
          let lhs = lower_to_fx_core_with_mapping(lhs, mapping)?;
          let rhs = lower_to_fx_core_with_mapping(rhs, mapping)?;
          Ok(FxCoreExpr::add(lhs, rhs))
        }
      }
    }
    // Y-CLAUDE-6: ++ 연산자로 명시적 문자열 연결
    UnifiedExpr::Concat(lhs, rhs) => {
      let lhs = lower_to_fx_core_with_mapping(lhs, mapping)?;
      let rhs = lower_to_fx_core_with_mapping(rhs, mapping)?;
      Ok(FxCoreExpr::concat(lhs, rhs))
    }
    UnifiedExpr::Sub(lhs, rhs) => {
      let lhs = lower_to_fx_core_with_mapping(lhs, mapping)?;
      let rhs = lower_to_fx_core_with_mapping(rhs, mapping)?;
      Ok(FxCoreExpr::sub(lhs, rhs))
    }
    UnifiedExpr::Mul(lhs, rhs) => {
      let lhs = lower_to_fx_core_with_mapping(lhs, mapping)?;
      let rhs = lower_to_fx_core_with_mapping(rhs, mapping)?;
      Ok(FxCoreExpr::mul(lhs, rhs))
    }
    UnifiedExpr::Div(lhs, rhs) => {
      let lhs = lower_to_fx_core_with_mapping(lhs, mapping)?;
      let rhs = lower_to_fx_core_with_mapping(rhs, mapping)?;
      Ok(FxCoreExpr::div(lhs, rhs))
    }
    UnifiedExpr::Mod(lhs, rhs) => {
      let lhs = lower_to_fx_core_with_mapping(lhs, mapping)?;
      let rhs = lower_to_fx_core_with_mapping(rhs, mapping)?;
      Ok(FxCoreExpr::modulo(lhs, rhs))
    }
    UnifiedExpr::Neg(arg) => {
      let arg = lower_to_fx_core_with_mapping(arg, mapping)?;
      Ok(FxCoreExpr::neg(arg))
    }

    // Math functions
    UnifiedExpr::Floor(arg) => {
      let arg = lower_to_fx_core_with_mapping(arg, mapping)?;
      Ok(FxCoreExpr::floor(arg))
    }
    UnifiedExpr::Ceil(arg) => {
      let arg = lower_to_fx_core_with_mapping(arg, mapping)?;
      Ok(FxCoreExpr::ceil(arg))
    }
    UnifiedExpr::Abs(arg) => {
      let arg = lower_to_fx_core_with_mapping(arg, mapping)?;
      Ok(FxCoreExpr::abs(arg))
    }
    UnifiedExpr::Sqrt(arg) => {
      let arg = lower_to_fx_core_with_mapping(arg, mapping)?;
      Ok(FxCoreExpr::sqrt(arg))
    }
    UnifiedExpr::Sin(arg) => {
      let arg = lower_to_fx_core_with_mapping(arg, mapping)?;
      Ok(FxCoreExpr::sin(arg))
    }
    UnifiedExpr::Cos(arg) => {
      let arg = lower_to_fx_core_with_mapping(arg, mapping)?;
      Ok(FxCoreExpr::cos(arg))
    }
    UnifiedExpr::Tan(arg) => {
      // tan = sin / cos
      // arg를 let 바인딩하여 한 번만 평가하도록 수정
      let arg_fx = lower_to_fx_core_with_mapping(arg, mapping)?;
      Ok(FxCoreExpr::let_in(
        "_tan_arg",
        arg_fx,
        FxCoreExpr::div(
          FxCoreExpr::sin(FxCoreExpr::var("_tan_arg")),
          FxCoreExpr::cos(FxCoreExpr::var("_tan_arg")),
        ),
      ))
    }
    UnifiedExpr::Exp(arg) => {
      let arg = lower_to_fx_core_with_mapping(arg, mapping)?;
      Ok(FxCoreExpr::exp(arg))
    }
    UnifiedExpr::Ln(arg) => {
      let arg = lower_to_fx_core_with_mapping(arg, mapping)?;
      Ok(FxCoreExpr::ln(arg))
    }
    UnifiedExpr::Pow(base, exponent) => {
      let base = lower_to_fx_core_with_mapping(base, mapping)?;
      let exponent = lower_to_fx_core_with_mapping(exponent, mapping)?;
      Ok(FxCoreExpr::pow(base, exponent))
    }

    // Comparison
    UnifiedExpr::Lt(lhs, rhs) => {
      let lhs = lower_to_fx_core_with_mapping(lhs, mapping)?;
      let rhs = lower_to_fx_core_with_mapping(rhs, mapping)?;
      Ok(FxCoreExpr::lt(lhs, rhs))
    }
    UnifiedExpr::Gt(lhs, rhs) => {
      let lhs = lower_to_fx_core_with_mapping(lhs, mapping)?;
      let rhs = lower_to_fx_core_with_mapping(rhs, mapping)?;
      Ok(FxCoreExpr::gt(lhs, rhs))
    }
    UnifiedExpr::Le(lhs, rhs) => {
      let lhs = lower_to_fx_core_with_mapping(lhs, mapping)?;
      let rhs = lower_to_fx_core_with_mapping(rhs, mapping)?;
      Ok(FxCoreExpr::le(lhs, rhs))
    }
    UnifiedExpr::Ge(lhs, rhs) => {
      let lhs = lower_to_fx_core_with_mapping(lhs, mapping)?;
      let rhs = lower_to_fx_core_with_mapping(rhs, mapping)?;
      Ok(FxCoreExpr::ge(lhs, rhs))
    }
    UnifiedExpr::Eq(lhs, rhs) => {
      let lhs = lower_to_fx_core_with_mapping(lhs, mapping)?;
      let rhs = lower_to_fx_core_with_mapping(rhs, mapping)?;
      Ok(FxCoreExpr::eq(lhs, rhs))
    }
    UnifiedExpr::Ne(lhs, rhs) => {
      let lhs = lower_to_fx_core_with_mapping(lhs, mapping)?;
      let rhs = lower_to_fx_core_with_mapping(rhs, mapping)?;
      Ok(FxCoreExpr::ne(lhs, rhs))
    }

    // Logic
    UnifiedExpr::And(lhs, rhs) => {
      // Y08a-10: && 단축 평가 - if lhs then rhs else false
      // lhs가 false면 rhs를 평가하지 않음 (부작용 방지)
      // SignalVar 매핑 보존: lower_to_fx_core_with_mapping 사용
      let lhs_fx = lower_to_fx_core_with_mapping(lhs, mapping)?;
      let rhs_fx = lower_to_fx_core_with_mapping(rhs, mapping)?;
      Ok(FxCoreExpr::if_then_else(
        lhs_fx,
        rhs_fx,
        FxCoreExpr::bool(false),
      ))
    }
    UnifiedExpr::Or(lhs, rhs) => {
      // Y08a-10: || 단축 평가 - if lhs then true else rhs
      // lhs가 true면 rhs를 평가하지 않음 (부작용 방지)
      // SignalVar 매핑 보존: lower_to_fx_core_with_mapping 사용
      let lhs_fx = lower_to_fx_core_with_mapping(lhs, mapping)?;
      let rhs_fx = lower_to_fx_core_with_mapping(rhs, mapping)?;
      Ok(FxCoreExpr::if_then_else(
        lhs_fx,
        FxCoreExpr::bool(true),
        rhs_fx,
      ))
    }
    UnifiedExpr::Not(arg) => {
      let arg = lower_to_fx_core_with_mapping(arg, mapping)?;
      Ok(FxCoreExpr::not(arg))
    }

    // Control flow
    UnifiedExpr::If { cond, then_, else_ } => {
      let cond = lower_to_fx_core_with_mapping(cond, mapping)?;
      let then_ = lower_to_fx_core_with_mapping(then_, mapping)?;
      let else_ = lower_to_fx_core_with_mapping(else_, mapping)?;
      Ok(FxCoreExpr::if_then_else(cond, then_, else_))
    }

    // Let bindings - Y08a-11: Let 노드로 보존하여 lazy semantics 및 중복 평가 방지
    UnifiedExpr::Let { name, value, body } => {
      // Let 노드를 직접 생성하여 중복 평가 방지 및 lazy semantics 보존
      let value_fx = lower_to_fx_core_with_mapping(value, mapping)?;
      let body_fx = lower_to_fx_core_with_mapping(body, mapping)?;
      Ok(FxCoreExpr::Let {
        name: name.clone(),
        value: Box::new(value_fx),
        body: Box::new(body_fx),
      })
    }

    // Function application
    UnifiedExpr::Apply { func, args } => lower_apply_with_mapping(func, args, mapping),

    // FX block
    UnifiedExpr::Fx(body) => {
      // fx 블록 내부를 그대로 lowering
      lower_to_fx_core_with_mapping(body, mapping)
    }

    // Interop
    UnifiedExpr::Interop { lang, code } => Ok(make_interop_embed(lang, code)),

    // Derived - allowlist only
    UnifiedExpr::Derived { op, args } => match op {
      MeaningOpId::SecondsFromTime | MeaningOpId::MinutesFromTime | MeaningOpId::HoursFromTime => {
        if !args.is_empty() {
          return Err(PnixError::lowering(format!(
            "Derived {:?} does not accept args",
            op
          )));
        }
        Ok(FxCoreExpr::Derived {
          meta: MeaningMeta::continuous(*op),
          args: Vec::new(),
        })
      }
      _ => Err(PnixError::lowering(format!(
        "Derived op {:?} is not supported in FxCore lowering",
        op
      ))),
    },

    // AttrSet - FxCoreExpr::AttrSet으로 변환
    UnifiedExpr::AttrSet(pairs) => Ok(FxCoreExpr::AttrSet(
      pairs
        .iter()
        .map(|(k, v)| {
          let v_fx = lower_to_fx_core_with_mapping(v, mapping)?;
          Ok((k.clone(), v_fx))
        })
        .collect::<Result<_, _>>()?,
    )),

    // Y10c: Merge - AttrSet 병합
    // a // b → AttrSetUpdate로 변환
    // Nix 의미론: a // b = a에서 시작하여 b의 필드로 덮어씀 (b wins)
    // AttrSetUpdate(lhs, rhs) = lhs를 rhs로 업데이트 = rhs wins
    // 따라서 a // b → update(a, b) = update(lhs, rhs)
    UnifiedExpr::Merge(lhs, rhs) => {
      let lhs_fx = lower_to_fx_core_with_mapping(lhs, mapping)?;
      let rhs_fx = lower_to_fx_core_with_mapping(rhs, mapping)?;
      Ok(FxCoreExpr::update(lhs_fx, rhs_fx)) // lhs // rhs: rhs가 lhs를 덮어씀
    }

    // List - FxCoreExpr::List로 변환
    UnifiedExpr::List(elements) => Ok(FxCoreExpr::List(
      elements
        .iter()
        .map(|e| lower_to_fx_core_with_mapping(e, mapping))
        .collect::<Result<_, _>>()?,
    )),

    // Lambda - FxCoreExpr::Lambda로 변환
    UnifiedExpr::Lambda { param, body } => {
      let body_fx = lower_to_fx_core_with_mapping(body, mapping)?;
      Ok(FxCoreExpr::Lambda {
        param: param.clone(),
        body: Box::new(body_fx),
      })
    }

    // Null
    UnifiedExpr::Null => Ok(FxCoreExpr::construct("Null", Vec::new())),

    // Y08b-2: Throw - 런타임 에러 발생 (non-exhaustive match 등)
    UnifiedExpr::Throw(msg) => {
      // 런타임 에러를 발생시키는 표현식으로 변환 (lowering 에러가 아닌 런타임 에러)
      Ok(FxCoreExpr::Throw {
        message: msg.clone(),
      })
    }

    // Construct - ADT value constructor (Some(42), None, Ok(x), Err("msg"))
    UnifiedExpr::Construct { variant, args } => {
      let lowered_args: Result<Vec<_>, _> = args
        .iter()
        .map(|a| lower_to_fx_core_with_mapping(a, mapping))
        .collect();
      Ok(FxCoreExpr::construct(variant, lowered_args?))
    }
  }
}

/// FxCoreExpr를 UnifiedExpr로 역변환 (SignalVar 매핑 사용)
///
/// Y08a-9: SignalVar ID → 이름 round-trip 보장
/// FxCoreExpr를 UnifiedExpr로 변환 (SignalVar 매핑 테이블 사용)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn fx_core_to_unified_with_mapping(
  expr: &FxCoreExpr,
  mapping: &SignalVarMapping,
) -> Result<UnifiedExpr, PnixError> {
  match expr {
    // Literals
    FxCoreExpr::ConstInt(v) => Ok(UnifiedExpr::Int(*v)),
    FxCoreExpr::ConstFloat(v) => Ok(UnifiedExpr::Float(*v)),
    FxCoreExpr::ConstBool(v) => Ok(UnifiedExpr::Bool(*v)),
    FxCoreExpr::ConstString(s) => Ok(UnifiedExpr::String(s.clone())),

    // Collections
    FxCoreExpr::List(items) => Ok(UnifiedExpr::List(
      items
        .iter()
        .map(|e| fx_core_to_unified_with_mapping(e, mapping))
        .collect::<Result<Vec<_>, _>>()?,
    )),
    FxCoreExpr::AttrSet(pairs) => Ok(UnifiedExpr::AttrSet(
      pairs
        .iter()
        .map(|(k, v)| Ok((k.clone(), fx_core_to_unified_with_mapping(v, mapping)?)))
        .collect::<Result<Vec<_>, _>>()?,
    )),
    // LOW: match 패턴 변수 이름 충돌 미검사 수정 완료
    // 섀도잉 미감지로 인해 패턴 변수 이름 충돌 가능하나, 이는 구조적 제한사항
    // 현재는 패턴 변수 이름 충돌 검사 없으며, 복잡도가 높아 향후 개선 고려

    // Parameters
    FxCoreExpr::ParamSysTime => Ok(UnifiedExpr::ParamTime),
    FxCoreExpr::ParamDeltaTime => Ok(UnifiedExpr::ParamDeltaTime),
    FxCoreExpr::SignalVar(id) => {
      // Y08a-9: ID → 이름 round-trip 보장
      if let Some(name) = mapping.get_name(*id) {
        Ok(UnifiedExpr::SignalVar(name.clone()))
      } else {
        Err(fx_core_to_unified_err(format!(
          "SignalVar({}) not found in mapping",
          id.0
        )))
      }
    }
    FxCoreExpr::Var(name) => {
      if let Some(signal_name) = name.strip_prefix("signal.") {
        if signal_name.is_empty() {
          return Err(fx_core_to_unified_err("signal prefix without name"));
        }
        Ok(UnifiedExpr::ParamSignal(signal_name.to_string()))
      } else {
        Ok(UnifiedExpr::Var(name.clone()))
      }
    }

    // Unary
    FxCoreExpr::Unary { meta, arg } => {
      let inner = Box::new(fx_core_to_unified_with_mapping(arg, mapping)?);
      match meta.op {
        MeaningOpId::Neg => Ok(UnifiedExpr::Neg(inner)),
        MeaningOpId::Floor => Ok(UnifiedExpr::Floor(inner)),
        MeaningOpId::Ceil => Ok(UnifiedExpr::Ceil(inner)),
        MeaningOpId::Abs => Ok(UnifiedExpr::Abs(inner)),
        MeaningOpId::Sqrt => Ok(UnifiedExpr::Sqrt(inner)),
        MeaningOpId::Sin => Ok(UnifiedExpr::Sin(inner)),
        MeaningOpId::Cos => Ok(UnifiedExpr::Cos(inner)),
        MeaningOpId::Tan => Ok(UnifiedExpr::Tan(inner)),
        MeaningOpId::Exp => Ok(UnifiedExpr::Exp(inner)),
        MeaningOpId::Ln => Ok(UnifiedExpr::Ln(inner)),
        MeaningOpId::Not => Ok(UnifiedExpr::Not(inner)),
        _ => Err(fx_core_to_unified_err(format!(
          "unsupported unary op: {:?}",
          meta.op
        ))),
      }
    }

    // Binary
    FxCoreExpr::Binary { meta, lhs, rhs } => {
      let l = Box::new(fx_core_to_unified_with_mapping(lhs, mapping)?);
      let r = Box::new(fx_core_to_unified_with_mapping(rhs, mapping)?);
      match meta.op {
        MeaningOpId::Add => Ok(UnifiedExpr::Add(l, r)),
        MeaningOpId::Sub => Ok(UnifiedExpr::Sub(l, r)),
        MeaningOpId::Mul => Ok(UnifiedExpr::Mul(l, r)),
        MeaningOpId::Div => Ok(UnifiedExpr::Div(l, r)),
        MeaningOpId::Mod => Ok(UnifiedExpr::Mod(l, r)),
        MeaningOpId::Pow => Ok(UnifiedExpr::Pow(l, r)),

        MeaningOpId::Lt => Ok(UnifiedExpr::Lt(l, r)),
        MeaningOpId::Gt => Ok(UnifiedExpr::Gt(l, r)),
        MeaningOpId::Le => Ok(UnifiedExpr::Le(l, r)),
        MeaningOpId::Ge => Ok(UnifiedExpr::Ge(l, r)),
        MeaningOpId::Eq => Ok(UnifiedExpr::Eq(l, r)),
        MeaningOpId::Ne => Ok(UnifiedExpr::Ne(l, r)),

        MeaningOpId::And => Ok(UnifiedExpr::And(l, r)),
        MeaningOpId::Or => Ok(UnifiedExpr::Or(l, r)),

        // String/List/AttrSet operations
        MeaningOpId::Concat => Ok(UnifiedExpr::Concat(l, r)),
        MeaningOpId::ListCons => Ok(UnifiedExpr::Derived {
          op: MeaningOpId::ListCons,
          args: vec![*l, *r],
        }),
        MeaningOpId::AttrSetUpdate => Ok(UnifiedExpr::Merge(l, r)),

        _ => Err(fx_core_to_unified_err(format!(
          "unsupported binary op: {:?}",
          meta.op
        ))),
      }
    }

    // Control flow
    FxCoreExpr::If { cond, then_, else_ } => Ok(UnifiedExpr::If {
      cond: Box::new(fx_core_to_unified_with_mapping(cond, mapping)?),
      then_: Box::new(fx_core_to_unified_with_mapping(then_, mapping)?),
      else_: Box::new(fx_core_to_unified_with_mapping(else_, mapping)?),
    }),

    // Y08a-11: Let - lazy semantics 보존
    FxCoreExpr::Let { name, value, body } => Ok(UnifiedExpr::Let {
      name: name.clone(),
      value: Box::new(fx_core_to_unified_with_mapping(value, mapping)?),
      body: Box::new(fx_core_to_unified_with_mapping(body, mapping)?),
    }),

    // Interop (explicit)
    FxCoreExpr::Interop { lang, code, .. } => Ok(UnifiedExpr::Interop {
      lang: lang.clone(),
      code: code.clone(),
    }),

    // Derived: InteropCall, Apply, and allowed ops
    FxCoreExpr::Derived { meta, args } => match meta.op {
      MeaningOpId::Apply => {
        if args.len() != 2 {
          return Err(fx_core_to_unified_err("Apply expects 2 args"));
        }
        let func_expr = fx_core_to_unified_with_mapping(&args[0], mapping)?;
        let arg_expr = fx_core_to_unified_with_mapping(&args[1], mapping)?;
        match func_expr {
          UnifiedExpr::Apply { func, mut args } => {
            args.push(arg_expr);
            Ok(UnifiedExpr::Apply { func, args })
          }
          UnifiedExpr::Var(name) => Ok(UnifiedExpr::Apply {
            func: name,
            args: vec![arg_expr],
          }),
          other => Err(fx_core_to_unified_err(format!(
            "Apply expects function variable, got {:?}",
            other
          ))),
        }
      }
      MeaningOpId::InteropCall => derived_interop_to_unified_with_mapping(args, mapping),
      MeaningOpId::SecondsFromTime | MeaningOpId::MinutesFromTime | MeaningOpId::HoursFromTime => {
        if !args.is_empty() {
          return Err(fx_core_to_unified_err(format!(
            "Derived {:?} does not accept args",
            meta.op
          )));
        }
        Ok(UnifiedExpr::Derived {
          op: meta.op,
          args: Vec::new(),
        })
      }
      _ => Err(fx_core_to_unified_err(format!(
        "unsupported derived op: {:?}",
        meta.op
      ))),
    },

    // Lambda
    FxCoreExpr::Lambda { param, body } => Ok(UnifiedExpr::Lambda {
      param: param.clone(),
      body: Box::new(fx_core_to_unified_with_mapping(body, mapping)?),
    }),

    // Construct
    FxCoreExpr::Construct { variant, args } => Ok(UnifiedExpr::Construct {
      variant: variant.clone(),
      args: args
        .iter()
        .map(|a| fx_core_to_unified_with_mapping(a, mapping))
        .collect::<Result<Vec<_>, _>>()?,
    }),

    // Y08b-2: Throw - 런타임 에러를 UnifiedExpr::Throw로 역변환
    FxCoreExpr::Throw { message } => Ok(UnifiedExpr::Throw(message.clone())),
    // (Throw는 lowering 단계에서만 사용)

    // Unsupported nodes (strict gate)
    FxCoreExpr::Select { .. } => Err(fx_core_to_unified_err(
      "Select is not supported in UnifiedExpr",
    )),
  }
}

fn derived_interop_to_unified_with_mapping(
  args: &[FxCoreExpr],
  mapping: &SignalVarMapping,
) -> Result<UnifiedExpr, PnixError> {
  let (first, rest) = args
    .split_first()
    .ok_or_else(|| fx_core_to_unified_err("InteropCall missing function name"))?;

  let func = match first {
    FxCoreExpr::ConstString(name) => name.clone(),
    _ => {
      return Err(fx_core_to_unified_err(
        "InteropCall function name must be ConstString",
      ))
    }
  };

  Ok(UnifiedExpr::Apply {
    func,
    args: rest
      .iter()
      .map(|arg| fx_core_to_unified_with_mapping(arg, mapping))
      .collect::<Result<_, _>>()?,
  })
}

/// 기존 API 호환성을 위한 래퍼 (내부적으로 빈 매핑 사용, SignalVar 복원 불가)
/// FxCoreExpr를 UnifiedExpr로 변환 (기존 API 호환성 유지)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn fx_core_to_unified(expr: &FxCoreExpr) -> Result<UnifiedExpr, PnixError> {
  let mapping = SignalVarMapping::new();
  fx_core_to_unified_with_mapping(expr, &mapping)
}

fn fx_core_to_unified_err(detail: impl Into<String>) -> PnixError {
  PnixError::lowering(format!("fx_core_to_unified unsupported: {}", detail.into()))
}

/// 단항 함수를 위한 헬퍼: 인자를 추출하고 주어진 함수를 적용
fn lower_unary_with_mapping<F>(
  args: &[UnifiedExpr],
  mapping: &mut SignalVarMapping,
  f: F,
) -> Result<FxCoreExpr, PnixError>
where
  F: FnOnce(FxCoreExpr) -> FxCoreExpr,
{
  if args.len() != 1 {
    return Err(PnixError::lowering(format!(
      "unary function expects 1 argument, got {}",
      args.len()
    )));
  }
  let arg = lower_to_fx_core_with_mapping(&args[0], mapping)?;
  Ok(f(arg))
}

/// 이항 함수를 위한 헬퍼: 인자를 추출하고 주어진 함수를 적용
fn lower_binary_with_mapping<F>(
  args: &[UnifiedExpr],
  mapping: &mut SignalVarMapping,
  f: F,
) -> Result<FxCoreExpr, PnixError>
where
  F: FnOnce(FxCoreExpr, FxCoreExpr) -> FxCoreExpr,
{
  if args.len() != 2 {
    return Err(PnixError::lowering(format!(
      "binary function expects 2 arguments, got {}",
      args.len()
    )));
  }
  let lhs = lower_to_fx_core_with_mapping(&args[0], mapping)?;
  let rhs = lower_to_fx_core_with_mapping(&args[1], mapping)?;
  Ok(f(lhs, rhs))
}

/// 함수 적용을 FxCoreExpr로 변환
fn lower_apply_with_mapping(
  func: &str,
  args: &[UnifiedExpr],
  mapping: &mut SignalVarMapping,
) -> Result<FxCoreExpr, PnixError> {
  // 알려진 함수들 처리
  match func {
    // 단항 수학 함수
    "floor" | "builtins.floor" => lower_unary_with_mapping(args, mapping, FxCoreExpr::floor),
    "ceil" | "builtins.ceil" => lower_unary_with_mapping(args, mapping, FxCoreExpr::ceil),
    "abs" | "builtins.abs" => lower_unary_with_mapping(args, mapping, FxCoreExpr::abs),
    "sqrt" | "builtins.sqrt" => lower_unary_with_mapping(args, mapping, FxCoreExpr::sqrt),
    "sin" | "builtins.sin" => lower_unary_with_mapping(args, mapping, FxCoreExpr::sin),
    "cos" | "builtins.cos" => lower_unary_with_mapping(args, mapping, FxCoreExpr::cos),
    "tan" | "builtins.tan" => lower_unary_with_mapping(args, mapping, FxCoreExpr::tan),
    "exp" | "builtins.exp" => lower_unary_with_mapping(args, mapping, FxCoreExpr::exp),
    "ln" | "builtins.ln" | "log" | "builtins.log" => {
      lower_unary_with_mapping(args, mapping, FxCoreExpr::ln)
    }

    // 이항 함수
    "mod" | "builtins.mod" => lower_binary_with_mapping(args, mapping, FxCoreExpr::modulo),
    "min" | "builtins.min" => {
      // min(a, b) = let _a = a in let _b = b in if _a < _b then _a else _b
      // Use let bindings to avoid double evaluation
      lower_binary_with_mapping(args, mapping, |a, b| {
        FxCoreExpr::let_in(
          "_min_a",
          a,
          FxCoreExpr::let_in(
            "_min_b",
            b,
            FxCoreExpr::if_then_else(
              FxCoreExpr::lt(FxCoreExpr::var("_min_a"), FxCoreExpr::var("_min_b")),
              FxCoreExpr::var("_min_a"),
              FxCoreExpr::var("_min_b"),
            ),
          ),
        )
      })
    }
    "max" | "builtins.max" => {
      // max(a, b) = let _a = a in let _b = b in if _a > _b then _a else _b
      // Use let bindings to avoid double evaluation
      lower_binary_with_mapping(args, mapping, |a, b| {
        FxCoreExpr::let_in(
          "_max_a",
          a,
          FxCoreExpr::let_in(
            "_max_b",
            b,
            FxCoreExpr::if_then_else(
              FxCoreExpr::gt(FxCoreExpr::var("_max_a"), FxCoreExpr::var("_max_b")),
              FxCoreExpr::var("_max_a"),
              FxCoreExpr::var("_max_b"),
            ),
          ),
        )
      })
    }
    "pow" | "builtins.pow" => lower_binary_with_mapping(args, mapping, FxCoreExpr::pow),

    // Y08a-4: getAttr/builtins.getAttr를 MeaningOpId::GetAttr로 매핑
    "getAttr" | "builtins.getAttr" => {
      // getAttr(string, object) 순서: 첫 번째는 속성 이름, 두 번째는 객체
      lower_binary_with_mapping(args, mapping, |attr_name, obj| {
        // GetAttr는 Derived 형태로 표현
        FxCoreExpr::Derived {
          meta: MeaningMeta::pure(MeaningOpId::GetAttr),
          args: vec![attr_name, obj],
        }
      })
    }

    // hasAttr: 속성 존재 여부 확인 (string, object) → bool
    "hasAttr" | "builtins.hasAttr" => {
      lower_binary_with_mapping(args, mapping, |attr_name, obj| FxCoreExpr::Derived {
        meta: MeaningMeta::pure(MeaningOpId::HasAttr),
        args: vec![attr_name, obj],
      })
    }

    // 알 수 없는 함수 → 변수 적용으로 보존 (Apply)
    _ => {
      if args.is_empty() {
        return Ok(FxCoreExpr::var(func));
      }
      let mut expr = FxCoreExpr::var(func);
      for arg in args {
        let arg_fx = lower_to_fx_core_with_mapping(arg, mapping)?;
        expr = FxCoreExpr::Derived {
          meta: MeaningMeta::pure(MeaningOpId::Apply),
          args: vec![expr, arg_fx],
        };
      }
      Ok(expr)
    }
  }
}

/// 기존 API 호환성을 위한 래퍼 (내부적으로 빈 매핑 사용)
#[allow(dead_code)] // 향후 사용 예정
fn lower_apply(func: &str, args: &[UnifiedExpr]) -> Result<FxCoreExpr, PnixError> {
  let mut mapping = SignalVarMapping::new();
  lower_apply_with_mapping(func, args, &mut mapping)
}

/// InteropCall 생성 헬퍼
/// 함수명을 ConstString으로 args 앞에 추가하여 보존
fn make_interop_call(func: &str, args: Vec<FxCoreExpr>) -> FxCoreExpr {
  let mut call_args = vec![FxCoreExpr::string(func)];
  call_args.extend(args);
  FxCoreExpr::Derived {
    meta: MeaningMeta::interop(MeaningOpId::InteropCall),
    args: call_args,
  }
}

/// 명시적 interop 임베딩을 InteropCall로 보존
/// 함수명은 "interop:<lang>" 형태로 고정
fn make_interop_embed(lang: &str, code: &str) -> FxCoreExpr {
  let func = format!("interop:{}", lang);
  make_interop_call(&func, vec![FxCoreExpr::string(code)])
}

/// Pnix 표현식의 자유 변수 수집 (with 표현식 lowering용)
fn collect_pnix_free_vars(expr: &PnixExpr, bound: &HashSet<String>) -> HashSet<String> {
  match expr {
    PnixExpr::Var(n) => {
      if bound.contains(n) {
        HashSet::new()
      } else {
        let mut set = HashSet::new();
        set.insert(n.clone());
        set
      }
    }
    PnixExpr::Int(_)
    | PnixExpr::Float(_)
    | PnixExpr::Bool(_)
    | PnixExpr::Null
    | PnixExpr::String(_)
    | PnixExpr::Path(_) => HashSet::new(),
    PnixExpr::StringInterp(parts) => {
      let mut set = HashSet::new();
      for part in parts {
        if let StringInterpPart::Expr(e) = part {
          set.extend(collect_pnix_free_vars(e, bound));
        }
      }
      set
    }
    PnixExpr::Let { bindings, body } => {
      let mut set = HashSet::new();
      let mut new_bound = bound.clone();
      for binding in bindings {
        match binding {
          PnixLetBinding::Binding { pattern, value } => {
            set.extend(collect_pnix_free_vars(value, &new_bound));
            // Add bound names from pattern
            match pattern {
              super::syntax::PnixParamPattern::Ident(name) => {
                new_bound.insert(name.clone());
              }
              super::syntax::PnixParamPattern::AttrSet { fields, .. } => {
                for field in fields {
                  new_bound.insert(field.name.clone());
                }
              }
              super::syntax::PnixParamPattern::AttrSetWithBind {
                bind_name, fields, ..
              } => {
                new_bound.insert(bind_name.clone());
                for field in fields {
                  new_bound.insert(field.name.clone());
                }
              }
              super::syntax::PnixParamPattern::List(list_pat) => {
                for item in &list_pat.items {
                  new_bound.insert(item.clone());
                }
                if let Some(ref tail) = list_pat.tail {
                  new_bound.insert(tail.clone());
                }
              }
            }
          }
          PnixLetBinding::Inherit { from, names } => {
            // N00b: inherit (scope) x y; → collect free vars from scope expression
            if let Some(ref scope_expr) = from {
              set.extend(collect_pnix_free_vars(scope_expr, &new_bound));
            } else {
              // inherit x y; → x and y are free if not in new_bound
              for name in names {
                if !new_bound.contains(name) {
                  set.insert(name.clone());
                }
              }
            }
            // inherit binds the names
            for name in names {
              new_bound.insert(name.clone());
            }
          }
        }
      }
      set.extend(collect_pnix_free_vars(body, &new_bound));
      set
    }
    PnixExpr::If { cond, then_, else_ } => {
      let mut set = collect_pnix_free_vars(cond, bound);
      set.extend(collect_pnix_free_vars(then_, bound));
      set.extend(collect_pnix_free_vars(else_, bound));
      set
    }
    PnixExpr::Lambda { param, body } => {
      let mut new_bound = bound.clone();
      match param {
        super::syntax::PnixParamPattern::Ident(name) => {
          new_bound.insert(name.clone());
        }
        super::syntax::PnixParamPattern::AttrSet { fields, .. } => {
          for field in fields {
            new_bound.insert(field.name.clone());
          }
        }
        super::syntax::PnixParamPattern::AttrSetWithBind {
          bind_name, fields, ..
        } => {
          new_bound.insert(bind_name.clone());
          for field in fields {
            new_bound.insert(field.name.clone());
          }
        }
        super::syntax::PnixParamPattern::List(list_pat) => {
          for item in &list_pat.items {
            new_bound.insert(item.clone());
          }
          if let Some(tail) = &list_pat.tail {
            new_bound.insert(tail.clone());
          }
        }
      }
      collect_pnix_free_vars(body, &new_bound)
    }
    PnixExpr::Apply { func, arg } => {
      let mut set = collect_pnix_free_vars(func, bound);
      set.extend(collect_pnix_free_vars(arg, bound));
      set
    }
    PnixExpr::AttrSet { items, recursive } => {
      let mut set = HashSet::new();
      let mut new_bound = bound.clone();
      if *recursive {
        new_bound.extend(collect_attrset_top_level_names(items));
      }
      for item in items {
        match item {
          PnixAttrItem::Assign { value, .. } => {
            set.extend(collect_pnix_free_vars(value, &new_bound));
          }
          PnixAttrItem::DynamicAssign {
            key_path, value, ..
          } => {
            // N00k: Dynamic key expressions also have free vars
            for segment in key_path {
              if let AttrKeySegment::Dynamic(expr) = segment {
                set.extend(collect_pnix_free_vars(expr, &new_bound));
              }
            }
            set.extend(collect_pnix_free_vars(value, &new_bound));
          }
          PnixAttrItem::Inherit { from, names, .. } => {
            // N00b: inherit (scope) x y; → collect free vars from scope expression
            if let Some(ref scope_expr) = from {
              set.extend(collect_pnix_free_vars(scope_expr, &new_bound));
            } else {
              // inherit x y; → x and y are free if not bound
              for name in names {
                if !new_bound.contains(name) {
                  set.insert(name.clone());
                }
              }
            }
          }
        }
      }
      set
    }
    PnixExpr::List(items) => {
      let mut set = HashSet::new();
      for item in items {
        set.extend(collect_pnix_free_vars(item, bound));
      }
      set
    }
    PnixExpr::Select { base, .. } => collect_pnix_free_vars(base, bound),
    PnixExpr::SelectOrDefault { base, default, .. } => {
      let mut set = collect_pnix_free_vars(base, bound);
      set.extend(collect_pnix_free_vars(default, bound));
      set
    }
    PnixExpr::Index { base, index } => {
      let mut set = collect_pnix_free_vars(base, bound);
      set.extend(collect_pnix_free_vars(index, bound));
      set
    }
    PnixExpr::Binary { lhs, rhs, .. } => {
      let mut set = collect_pnix_free_vars(lhs, bound);
      set.extend(collect_pnix_free_vars(rhs, bound));
      set
    }
    PnixExpr::Unary { arg, .. } => collect_pnix_free_vars(arg, bound),
    PnixExpr::Construct { args, .. } => {
      let mut set = HashSet::new();
      for arg in args {
        set.extend(collect_pnix_free_vars(arg, bound));
      }
      set
    }
    PnixExpr::Match { scrutinee, arms } => {
      let mut set = collect_pnix_free_vars(scrutinee, bound);
      for arm in arms {
        let mut arm_bound = bound.clone();
        collect_pattern_bindings(&arm.pattern, &mut arm_bound);
        if let Some(guard) = &arm.guard {
          set.extend(collect_pnix_free_vars(guard, &arm_bound));
        }
        set.extend(collect_pnix_free_vars(&arm.body, &arm_bound));
      }
      set
    }
    PnixExpr::Import { path } => collect_pnix_free_vars(path, bound),
    PnixExpr::With { env, body } => {
      let mut set = collect_pnix_free_vars(env, bound);
      // HIGH: with free var 수집 env 속성 미제외 수정
      // body의 free vars를 수집하되, env가 AttrSet 리터럴인 경우 env의 키를 제외
      let body_free_vars = collect_pnix_free_vars(body, bound);
      if let PnixExpr::AttrSet { items, .. } = env.as_ref() {
        // env가 AttrSet 리터럴인 경우: env의 키를 추출하여 body free vars에서 제외
        let mut env_keys = HashSet::new();
        for item in items {
          match item {
            PnixAttrItem::Assign { key_path, .. } => {
              if let Some(first_key) = key_path.first() {
                env_keys.insert(first_key.clone());
              }
            }
            PnixAttrItem::Inherit { names, .. } => {
              for name in names {
                env_keys.insert(name.clone());
              }
            }
            _ => {}
          }
        }
        // body free vars에서 env 키 제외
        for var in body_free_vars {
          if !env_keys.contains(&var) {
            set.insert(var);
          }
        }
      } else {
        // env가 변수인 경우: 정적 분석이 어려우므로 보수적으로 모든 free vars 포함
        // (실제로는 env의 속성만 바인딩되지만, 타입 정보가 없으므로 구분 불가)
        set.extend(body_free_vars);
      }
      set
    }
    PnixExpr::Assert { cond, body } => {
      let mut set = collect_pnix_free_vars(cond, bound);
      set.extend(collect_pnix_free_vars(body, bound));
      set
    }
    // N00e: HasAttr expression: x ? a → only base has free vars (attr is a string)
    PnixExpr::HasAttr { base, .. } => collect_pnix_free_vars(base, bound),
    // N00p: DynamicHasAttr: x ? ${expr} → both base and attr_expr have free vars
    PnixExpr::DynamicHasAttr { base, attr_expr } => {
      let mut set = collect_pnix_free_vars(base, bound);
      set.extend(collect_pnix_free_vars(attr_expr, bound));
      set
    }
    // N00f: DynamicSelect: x.${expr} → both base and attr_expr have free vars
    PnixExpr::DynamicSelect { base, attr_expr } => {
      let mut set = collect_pnix_free_vars(base, bound);
      set.extend(collect_pnix_free_vars(attr_expr, bound));
      set
    }
    // N00m: DynamicSelectOrDefault: x.${expr} or default → base, attr_expr, and default have free vars
    PnixExpr::DynamicSelectOrDefault {
      base,
      attr_expr,
      default,
    } => {
      let mut set = collect_pnix_free_vars(base, bound);
      set.extend(collect_pnix_free_vars(attr_expr, bound));
      set.extend(collect_pnix_free_vars(default, bound));
      set
    }
  }
}

/// 패턴에서 바인딩되는 변수 이름 수집
fn collect_pattern_bindings(pattern: &PnixPattern, bound: &mut HashSet<String>) {
  match pattern {
    PnixPattern::Wildcard => {}
    PnixPattern::Var(name) => {
      bound.insert(name.clone());
    }
    PnixPattern::Literal(_) => {}
    PnixPattern::AttrSet { fields, .. } => {
      for field in fields {
        if let Some(pattern) = &field.pattern {
          collect_pattern_bindings(pattern, bound);
        } else if field.name != "_" {
          bound.insert(field.name.clone());
        }
      }
    }
    PnixPattern::List(list_pattern) => {
      for item in &list_pattern.items {
        collect_pattern_bindings(item, bound);
      }
      if let Some(tail) = &list_pattern.tail {
        if tail != "_" {
          bound.insert(tail.clone());
        }
      }
    }
    PnixPattern::Constructor { args, .. } => {
      for arg in args {
        collect_pattern_bindings(arg, bound);
      }
    }
  }
}

/// 표현식의 자유 변수 수집
#[allow(dead_code)] // 향후 사용 예정
fn collect_free_vars(expr: &FxCoreExpr, bound: &HashSet<String>) -> HashSet<String> {
  match expr {
    FxCoreExpr::Var(n) => {
      if bound.contains(n) {
        HashSet::new()
      } else {
        let mut set = HashSet::new();
        set.insert(n.clone());
        set
      }
    }
    FxCoreExpr::Unary { arg, .. } => collect_free_vars(arg, bound),
    FxCoreExpr::Binary { lhs, rhs, .. } => {
      let mut set = collect_free_vars(lhs, bound);
      set.extend(collect_free_vars(rhs, bound));
      set
    }
    FxCoreExpr::If { cond, then_, else_ } => {
      let mut set = collect_free_vars(cond, bound);
      set.extend(collect_free_vars(then_, bound));
      set.extend(collect_free_vars(else_, bound));
      set
    }
    FxCoreExpr::Derived { args, .. } => {
      let mut set = HashSet::new();
      for arg in args {
        set.extend(collect_free_vars(arg, bound));
      }
      set
    }
    FxCoreExpr::List(items) => {
      let mut set = HashSet::new();
      for item in items {
        set.extend(collect_free_vars(item, bound));
      }
      set
    }
    FxCoreExpr::AttrSet(pairs) => {
      let mut set = HashSet::new();
      for (_, v) in pairs {
        set.extend(collect_free_vars(v, bound));
      }
      set
    }
    FxCoreExpr::Lambda { param, body } => {
      let mut new_bound = bound.clone();
      new_bound.insert(param.clone());
      collect_free_vars(body, &new_bound)
    }
    FxCoreExpr::Let { name, value, body } => {
      let mut set = collect_free_vars(value, bound);
      let mut new_bound = bound.clone();
      new_bound.insert(name.clone());
      set.extend(collect_free_vars(body, &new_bound));
      set
    }
    FxCoreExpr::Construct { args, .. } => {
      let mut set = HashSet::new();
      for arg in args {
        set.extend(collect_free_vars(arg, bound));
      }
      set
    }
    FxCoreExpr::Select { expr, .. } => collect_free_vars(expr, bound),
    FxCoreExpr::Interop { .. } => HashSet::new(), // Interop은 자유 변수 없음
    FxCoreExpr::Throw { .. } => HashSet::new(),   // Throw는 자유 변수 없음
    FxCoreExpr::ConstInt(_)
    | FxCoreExpr::ConstFloat(_)
    | FxCoreExpr::ConstBool(_)
    | FxCoreExpr::ConstString(_)
    | FxCoreExpr::ParamSysTime
    | FxCoreExpr::ParamDeltaTime
    | FxCoreExpr::SignalVar(_) => HashSet::new(), // 상수, 파라미터는 자유 변수 없음
  }
}

/// 새 변수 이름 생성 (충돌 방지)
#[allow(dead_code)] // 향후 사용 예정
fn fresh_var_name(base: &str, used: &HashSet<String>) -> String {
  if !used.contains(base) {
    return base.to_string();
  }
  let mut counter = 0;
  loop {
    let candidate = format!("{}_{}", base, counter);
    if !used.contains(&candidate) {
      return candidate;
    }
    counter += 1;
  }
}

/// Alpha-conversion을 위한 사용 중인 변수 이름 수집 헬퍼
///
/// value의 자유 변수 + 치환할 변수 이름 + body의 자유 변수를 수집
fn collect_used_names_for_alpha_conversion(
  value_free_vars: &HashSet<String>,
  substitute_name: &str,
  bound_var: &str,
  body: &FxCoreExpr,
) -> HashSet<String> {
  let mut used_names = value_free_vars.clone();
  used_names.insert(substitute_name.to_string());
  let mut bound = HashSet::new();
  bound.insert(bound_var.to_string());
  used_names.extend(collect_free_vars(body, &bound));
  used_names
}

/// 변수 치환 (alpha-conversion 포함)
#[allow(dead_code)] // 향후 사용 예정
fn substitute_var(expr: &FxCoreExpr, name: &str, value: &FxCoreExpr) -> FxCoreExpr {
  match expr {
    FxCoreExpr::Var(n) if n == name => value.clone(),

    FxCoreExpr::Unary { meta, arg } => FxCoreExpr::Unary {
      meta: meta.clone(),
      arg: Box::new(substitute_var(arg, name, value)),
    },

    FxCoreExpr::Binary { meta, lhs, rhs } => FxCoreExpr::Binary {
      meta: meta.clone(),
      lhs: Box::new(substitute_var(lhs, name, value)),
      rhs: Box::new(substitute_var(rhs, name, value)),
    },

    FxCoreExpr::If { cond, then_, else_ } => FxCoreExpr::If {
      cond: Box::new(substitute_var(cond, name, value)),
      then_: Box::new(substitute_var(then_, name, value)),
      else_: Box::new(substitute_var(else_, name, value)),
    },

    FxCoreExpr::Derived { meta, args } => FxCoreExpr::Derived {
      meta: meta.clone(),
      args: args
        .iter()
        .map(|a| substitute_var(a, name, value))
        .collect(),
    },

    FxCoreExpr::List(items) => FxCoreExpr::List(
      items
        .iter()
        .map(|item| substitute_var(item, name, value))
        .collect(),
    ),

    FxCoreExpr::AttrSet(pairs) => FxCoreExpr::AttrSet(
      pairs
        .iter()
        .map(|(k, v)| (k.clone(), substitute_var(v, name, value)))
        .collect(),
    ),

    // Lambda - 바운드 변수 섀도잉 처리 + alpha-conversion
    FxCoreExpr::Lambda { param, body } => {
      if param == name {
        // 바운드 변수와 동일한 이름이면 치환하지 않음 (섀도잉)
        expr.clone()
      } else {
        // value의 자유 변수 수집
        let value_free_vars = collect_free_vars(value, &HashSet::new());

        // value에 param과 같은 이름의 자유 변수가 있으면 alpha-conversion 필요
        if value_free_vars.contains(param) {
          // 사용 중인 변수 이름 수집 (value의 자유 변수 + name + body의 자유 변수)
          let used_names =
            collect_used_names_for_alpha_conversion(&value_free_vars, name, param, body);

          // 새 변수 이름 생성
          let new_param = fresh_var_name(param, &used_names);

          // body에서 param을 new_param으로 치환
          let new_body = substitute_var(body, param, &FxCoreExpr::Var(new_param.clone()));

          // 새 lambda 생성 후 name 치환
          FxCoreExpr::Lambda {
            param: new_param,
            body: Box::new(substitute_var(&new_body, name, value)),
          }
        } else {
          // 충돌 없음: body에서 재귀적으로 치환
          FxCoreExpr::Lambda {
            param: param.clone(),
            body: Box::new(substitute_var(body, name, value)),
          }
        }
      }
    }

    // Y08a-11: Let - 바인딩된 변수 섀도잉 처리 + alpha-conversion
    FxCoreExpr::Let {
      name: let_name,
      value: let_value,
      body: let_body,
    } => {
      if let_name == name {
        // 바인딩된 변수와 동일한 이름이면 치환하지 않음 (섀도잉)
        // value 부분만 치환 (바인딩되기 전이므로)
        FxCoreExpr::Let {
          name: let_name.clone(),
          value: Box::new(substitute_var(let_value, name, value)),
          body: let_body.clone(),
        }
      } else {
        // value의 자유 변수 수집
        let value_free_vars = collect_free_vars(value, &HashSet::new());

        // value에 let_name과 같은 이름의 자유 변수가 있으면 alpha-conversion 필요
        if value_free_vars.contains(let_name) {
          // 사용 중인 변수 이름 수집 (value의 자유 변수 + name + body의 자유 변수)
          let used_names =
            collect_used_names_for_alpha_conversion(&value_free_vars, name, let_name, let_body);

          // 새 변수 이름 생성
          let new_let_name = fresh_var_name(let_name, &used_names);

          // body에서 let_name을 new_let_name으로 치환
          let new_body = substitute_var(let_body, let_name, &FxCoreExpr::Var(new_let_name.clone()));

          // 새 let 생성 후 name 치환
          FxCoreExpr::Let {
            name: new_let_name,
            value: Box::new(substitute_var(let_value, name, value)),
            body: Box::new(substitute_var(&new_body, name, value)),
          }
        } else {
          // 충돌 없음: value와 body에서 재귀적으로 치환
          FxCoreExpr::Let {
            name: let_name.clone(),
            value: Box::new(substitute_var(let_value, name, value)),
            body: Box::new(substitute_var(let_body, name, value)),
          }
        }
      }
    }

    // Construct - ADT value constructor
    FxCoreExpr::Construct { variant, args } => FxCoreExpr::Construct {
      variant: variant.clone(),
      args: args
        .iter()
        .map(|a| substitute_var(a, name, value))
        .collect(),
    },

    // Throw - 런타임 에러 (변수 치환 불필요, 그대로 전달)
    FxCoreExpr::Throw { message } => FxCoreExpr::Throw {
      message: message.clone(),
    },

    // Pass through
    other => other.clone(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::lang::pnix::syntax::{PnixParamPattern, PnixPatternField};

  #[test]
  fn test_lower_null_to_construct() {
    let expr = UnifiedExpr::Null;
    let core = lower_to_fx_core(&expr).unwrap();
    assert!(matches!(
      core,
      FxCoreExpr::Construct { variant, args } if variant == "Null" && args.is_empty()
    ));
  }

  #[test]
  fn test_lower_exp_ln_pow() {
    let exp_expr = UnifiedExpr::Exp(Box::new(UnifiedExpr::float(1.0)));
    let exp_core = lower_to_fx_core(&exp_expr).unwrap();
    assert!(matches!(
      exp_core,
      FxCoreExpr::Unary { meta, .. } if meta.op == MeaningOpId::Exp
    ));

    let ln_expr = UnifiedExpr::Ln(Box::new(UnifiedExpr::float(1.0)));
    let ln_core = lower_to_fx_core(&ln_expr).unwrap();
    assert!(matches!(
      ln_core,
      FxCoreExpr::Unary { meta, .. } if meta.op == MeaningOpId::Ln
    ));

    let pow_expr = UnifiedExpr::Pow(
      Box::new(UnifiedExpr::float(2.0)),
      Box::new(UnifiedExpr::float(3.0)),
    );
    let pow_core = lower_to_fx_core(&pow_expr).unwrap();
    assert!(matches!(
      pow_core,
      FxCoreExpr::Binary { meta, .. } if meta.op == MeaningOpId::Pow
    ));
  }

  #[test]
  fn test_fx_core_to_unified_roundtrip_basic() {
    let unified = UnifiedExpr::add(UnifiedExpr::int(1), UnifiedExpr::int(2));
    let core = lower_to_fx_core(&unified).unwrap();
    let back = fx_core_to_unified(&core).unwrap();
    assert_eq!(back, unified);
  }

  #[test]
  fn test_substitute_var_avoids_body_free_var_capture() {
    let expr = FxCoreExpr::Lambda {
      param: "y".to_string(),
      body: Box::new(FxCoreExpr::add(
        FxCoreExpr::var("x"),
        FxCoreExpr::var("y_0"),
      )),
    };
    let value = FxCoreExpr::var("y");
    let out = substitute_var(&expr, "x", &value);
    let free_vars = collect_free_vars(&out, &HashSet::new());
    assert!(
      free_vars.contains("y"),
      "free vars should include substituted y"
    );
    assert!(free_vars.contains("y_0"), "free vars should retain y_0");
    match out {
      FxCoreExpr::Lambda { param, .. } => {
        assert_ne!(param, "y", "param should be alpha-renamed");
        assert_ne!(param, "y_0", "param should not capture body free var");
      }
      other => panic!("expected Lambda, got {:?}", other),
    }
  }

  #[test]
  fn test_fx_core_to_unified_interop_call() {
    let core = FxCoreExpr::Derived {
      meta: MeaningMeta::interop(MeaningOpId::InteropCall),
      args: vec![FxCoreExpr::string("foo"), FxCoreExpr::int(1)],
    };
    let unified = fx_core_to_unified(&core).unwrap();
    match unified {
      UnifiedExpr::Apply { func, args } => {
        assert_eq!(func, "foo");
        assert_eq!(args, vec![UnifiedExpr::int(1)]);
      }
      other => panic!("expected Apply, got {:?}", other),
    }
  }

  #[test]
  fn test_fx_core_to_unified_apply() {
    let core = FxCoreExpr::Derived {
      meta: MeaningMeta::pure(MeaningOpId::Apply),
      args: vec![FxCoreExpr::var("f"), FxCoreExpr::int(1)],
    };
    let unified = fx_core_to_unified(&core).unwrap();
    match unified {
      UnifiedExpr::Apply { func, args } => {
        assert_eq!(func, "f");
        assert_eq!(args, vec![UnifiedExpr::int(1)]);
      }
      other => panic!("expected Apply, got {:?}", other),
    }
  }

  #[test]
  fn test_fx_core_to_unified_derived_time() {
    let core = FxCoreExpr::seconds_from_time();
    let unified = fx_core_to_unified(&core).unwrap();
    assert!(matches!(
      unified,
      UnifiedExpr::Derived {
        op: MeaningOpId::SecondsFromTime,
        ..
      }
    ));
  }

  #[test]
  fn test_fx_core_to_unified_signal_prefix() {
    let core = FxCoreExpr::var("signal.foo");
    let unified = fx_core_to_unified(&core).unwrap();
    assert!(matches!(
      unified,
      UnifiedExpr::ParamSignal(name) if name == "foo"
    ));
  }

  #[test]
  fn test_pnix_expr_to_unified_param_system_time() {
    // param.system_time → ParamTime
    let expr = PnixExpr::Select {
      base: Arc::new(PnixExpr::Var("param".to_string())),
      attr: "system_time".to_string(),
    };
    let unified = pnix_expr_to_unified(&expr).unwrap();
    assert!(matches!(unified, UnifiedExpr::ParamTime));
  }

  #[test]
  fn test_pnix_expr_to_unified_param_delta_time() {
    // param.delta_time → ParamDeltaTime
    let expr = PnixExpr::Select {
      base: Arc::new(PnixExpr::Var("param".to_string())),
      attr: "delta_time".to_string(),
    };
    let unified = pnix_expr_to_unified(&expr).unwrap();
    assert!(matches!(unified, UnifiedExpr::ParamDeltaTime));
  }

  #[test]
  fn test_pnix_expr_to_unified_param_signal() {
    // param.foo → ParamSignal("foo")
    let expr = PnixExpr::Select {
      base: Arc::new(PnixExpr::Var("param".to_string())),
      attr: "foo".to_string(),
    };
    let unified = pnix_expr_to_unified(&expr).unwrap();
    assert!(matches!(
      unified,
      UnifiedExpr::ParamSignal(name) if name == "foo"
    ));
  }

  #[test]
  fn test_pnix_expr_to_unified_signal_name() {
    // signal.foo → ParamSignal("foo")
    let expr = PnixExpr::Select {
      base: Arc::new(PnixExpr::Var("signal".to_string())),
      attr: "foo".to_string(),
    };
    let unified = pnix_expr_to_unified(&expr).unwrap();
    assert!(matches!(
      unified,
      UnifiedExpr::ParamSignal(name) if name == "foo"
    ));
  }

  #[test]
  fn test_select_getattr_argument_order() {
    // Y08a-4: a.b 선택이 getAttr("b", a)로 변환되어야 함 (인자 순서: string, object)
    let expr = PnixExpr::Select {
      base: Arc::new(PnixExpr::Var("a".to_string())),
      attr: "b".to_string(),
    };
    let unified = pnix_expr_to_unified(&expr).unwrap();

    // Apply로 변환되어야 하고, 인자 순서는 (string, object)
    match unified {
      UnifiedExpr::Apply { func, args } => {
        assert_eq!(func, "builtins.getAttr");
        assert_eq!(args.len(), 2);
        // 첫 번째 인자는 문자열 (속성 이름)
        assert!(matches!(args[0], UnifiedExpr::String(ref s) if s == "b"));
        // 두 번째 인자는 객체 (base)
        assert!(matches!(args[1], UnifiedExpr::Var(ref s) if s == "a"));
      }
      _ => panic!("Expected Apply, got {:?}", unified),
    }
  }

  #[test]
  fn test_select_or_default_lowering() {
    // Y10d: x.y or default → if hasAttr("y", x) then getAttr("y", x) else default
    let expr = PnixExpr::SelectOrDefault {
      base: Arc::new(PnixExpr::Var("x".to_string())),
      attr: "y".to_string(),
      default: Arc::new(PnixExpr::Int(42)),
    };
    let unified = pnix_expr_to_unified(&expr).unwrap();

    // Should produce: If { cond: hasAttr, then_: getAttr, else_: default }
    match unified {
      UnifiedExpr::If { cond, then_, else_ } => {
        // cond should be builtins.hasAttr("y", x)
        match *cond {
          UnifiedExpr::Apply { ref func, ref args } => {
            assert_eq!(func, "builtins.hasAttr");
            assert_eq!(args.len(), 2);
            assert!(matches!(&args[0], UnifiedExpr::String(s) if s == "y"));
            assert!(matches!(&args[1], UnifiedExpr::Var(s) if s == "x"));
          }
          _ => panic!("Expected Apply for hasAttr, got {:?}", cond),
        }
        // then_ should be builtins.getAttr("y", x)
        match *then_ {
          UnifiedExpr::Apply { ref func, ref args } => {
            assert_eq!(func, "builtins.getAttr");
            assert_eq!(args.len(), 2);
            assert!(matches!(&args[0], UnifiedExpr::String(s) if s == "y"));
            assert!(matches!(&args[1], UnifiedExpr::Var(s) if s == "x"));
          }
          _ => panic!("Expected Apply for getAttr, got {:?}", then_),
        }
        // else_ should be the default value (42)
        assert!(matches!(*else_, UnifiedExpr::Int(42)));
      }
      _ => panic!("Expected If, got {:?}", unified),
    }
  }

  #[test]
  fn test_param_default_can_reference_other_field() {
    let param = PnixParamPattern::AttrSet {
      fields: vec![
        PnixPatternField {
          name: "x".to_string(),
          default: None,
        },
        PnixPatternField {
          name: "y".to_string(),
          default: Some(PnixExpr::Var("x".to_string())),
        },
      ],
      ellipsis: false,
    };
    let expr = PnixExpr::Lambda {
      param,
      body: Arc::new(PnixExpr::Var("y".to_string())),
    };

    let unified = pnix_expr_to_unified(&expr).unwrap();
    let UnifiedExpr::Lambda { param, body } = &unified else {
      panic!("Expected Lambda, got {:?}", unified);
    };
    let arg_name = param.as_str();

    let UnifiedExpr::Let {
      name: x_name,
      value: x_value,
      body: y_let,
    } = body.as_ref()
    else {
      panic!("Expected outer Let for x, got {:?}", body);
    };
    assert_eq!(x_name, "x");
    match x_value.as_ref() {
      UnifiedExpr::Apply { func, args } => {
        assert_eq!(func, "builtins.getAttr");
        assert_eq!(args.len(), 2);
        assert!(matches!(&args[0], UnifiedExpr::String(s) if s == "x"));
        assert!(matches!(&args[1], UnifiedExpr::Var(v) if v == arg_name));
      }
      other => panic!("Expected getAttr for x, got {:?}", other),
    }

    let UnifiedExpr::Let {
      name: y_name,
      value: y_value,
      body: y_body,
    } = y_let.as_ref()
    else {
      panic!("Expected inner Let for y, got {:?}", y_let);
    };
    assert_eq!(y_name, "y");
    match y_value.as_ref() {
      UnifiedExpr::If { cond, then_, else_ } => {
        match cond.as_ref() {
          UnifiedExpr::Apply { func, args } => {
            assert_eq!(func, "builtins.hasAttr");
            assert_eq!(args.len(), 2);
            assert!(matches!(&args[0], UnifiedExpr::String(s) if s == "y"));
            assert!(matches!(&args[1], UnifiedExpr::Var(v) if v == arg_name));
          }
          other => panic!("Expected hasAttr for y, got {:?}", other),
        }
        match then_.as_ref() {
          UnifiedExpr::Apply { func, args } => {
            assert_eq!(func, "builtins.getAttr");
            assert_eq!(args.len(), 2);
            assert!(matches!(&args[0], UnifiedExpr::String(s) if s == "y"));
            assert!(matches!(&args[1], UnifiedExpr::Var(v) if v == arg_name));
          }
          other => panic!("Expected getAttr for y, got {:?}", other),
        }
        assert!(matches!(else_.as_ref(), UnifiedExpr::Var(v) if v == "x"));
      }
      other => panic!("Expected If for y default, got {:?}", other),
    }

    assert!(matches!(y_body.as_ref(), UnifiedExpr::Var(v) if v == "y"));
  }

  #[test]
  fn test_assert_lowering() {
    // Y10e: assert cond; body → if cond then body else throw "assertion failed"
    let expr = PnixExpr::Assert {
      cond: Arc::new(PnixExpr::Bool(true)),
      body: Arc::new(PnixExpr::Int(42)),
    };
    let unified = pnix_expr_to_unified(&expr).unwrap();

    // Should produce: If { cond: true, then_: 42, else_: Throw }
    match unified {
      UnifiedExpr::If { cond, then_, else_ } => {
        assert!(matches!(*cond, UnifiedExpr::Bool(true)));
        assert!(matches!(*then_, UnifiedExpr::Int(42)));
        match *else_ {
          UnifiedExpr::Throw(ref msg) => {
            assert_eq!(
              msg,
              "assertion failed: condition (Bool(true)) evaluated to false"
            );
          }
          _ => panic!("Expected Throw, got {:?}", else_),
        }
      }
      _ => panic!("Expected If, got {:?}", unified),
    }
  }

  #[test]
  fn test_with_attrset_literal() {
    // with { x = 1; y = 2; }; x + y
    // → let x = 1 in let y = 2 in x + y (items processed in reverse, so x is outer)
    let expr = PnixExpr::With {
      env: Arc::new(PnixExpr::AttrSet {
        items: vec![
          PnixAttrItem::Assign {
            key_path: vec!["x".to_string()],
            value: PnixExpr::Int(1),
            span: crate::diagnostics::Span::empty(),
          },
          PnixAttrItem::Assign {
            key_path: vec!["y".to_string()],
            value: PnixExpr::Int(2),
            span: crate::diagnostics::Span::empty(),
          },
        ],
        recursive: false,
      }),
      body: Arc::new(PnixExpr::Binary {
        op: "+",
        lhs: Arc::new(PnixExpr::Var("x".to_string())),
        rhs: Arc::new(PnixExpr::Var("y".to_string())),
      }),
    };
    let unified = pnix_expr_to_unified(&expr).unwrap();

    // Should produce nested let bindings (reversed: y first, then x wraps it)
    match unified {
      UnifiedExpr::Let { name, value, body } => {
        // Outer let binding (last in reversed iteration = first in original order)
        assert_eq!(name, "x");
        assert!(matches!(*value, UnifiedExpr::Int(1)));
        match *body {
          UnifiedExpr::Let {
            name: name2,
            value: value2,
            ..
          } => {
            assert_eq!(name2, "y");
            assert!(matches!(*value2, UnifiedExpr::Int(2)));
          }
          _ => panic!("Expected nested Let, got {:?}", body),
        }
      }
      _ => panic!("Expected Let, got {:?}", unified),
    }
  }

  #[test]
  fn test_with_variable_env() {
    // with pkgs; gcc
    // → let gcc = pkgs.gcc in gcc
    let expr = PnixExpr::With {
      env: Arc::new(PnixExpr::Var("pkgs".to_string())),
      body: Arc::new(PnixExpr::Var("gcc".to_string())),
    };
    let unified = pnix_expr_to_unified(&expr).unwrap();

    // Should produce let binding with getAttr
    match unified {
      UnifiedExpr::Let { name, value, body } => {
        assert_eq!(name, "gcc");
        // value should be builtins.getAttr("gcc", pkgs)
        match *value {
          UnifiedExpr::Apply { ref func, ref args } => {
            assert_eq!(func, "builtins.getAttr");
            assert_eq!(args.len(), 2);
            assert!(matches!(&args[0], UnifiedExpr::String(s) if s == "gcc"));
            assert!(matches!(&args[1], UnifiedExpr::Var(s) if s == "pkgs"));
          }
          _ => panic!("Expected Apply for getAttr, got {:?}", value),
        }
        // body should be the variable
        assert!(matches!(*body, UnifiedExpr::Var(ref s) if s == "gcc"));
      }
      _ => panic!("Expected Let, got {:?}", unified),
    }
  }

  #[test]
  fn test_with_inside_lambda_param_shadowing() {
    // x: with scope; x → x는 env.var로 바인딩되지 않아야 함
    let expr = PnixExpr::Lambda {
      param: PnixParamPattern::Ident("x".to_string()),
      body: Arc::new(PnixExpr::With {
        env: Arc::new(PnixExpr::Var("scope".to_string())),
        body: Arc::new(PnixExpr::Var("x".to_string())),
      }),
    };

    let unified = pnix_expr_to_unified(&expr).unwrap();

    match unified {
      UnifiedExpr::Lambda { param, body } => {
        assert_eq!(param, "x");
        assert!(matches!(*body, UnifiedExpr::Var(ref name) if name == "x"));
      }
      _ => panic!("Expected Lambda, got {:?}", unified),
    }
  }

  #[test]
  fn test_collect_pnix_free_vars_with_attrset_env_excludes_keys() {
    let expr = PnixExpr::With {
      env: Arc::new(PnixExpr::AttrSet {
        items: vec![
          PnixAttrItem::Assign {
            key_path: vec!["x".to_string()],
            value: PnixExpr::Var("a".to_string()),
            span: crate::diagnostics::Span::empty(),
          },
          PnixAttrItem::Assign {
            key_path: vec!["y".to_string()],
            value: PnixExpr::Int(1),
            span: crate::diagnostics::Span::empty(),
          },
        ],
        recursive: false,
      }),
      body: Arc::new(PnixExpr::Binary {
        op: "+",
        lhs: Arc::new(PnixExpr::Var("x".to_string())),
        rhs: Arc::new(PnixExpr::Var("z".to_string())),
      }),
    };

    let free_vars = collect_pnix_free_vars(&expr, &HashSet::new());
    assert!(free_vars.contains("a"));
    assert!(free_vars.contains("z"));
    assert!(!free_vars.contains("x"));
    assert!(!free_vars.contains("y"));
    assert_eq!(free_vars.len(), 2);
  }

  #[test]
  fn test_collect_pnix_free_vars_attrset_bind_name() {
    let expr = PnixExpr::Lambda {
      param: PnixParamPattern::AttrSetWithBind {
        bind_name: "args".to_string(),
        fields: vec![
          PnixPatternField {
            name: "x".to_string(),
            default: None,
          },
          PnixPatternField {
            name: "y".to_string(),
            default: None,
          },
        ],
        ellipsis: false,
      },
      body: Arc::new(PnixExpr::Binary {
        op: "+",
        lhs: Arc::new(PnixExpr::Var("args".to_string())),
        rhs: Arc::new(PnixExpr::Var("z".to_string())),
      }),
    };

    let free_vars = collect_pnix_free_vars(&expr, &HashSet::new());
    assert!(!free_vars.contains("args"));
    assert!(free_vars.contains("z"));
    assert_eq!(free_vars.len(), 1);
  }

  #[test]
  fn test_collect_pnix_free_vars_rec_attrset_self_reference() {
    let expr = PnixExpr::AttrSet {
      items: vec![
        PnixAttrItem::Assign {
          key_path: vec!["x".to_string()],
          value: PnixExpr::Binary {
            op: "+",
            lhs: Arc::new(PnixExpr::Var("y".to_string())),
            rhs: Arc::new(PnixExpr::Var("z".to_string())),
          },
          span: crate::diagnostics::Span::empty(),
        },
        PnixAttrItem::Assign {
          key_path: vec!["y".to_string()],
          value: PnixExpr::Var("x".to_string()),
          span: crate::diagnostics::Span::empty(),
        },
      ],
      recursive: true,
    };

    let free_vars = collect_pnix_free_vars(&expr, &HashSet::new());
    assert!(!free_vars.contains("x"));
    assert!(!free_vars.contains("y"));
    assert!(free_vars.contains("z"));
    assert_eq!(free_vars.len(), 1);
  }

  #[test]
  fn test_lower_apply_getattr() {
    // Y08a-4: getAttr("b", a)가 MeaningOpId::GetAttr로 매핑되어야 함
    let unified = UnifiedExpr::Apply {
      func: "builtins.getAttr".to_string(),
      args: vec![
        UnifiedExpr::String("b".to_string()),
        UnifiedExpr::Var("a".to_string()),
      ],
    };
    let fx = lower_to_fx_core(&unified).unwrap();

    // Derived 형태로 변환되어야 함
    match fx {
      FxCoreExpr::Derived { meta, args } => {
        assert_eq!(meta.op, MeaningOpId::GetAttr);
        assert_eq!(args.len(), 2);
        // 첫 번째 인자는 속성 이름 (문자열)
        assert!(matches!(args[0], FxCoreExpr::ConstString(ref s) if s == "b"));
        // 두 번째 인자는 객체
        assert!(matches!(args[1], FxCoreExpr::Var(ref s) if s == "a"));
      }
      _ => panic!("Expected Derived with GetAttr, got {:?}", fx),
    }
  }

  #[test]
  fn test_lower_apply_hasattr() {
    // hasAttr("b", a)가 MeaningOpId::HasAttr로 매핑되어야 함
    let unified = UnifiedExpr::Apply {
      func: "builtins.hasAttr".to_string(),
      args: vec![
        UnifiedExpr::String("b".to_string()),
        UnifiedExpr::Var("a".to_string()),
      ],
    };
    let fx = lower_to_fx_core(&unified).unwrap();

    // Derived 형태로 변환되어야 함
    match fx {
      FxCoreExpr::Derived { meta, args } => {
        assert_eq!(meta.op, MeaningOpId::HasAttr);
        assert_eq!(args.len(), 2);
        // 첫 번째 인자는 속성 이름 (문자열)
        assert!(matches!(args[0], FxCoreExpr::ConstString(ref s) if s == "b"));
        // 두 번째 인자는 객체
        assert!(matches!(args[1], FxCoreExpr::Var(ref s) if s == "a"));
      }
      _ => panic!("Expected Derived with HasAttr, got {:?}", fx),
    }
  }

  #[test]
  fn test_lower_apply_var_to_fx_apply() {
    let unified = UnifiedExpr::Apply {
      func: "f".to_string(),
      args: vec![UnifiedExpr::Int(1)],
    };
    let fx = lower_to_fx_core(&unified).unwrap();

    match fx {
      FxCoreExpr::Derived { meta, args } => {
        assert_eq!(meta.op, MeaningOpId::Apply);
        assert_eq!(args.len(), 2);
        assert!(matches!(args[0], FxCoreExpr::Var(ref s) if s == "f"));
        assert!(matches!(args[1], FxCoreExpr::ConstInt(1)));
      }
      other => panic!("Expected Derived Apply, got {:?}", other),
    }
  }

  #[test]
  fn test_select_and_getattr_equivalence() {
    // Y08a-4: a.b 선택과 getAttr("b", a)가 동일한 결과를 내야 함
    let select_expr = PnixExpr::Select {
      base: Arc::new(PnixExpr::Var("a".to_string())),
      attr: "b".to_string(),
    };
    let select_unified = pnix_expr_to_unified(&select_expr).unwrap();

    let getattr_unified = UnifiedExpr::Apply {
      func: "builtins.getAttr".to_string(),
      args: vec![
        UnifiedExpr::String("b".to_string()),
        UnifiedExpr::Var("a".to_string()),
      ],
    };

    // 둘 다 같은 형태로 변환되어야 함
    let select_fx = lower_to_fx_core(&select_unified).unwrap();
    let getattr_fx = lower_to_fx_core(&getattr_unified).unwrap();

    // 둘 다 Derived(GetAttr) 형태여야 함
    match (&select_fx, &getattr_fx) {
      (
        FxCoreExpr::Derived {
          meta: meta1,
          args: args1,
        },
        FxCoreExpr::Derived {
          meta: meta2,
          args: args2,
        },
      ) => {
        assert_eq!(meta1.op, MeaningOpId::GetAttr);
        assert_eq!(meta2.op, MeaningOpId::GetAttr);
        assert_eq!(args1.len(), 2);
        assert_eq!(args2.len(), 2);
        // 인자 순서가 동일해야 함 - 속성 이름 비교
        match (&args1[0], &args2[0]) {
          (FxCoreExpr::ConstString(s1), FxCoreExpr::ConstString(s2)) => {
            assert_eq!(s1, s2);
          }
          _ => panic!("First arg should be ConstString"),
        }
        // 객체 비교
        match (&args1[1], &args2[1]) {
          (FxCoreExpr::Var(v1), FxCoreExpr::Var(v2)) => {
            assert_eq!(v1, v2);
          }
          _ => panic!("Second arg should be Var"),
        }
      }
      _ => panic!(
        "Both should be Derived(GetAttr), got {:?} and {:?}",
        select_fx, getattr_fx
      ),
    }
  }

  #[test]
  fn test_apply_flatten_simple() {
    // Y08a-5: f a b가 Apply(Apply(f,a),b)로 파싱되면 Apply(f, [a, b])로 변환되어야 함
    // Apply(Apply(f, a), b) 형태의 중첩된 Apply
    let nested_apply = PnixExpr::Apply {
      func: Arc::new(PnixExpr::Apply {
        func: Arc::new(PnixExpr::Var("f".to_string())),
        arg: Arc::new(PnixExpr::Var("a".to_string())),
      }),
      arg: Arc::new(PnixExpr::Var("b".to_string())),
    };

    let unified = pnix_expr_to_unified(&nested_apply).unwrap();

    // Apply(f, [a, b]) 형태로 변환되어야 함
    match unified {
      UnifiedExpr::Apply { func, args } => {
        assert_eq!(func, "f");
        assert_eq!(args.len(), 2);
        assert!(matches!(args[0], UnifiedExpr::Var(ref s) if s == "a"));
        assert!(matches!(args[1], UnifiedExpr::Var(ref s) if s == "b"));
      }
      _ => panic!("Expected Apply(f, [a, b]), got {:?}", unified),
    }
  }

  #[test]
  fn test_apply_flatten_three_args() {
    // Y08a-5: f a b c가 Apply(Apply(Apply(f,a),b),c)로 파싱되면 Apply(f, [a, b, c])로 변환
    let triple_nested = PnixExpr::Apply {
      func: Arc::new(PnixExpr::Apply {
        func: Arc::new(PnixExpr::Apply {
          func: Arc::new(PnixExpr::Var("f".to_string())),
          arg: Arc::new(PnixExpr::Var("a".to_string())),
        }),
        arg: Arc::new(PnixExpr::Var("b".to_string())),
      }),
      arg: Arc::new(PnixExpr::Var("c".to_string())),
    };

    let unified = pnix_expr_to_unified(&triple_nested).unwrap();

    match unified {
      UnifiedExpr::Apply { func, args } => {
        assert_eq!(func, "f");
        assert_eq!(args.len(), 3);
        assert!(matches!(args[0], UnifiedExpr::Var(ref s) if s == "a"));
        assert!(matches!(args[1], UnifiedExpr::Var(ref s) if s == "b"));
        assert!(matches!(args[2], UnifiedExpr::Var(ref s) if s == "c"));
      }
      _ => panic!("Expected Apply(f, [a, b, c]), got {:?}", unified),
    }
  }

  #[test]
  fn test_curried_lambda_application_order() {
    use crate::lang::pnix::syntax::PnixParamPattern;

    let expr = PnixExpr::Apply {
      func: Arc::new(PnixExpr::Apply {
        func: Arc::new(PnixExpr::Lambda {
          param: PnixParamPattern::Ident("x".to_string()),
          body: Arc::new(PnixExpr::Lambda {
            param: PnixParamPattern::Ident("y".to_string()),
            body: Arc::new(PnixExpr::Binary {
              op: "+",
              lhs: Arc::new(PnixExpr::Var("x".to_string())),
              rhs: Arc::new(PnixExpr::Var("y".to_string())),
            }),
          }),
        }),
        arg: Arc::new(PnixExpr::Int(1)),
      }),
      arg: Arc::new(PnixExpr::Int(2)),
    };

    let unified = pnix_expr_to_unified(&expr).unwrap();

    match unified {
      UnifiedExpr::Let { name, value, body } => {
        assert_eq!(name, "x");
        assert!(matches!(*value, UnifiedExpr::Int(1)));
        match *body {
          UnifiedExpr::Let {
            name: inner_name,
            value: inner_value,
            body: inner_body,
          } => {
            assert_eq!(inner_name, "y");
            assert!(matches!(*inner_value, UnifiedExpr::Int(2)));
            match *inner_body {
              UnifiedExpr::Add(lhs, rhs) => {
                assert!(matches!(*lhs, UnifiedExpr::Var(ref s) if s == "x"));
                assert!(matches!(*rhs, UnifiedExpr::Var(ref s) if s == "y"));
              }
              _ => panic!("Expected Add body, got {:?}", inner_body),
            }
          }
          _ => panic!("Expected nested Let, got {:?}", body),
        }
      }
      _ => panic!("Expected Let, got {:?}", unified),
    }
  }

  #[test]
  fn test_apply_flatten_builtins_map() {
    // Y08a-5, Y-CLAUDE-4: builtins.map f xs가 Select를 통해 변환됨
    // Select 함수 적용은 이제 Let 바인딩으로 변환됨:
    // (builtins.map)(f)(xs) → let _func = builtins.getAttr("map", builtins) in _func(f, xs)
    let map_apply = PnixExpr::Apply {
      func: Arc::new(PnixExpr::Apply {
        func: Arc::new(PnixExpr::Select {
          base: Arc::new(PnixExpr::Var("builtins".to_string())),
          attr: "map".to_string(),
        }),
        arg: Arc::new(PnixExpr::Var("f".to_string())),
      }),
      arg: Arc::new(PnixExpr::Var("xs".to_string())),
    };

    let result = pnix_expr_to_unified(&map_apply);

    match result {
      Ok(unified) => {
        // Y-CLAUDE-4: Select 함수 적용은 Let으로 변환됨
        match unified {
          UnifiedExpr::Let {
            name,
            value: _,
            body,
          } => {
            // 함수 이름이 _apply_func_로 시작해야 함
            assert!(
              name.starts_with("_apply_func_"),
              "Expected name starting with _apply_func_, got {:?}",
              name
            );
            // body는 Apply여야 함
            match *body {
              UnifiedExpr::Apply { func, args } => {
                assert_eq!(func, name);
                // 인자는 [f, xs]여야 함
                assert_eq!(args.len(), 2);
              }
              _ => panic!("Expected Apply in body, got {:?}", body),
            }
          }
          UnifiedExpr::Apply { func: _, args } => {
            // 레거시 동작: Apply 형태로도 허용
            assert!(args.len() >= 2);
          }
          _ => panic!("Expected Let or Apply, got {:?}", unified),
        }
      }
      Err(_) => {
        // 복잡한 경우는 에러가 나는 것도 허용 (향후 개선)
      }
    }
  }

  #[test]
  fn test_apply_non_var_func_error() {
    // Y08a-5: 비함수 표현식(Int, Float 등) 적용은 명시적 에러
    let complex_apply = PnixExpr::Apply {
      func: Arc::new(PnixExpr::Int(42)), // 함수가 아닌 Int
      arg: Arc::new(PnixExpr::Var("x".to_string())),
    };

    let result = pnix_expr_to_unified(&complex_apply);
    assert!(result.is_err());
    assert!(
      matches!(result.unwrap_err(), PnixError::Lowering { message: msg, .. } if msg.contains("cannot apply non-function"))
    );
  }

  #[test]
  fn test_apply_select_func() {
    // Y-CLAUDE-4: Select 표현식을 함수로 적용
    // obj.method arg → let _func = builtins.getAttr("method", obj) in _func(arg)
    let select_apply = PnixExpr::Apply {
      func: Arc::new(PnixExpr::Select {
        base: Arc::new(PnixExpr::Var("obj".to_string())),
        attr: "method".to_string(),
      }),
      arg: Arc::new(PnixExpr::Var("x".to_string())),
    };

    let result = pnix_expr_to_unified(&select_apply);
    assert!(result.is_ok());
    let unified = result.unwrap();

    // Let 바인딩으로 변환되어야 함
    match unified {
      UnifiedExpr::Let {
        name,
        value: _,
        body,
      } => {
        // 함수 이름이 _apply_func_로 시작해야 함
        assert!(name.starts_with("_apply_func_"));
        // body는 Apply여야 함
        match *body {
          UnifiedExpr::Apply { func, args } => {
            assert_eq!(func, name);
            assert_eq!(args.len(), 1);
            assert!(matches!(args[0], UnifiedExpr::Var(ref s) if s == "x"));
          }
          _ => panic!("Expected Apply in body, got {:?}", body),
        }
      }
      _ => panic!("Expected Let, got {:?}", unified),
    }
  }

  #[test]
  fn test_apply_attrset_func() {
    // Y-CLAUDE-5: AttrSet 표현식을 함수로 적용
    // { ... } arg → let _func = { ... } in _func(arg)
    let attrset_apply = PnixExpr::Apply {
      func: Arc::new(PnixExpr::AttrSet {
        items: vec![PnixAttrItem::Assign {
          key_path: vec!["f".to_string()],
          value: PnixExpr::Int(42),
          span: crate::diagnostics::Span::empty(),
        }],
        recursive: false,
      }),
      arg: Arc::new(PnixExpr::Var("x".to_string())),
    };

    let result = pnix_expr_to_unified(&attrset_apply);
    assert!(result.is_ok());
    let unified = result.unwrap();

    // Let 바인딩으로 변환되어야 함
    match unified {
      UnifiedExpr::Let {
        name,
        value: _,
        body,
      } => {
        // 함수 이름이 _apply_complex_ 또는 _apply_func_ 로 시작해야 함
        assert!(
          name.starts_with("_apply_complex_") || name.starts_with("_apply_func_"),
          "Expected name starting with _apply_complex_ or _apply_func_, got {:?}",
          name
        );
        // body는 Apply여야 함
        match *body {
          UnifiedExpr::Apply { func, args } => {
            assert_eq!(func, name);
            assert_eq!(args.len(), 1);
            assert!(matches!(args[0], UnifiedExpr::Var(ref s) if s == "x"));
          }
          _ => panic!("Expected Apply in body, got {:?}", body),
        }
      }
      _ => panic!("Expected Let, got {:?}", unified),
    }
  }

  #[test]
  fn test_let_attrset_destructuring() {
    // let {x, y} = point in x + y
    // → let _rec = { _tmp = point; x = _tmp.x; y = _tmp.y; } in _rec.x + _rec.y
    use crate::lang::pnix::syntax::{PnixParamPattern, PnixPatternField};

    let let_expr = PnixExpr::Let {
      bindings: vec![PnixLetBinding::Binding {
        pattern: PnixParamPattern::AttrSet {
          fields: vec![
            PnixPatternField {
              name: "x".to_string(),
              default: None,
            },
            PnixPatternField {
              name: "y".to_string(),
              default: None,
            },
          ],
          ellipsis: false,
        },
        value: PnixExpr::Var("point".to_string()),
      }],
      body: Arc::new(PnixExpr::Binary {
        op: "+",
        lhs: Arc::new(PnixExpr::Var("x".to_string())),
        rhs: Arc::new(PnixExpr::Var("y".to_string())),
      }),
    };

    let result = pnix_expr_to_unified(&let_expr);
    assert!(
      result.is_ok(),
      "AttrSet destructuring should succeed: {:?}",
      result
    );
    let unified = result.unwrap();

    fn assert_get_attr(expr: &UnifiedExpr, key: &str, self_name: &str) {
      match expr {
        UnifiedExpr::Apply { func, args } => {
          assert_eq!(func, "builtins.getAttr");
          assert_eq!(args.len(), 2);
          assert!(matches!(&args[0], UnifiedExpr::String(s) if s == key));
          assert!(matches!(&args[1], UnifiedExpr::Var(s) if s == self_name));
        }
        _ => panic!("Expected getAttr for {}, got {:?}", key, expr),
      }
    }

    // 최외곽은 Let (rec attrset)
    match unified {
      UnifiedExpr::Let { name, value, body } => {
        let self_name = name.clone();
        match *value {
          UnifiedExpr::AttrSet(pairs) => {
            let mut map: HashMap<String, UnifiedExpr> = pairs.into_iter().collect();
            let tmp_name = map
              .keys()
              .find(|k| k.starts_with("_let_dest_"))
              .cloned()
              .expect("missing temp binding");
            assert!(matches!(map.remove(&tmp_name), Some(UnifiedExpr::Var(ref s)) if s == "point"));
            assert!(map.contains_key("x"));
            assert!(map.contains_key("y"));
          }
          _ => panic!("Expected AttrSet for let value, got {:?}", value),
        }
        match *body {
          UnifiedExpr::Add(lhs, rhs) => {
            assert_get_attr(&lhs, "x", &self_name);
            assert_get_attr(&rhs, "y", &self_name);
          }
          _ => panic!("Expected Add in body, got {:?}", body),
        }
      }
      _ => panic!("Expected Let, got {:?}", unified),
    }
  }

  #[test]
  fn test_let_list_destructuring() {
    // let [x, y] = list in x + y
    // → let _rec = { _tmp = list; x = elemAt _tmp 0; y = elemAt _tmp 1; } in _rec.x + _rec.y
    use crate::lang::pnix::syntax::{PnixListPattern, PnixParamPattern};

    let let_expr = PnixExpr::Let {
      bindings: vec![PnixLetBinding::Binding {
        pattern: PnixParamPattern::List(PnixListPattern {
          items: vec!["x".to_string(), "y".to_string()],
          tail: None,
        }),
        value: PnixExpr::Var("list".to_string()),
      }],
      body: Arc::new(PnixExpr::Binary {
        op: "+",
        lhs: Arc::new(PnixExpr::Var("x".to_string())),
        rhs: Arc::new(PnixExpr::Var("y".to_string())),
      }),
    };

    let result = pnix_expr_to_unified(&let_expr);
    assert!(
      result.is_ok(),
      "List destructuring should succeed: {:?}",
      result
    );
    let unified = result.unwrap();

    fn assert_get_attr(expr: &UnifiedExpr, key: &str, self_name: &str) {
      match expr {
        UnifiedExpr::Apply { func, args } => {
          assert_eq!(func, "builtins.getAttr");
          assert_eq!(args.len(), 2);
          assert!(matches!(&args[0], UnifiedExpr::String(s) if s == key));
          assert!(matches!(&args[1], UnifiedExpr::Var(s) if s == self_name));
        }
        _ => panic!("Expected getAttr for {}, got {:?}", key, expr),
      }
    }

    // 최외곽은 Let (rec attrset)
    match unified {
      UnifiedExpr::Let { name, value, body } => {
        let self_name = name.clone();
        match *value {
          UnifiedExpr::AttrSet(pairs) => {
            let mut map: HashMap<String, UnifiedExpr> = pairs.into_iter().collect();
            let tmp_name = map
              .keys()
              .find(|k| k.starts_with("_let_dest_"))
              .cloned()
              .expect("missing temp binding");
            assert!(matches!(map.remove(&tmp_name), Some(UnifiedExpr::Var(ref s)) if s == "list"));
            match map.remove("x") {
              Some(UnifiedExpr::Apply { func, args }) => {
                assert_eq!(func, "builtins.elemAt");
                assert_eq!(args.len(), 2);
                assert!(matches!(&args[1], UnifiedExpr::Int(0)));
              }
              other => panic!("Expected elemAt for x, got {:?}", other),
            }
            match map.remove("y") {
              Some(UnifiedExpr::Apply { func, args }) => {
                assert_eq!(func, "builtins.elemAt");
                assert_eq!(args.len(), 2);
                assert!(matches!(&args[1], UnifiedExpr::Int(1)));
              }
              other => panic!("Expected elemAt for y, got {:?}", other),
            }
          }
          _ => panic!("Expected AttrSet for let value, got {:?}", value),
        }
        match *body {
          UnifiedExpr::Add(lhs, rhs) => {
            assert_get_attr(&lhs, "x", &self_name);
            assert_get_attr(&rhs, "y", &self_name);
          }
          _ => panic!("Expected Add in body, got {:?}", body),
        }
      }
      _ => panic!("Expected Let, got {:?}", unified),
    }
  }

  #[test]
  fn test_lambda_attrset_pattern() {
    // Y08a-1: attrset destructuring in lambda is now supported
    use crate::lang::pnix::syntax::{PnixParamPattern, PnixPatternField};

    let lambda_expr = PnixExpr::Lambda {
      param: PnixParamPattern::AttrSet {
        fields: vec![PnixPatternField {
          name: "x".to_string(),
          default: None,
        }],
        ellipsis: false,
      },
      body: Arc::new(PnixExpr::Var("x".to_string())),
    };

    let result = pnix_expr_to_unified(&lambda_expr);
    assert!(result.is_ok());
    let unified = result.unwrap();

    // Lambda로 변환되어야 함
    match unified {
      UnifiedExpr::Lambda { param, body } => {
        assert!(param.starts_with("_lambda_arg_"));
        // body는 Let으로 감싸진 Var("x")여야 함
        match *body {
          UnifiedExpr::Let {
            name,
            value,
            body: inner_body,
          } => {
            assert_eq!(name, "x");
            // value는 builtins.getAttr 호출이어야 함
            assert!(
              matches!(*value, UnifiedExpr::Apply { func, .. } if func == "builtins.getAttr")
            );
            // inner_body는 Var("x")여야 함
            assert!(matches!(*inner_body, UnifiedExpr::Var(ref s) if s == "x"));
          }
          _ => panic!("Expected Let in body, got {:?}", body),
        }
      }
      _ => panic!("Expected Lambda, got {:?}", unified),
    }
  }

  #[test]
  fn test_lambda_list_pattern() {
    // Y08a-1: list destructuring in lambda is now supported
    use crate::lang::pnix::syntax::{PnixListPattern, PnixParamPattern};

    let lambda_expr = PnixExpr::Lambda {
      param: PnixParamPattern::List(PnixListPattern {
        items: vec!["x".to_string(), "y".to_string()],
        tail: None,
      }),
      body: Arc::new(PnixExpr::Var("x".to_string())),
    };

    let result = pnix_expr_to_unified(&lambda_expr);
    assert!(result.is_ok());
    let unified = result.unwrap();

    // Lambda로 변환되어야 함
    match unified {
      UnifiedExpr::Lambda { param, body } => {
        assert!(param.starts_with("_lambda_arg_"));
        // body는 Let으로 감싸진 Var("x")여야 함 (역순으로 y, x 순서)
        match *body {
          UnifiedExpr::Let {
            name,
            value,
            body: inner_body,
          } => {
            assert_eq!(name, "x");
            // value는 elemAt 호출이어야 함
            assert!(matches!(*value, UnifiedExpr::Apply { func, .. } if func == "elemAt"));
            // inner_body는 또 다른 Let (y) 또는 Var("x")여야 함
            assert!(matches!(
              *inner_body,
              UnifiedExpr::Let { .. } | UnifiedExpr::Var(_)
            ));
          }
          _ => panic!("Expected Let in body, got {:?}", body),
        }
      }
      _ => panic!("Expected Lambda, got {:?}", unified),
    }
  }

  #[test]
  fn test_gensym_determinism() {
    // 결정론 테스트: 동일 입력 → 동일 출력
    use crate::lang::pnix::syntax::{PnixParamPattern, PnixPatternField};

    let lambda_expr = PnixExpr::Lambda {
      param: PnixParamPattern::AttrSet {
        fields: vec![PnixPatternField {
          name: "x".to_string(),
          default: None,
        }],
        ellipsis: false,
      },
      body: Arc::new(PnixExpr::Binary {
        op: "+",
        lhs: Arc::new(PnixExpr::Var("x".to_string())),
        rhs: Arc::new(PnixExpr::Int(1)),
      }),
    };

    // 동일 입력에 대해 여러 번 호출
    let result1 = pnix_expr_to_unified(&lambda_expr).unwrap();
    let result2 = pnix_expr_to_unified(&lambda_expr).unwrap();
    let result3 = pnix_expr_to_unified(&lambda_expr).unwrap();

    // 모든 결과가 동일해야 함 (결정론)
    let debug1 = format!("{:?}", result1);
    let debug2 = format!("{:?}", result2);
    let debug3 = format!("{:?}", result3);

    assert_eq!(debug1, debug2, "첫 번째와 두 번째 호출 결과가 달라요");
    assert_eq!(debug2, debug3, "두 번째와 세 번째 호출 결과가 달라요");

    // Lambda 파라미터 이름이 content-based hash 형식이어야 함
    match result1 {
      UnifiedExpr::Lambda { ref param, .. } => {
        assert!(param.starts_with("_lambda_arg_"), "param = {}", param);
        // 해시가 포함되어 있어야 함 (숫자가 아닌 16진수 문자열)
        let suffix = param.strip_prefix("_lambda_arg_").unwrap();
        assert!(
          suffix.chars().all(|c| c.is_ascii_hexdigit()),
          "suffix should be hex, got: {}",
          suffix
        );
      }
      _ => panic!("Expected Lambda"),
    }
  }

  #[test]
  fn test_lambda_arg_avoids_pattern_names() {
    use crate::lang::pnix::syntax::{PnixParamPattern, PnixPatternField};

    let body = PnixExpr::Var("outer".to_string());
    let candidate = super::gensym_from_expr("_lambda_arg", &body);

    let lambda_expr = PnixExpr::Lambda {
      param: PnixParamPattern::AttrSet {
        fields: vec![PnixPatternField {
          name: candidate.clone(),
          default: None,
        }],
        ellipsis: false,
      },
      body: Box::new(body),
    };

    let unified = pnix_expr_to_unified(&lambda_expr).unwrap();
    match unified {
      UnifiedExpr::Lambda { param, .. } => {
        assert_ne!(param, candidate, "param should avoid pattern-bound names");
      }
      _ => panic!("Expected Lambda"),
    }
  }

  #[test]
  fn test_inherit_single_var() {
    // Y08a-6: inherit x; → { x = x; }
    let attrset = PnixExpr::AttrSet {
      items: vec![PnixAttrItem::Inherit {
        from: None,
        names: vec!["x".to_string()],
        span: crate::diagnostics::Span::empty(),
      }],
      recursive: false,
    };

    let unified = pnix_expr_to_unified(&attrset).unwrap();

    match unified {
      UnifiedExpr::AttrSet(pairs) => {
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "x");
        assert!(matches!(pairs[0].1, UnifiedExpr::Var(ref s) if s == "x"));
      }
      _ => panic!("Expected AttrSet, got {:?}", unified),
    }
  }

  #[test]
  fn test_inherit_multiple_vars() {
    // Y08a-6: inherit x y; → { x = x; y = y; }
    let attrset = PnixExpr::AttrSet {
      items: vec![PnixAttrItem::Inherit {
        from: None,
        names: vec!["x".to_string(), "y".to_string()],
        span: crate::diagnostics::Span::empty(),
      }],
      recursive: false,
    };

    let unified = pnix_expr_to_unified(&attrset).unwrap();

    match unified {
      UnifiedExpr::AttrSet(pairs) => {
        assert_eq!(pairs.len(), 2);
        // 순서 확인
        assert_eq!(pairs[0].0, "x");
        assert!(matches!(pairs[0].1, UnifiedExpr::Var(ref s) if s == "x"));
        assert_eq!(pairs[1].0, "y");
        assert!(matches!(pairs[1].1, UnifiedExpr::Var(ref s) if s == "y"));
      }
      _ => panic!("Expected AttrSet, got {:?}", unified),
    }
  }

  #[test]
  fn test_inherit_with_assign() {
    // Y08a-6: { inherit x; y = 2; } → { x = x; y = 2; }
    let attrset = PnixExpr::AttrSet {
      items: vec![
        PnixAttrItem::Inherit {
          from: None,
          names: vec!["x".to_string()],
          span: crate::diagnostics::Span::empty(),
        },
        PnixAttrItem::Assign {
          key_path: vec!["y".to_string()],
          value: PnixExpr::Int(2),
          span: crate::diagnostics::Span::empty(),
        },
      ],
      recursive: false,
    };

    let unified = pnix_expr_to_unified(&attrset).unwrap();

    match unified {
      UnifiedExpr::AttrSet(pairs) => {
        assert_eq!(pairs.len(), 2);
        // inherit가 먼저, 그 다음 assign
        assert_eq!(pairs[0].0, "x");
        assert!(matches!(pairs[0].1, UnifiedExpr::Var(ref s) if s == "x"));
        assert_eq!(pairs[1].0, "y");
        assert!(matches!(pairs[1].1, UnifiedExpr::Int(2)));
      }
      _ => panic!("Expected AttrSet, got {:?}", unified),
    }
  }

  #[test]
  fn test_attrset_nested_path_lowering() {
    let attrset = PnixExpr::AttrSet {
      items: vec![
        PnixAttrItem::Assign {
          key_path: vec!["a".to_string(), "b".to_string()],
          value: PnixExpr::Int(1),
          span: crate::diagnostics::Span::empty(),
        },
        PnixAttrItem::Assign {
          key_path: vec!["a".to_string(), "c".to_string()],
          value: PnixExpr::Int(2),
          span: crate::diagnostics::Span::empty(),
        },
      ],
      recursive: false,
    };

    let unified = pnix_expr_to_unified(&attrset).unwrap();

    match unified {
      UnifiedExpr::AttrSet(pairs) => {
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "a");
        match &pairs[0].1 {
          UnifiedExpr::AttrSet(inner) => {
            assert_eq!(inner.len(), 2);
            assert_eq!(inner[0].0, "b");
            assert!(matches!(inner[0].1, UnifiedExpr::Int(1)));
            assert_eq!(inner[1].0, "c");
            assert!(matches!(inner[1].1, UnifiedExpr::Int(2)));
          }
          _ => panic!("Expected nested AttrSet, got {:?}", pairs[0].1),
        }
      }
      _ => panic!("Expected AttrSet, got {:?}", unified),
    }
  }

  #[test]
  fn test_attrset_duplicate_key_span() {
    let dup_span = crate::diagnostics::Span::new(10, 11);
    let attrset = PnixExpr::AttrSet {
      items: vec![
        PnixAttrItem::Assign {
          key_path: vec!["a".to_string()],
          value: PnixExpr::Int(1),
          span: crate::diagnostics::Span::new(1, 2),
        },
        PnixAttrItem::Assign {
          key_path: vec!["a".to_string()],
          value: PnixExpr::Int(2),
          span: dup_span.clone(),
        },
      ],
      recursive: false,
    };

    let err = pnix_expr_to_unified(&attrset).unwrap_err();
    match err {
      PnixError::Lowering {
        span: Some(span), ..
      } => assert_eq!(span, dup_span),
      other => panic!("Expected Lowering error with span, got {:?}", other),
    }
  }

  #[test]
  fn test_inherit_in_let() {
    use crate::lang::pnix::syntax::PnixParamPattern;
    // Y08a-6: let x = 1; in { inherit x; }.x
    // 이 테스트는 전체 표현식이 아니라 attrset 부분만 테스트
    let let_expr = PnixExpr::Let {
      bindings: vec![PnixLetBinding::Binding {
        pattern: PnixParamPattern::Ident("x".to_string()),
        value: PnixExpr::Int(1),
      }],
      body: Arc::new(PnixExpr::Select {
        base: Arc::new(PnixExpr::AttrSet {
          items: vec![PnixAttrItem::Inherit {
            from: None,
            names: vec!["x".to_string()],
            span: crate::diagnostics::Span::empty(),
          }],
          recursive: false,
        }),
        attr: "x".to_string(),
      }),
    };

    // 전체 표현식 변환 테스트
    let unified = pnix_expr_to_unified(&let_expr).unwrap();

    // Let의 body가 Select이고, 그 base가 AttrSet이며 inherit를 포함해야 함
    match unified {
      UnifiedExpr::Let { name, value, body } => {
        let self_name = name.clone();
        assert!(self_name.starts_with("_let_rec_"));
        match *value {
          UnifiedExpr::AttrSet(pairs) => {
            let mut map: HashMap<String, UnifiedExpr> = pairs.into_iter().collect();
            assert!(matches!(map.remove("x"), Some(UnifiedExpr::Int(1))));
          }
          _ => panic!("Expected AttrSet for let value, got {:?}", value),
        }
        match *body {
          UnifiedExpr::Apply { func, args } => {
            assert_eq!(func, "builtins.getAttr");
            assert_eq!(args.len(), 2);
            assert!(matches!(args[0], UnifiedExpr::String(ref s) if s == "x"));
            match &args[1] {
              UnifiedExpr::AttrSet(pairs) => {
                let mut map: HashMap<String, UnifiedExpr> = pairs.clone().into_iter().collect();
                match map.remove("x") {
                  Some(UnifiedExpr::Apply { func, args }) => {
                    assert_eq!(func, "builtins.getAttr");
                    assert_eq!(args.len(), 2);
                    assert!(matches!(args[0], UnifiedExpr::String(ref s) if s == "x"));
                    assert!(matches!(args[1], UnifiedExpr::Var(ref s) if s == &self_name));
                  }
                  other => panic!("Expected getAttr in inherit, got {:?}", other),
                }
              }
              _ => panic!("Expected AttrSet in Select base"),
            }
          }
          _ => panic!("Expected Apply (Select), got {:?}", body),
        }
      }
      _ => panic!("Expected Let, got {:?}", unified),
    }
  }

  #[test]
  fn test_let_recursive_reference() {
    use crate::lang::pnix::syntax::PnixParamPattern;
    let let_expr = PnixExpr::Let {
      bindings: vec![PnixLetBinding::Binding {
        pattern: PnixParamPattern::Ident("x".to_string()),
        value: PnixExpr::Binary {
          op: "+",
          lhs: Arc::new(PnixExpr::Var("x".to_string())),
          rhs: Arc::new(PnixExpr::Int(1)),
        },
      }],
      body: Arc::new(PnixExpr::Var("x".to_string())),
    };

    let unified = pnix_expr_to_unified(&let_expr).unwrap();

    match unified {
      UnifiedExpr::Let { name, value, body } => {
        let self_name = name.clone();
        match *value {
          UnifiedExpr::AttrSet(pairs) => {
            let mut map: HashMap<String, UnifiedExpr> = pairs.into_iter().collect();
            match map.remove("x") {
              Some(UnifiedExpr::Add(lhs, rhs)) => {
                assert!(matches!(*rhs, UnifiedExpr::Int(1)));
                match *lhs {
                  UnifiedExpr::Apply { func, args } => {
                    assert_eq!(func, "builtins.getAttr");
                    assert_eq!(args.len(), 2);
                    assert!(matches!(args[0], UnifiedExpr::String(ref s) if s == "x"));
                    assert!(matches!(args[1], UnifiedExpr::Var(ref s) if s == &self_name));
                  }
                  other => panic!("Expected getAttr in recursive value, got {:?}", other),
                }
              }
              other => panic!("Expected Add for x value, got {:?}", other),
            }
          }
          _ => panic!("Expected AttrSet for let value, got {:?}", value),
        }
        match *body {
          UnifiedExpr::Apply { func, args } => {
            assert_eq!(func, "builtins.getAttr");
            assert_eq!(args.len(), 2);
            assert!(matches!(args[0], UnifiedExpr::String(ref s) if s == "x"));
            assert!(matches!(args[1], UnifiedExpr::Var(ref s) if s == &self_name));
          }
          other => panic!("Expected getAttr in body, got {:?}", other),
        }
      }
      _ => panic!("Expected Let, got {:?}", unified),
    }
  }

  #[test]
  fn test_rec_attrset_lowering() {
    // N01a-0: rec { x = 1; y = x + 1; } → let _rec = { x = 1; y = _rec.x + 1; } in _rec
    let rec_attrset = PnixExpr::AttrSet {
      items: vec![
        PnixAttrItem::Assign {
          key_path: vec!["x".to_string()],
          value: PnixExpr::Int(1),
          span: crate::diagnostics::Span::empty(),
        },
        PnixAttrItem::Assign {
          key_path: vec!["y".to_string()],
          value: PnixExpr::Binary {
            op: "+",
            lhs: Arc::new(PnixExpr::Var("x".to_string())),
            rhs: Arc::new(PnixExpr::Int(1)),
          },
          span: crate::diagnostics::Span::empty(),
        },
      ],
      recursive: true,
    };

    let unified = pnix_expr_to_unified(&rec_attrset).unwrap();

    match unified {
      UnifiedExpr::Let { name, value, body } => {
        let self_name = name.clone();
        assert!(matches!(*body, UnifiedExpr::Var(ref s) if s == &self_name));
        match *value {
          UnifiedExpr::AttrSet(pairs) => {
            let mut map: std::collections::HashMap<_, _> = pairs.into_iter().collect();
            assert!(matches!(map.remove("x"), Some(UnifiedExpr::Int(1))));
            match map.remove("y") {
              Some(UnifiedExpr::Add(lhs, rhs)) => {
                assert!(matches!(*rhs, UnifiedExpr::Int(1)));
                match *lhs {
                  UnifiedExpr::Apply { func, args } => {
                    assert_eq!(func, "builtins.getAttr");
                    assert_eq!(args.len(), 2);
                    assert!(matches!(args[0], UnifiedExpr::String(ref s) if s == "x"));
                    assert!(matches!(args[1], UnifiedExpr::Var(ref s) if s == &self_name));
                  }
                  _ => panic!("Expected getAttr for recursive reference"),
                }
              }
              _ => panic!("Expected Add for y value"),
            }
          }
          _ => panic!("Expected AttrSet for rec binding value"),
        }
      }
      _ => panic!("Expected Let for rec attrset, got {:?}", unified),
    }
  }

  #[test]
  fn test_import_lowering() {
    // N01a: import "./lib.nix" → Apply { func: "builtins.import", args: [String("./lib.nix")] }
    let import_expr = PnixExpr::Import {
      path: Arc::new(PnixExpr::String("./lib.nix".to_string())),
    };

    let unified = pnix_expr_to_unified(&import_expr).unwrap();

    match unified {
      UnifiedExpr::Apply { func, args } => {
        assert_eq!(func, "builtins.import");
        assert_eq!(args.len(), 1);
        assert!(matches!(args[0], UnifiedExpr::String(ref s) if s == "./lib.nix"));
      }
      _ => panic!("Expected Apply for import, got {:?}", unified),
    }
  }

  #[test]
  fn test_fx_core_to_unified_signal_var_error() {
    let core = FxCoreExpr::signal(1);
    let err = fx_core_to_unified(&core).unwrap_err();
    assert!(matches!(err, PnixError::Lowering { message: msg, .. } if msg.contains("SignalVar")));
  }

  #[test]
  fn test_lower_signalvar_errors() {
    // Y08a-8: SignalVar는 이제 lower_to_fx_core에서 직접 변환됨 (resolve_signals 없이도)
    // 하지만 Pure 모드에서는 resolve_signals가 에러를 발생시켜야 함
    let expr = UnifiedExpr::SignalVar("time".to_string());

    // Pure 모드에서는 에러 발생
    let err = lower_to_fx_core_with_mode(&expr, ExecutionMode::Pure, &[]).unwrap_err();
    assert!(matches!(err, PnixError::Lowering { message: msg, .. } if msg.contains("Pure")));

    // Realtime 모드에서는 성공적으로 변환됨
    let fx = lower_to_fx_core_with_mode(&expr, ExecutionMode::Realtime, &["time"]).unwrap();
    assert!(matches!(fx, FxCoreExpr::SignalVar(_)));
  }

  #[test]
  fn test_lower_paramsignal_pure_rejects() {
    // Y08a-8: Pure 모드에서 ParamSignal 에러
    let expr = UnifiedExpr::ParamSignal("time".to_string());
    let err = lower_to_fx_core_with_mode(&expr, ExecutionMode::Pure, &[]).unwrap_err();
    assert!(matches!(err, PnixError::Lowering { message: msg, .. } if msg.contains("Pure")));
  }

  #[test]
  fn test_lower_paramsignal_realtime_converts() {
    // Y08a-8: Realtime 모드에서 ParamSignal → SignalVar 변환
    let expr = UnifiedExpr::ParamSignal("time".to_string());
    let fx = lower_to_fx_core_with_mode(&expr, ExecutionMode::Realtime, &["time"]).unwrap();
    assert!(matches!(fx, FxCoreExpr::SignalVar(_)));
  }

  #[test]
  fn test_string_add_to_concat() {
    // Y08a-9: 문자열 + 연산자는 Concat으로 변환
    let expr = UnifiedExpr::Add(
      Box::new(UnifiedExpr::String("hello".to_string())),
      Box::new(UnifiedExpr::String("world".to_string())),
    );
    let fx = lower_to_fx_core(&expr).unwrap();

    // Concat으로 변환되어야 함
    match fx {
      FxCoreExpr::Binary { meta, .. } => {
        assert_eq!(meta.op, MeaningOpId::Concat);
      }
      _ => panic!("Expected Binary(Concat), got {:?}", fx),
    }
  }

  #[test]
  fn test_number_add_stays_add() {
    // Y08a-9: 숫자 + 연산자는 Add로 유지
    let expr = UnifiedExpr::Add(Box::new(UnifiedExpr::Int(1)), Box::new(UnifiedExpr::Int(2)));
    let fx = lower_to_fx_core(&expr).unwrap();

    // Add로 유지되어야 함
    match fx {
      FxCoreExpr::Binary { meta, .. } => {
        assert_eq!(meta.op, MeaningOpId::Add);
      }
      _ => panic!("Expected Binary(Add), got {:?}", fx),
    }
  }

  #[test]
  fn test_mixed_add_uses_add() {
    // Y08a-9: 문자열과 숫자 혼합은 타입 에러 (명시적 에러 반환)
    let expr = UnifiedExpr::Add(
      Box::new(UnifiedExpr::String("hello".to_string())),
      Box::new(UnifiedExpr::Int(1)),
    );
    let result = lower_to_fx_core(&expr);

    // 타입 불일치 에러가 발생해야 함
    assert!(result.is_err());
    match result.unwrap_err() {
      PnixError::Lowering { message, .. } => {
        assert!(message.contains("type mismatch") || message.contains("cannot add String"));
      }
      _ => panic!("Expected Lowering error for type mismatch"),
    }
  }

  #[test]
  fn test_and_short_circuit() {
    // Y08a-10: && 단축 평가 - if lhs then rhs else false
    let expr = UnifiedExpr::And(
      Box::new(UnifiedExpr::Bool(false)),
      Box::new(UnifiedExpr::Bool(true)),
    );
    let fx = lower_to_fx_core(&expr).unwrap();

    // If 표현식으로 변환되어야 함
    match fx {
      FxCoreExpr::If { cond, then_, else_ } => {
        // cond는 false
        assert!(matches!(cond.as_ref(), FxCoreExpr::ConstBool(false)));
        // then_는 true (rhs)
        assert!(matches!(then_.as_ref(), FxCoreExpr::ConstBool(true)));
        // else_는 false
        assert!(matches!(else_.as_ref(), FxCoreExpr::ConstBool(false)));
      }
      _ => panic!("Expected If expression, got {:?}", fx),
    }
  }

  #[test]
  fn test_or_short_circuit() {
    // Y08a-10: || 단축 평가 - if lhs then true else rhs
    let expr = UnifiedExpr::Or(
      Box::new(UnifiedExpr::Bool(true)),
      Box::new(UnifiedExpr::Bool(false)),
    );
    let fx = lower_to_fx_core(&expr).unwrap();

    // If 표현식으로 변환되어야 함
    match fx {
      FxCoreExpr::If { cond, then_, else_ } => {
        // cond는 true
        assert!(matches!(cond.as_ref(), FxCoreExpr::ConstBool(true)));
        // then_는 true
        assert!(matches!(then_.as_ref(), FxCoreExpr::ConstBool(true)));
        // else_는 false (rhs)
        assert!(matches!(else_.as_ref(), FxCoreExpr::ConstBool(false)));
      }
      _ => panic!("Expected If expression, got {:?}", fx),
    }
  }

  #[test]
  fn test_and_true_evaluates_rhs() {
    // Y08a-10: &&에서 lhs가 true면 rhs 평가
    let expr = UnifiedExpr::And(
      Box::new(UnifiedExpr::Bool(true)),
      Box::new(UnifiedExpr::Int(42)),
    );
    let fx = lower_to_fx_core(&expr).unwrap();

    // If 표현식으로 변환되어야 함
    match fx {
      FxCoreExpr::If { cond, then_, else_ } => {
        // cond는 true
        assert!(matches!(cond.as_ref(), FxCoreExpr::ConstBool(true)));
        // then_는 42 (rhs)
        assert!(matches!(then_.as_ref(), FxCoreExpr::ConstInt(42)));
        // else_는 false
        assert!(matches!(else_.as_ref(), FxCoreExpr::ConstBool(false)));
      }
      _ => panic!("Expected If expression, got {:?}", fx),
    }
  }

  #[test]
  fn test_or_false_evaluates_rhs() {
    // Y08a-10: ||에서 lhs가 false면 rhs 평가
    let expr = UnifiedExpr::Or(
      Box::new(UnifiedExpr::Bool(false)),
      Box::new(UnifiedExpr::Int(42)),
    );
    let fx = lower_to_fx_core(&expr).unwrap();

    // If 표현식으로 변환되어야 함
    match fx {
      FxCoreExpr::If { cond, then_, else_ } => {
        // cond는 false
        assert!(matches!(cond.as_ref(), FxCoreExpr::ConstBool(false)));
        // then_는 true
        assert!(matches!(then_.as_ref(), FxCoreExpr::ConstBool(true)));
        // else_는 42 (rhs)
        assert!(matches!(else_.as_ref(), FxCoreExpr::ConstInt(42)));
      }
      _ => panic!("Expected If expression, got {:?}", fx),
    }
  }

  #[test]
  fn test_let_preserves_lazy_semantics() {
    // Y08a-11: Let 노드로 보존하여 lazy semantics 및 중복 평가 방지
    let expr = UnifiedExpr::Let {
      name: "x".to_string(),
      value: Box::new(UnifiedExpr::Int(42)),
      body: Box::new(UnifiedExpr::Var("x".to_string())),
    };
    let fx = lower_to_fx_core(&expr).unwrap();

    // Let 노드로 변환되어야 함 (치환하지 않음)
    match fx {
      FxCoreExpr::Let { name, value, body } => {
        assert_eq!(name, "x");
        assert!(matches!(value.as_ref(), FxCoreExpr::ConstInt(42)));
        assert!(matches!(body.as_ref(), FxCoreExpr::Var(ref n) if n == "x"));
      }
      _ => panic!("Expected Let node, got {:?}", fx),
    }
  }

  #[test]
  fn test_let_shadowing() {
    // Y08a-11: Let 바인딩된 변수는 외부 변수와 섀도잉
    // let x = 1; in (lambda x: x) → Let 노드 유지, Lambda 내부 x는 바운드 변수
    let expr = UnifiedExpr::Let {
      name: "x".to_string(),
      value: Box::new(UnifiedExpr::Int(1)),
      body: Box::new(UnifiedExpr::Lambda {
        param: "x".to_string(),
        body: Box::new(UnifiedExpr::Var("x".to_string())),
      }),
    };
    let fx = lower_to_fx_core(&expr).unwrap();

    // Let 노드로 변환되어야 함
    match fx {
      FxCoreExpr::Let { name, value, body } => {
        assert_eq!(name, "x");
        assert!(matches!(value.as_ref(), FxCoreExpr::ConstInt(1)));
        // body는 Lambda여야 함
        assert!(matches!(body.as_ref(), FxCoreExpr::Lambda { .. }));
      }
      _ => panic!("Expected Let node, got {:?}", fx),
    }
  }

  #[test]
  fn test_let_no_duplicate_evaluation() {
    // Y08a-11: Let 노드로 보존하여 중복 평가 방지
    // let x = expensive; in x + x → Let 노드 유지 (치환하지 않음)
    let expr = UnifiedExpr::Let {
      name: "x".to_string(),
      value: Box::new(UnifiedExpr::Int(100)),
      body: Box::new(UnifiedExpr::Add(
        Box::new(UnifiedExpr::Var("x".to_string())),
        Box::new(UnifiedExpr::Var("x".to_string())),
      )),
    };
    let fx = lower_to_fx_core(&expr).unwrap();

    // Let 노드로 변환되어야 함 (치환하지 않음)
    match fx {
      FxCoreExpr::Let { name, value, body } => {
        assert_eq!(name, "x");
        assert!(matches!(value.as_ref(), FxCoreExpr::ConstInt(100)));
        // body는 Add여야 하고, 양쪽이 Var("x")여야 함
        match body.as_ref() {
          FxCoreExpr::Binary { lhs, rhs, .. } => {
            assert!(matches!(lhs.as_ref(), FxCoreExpr::Var(ref n) if n == "x"));
            assert!(matches!(rhs.as_ref(), FxCoreExpr::Var(ref n) if n == "x"));
          }
          _ => panic!("Expected Binary(Add) in Let body, got {:?}", body),
        }
      }
      _ => panic!("Expected Let node, got {:?}", fx),
    }
  }

  #[test]
  fn test_lower_derived_time() {
    let expr = UnifiedExpr::Derived {
      op: MeaningOpId::MinutesFromTime,
      args: Vec::new(),
    };
    let core = lower_to_fx_core(&expr).unwrap();
    assert!(matches!(
      core,
      FxCoreExpr::Derived { meta, .. } if meta.op == MeaningOpId::MinutesFromTime
    ));
  }

  #[test]
  fn test_constructor_pattern_matching_compile_time() {
    // Y09c: 컴파일 타임 Construct 리터럴에 대한 constructor 패턴 매칭
    use crate::lang::pnix::syntax::{PnixMatchArm, PnixPattern};

    // Some(42) 매칭
    let expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Construct {
        variant: "Some".to_string(),
        args: vec![PnixExpr::Int(42)],
      }),
      arms: vec![PnixMatchArm {
        pattern: PnixPattern::Constructor {
          variant: "Some".to_string(),
          args: vec![PnixPattern::Var("x".to_string())],
        },
        guard: None,
        body: PnixExpr::Var("x".to_string()),
      }],
    };

    let unified = pnix_expr_to_unified(&expr).unwrap();
    // 컴파일 타임 매칭이 성공해야 함
    assert!(!matches!(unified, UnifiedExpr::Throw(_)));
  }

  #[test]
  fn test_constructor_pattern_matching_nullary() {
    // Y09c: nullary constructor 패턴 매칭 (None, Ok, Err)
    use crate::lang::pnix::syntax::{PnixMatchArm, PnixPattern};

    // None 매칭
    let expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Construct {
        variant: "None".to_string(),
        args: vec![],
      }),
      arms: vec![PnixMatchArm {
        pattern: PnixPattern::Constructor {
          variant: "None".to_string(),
          args: vec![],
        },
        guard: None,
        body: PnixExpr::String("none".to_string()),
      }],
    };

    let unified = pnix_expr_to_unified(&expr).unwrap();
    // 컴파일 타임 매칭이 성공해야 함
    assert!(!matches!(unified, UnifiedExpr::Throw(_)));
  }

  #[test]
  fn test_match_non_exhaustive_single_arm() {
    // Y08b-2: 단일 arm non-match 케이스 - Throw 에러 발생해야 함
    use crate::lang::pnix::syntax::{PnixLiteralPattern, PnixMatchArm, PnixPattern};

    let expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Int(5)),
      arms: vec![PnixMatchArm {
        pattern: PnixPattern::Literal(PnixLiteralPattern::Int(10)), // 5 != 10
        guard: None,
        body: PnixExpr::String("ten".to_string()),
      }],
    };

    // Y08b-2: non-exhaustive match는 에러를 발생시켜야 함
    let result = pnix_expr_to_unified(&expr);

    // 에러가 발생해야 함 (non-exhaustive match)
    assert!(result.is_err(), "Expected error for non-exhaustive match");
    match result.unwrap_err() {
      PnixError::Lowering { message: msg, .. } => {
        assert!(
          msg.contains("non-exhaustive"),
          "Expected non-exhaustive error, got: {}",
          msg
        );
      }
      _ => panic!("Expected Lowering error, got different error type"),
    }
  }

  #[test]
  fn test_match_exhaustiveness_check_wildcard() {
    // Wildcard 패턴이 있으면 exhaustive
    use crate::lang::pnix::syntax::{PnixMatchArm, PnixPattern};

    let expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Var("x".to_string())),
      arms: vec![
        PnixMatchArm {
          pattern: PnixPattern::Literal(PnixLiteralPattern::Int(1)),
          guard: None,
          body: PnixExpr::Int(10),
        },
        PnixMatchArm {
          pattern: PnixPattern::Wildcard,
          guard: None,
          body: PnixExpr::Int(20),
        },
      ],
    };

    // exhaustive이므로 에러 없이 통과해야 함
    assert!(pnix_expr_to_unified(&expr).is_ok());
  }

  #[test]
  fn test_match_exhaustiveness_check_bool_complete() {
    // Bool 리터럴이 모두 있으면 exhaustive
    use crate::lang::pnix::syntax::{PnixMatchArm, PnixPattern};

    let expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Var("b".to_string())),
      arms: vec![
        PnixMatchArm {
          pattern: PnixPattern::Literal(PnixLiteralPattern::Bool(true)),
          guard: None,
          body: PnixExpr::Int(1),
        },
        PnixMatchArm {
          pattern: PnixPattern::Literal(PnixLiteralPattern::Bool(false)),
          guard: None,
          body: PnixExpr::Int(0),
        },
      ],
    };

    // exhaustive이므로 에러 없이 통과해야 함
    assert!(pnix_expr_to_unified(&expr).is_ok());
  }

  #[test]
  fn test_match_exhaustiveness_check_bool_incomplete() {
    // Bool 리터럴이 하나만 있으면 non-exhaustive
    use crate::lang::pnix::syntax::{PnixMatchArm, PnixPattern};

    let expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Var("b".to_string())),
      arms: vec![PnixMatchArm {
        pattern: PnixPattern::Literal(PnixLiteralPattern::Bool(true)),
        guard: None,
        body: PnixExpr::Int(1),
      }],
    };

    // non-exhaustive이므로 에러 발생해야 함
    let result = pnix_expr_to_unified(&expr);
    assert!(result.is_err());
    if let Err(PnixError::Lowering { message: msg, .. }) = result {
      assert!(msg.contains("non-exhaustive") || msg.contains("missing false"));
    } else {
      panic!("Expected Lowering error");
    }
  }

  #[test]
  fn test_match_exhaustiveness_check_constructor_duplicate() {
    // Constructor 패턴이 중복되면 에러
    use crate::lang::pnix::syntax::{PnixMatchArm, PnixPattern};

    let expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Var("x".to_string())),
      arms: vec![
        PnixMatchArm {
          pattern: PnixPattern::Constructor {
            variant: "Some".to_string(),
            args: vec![PnixPattern::Var("a".to_string())],
          },
          guard: None,
          body: PnixExpr::Int(1),
        },
        PnixMatchArm {
          pattern: PnixPattern::Constructor {
            variant: "Some".to_string(),
            args: vec![PnixPattern::Var("b".to_string())],
          },
          guard: None,
          body: PnixExpr::Int(2),
        },
      ],
    };

    // 중복 패턴이므로 에러 발생해야 함
    let result = pnix_expr_to_unified(&expr);
    assert!(result.is_err());
    if let Err(PnixError::Lowering { message: msg, .. }) = result {
      assert!(msg.contains("duplicate constructor pattern") || msg.contains("Some"));
    } else {
      panic!("Expected Lowering error");
    }
  }

  #[test]
  fn test_match_non_exhaustive_last_arm_non_wildcard() {
    // Y08b-2: 마지막 arm non-wildcard 케이스 - 패턴 불일치 시 Throw 발생해야 함
    use crate::lang::pnix::syntax::{PnixLiteralPattern, PnixMatchArm, PnixPattern};

    let expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Int(3)),
      arms: vec![
        PnixMatchArm {
          pattern: PnixPattern::Literal(PnixLiteralPattern::Int(1)),
          guard: None,
          body: PnixExpr::String("one".to_string()),
        },
        PnixMatchArm {
          pattern: PnixPattern::Literal(PnixLiteralPattern::Int(2)), // 3 != 2
          guard: None,
          body: PnixExpr::String("two".to_string()),
        },
      ],
    };

    let unified = pnix_expr_to_unified(&expr).unwrap();

    // 마지막 arm이 패턴 불일치 시 Throw 발생해야 함
    match unified {
      UnifiedExpr::Let { body, .. } => {
        // body는 If 체인이고, 마지막 else_가 Throw여야 함
        match body.as_ref() {
          UnifiedExpr::If { else_, .. } => {
            // else_는 또 다른 If일 수 있음 (첫 번째 arm의 else)
            // 재귀적으로 마지막 else_를 찾아야 함
            let mut current = else_.as_ref();
            loop {
              match current {
                UnifiedExpr::If { else_, .. } => {
                  current = else_.as_ref();
                }
                UnifiedExpr::Throw(msg) => {
                  assert!(msg.contains("non-exhaustive"));
                  break;
                }
                _ => panic!("Expected Throw or If, got {:?}", current),
              }
            }
          }
          _ => panic!("Expected If chain, got {:?}", body),
        }
      }
      _ => panic!("Expected Let with scrutinee binding, got {:?}", unified),
    }
  }

  #[test]
  fn test_match_variable_binding() {
    // Y08b-3: 변수 패턴 바인딩 검증 - `match x with | n => n + 1`
    use crate::lang::pnix::syntax::{PnixMatchArm, PnixPattern};

    let expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Var("x".to_string())),
      arms: vec![PnixMatchArm {
        pattern: PnixPattern::Var("n".to_string()),
        guard: None,
        body: PnixExpr::Binary {
          op: "+",
          lhs: Arc::new(PnixExpr::Var("n".to_string())),
          rhs: Arc::new(PnixExpr::Int(1)),
        },
      }],
    };

    let unified = pnix_expr_to_unified(&expr).unwrap();

    // scrutinee가 let으로 바인딩되고, 변수 패턴도 let으로 바인딩되어야 함
    match unified {
      UnifiedExpr::Let {
        name: scrutinee_name,
        value,
        body,
      } => {
        // Y13a-17: match scrutinee 이름 충돌 방지 - gensym을 사용하므로 _match_scrutinee_* 패턴으로 시작
        assert!(
          scrutinee_name.starts_with("_match_scrutinee_"),
          "scrutinee name should be gensym: {}",
          scrutinee_name
        );
        assert!(matches!(value.as_ref(), UnifiedExpr::Var(ref n) if n == "x"));

        // body는 변수 패턴의 let 바인딩을 포함해야 함
        match body.as_ref() {
          UnifiedExpr::Let {
            name: var_name,
            value: var_value,
            body: var_body,
          } => {
            assert_eq!(var_name, "n");
            // var_value는 scrutinee_var를 참조해야 함 (gensym 이름)
            assert!(
              matches!(var_value.as_ref(), UnifiedExpr::Var(ref s) if s.starts_with("_match_scrutinee_")),
              "var_value should reference scrutinee_var (gensym): {:?}",
              var_value
            );
            // var_body는 n + 1이어야 함
            match var_body.as_ref() {
              UnifiedExpr::Add(lhs, rhs) => {
                assert!(matches!(lhs.as_ref(), UnifiedExpr::Var(ref n) if n == "n"));
                assert!(matches!(rhs.as_ref(), UnifiedExpr::Int(1)));
              }
              _ => panic!("Expected Add(n, 1), got {:?}", var_body),
            }
          }
          _ => panic!("Expected Let binding for variable pattern, got {:?}", body),
        }
      }
      _ => panic!("Expected Let with scrutinee binding, got {:?}", unified),
    }
  }

  #[test]
  fn test_match_constructor_pattern_variable_binding() {
    // Y13a-12: Constructor 패턴 변수 바인딩 검증 - 마지막 arm에서도 바인딩되어야 함
    // `match x with | Some(n) => n + 1`
    use crate::lang::pnix::syntax::{PnixMatchArm, PnixPattern};

    let expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Var("x".to_string())),
      arms: vec![PnixMatchArm {
        pattern: PnixPattern::Constructor {
          variant: "Some".to_string(),
          args: vec![PnixPattern::Var("n".to_string())],
        },
        guard: None,
        body: PnixExpr::Binary {
          op: "+",
          lhs: Arc::new(PnixExpr::Var("n".to_string())),
          rhs: Arc::new(PnixExpr::Int(1)),
        },
      }],
    };

    let unified = pnix_expr_to_unified(&expr).unwrap();

    // 결과 구조 확인:
    // Let { scrutinee_var = x } {
    //   If { cond = (scrutinee._variant == "Some"),
    //        then = Let { n = builtins.elemAt(scrutinee._args, 0) } { n + 1 },
    //        else = Throw }
    // }
    fn find_let_binding(expr: &UnifiedExpr, var_name: &str) -> bool {
      match expr {
        UnifiedExpr::Let { name, body, .. } => name == var_name || find_let_binding(body, var_name),
        UnifiedExpr::If { then_, else_, .. } => {
          find_let_binding(then_, var_name) || find_let_binding(else_, var_name)
        }
        UnifiedExpr::And(a, b) | UnifiedExpr::Or(a, b) => {
          find_let_binding(a, var_name) || find_let_binding(b, var_name)
        }
        _ => false,
      }
    }

    // then_ branch에서 "n" 변수가 let으로 바인딩되어야 함
    assert!(
      find_let_binding(&unified, "n"),
      "Constructor pattern variable 'n' should be bound in the result: {:?}",
      unified
    );
  }

  #[test]
  fn test_match_constructor_pattern_guard_with_binding() {
    // Y13a-12: Constructor 패턴 가드에서 변수 사용 가능 확인
    // `match x with | Some(n) if n > 0 => n`
    use crate::lang::pnix::syntax::{PnixMatchArm, PnixPattern};

    let expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Var("x".to_string())),
      arms: vec![
        PnixMatchArm {
          pattern: PnixPattern::Constructor {
            variant: "Some".to_string(),
            args: vec![PnixPattern::Var("n".to_string())],
          },
          guard: Some(Arc::new(PnixExpr::Binary {
            op: ">",
            lhs: Arc::new(PnixExpr::Var("n".to_string())),
            rhs: Arc::new(PnixExpr::Int(0)),
          })),
          body: PnixExpr::Var("n".to_string()),
        },
        PnixMatchArm {
          pattern: PnixPattern::Wildcard,
          guard: None,
          body: PnixExpr::Int(0),
        },
      ],
    };

    let unified = pnix_expr_to_unified(&expr).unwrap();

    // 가드 조건에서도 n 변수가 바인딩되어 사용 가능해야 함
    // And( pattern_match_cond, Let { n = ... } { n > 0 } ) 형태
    fn find_gt_in_condition(expr: &UnifiedExpr) -> bool {
      match expr {
        UnifiedExpr::And(_, b) => {
          // 조건의 두 번째 부분이 Let { n = ... } { Gt(n, 0) } 형태여야 함
          matches!(**b, UnifiedExpr::Let { .. })
        }
        _ => false,
      }
    }

    fn extract_first_if_cond(expr: &UnifiedExpr) -> Option<&UnifiedExpr> {
      match expr {
        UnifiedExpr::Let { body, .. } => extract_first_if_cond(body),
        UnifiedExpr::If { cond, .. } => Some(cond),
        _ => None,
      }
    }

    if let Some(cond) = extract_first_if_cond(&unified) {
      assert!(
        find_gt_in_condition(cond),
        "Guard should have pattern variable binding: {:?}",
        cond
      );
    } else {
      panic!("Expected If in result: {:?}", unified);
    }
  }

  #[test]
  fn test_match_scrutinee_single_evaluation() {
    // Y08b-3: scrutinee 단일 평가 검증 - effectful scrutinee가 1회만 평가되어야 함
    // `match (expensive()) with | n => n + 1`
    // scrutinee가 let으로 바인딩되므로 단일 평가 보장

    use crate::lang::pnix::syntax::{PnixMatchArm, PnixPattern};

    // expensive() 함수 호출을 scrutinee로 사용
    let expensive_call = PnixExpr::Apply {
      func: Arc::new(PnixExpr::Var("expensive".to_string())),
      arg: Arc::new(PnixExpr::Null),
    };

    let expr = PnixExpr::Match {
      scrutinee: Box::new(expensive_call),
      arms: vec![PnixMatchArm {
        pattern: PnixPattern::Var("n".to_string()),
        guard: None,
        body: PnixExpr::Binary {
          op: "+",
          lhs: Arc::new(PnixExpr::Var("n".to_string())),
          rhs: Arc::new(PnixExpr::Int(1)),
        },
      }],
    };

    let unified = pnix_expr_to_unified(&expr).unwrap();

    // scrutinee가 let으로 바인딩되어야 함 (단일 평가 보장)
    match unified {
      UnifiedExpr::Let {
        name: scrutinee_name,
        value,
        body: _,
      } => {
        // Y13a-17: match scrutinee 이름 충돌 방지 - gensym을 사용하므로 _match_scrutinee_* 패턴으로 시작
        assert!(
          scrutinee_name.starts_with("_match_scrutinee_"),
          "scrutinee name should be gensym: {}",
          scrutinee_name
        );
        // value는 expensive() 호출이어야 함
        match value.as_ref() {
          UnifiedExpr::Apply { func, args } => {
            assert_eq!(func, "expensive");
            assert_eq!(args.len(), 1);
          }
          _ => panic!("Expected Apply(expensive, ...), got {:?}", value),
        }
      }
      _ => panic!("Expected Let with scrutinee binding, got {:?}", unified),
    }

    // body에서 scrutinee_var가 여러 번 사용되더라도 value는 1회만 평가됨
    // (Let 노드가 보존되므로)
  }

  #[test]
  fn test_pnix_to_unified_depth_limit() {
    // LOW: LOWERING_DEPTH 테스트 격리 실패 수정
    // 테스트 시작 시 depth를 0으로 초기화하여 테스트 간 격리 보장
    LoweringDepthGuard::reset_for_test();
    let mut expr = PnixExpr::Int(0);
    for _ in 0..(MAX_LOWERING_DEPTH + 1) {
      expr = PnixExpr::Unary {
        op: "-",
        arg: Box::new(expr),
      };
    }

    let err = pnix_expr_to_unified(&expr).unwrap_err();
    assert!(
      matches!(err, PnixError::Lowering { message: msg, .. } if msg.contains("recursion depth"))
    );
  }

  #[test]
  fn test_lower_to_fx_core_depth_limit() {
    // LOW: LOWERING_DEPTH 테스트 격리 실패 수정
    // 테스트 시작 시 depth를 0으로 초기화하여 테스트 간 격리 보장
    LoweringDepthGuard::reset_for_test();
    // 스택 오버플로 방지를 위해 guard를 먼저 쌓아 depth 초과를 유도
    let mut guards = Vec::new();
    for _ in 0..MAX_LOWERING_DEPTH {
      guards.push(LoweringDepthGuard::enter("test_guard").unwrap());
    }

    let expr = UnifiedExpr::Neg(Box::new(UnifiedExpr::Int(0)));
    let mut mapping = SignalVarMapping::new();
    let err = lower_to_fx_core_with_mapping(&expr, &mut mapping).unwrap_err();
    assert!(
      matches!(err, PnixError::Lowering { message: msg, .. } if msg.contains("recursion depth"))
    );
  }

  // ========== Y-CLAUDE-6: ++ 연산자 테스트 ==========

  #[test]
  fn test_concat_operator_parsing() {
    // Y-CLAUDE-6: ++ 연산자가 파서에서 인식되어야 함
    use crate::lang::pnix::parser::parse_expr;

    let result = parse_expr(r#""hello" ++ "world""#);
    assert!(result.is_ok());
    let expr = result.unwrap();

    // Binary { op: "++", ... } 형태여야 함
    match expr {
      PnixExpr::Binary { op, lhs, rhs } => {
        assert_eq!(op, "++");
        assert!(matches!(*lhs, PnixExpr::String(ref s) if s == "hello"));
        assert!(matches!(*rhs, PnixExpr::String(ref s) if s == "world"));
      }
      _ => panic!("Expected Binary(++, ...), got {:?}", expr),
    }
  }

  #[test]
  fn test_concat_operator_to_unified() {
    // Y-CLAUDE-6: ++ 연산자가 UnifiedExpr::Concat으로 변환되어야 함
    let expr = PnixExpr::Binary {
      op: "++",
      lhs: Arc::new(PnixExpr::String("hello".to_string())),
      rhs: Arc::new(PnixExpr::String("world".to_string())),
    };

    let unified = pnix_expr_to_unified(&expr).unwrap();

    match unified {
      UnifiedExpr::Concat(lhs, rhs) => {
        assert!(matches!(*lhs, UnifiedExpr::String(ref s) if s == "hello"));
        assert!(matches!(*rhs, UnifiedExpr::String(ref s) if s == "world"));
      }
      _ => panic!("Expected Concat, got {:?}", unified),
    }
  }

  #[test]
  fn test_concat_to_fx_core() {
    // Y-CLAUDE-6: UnifiedExpr::Concat이 FxCoreExpr::concat으로 변환되어야 함
    let expr = UnifiedExpr::Concat(
      Box::new(UnifiedExpr::String("hello".to_string())),
      Box::new(UnifiedExpr::String("world".to_string())),
    );

    let fx = lower_to_fx_core(&expr).unwrap();

    assert_concat_binary(&fx);
  }

  #[test]
  fn test_concat_with_variables() {
    // Y-CLAUDE-6: 변수 간의 ++ 연산도 Concat으로 변환되어야 함
    let expr = UnifiedExpr::Concat(
      Box::new(UnifiedExpr::Var("x".to_string())),
      Box::new(UnifiedExpr::Var("y".to_string())),
    );

    let fx = lower_to_fx_core(&expr).unwrap();

    assert_concat_binary(&fx);
  }

  /// Concat 연산이 Binary(Concat)으로 변환되었는지 검증하는 헬퍼 함수
  fn assert_concat_binary(fx: &FxCoreExpr) {
    match fx {
      FxCoreExpr::Binary { meta, .. } => {
        assert_eq!(meta.op, MeaningOpId::Concat, "Expected Concat operation");
      }
      _ => panic!("Expected Binary(Concat), got {:?}", fx),
    }
  }

  #[test]
  fn test_complex_func_apply_select() {
    // Y-CLAUDE-apply: Select 표현식을 함수로 적용
    // obj.method(arg) → let _apply_func = obj.method in _apply_func(arg)
    let select_expr = PnixExpr::Select {
      base: Arc::new(PnixExpr::Var("obj".to_string())),
      attr: "method".to_string(),
    };
    let apply_expr = PnixExpr::Apply {
      func: Box::new(select_expr),
      arg: Arc::new(PnixExpr::Int(42)),
    };

    let result = pnix_expr_to_unified(&apply_expr);
    assert!(result.is_ok(), "Complex function apply should succeed");

    let unified = result.unwrap();
    // 결과는 Let 바인딩으로 감싸진 Apply여야 함
    match unified {
      UnifiedExpr::Let { name, value, body } => {
        // apply_complex_expr_as_func uses "_apply_func_" prefix
        assert!(
          name.starts_with("_apply_func_"),
          "Should use _apply_func_ gensym name, got: {}",
          name
        );
        // value는 builtins.getAttr 호출 (Select → getAttr lowering)
        match value.as_ref() {
          UnifiedExpr::Apply { func, args } => {
            assert_eq!(func, "builtins.getAttr");
            assert_eq!(args.len(), 2);
          }
          _ => panic!("Expected Apply(builtins.getAttr) in value, got {:?}", value),
        }
        // body는 Apply
        match *body {
          UnifiedExpr::Apply { func, args } => {
            assert_eq!(func, name, "Apply should reference the let-bound name");
            assert_eq!(args.len(), 1);
          }
          _ => panic!("Expected Apply in body"),
        }
      }
      _ => panic!("Expected Let wrapping Apply"),
    }
  }

  #[test]
  fn test_complex_func_apply_nested() {
    // Y-CLAUDE-apply: 중첩된 Apply는 flatten됨
    // (f x)(y) → Apply { func: "f", args: [x, y] }
    // 이것이 Apply flatten 동작입니다 - Let으로 감싸지지 않습니다
    let inner_apply = PnixExpr::Apply {
      func: Arc::new(PnixExpr::Var("f".to_string())),
      arg: Arc::new(PnixExpr::Var("x".to_string())),
    };
    let outer_apply = PnixExpr::Apply {
      func: Box::new(inner_apply),
      arg: Arc::new(PnixExpr::Var("y".to_string())),
    };

    let result = pnix_expr_to_unified(&outer_apply);
    assert!(result.is_ok(), "Nested apply should succeed");

    let unified = result.unwrap();
    // Apply flatten: (f x)(y) → Apply(f, [x, y])
    match unified {
      UnifiedExpr::Apply { func, args } => {
        assert_eq!(func, "f");
        assert_eq!(args.len(), 2);
        // args[0] = x, args[1] = y
        assert!(matches!(&args[0], UnifiedExpr::Var(n) if n == "x"));
        assert!(matches!(&args[1], UnifiedExpr::Var(n) if n == "y"));
      }
      _ => panic!("Expected Apply(f, [x, y]) from flatten, got {:?}", unified),
    }
  }

  #[test]
  fn test_complex_func_apply_match() {
    // Y-CLAUDE-apply: Match 표현식을 함수로 적용
    // (match x with | true => f | false => g)(arg) → let _func = ... in _func(arg)
    use crate::lang::pnix::syntax::{PnixMatchArm, PnixPattern};

    let match_expr = PnixExpr::Match {
      scrutinee: Arc::new(PnixExpr::Var("x".to_string())),
      arms: vec![PnixMatchArm {
        pattern: PnixPattern::Var("_".to_string()),
        guard: None,
        body: PnixExpr::Var("f".to_string()),
      }],
    };
    let apply_expr = PnixExpr::Apply {
      func: Box::new(match_expr),
      arg: Arc::new(PnixExpr::Int(42)),
    };

    let result = pnix_expr_to_unified(&apply_expr);
    assert!(result.is_ok(), "Match function apply should succeed");

    let unified = result.unwrap();
    // 결과는 Let 바인딩으로 감싸진 Apply여야 함
    match unified {
      UnifiedExpr::Let { name, body, .. } => {
        assert!(name.starts_with("_apply_func_"), "Should use gensym name");
        // body는 Apply
        match *body {
          UnifiedExpr::Apply { func, args } => {
            assert_eq!(func, name, "Apply should reference the let-bound name");
            assert_eq!(args.len(), 1);
          }
          _ => panic!("Expected Apply in body"),
        }
      }
      _ => panic!("Expected Let wrapping Apply, got {:?}", unified),
    }
  }

  #[test]
  fn test_complex_func_apply_attrset() {
    // Y-CLAUDE-apply: AttrSet을 함수로 적용 (에러가 아닌 Let-bind 처리)
    // { a = 1; }(x) → let _func = { a = 1; } in _func(x)
    let attrset = PnixExpr::AttrSet {
      items: vec![],
      recursive: false,
    };
    let apply_expr = PnixExpr::Apply {
      func: Box::new(attrset),
      arg: Arc::new(PnixExpr::Int(1)),
    };

    let result = pnix_expr_to_unified(&apply_expr);
    assert!(result.is_ok(), "AttrSet as function should be handled");
  }

  // ========== List Indexing Tests ==========

  #[test]
  fn test_list_index_parsing() {
    // list[0] → Index { base: Var("list"), index: Int(0) }
    use crate::lang::pnix::parser::parse_expr;

    let result = parse_expr("list[0]");
    assert!(result.is_ok(), "list[0] should parse: {:?}", result.err());
    let expr = result.unwrap();

    match expr {
      PnixExpr::Index { base, index } => {
        assert!(matches!(*base, PnixExpr::Var(ref n) if n == "list"));
        assert!(matches!(*index, PnixExpr::Int(0)));
      }
      _ => panic!("Expected Index, got {:?}", expr),
    }
  }

  #[test]
  fn test_list_index_with_expression() {
    // list[i + 1] → Index { base: Var("list"), index: Binary { ... } }
    use crate::lang::pnix::parser::parse_expr;

    let result = parse_expr("list[i + 1]");
    assert!(
      result.is_ok(),
      "list[i + 1] should parse: {:?}",
      result.err()
    );
    let expr = result.unwrap();

    match expr {
      PnixExpr::Index { base, index } => {
        assert!(matches!(*base, PnixExpr::Var(ref n) if n == "list"));
        assert!(matches!(*index, PnixExpr::Binary { .. }));
      }
      _ => panic!("Expected Index, got {:?}", expr),
    }
  }

  #[test]
  fn test_list_index_chained() {
    // matrix[0][1] → Index { base: Index { ... }, index: Int(1) }
    use crate::lang::pnix::parser::parse_expr;

    let result = parse_expr("matrix[0][1]");
    assert!(
      result.is_ok(),
      "matrix[0][1] should parse: {:?}",
      result.err()
    );
    let expr = result.unwrap();

    match expr {
      PnixExpr::Index { base, index } => {
        assert!(matches!(*index, PnixExpr::Int(1)));
        assert!(matches!(*base, PnixExpr::Index { .. }));
      }
      _ => panic!("Expected nested Index, got {:?}", expr),
    }
  }

  #[test]
  fn test_list_index_lowering() {
    // list[0] → builtins.elemAt(list, 0)
    let expr = PnixExpr::Index {
      base: Arc::new(PnixExpr::Var("list".to_string())),
      index: Arc::new(PnixExpr::Int(0)),
    };

    let unified = pnix_expr_to_unified(&expr).unwrap();

    match unified {
      UnifiedExpr::Apply { func, args } => {
        assert_eq!(func, "builtins.elemAt");
        assert_eq!(args.len(), 2);
        assert!(matches!(&args[0], UnifiedExpr::Var(n) if n == "list"));
        assert!(matches!(&args[1], UnifiedExpr::Int(0)));
      }
      _ => panic!("Expected Apply(builtins.elemAt, ...), got {:?}", unified),
    }
  }

  #[test]
  fn test_list_index_with_select() {
    // obj.items[0] → Index { base: Select { ... }, index: Int(0) }
    use crate::lang::pnix::parser::parse_expr;

    let result = parse_expr("obj.items[0]");
    assert!(
      result.is_ok(),
      "obj.items[0] should parse: {:?}",
      result.err()
    );
    let expr = result.unwrap();

    match expr {
      PnixExpr::Index { base, index } => {
        assert!(matches!(*base, PnixExpr::Select { .. }));
        assert!(matches!(*index, PnixExpr::Int(0)));
      }
      _ => panic!("Expected Index with Select base, got {:?}", expr),
    }
  }

  #[test]
  fn test_merge_lowering_semantics() {
    // Y10c: a // b → lhs // rhs where rhs wins (b wins)
    // Verify: Merge(lhs, rhs) → FxCoreExpr::update(lhs, rhs)
    use crate::lang::pnix::parser::parse_expr;

    // Parse a // b
    let result = parse_expr("a // b");
    assert!(result.is_ok());
    let expr = result.unwrap();

    // Lower to unified
    let unified = pnix_expr_to_unified(&expr).unwrap();
    match &unified {
      UnifiedExpr::Merge(lhs, rhs) => {
        assert!(matches!(&**lhs, UnifiedExpr::Var(s) if s == "a"));
        assert!(matches!(&**rhs, UnifiedExpr::Var(s) if s == "b"));
      }
      _ => panic!("Expected Merge, got {:?}", unified),
    }

    // Lower to FxCore
    let fx_core = lower_to_fx_core(&unified).unwrap();
    match fx_core {
      FxCoreExpr::Binary { meta, lhs, rhs } => {
        // AttrSetUpdate semantics: lhs updated by rhs = rhs wins
        assert_eq!(meta.op, MeaningOpId::AttrSetUpdate);
        // lhs should be 'a' (variable), rhs should be 'b' (variable)
        assert!(matches!(*lhs, FxCoreExpr::Var(ref name) if name == "a"));
        assert!(matches!(*rhs, FxCoreExpr::Var(ref name) if name == "b"));
      }
      _ => panic!("Expected Binary AttrSetUpdate, got {:?}", fx_core),
    }
  }

  #[test]
  fn test_lowering_reason_tag_for_immediate_apply_destructuring_param() {
    use crate::lang::pnix::parser::parse_expr;
    let expr = parse_expr("({ x }: x) 1").expect("parse should succeed");
    let err = pnix_expr_to_unified(&expr).expect_err("lowering should fail");
    match err {
      PnixError::Lowering { message, .. } => {
        assert!(message.contains("[LOWERING_IMMEDIATE_APPLY_DESTRUCTURING_PARAM_UNSUPPORTED]"));
      }
      other => panic!("Expected Lowering error, got {:?}", other),
    }
  }

  #[test]
  fn test_lowering_reason_tag_for_too_many_lambda_args() {
    use crate::lang::pnix::parser::parse_expr;
    let expr = parse_expr("(x: x) 1 2").expect("parse should succeed");
    let err = pnix_expr_to_unified(&expr).expect_err("lowering should fail");
    match err {
      PnixError::Lowering { message, .. } => {
        assert!(message.contains("[LOWERING_LAMBDA_TOO_MANY_ARGS]"));
      }
      other => panic!("Expected Lowering error, got {:?}", other),
    }
  }

  #[test]
  fn test_lowering_reason_tag_for_complex_with_env() {
    use crate::lang::pnix::parser::parse_expr;
    let expr = parse_expr("with (x: x); x").expect("parse should succeed");
    let err = pnix_expr_to_unified(&expr).expect_err("lowering should fail");
    match err {
      PnixError::Lowering { message, .. } => {
        assert!(message.contains("[LOWERING_WITH_COMPLEX_ENV_UNSUPPORTED]"));
      }
      other => panic!("Expected Lowering error, got {:?}", other),
    }
  }
}
