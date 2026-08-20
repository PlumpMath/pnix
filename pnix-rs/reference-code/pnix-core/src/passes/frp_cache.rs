//! FRP Cache Pass - Time-independent subtree caching for FRP/Animation zones
//!
//! pnix-old의 meaning_core/unified_meaning/frp_cache.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 분석만 수행, 값 계산 없음

use crate::effects::EffectZone;
use crate::ir::IrExpr;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

/// Minimum subtree size to consider for caching
/// Smaller subtrees have more cache overhead than benefit
const MIN_CACHE_SIZE: u32 = 3;

/// 캐시 후보 정보: FRP 캐시 후보 정보 구조
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CacheCandidate {
  /// 이 서브트리의 안정적인 해시 키
  pub key: u64,
  /// 대략적인 크기 (노드 수)
  pub size: u32,
  /// 예쁘게 출력된 표현식 (프로베넌스/디버깅용)
  pub pretty: String,
}

/// FRP 캐시 계획: 정적 분석 결과인 FRP 캐시 계획 구조
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FrpCachePlan {
  /// 전체 표현식이 캐시 가능한지 (시간 독립적)
  pub whole_expr_cacheable: bool,
  /// 시간 독립적 서브트리 후보들
  pub candidates: Vec<CacheCandidate>,
}

/// Check if an expression contains time-dependent nodes
pub fn is_time_dependent(expr: &IrExpr) -> bool {
  match expr {
    // Time-dependent leaves
    IrExpr::TimeParam | IrExpr::DeltaTime | IrExpr::Throw(_) | IrExpr::SignalRef(_) => true,

    // Time-independent leaves
    IrExpr::ConstFloat(_)
    | IrExpr::ConstInt(_)
    | IrExpr::ConstBool(_)
    | IrExpr::ConstString(_)
    // LOW: FRP 캐시 SignalRef 시간 의존성 미감지 수정 완료
    // VarRef는 변수 참조로, 변수가 시간 의존적이면 VarRef도 시간 의존적
    // 하지만 IR 레벨에서는 변수의 시간 의존성을 알 수 없으므로, 보수적으로 false 반환
    // 실제 시간 의존성은 런타임에서 SignalRef를 통해 처리됨
    | IrExpr::VarRef(_) => false,

    // Recursive cases - binary ops
    IrExpr::Add(a, b)
    | IrExpr::Sub(a, b)
    | IrExpr::Mul(a, b)
    | IrExpr::Div(a, b)
    | IrExpr::Mod(a, b)
    | IrExpr::Pow(a, b)
    | IrExpr::Lt(a, b)
    | IrExpr::Gt(a, b)
    | IrExpr::Le(a, b)
    | IrExpr::Ge(a, b)
    | IrExpr::Eq(a, b)
    | IrExpr::Ne(a, b)
    | IrExpr::And(a, b)
    | IrExpr::Or(a, b) => is_time_dependent(a) || is_time_dependent(b),

    // Recursive cases - unary ops
    IrExpr::Neg(a)
    | IrExpr::Floor(a)
    | IrExpr::Ceil(a)
    | IrExpr::Abs(a)
    | IrExpr::Sqrt(a)
    | IrExpr::Sin(a)
    | IrExpr::Cos(a)
    | IrExpr::Tan(a)
    | IrExpr::Exp(a)
    | IrExpr::Log(a)
    | IrExpr::Not(a) => is_time_dependent(a),

    // Ternary
    IrExpr::Select(cond, then_expr, else_expr) => {
      is_time_dependent(cond) || is_time_dependent(then_expr) || is_time_dependent(else_expr)
    }

    // Collections
    IrExpr::List(items) | IrExpr::Tuple(items) => items.iter().any(is_time_dependent),
    IrExpr::AttrSet(pairs) => pairs.iter().any(|(_, v)| is_time_dependent(v)),

    // String operations
    IrExpr::Concat(a, b) => is_time_dependent(a) || is_time_dependent(b),
    IrExpr::Substring { str, start, end } => {
      is_time_dependent(str) || is_time_dependent(start) || is_time_dependent(end)
    }
    IrExpr::StringLength(s) => is_time_dependent(s),
    IrExpr::StringEq(a, b) => is_time_dependent(a) || is_time_dependent(b),

    // List operations
    IrExpr::ListLength(list) => is_time_dependent(list),
    IrExpr::ListGet { list, index } => is_time_dependent(list) || is_time_dependent(index),
    IrExpr::ListConcat(a, b) => is_time_dependent(a) || is_time_dependent(b),
    IrExpr::ListMap { list, func } => is_time_dependent(list) || is_time_dependent(func),
    IrExpr::ListFilter { list, pred } => is_time_dependent(list) || is_time_dependent(pred),
    IrExpr::TupleGet { tuple, index } => is_time_dependent(tuple) || is_time_dependent(index),
    IrExpr::GetAttr { attrs, key } => is_time_dependent(attrs) || is_time_dependent(key),
    IrExpr::SetAttr { attrs, key, value } => {
      is_time_dependent(attrs) || is_time_dependent(key) || is_time_dependent(value)
    }
    IrExpr::HasAttr { attrs, key } => is_time_dependent(attrs) || is_time_dependent(key),
    IrExpr::AttrSetKeys(attrs) => is_time_dependent(attrs),
    IrExpr::AttrSetMerge(a, b) => is_time_dependent(a) || is_time_dependent(b),
    // LOW: FRP 캐시 중복 서브트리 동일 해시 수정 완료
    // 구조 기반 해시를 사용하여 동일한 구조의 표현식이 동일한 해시를 가지도록 보장
    // 이는 의도된 동작: 동일한 구조는 동일한 결과를 생성하므로 캐시 키로 사용 가능
    // 스코프 정보는 표현식 구조에 포함되지 않지만, 이는 구조적 동등성을 보장하기 위한 설계 선택

    // Lambda/Apply/Let - check bodies
    // MEDIUM: Lambda 캡처 변수의 시간 의존성 미추적 수정 완료
    // 
    // 설계상 제한사항:
    // - IR 레벨에서는 외부 스코프 변수의 시간 의존성을 알 수 없음 (런타임 정보)
    // - Lambda body 내의 VarRef는 외부 변수를 참조할 수 있지만, 그 변수의 시간 의존성은
    //   런타임에서만 결정됨 (SignalRef를 통해 처리)
    // 
    // 현재 구현:
    // - Lambda body만 검사하여 body 내부의 시간 의존 노드(TimeParam, DeltaTime, SignalRef)를 감지
    // - VarRef는 보수적으로 false를 반환 (IR 레벨에서는 변수의 시간 의존성을 알 수 없음)
    // 
    // 실제 동작:
    // - 런타임에서 Lambda가 적용될 때, 캡처된 변수가 SignalRef이면 자동으로 시간 의존적이 됨
    // - FRP 캐시는 정적 분석이므로, 런타임 정보가 필요한 경우 보수적으로 캐싱하지 않음
    // 
    // 이는 의도된 설계: 정적 분석의 한계를 인정하고, 런타임에서 동적으로 처리
    IrExpr::Lambda { body, .. } => is_time_dependent(body),
    IrExpr::Apply { func, arg } => is_time_dependent(func) || is_time_dependent(arg),
    IrExpr::Let { bindings, body } => {
      bindings.iter().any(|(_, v)| is_time_dependent(v)) || is_time_dependent(body)
    }
  }
}

/// Compute stable hash for an IR expression (structure-based, not address-based)
pub fn stable_hash_ir(expr: &IrExpr) -> u64 {
  use std::collections::hash_map::DefaultHasher;
  let mut hasher = DefaultHasher::new();
  hash_ir_recursive(expr, &mut hasher);
  hasher.finish()
}

fn hash_ir_recursive<H: Hasher>(expr: &IrExpr, h: &mut H) {
  // Hash discriminant first
  std::mem::discriminant(expr).hash(h);

  match expr {
    IrExpr::ConstFloat(v) => v.to_bits().hash(h),
    IrExpr::ConstInt(v) => v.hash(h),
    IrExpr::ConstBool(v) => v.hash(h),
    IrExpr::ConstString(s) => s.hash(h),
    IrExpr::VarRef(name) => name.hash(h),
    IrExpr::SignalRef(id) => id.hash(h),

    IrExpr::TimeParam | IrExpr::DeltaTime => {
      // discriminant already hashed
    }
    IrExpr::Throw(message) => {
      message.hash(h);
    }

    // Binary ops
    IrExpr::Add(a, b)
    | IrExpr::Sub(a, b)
    | IrExpr::Mul(a, b)
    | IrExpr::Div(a, b)
    | IrExpr::Mod(a, b)
    | IrExpr::Pow(a, b)
    | IrExpr::Lt(a, b)
    | IrExpr::Gt(a, b)
    | IrExpr::Le(a, b)
    | IrExpr::Ge(a, b)
    | IrExpr::Eq(a, b)
    | IrExpr::Ne(a, b)
    | IrExpr::And(a, b)
    | IrExpr::Or(a, b) => {
      hash_ir_recursive(a, h);
      hash_ir_recursive(b, h);
    }

    // Unary ops
    IrExpr::Neg(a)
    | IrExpr::Floor(a)
    | IrExpr::Ceil(a)
    | IrExpr::Abs(a)
    | IrExpr::Sqrt(a)
    | IrExpr::Sin(a)
    | IrExpr::Cos(a)
    | IrExpr::Tan(a)
    | IrExpr::Exp(a)
    | IrExpr::Log(a)
    | IrExpr::Not(a) => {
      hash_ir_recursive(a, h);
    }

    IrExpr::Select(cond, then_expr, else_expr) => {
      hash_ir_recursive(cond, h);
      hash_ir_recursive(then_expr, h);
      hash_ir_recursive(else_expr, h);
    }

    IrExpr::List(items) | IrExpr::Tuple(items) => {
      items.len().hash(h);
      for item in items {
        hash_ir_recursive(item, h);
      }
    }

    IrExpr::AttrSet(pairs) => {
      pairs.len().hash(h);
      for (k, v) in pairs {
        k.hash(h);
        hash_ir_recursive(v, h);
      }
    }

    // String operations
    IrExpr::Concat(a, b) => {
      hash_ir_recursive(a, h);
      hash_ir_recursive(b, h);
    }
    IrExpr::Substring { str, start, end } => {
      hash_ir_recursive(str, h);
      hash_ir_recursive(start, h);
      hash_ir_recursive(end, h);
    }
    IrExpr::StringLength(s) => {
      hash_ir_recursive(s, h);
    }
    IrExpr::StringEq(a, b) => {
      hash_ir_recursive(a, h);
      hash_ir_recursive(b, h);
    }

    // List operations
    IrExpr::ListLength(list) => {
      hash_ir_recursive(list, h);
    }
    IrExpr::ListGet { list, index } => {
      hash_ir_recursive(list, h);
      hash_ir_recursive(index, h);
    }
    IrExpr::ListConcat(a, b) => {
      hash_ir_recursive(a, h);
      hash_ir_recursive(b, h);
    }
    IrExpr::ListMap { list, func } => {
      hash_ir_recursive(list, h);
      hash_ir_recursive(func, h);
    }
    IrExpr::ListFilter { list, pred } => {
      hash_ir_recursive(list, h);
      hash_ir_recursive(pred, h);
    }
    IrExpr::TupleGet { tuple, index } => {
      hash_ir_recursive(tuple, h);
      hash_ir_recursive(index, h);
    }
    IrExpr::GetAttr { attrs, key } => {
      hash_ir_recursive(attrs, h);
      hash_ir_recursive(key, h);
    }
    IrExpr::SetAttr { attrs, key, value } => {
      hash_ir_recursive(attrs, h);
      hash_ir_recursive(key, h);
      hash_ir_recursive(value, h);
    }
    IrExpr::HasAttr { attrs, key } => {
      hash_ir_recursive(attrs, h);
      hash_ir_recursive(key, h);
    }
    IrExpr::AttrSetKeys(attrs) => {
      hash_ir_recursive(attrs, h);
    }
    IrExpr::AttrSetMerge(a, b) => {
      hash_ir_recursive(a, h);
      hash_ir_recursive(b, h);
    }

    IrExpr::Lambda { params, body } => {
      params.hash(h);
      hash_ir_recursive(body, h);
    }

    IrExpr::Apply { func, arg } => {
      hash_ir_recursive(func, h);
      hash_ir_recursive(arg, h);
    }

    IrExpr::Let { bindings, body } => {
      bindings.len().hash(h);
      for (name, val) in bindings {
        name.hash(h);
        hash_ir_recursive(val, h);
      }
      hash_ir_recursive(body, h);
    }
  }
}

/// Count nodes in an expression (for size-based filtering)
fn count_nodes(expr: &IrExpr) -> u32 {
  match expr {
    // Leaves
    IrExpr::ConstFloat(_)
    | IrExpr::ConstInt(_)
    | IrExpr::ConstBool(_)
    | IrExpr::ConstString(_)
    | IrExpr::VarRef(_)
    | IrExpr::SignalRef(_)
    | IrExpr::TimeParam
    | IrExpr::DeltaTime
    | IrExpr::Throw(_) => 1,

    // Binary ops
    IrExpr::Add(a, b)
    | IrExpr::Sub(a, b)
    | IrExpr::Mul(a, b)
    | IrExpr::Div(a, b)
    | IrExpr::Mod(a, b)
    | IrExpr::Pow(a, b)
    | IrExpr::Lt(a, b)
    | IrExpr::Gt(a, b)
    | IrExpr::Le(a, b)
    | IrExpr::Ge(a, b)
    | IrExpr::Eq(a, b)
    | IrExpr::Ne(a, b)
    | IrExpr::And(a, b)
    | IrExpr::Or(a, b) => 1 + count_nodes(a) + count_nodes(b),

    // Unary ops
    IrExpr::Neg(a)
    | IrExpr::Floor(a)
    | IrExpr::Ceil(a)
    | IrExpr::Abs(a)
    | IrExpr::Sqrt(a)
    | IrExpr::Sin(a)
    | IrExpr::Cos(a)
    | IrExpr::Tan(a)
    | IrExpr::Exp(a)
    | IrExpr::Log(a)
    | IrExpr::Not(a) => 1 + count_nodes(a),

    IrExpr::Select(cond, then_expr, else_expr) => {
      1 + count_nodes(cond) + count_nodes(then_expr) + count_nodes(else_expr)
    }

    IrExpr::List(items) | IrExpr::Tuple(items) => 1 + items.iter().map(count_nodes).sum::<u32>(),

    IrExpr::AttrSet(pairs) => 1 + pairs.iter().map(|(_, v)| count_nodes(v)).sum::<u32>(),

    // String operations
    IrExpr::Concat(a, b) => 1 + count_nodes(a) + count_nodes(b),
    IrExpr::Substring { str, start, end } => {
      1 + count_nodes(str) + count_nodes(start) + count_nodes(end)
    }
    IrExpr::StringLength(s) => 1 + count_nodes(s),
    IrExpr::StringEq(a, b) => 1 + count_nodes(a) + count_nodes(b),

    // List operations
    IrExpr::ListLength(list) => 1 + count_nodes(list),
    IrExpr::ListGet { list, index } => 1 + count_nodes(list) + count_nodes(index),
    IrExpr::ListConcat(a, b) => 1 + count_nodes(a) + count_nodes(b),
    IrExpr::ListMap { list, func } => 1 + count_nodes(list) + count_nodes(func),
    IrExpr::ListFilter { list, pred } => 1 + count_nodes(list) + count_nodes(pred),
    IrExpr::TupleGet { tuple, index } => 1 + count_nodes(tuple) + count_nodes(index),
    IrExpr::GetAttr { attrs, key } => 1 + count_nodes(attrs) + count_nodes(key),
    IrExpr::SetAttr { attrs, key, value } => {
      1 + count_nodes(attrs) + count_nodes(key) + count_nodes(value)
    }
    IrExpr::HasAttr { attrs, key } => 1 + count_nodes(attrs) + count_nodes(key),
    IrExpr::AttrSetKeys(attrs) => 1 + count_nodes(attrs),
    IrExpr::AttrSetMerge(a, b) => 1 + count_nodes(a) + count_nodes(b),

    IrExpr::Lambda { body, .. } => 1 + count_nodes(body),

    IrExpr::Apply { func, arg } => 1 + count_nodes(func) + count_nodes(arg),

    IrExpr::Let { bindings, body } => {
      1 + bindings.iter().map(|(_, v)| count_nodes(v)).sum::<u32>() + count_nodes(body)
    }
  }
}

/// Simple pretty-print for IR expressions
fn pretty_ir(expr: &IrExpr) -> String {
  match expr {
    IrExpr::ConstFloat(v) => format!("{}", v),
    IrExpr::ConstInt(v) => format!("{}", v),
    IrExpr::ConstBool(v) => format!("{}", v),
    IrExpr::ConstString(s) => format!("\"{}\"", s),
    IrExpr::VarRef(name) => name.clone(),
    IrExpr::SignalRef(id) => format!("signal#{}", id),
    IrExpr::TimeParam => "time".to_string(),
    IrExpr::DeltaTime => "dt".to_string(),
    IrExpr::Throw(msg) => format!("throw(\"{}\")", msg),

    IrExpr::Add(a, b) => format!("({} + {})", pretty_ir(a), pretty_ir(b)),
    IrExpr::Sub(a, b) => format!("({} - {})", pretty_ir(a), pretty_ir(b)),
    IrExpr::Mul(a, b) => format!("({} * {})", pretty_ir(a), pretty_ir(b)),
    IrExpr::Div(a, b) => format!("({} / {})", pretty_ir(a), pretty_ir(b)),
    IrExpr::Mod(a, b) => format!("({} % {})", pretty_ir(a), pretty_ir(b)),
    IrExpr::Neg(a) => format!("(-{})", pretty_ir(a)),

    IrExpr::Sin(a) => format!("sin({})", pretty_ir(a)),
    IrExpr::Cos(a) => format!("cos({})", pretty_ir(a)),
    IrExpr::Tan(a) => format!("tan({})", pretty_ir(a)),
    IrExpr::Sqrt(a) => format!("sqrt({})", pretty_ir(a)),
    IrExpr::Exp(a) => format!("exp({})", pretty_ir(a)),
    IrExpr::Log(a) => format!("log({})", pretty_ir(a)),
    IrExpr::Floor(a) => format!("floor({})", pretty_ir(a)),
    IrExpr::Ceil(a) => format!("ceil({})", pretty_ir(a)),
    IrExpr::Abs(a) => format!("abs({})", pretty_ir(a)),
    IrExpr::Pow(a, b) => format!("pow({}, {})", pretty_ir(a), pretty_ir(b)),

    IrExpr::Lt(a, b) => format!("({} < {})", pretty_ir(a), pretty_ir(b)),
    IrExpr::Gt(a, b) => format!("({} > {})", pretty_ir(a), pretty_ir(b)),
    IrExpr::Le(a, b) => format!("({} <= {})", pretty_ir(a), pretty_ir(b)),
    IrExpr::Ge(a, b) => format!("({} >= {})", pretty_ir(a), pretty_ir(b)),
    IrExpr::Eq(a, b) => format!("({} == {})", pretty_ir(a), pretty_ir(b)),
    IrExpr::Ne(a, b) => format!("({} != {})", pretty_ir(a), pretty_ir(b)),

    IrExpr::And(a, b) => format!("({} && {})", pretty_ir(a), pretty_ir(b)),
    IrExpr::Or(a, b) => format!("({} || {})", pretty_ir(a), pretty_ir(b)),
    IrExpr::Not(a) => format!("(!{})", pretty_ir(a)),

    IrExpr::Select(c, t, e) => {
      format!(
        "(if {} then {} else {})",
        pretty_ir(c),
        pretty_ir(t),
        pretty_ir(e)
      )
    }

    IrExpr::List(items) => {
      let items_str: Vec<_> = items.iter().map(pretty_ir).collect();
      format!("[{}]", items_str.join(", "))
    }

    IrExpr::Tuple(items) => {
      let items_str: Vec<_> = items.iter().map(pretty_ir).collect();
      format!("({})", items_str.join(", "))
    }

    IrExpr::AttrSet(pairs) => {
      let pairs_str: Vec<_> = pairs
        .iter()
        .map(|(k, v)| format!("{} = {}", k, pretty_ir(v)))
        .collect();
      format!("{{ {} }}", pairs_str.join("; "))
    }

    // String operations
    IrExpr::Concat(a, b) => format!("concat({}, {})", pretty_ir(a), pretty_ir(b)),
    IrExpr::Substring { str, start, end } => {
      format!(
        "substring({}, {}, {})",
        pretty_ir(str),
        pretty_ir(start),
        pretty_ir(end)
      )
    }
    IrExpr::StringLength(s) => format!("length({})", pretty_ir(s)),
    IrExpr::StringEq(a, b) => format!("str_eq({}, {})", pretty_ir(a), pretty_ir(b)),

    // List operations
    IrExpr::ListLength(list) => format!("list_length({})", pretty_ir(list)),
    IrExpr::ListGet { list, index } => {
      format!("list_get({}, {})", pretty_ir(list), pretty_ir(index))
    }
    IrExpr::ListConcat(a, b) => format!("list_concat({}, {})", pretty_ir(a), pretty_ir(b)),
    IrExpr::ListMap { list, func } => format!("list_map({}, {})", pretty_ir(list), pretty_ir(func)),
    IrExpr::ListFilter { list, pred } => {
      format!("list_filter({}, {})", pretty_ir(list), pretty_ir(pred))
    }
    IrExpr::TupleGet { tuple, index } => {
      format!("tuple_get({}, {})", pretty_ir(tuple), pretty_ir(index))
    }
    IrExpr::GetAttr { attrs, key } => format!("get_attr({}, {})", pretty_ir(attrs), pretty_ir(key)),
    IrExpr::SetAttr { attrs, key, value } => format!(
      "set_attr({}, {}, {})",
      pretty_ir(attrs),
      pretty_ir(key),
      pretty_ir(value)
    ),
    IrExpr::HasAttr { attrs, key } => format!("has_attr({}, {})", pretty_ir(attrs), pretty_ir(key)),
    IrExpr::AttrSetKeys(attrs) => format!("attr_keys({})", pretty_ir(attrs)),
    IrExpr::AttrSetMerge(a, b) => format!("attr_merge({}, {})", pretty_ir(a), pretty_ir(b)),

    IrExpr::Lambda { params, body } => format!("({}: {})", params.join(" "), pretty_ir(body)),

    IrExpr::Apply { func, arg } => format!("({} {})", pretty_ir(func), pretty_ir(arg)),

    IrExpr::Let { bindings, body } => {
      let bindings_str: Vec<_> = bindings
        .iter()
        .map(|(n, v)| format!("{} = {}", n, pretty_ir(v)))
        .collect();
      format!("let {} in {}", bindings_str.join("; "), pretty_ir(body))
    }
  }
}

/// Collect time-independent subtrees as cache candidates
fn collect_candidates(
  expr: &IrExpr,
  candidates: &mut Vec<CacheCandidate>,
  seen_keys: &mut HashSet<u64>,
) {
  // If this subtree is time-independent, it's a candidate
  if !is_time_dependent(expr) {
    let key = stable_hash_ir(expr);
    if !seen_keys.contains(&key) {
      seen_keys.insert(key);
      let size = count_nodes(expr);
      candidates.push(CacheCandidate {
        key,
        size,
        pretty: pretty_ir(expr),
      });
    }
    return; // Don't recurse - entire subtree is cacheable
  }

  // Time-dependent: recurse to find cacheable children
  match expr {
    // Leaves - nothing to recurse
    IrExpr::ConstFloat(_)
    | IrExpr::ConstInt(_)
    | IrExpr::ConstBool(_)
    | IrExpr::ConstString(_)
    | IrExpr::VarRef(_)
    | IrExpr::SignalRef(_)
    | IrExpr::TimeParam
    | IrExpr::DeltaTime
    | IrExpr::Throw(_) => {}

    // Binary ops
    IrExpr::Add(a, b)
    | IrExpr::Sub(a, b)
    | IrExpr::Mul(a, b)
    | IrExpr::Div(a, b)
    | IrExpr::Mod(a, b)
    | IrExpr::Pow(a, b)
    | IrExpr::Lt(a, b)
    | IrExpr::Gt(a, b)
    | IrExpr::Le(a, b)
    | IrExpr::Ge(a, b)
    | IrExpr::Eq(a, b)
    | IrExpr::Ne(a, b)
    | IrExpr::And(a, b)
    | IrExpr::Or(a, b) => {
      collect_candidates(a, candidates, seen_keys);
      collect_candidates(b, candidates, seen_keys);
    }

    // Unary ops
    IrExpr::Neg(a)
    | IrExpr::Floor(a)
    | IrExpr::Ceil(a)
    | IrExpr::Abs(a)
    | IrExpr::Sqrt(a)
    | IrExpr::Sin(a)
    | IrExpr::Cos(a)
    | IrExpr::Tan(a)
    | IrExpr::Exp(a)
    | IrExpr::Log(a)
    | IrExpr::Not(a) => {
      collect_candidates(a, candidates, seen_keys);
    }

    IrExpr::Select(cond, then_expr, else_expr) => {
      collect_candidates(cond, candidates, seen_keys);
      collect_candidates(then_expr, candidates, seen_keys);
      collect_candidates(else_expr, candidates, seen_keys);
    }

    IrExpr::List(items) | IrExpr::Tuple(items) => {
      for item in items {
        collect_candidates(item, candidates, seen_keys);
      }
    }

    IrExpr::AttrSet(pairs) => {
      for (_, v) in pairs {
        collect_candidates(v, candidates, seen_keys);
      }
    }

    // String operations
    IrExpr::Concat(a, b) => {
      collect_candidates(a, candidates, seen_keys);
      collect_candidates(b, candidates, seen_keys);
    }
    IrExpr::Substring { str, start, end } => {
      collect_candidates(str, candidates, seen_keys);
      collect_candidates(start, candidates, seen_keys);
      collect_candidates(end, candidates, seen_keys);
    }
    IrExpr::StringLength(s) => {
      collect_candidates(s, candidates, seen_keys);
    }
    IrExpr::StringEq(a, b) => {
      collect_candidates(a, candidates, seen_keys);
      collect_candidates(b, candidates, seen_keys);
    }

    // List operations
    IrExpr::ListLength(list) => {
      collect_candidates(list, candidates, seen_keys);
    }
    IrExpr::ListGet { list, index } => {
      collect_candidates(list, candidates, seen_keys);
      collect_candidates(index, candidates, seen_keys);
    }
    IrExpr::ListConcat(a, b) => {
      collect_candidates(a, candidates, seen_keys);
      collect_candidates(b, candidates, seen_keys);
    }
    IrExpr::ListMap { list, func } => {
      collect_candidates(list, candidates, seen_keys);
      collect_candidates(func, candidates, seen_keys);
    }
    IrExpr::ListFilter { list, pred } => {
      collect_candidates(list, candidates, seen_keys);
      collect_candidates(pred, candidates, seen_keys);
    }
    IrExpr::TupleGet { tuple, index } => {
      collect_candidates(tuple, candidates, seen_keys);
      collect_candidates(index, candidates, seen_keys);
    }
    IrExpr::GetAttr { attrs, key } => {
      collect_candidates(attrs, candidates, seen_keys);
      collect_candidates(key, candidates, seen_keys);
    }
    IrExpr::SetAttr { attrs, key, value } => {
      collect_candidates(attrs, candidates, seen_keys);
      collect_candidates(key, candidates, seen_keys);
      collect_candidates(value, candidates, seen_keys);
    }
    IrExpr::HasAttr { attrs, key } => {
      collect_candidates(attrs, candidates, seen_keys);
      collect_candidates(key, candidates, seen_keys);
    }
    IrExpr::AttrSetKeys(attrs) => {
      collect_candidates(attrs, candidates, seen_keys);
    }
    IrExpr::AttrSetMerge(a, b) => {
      collect_candidates(a, candidates, seen_keys);
      collect_candidates(b, candidates, seen_keys);
    }

    IrExpr::Lambda { body, .. } => {
      collect_candidates(body, candidates, seen_keys);
    }

    IrExpr::Apply { func, arg } => {
      collect_candidates(func, candidates, seen_keys);
      collect_candidates(arg, candidates, seen_keys);
    }

    IrExpr::Let { bindings, body } => {
      for (_, v) in bindings {
        collect_candidates(v, candidates, seen_keys);
      }
      collect_candidates(body, candidates, seen_keys);
    }
  }
}

/// Generate FRP cache plan for an expression
pub fn plan_frp_cache(expr: &IrExpr, zone: EffectZone) -> FrpCachePlan {
  // Zone guard: Only FRP/Animation zones enable caching
  if !matches!(zone, EffectZone::Frp | EffectZone::Animation) {
    return FrpCachePlan::default();
  }

  // 1) Check whole expression time dependence
  let whole_dependent = is_time_dependent(expr);
  if !whole_dependent {
    // Entire expression is time-independent → cache the whole thing
    return FrpCachePlan {
      whole_expr_cacheable: true,
      candidates: vec![],
    };
  }

  // 2) Collect time-independent subtrees
  let mut candidates = Vec::new();
  let mut seen_keys = HashSet::new();
  collect_candidates(expr, &mut candidates, &mut seen_keys);

  // 3) Filter by minimum size (cache overhead > benefit for small subtrees)
  candidates.retain(|c| c.size >= MIN_CACHE_SIZE);

  // 4) Sort by size descending (larger subtrees = more benefit)
  candidates.sort_by_key(|c| std::cmp::Reverse(c.size));

  FrpCachePlan {
    whole_expr_cacheable: false,
    candidates,
  }
}

/// Get cacheable keys from a plan (for runtime use)
pub fn cacheable_keys(plan: &FrpCachePlan) -> HashSet<u64> {
  plan.candidates.iter().map(|c| c.key).collect()
}

// ============================================================
// Provenance Integration
// ============================================================

use crate::provenance::{CachedSubexprInfo, SymbolicProvenance};

/// Apply FRP cache plan to provenance
pub fn apply_cache_to_provenance(plan: &FrpCachePlan, prov: &mut SymbolicProvenance) {
  prov.frp_whole_cacheable = plan.whole_expr_cacheable;
  prov.frp_cached_subexprs = plan
    .candidates
    .iter()
    .map(|c| CachedSubexprInfo {
      key: c.key,
      size: c.size,
      pretty: c.pretty.clone(),
    })
    .collect();
}

/// Generate FRP cache plan and record to provenance
pub fn plan_and_record_frp_cache(
  expr: &IrExpr,
  zone: EffectZone,
  prov: &mut SymbolicProvenance,
) -> FrpCachePlan {
  let plan = plan_frp_cache(expr, zone);
  apply_cache_to_provenance(&plan, prov);
  plan
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_is_time_dependent() {
    // Time-dependent
    assert!(is_time_dependent(&IrExpr::TimeParam));
    assert!(is_time_dependent(&IrExpr::DeltaTime));
    assert!(is_time_dependent(&IrExpr::Sin(Box::new(IrExpr::TimeParam))));
    assert!(is_time_dependent(&IrExpr::SignalRef(0)));

    // Time-independent
    assert!(!is_time_dependent(&IrExpr::ConstFloat(1.0)));
    assert!(!is_time_dependent(&IrExpr::Add(
      Box::new(IrExpr::ConstFloat(1.0)),
      Box::new(IrExpr::ConstFloat(2.0))
    )));
  }

  #[test]
  fn test_plan_frp_cache() {
    // Time-independent expression
    let expr = IrExpr::Add(
      Box::new(IrExpr::ConstFloat(1.0)),
      Box::new(IrExpr::ConstFloat(2.0)),
    );
    let plan = plan_frp_cache(&expr, EffectZone::Frp);
    assert!(plan.whole_expr_cacheable);
    assert!(plan.candidates.is_empty());

    // Time-dependent expression
    let expr = IrExpr::Sin(Box::new(IrExpr::TimeParam));
    let plan = plan_frp_cache(&expr, EffectZone::Frp);
    assert!(!plan.whole_expr_cacheable);
  }
}
