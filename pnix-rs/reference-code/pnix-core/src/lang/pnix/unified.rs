//! UnifiedExpr - 언어 독립적 중간 표현
//!
//! pnix-old의 lang_pnix/unified.rs에서 마이그레이션.
//!
//! Nix/Clojure/Python 등 다양한 프론트엔드에서 공통으로 사용하는 AST
//! FxCoreExpr로 lowering 되기 전 단계
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 값 계산 없음

use serde::{Deserialize, Serialize};

use super::error::PnixError;
use crate::fx::meaning_op::MeaningOpId;

/// 언어 독립적 중간 표현: 다양한 언어에서 공통으로 사용하는 AST 표현
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnifiedExpr {
  // Literals
  /// 정수 리터럴
  Int(
    /// 정수 값
    i64,
  ),
  /// 부동소수점 리터럴
  Float(
    /// 실수 값
    f64,
  ),
  /// 불리언 리터럴
  Bool(
    /// 불리언 값
    bool,
  ),
  /// 문자열 리터럴
  String(
    /// 문자열 값
    String,
  ),

  // Variables & References
  /// 변수 참조
  Var(
    /// 변수 이름
    String,
  ),

  // Param (FRP parameters)
  /// 시간 파라미터: param.system_time or param/system-time
  ParamTime,
  /// 델타 시간 파라미터: param.delta_time
  ParamDeltaTime,
  /// 시그널 파라미터: param.signal_name
  ParamSignal(
    /// 시그널 이름
    String,
  ),
  /// 런타임 전용 시그널 참조: Realtime 컨텍스트에서 해결됨
  SignalVar(
    /// 시그널 이름
    String,
  ),

  // Arithmetic
  /// 덧셈 (+)
  Add(Box<UnifiedExpr>, Box<UnifiedExpr>),
  /// 뺄셈 (-)
  Sub(Box<UnifiedExpr>, Box<UnifiedExpr>),
  /// 곱셈 (*)
  Mul(Box<UnifiedExpr>, Box<UnifiedExpr>),
  /// 나눗셈 (/)
  Div(Box<UnifiedExpr>, Box<UnifiedExpr>),
  /// 나머지 (%)
  Mod(Box<UnifiedExpr>, Box<UnifiedExpr>),
  /// 부정 (-)
  Neg(Box<UnifiedExpr>),

  // String operations
  // Y-CLAUDE-6: ++ 연산자로 명시적 문자열 연결 지원
  Concat(Box<UnifiedExpr>, Box<UnifiedExpr>),

  // Math functions
  /// 내림 (floor)
  Floor(Box<UnifiedExpr>),
  /// 올림 (ceil)
  Ceil(Box<UnifiedExpr>),
  /// 절댓값 (abs)
  Abs(Box<UnifiedExpr>),
  /// 제곱근 (sqrt)
  Sqrt(Box<UnifiedExpr>),
  /// 사인 (sin)
  Sin(Box<UnifiedExpr>),
  /// 코사인 (cos)
  Cos(Box<UnifiedExpr>),
  /// 탄젠트 (tan)
  Tan(Box<UnifiedExpr>),
  /// 지수 (exp)
  Exp(Box<UnifiedExpr>),
  /// 자연 로그 (ln)
  Ln(Box<UnifiedExpr>),
  /// 거듭제곱 (pow)
  Pow(Box<UnifiedExpr>, Box<UnifiedExpr>),

  // Comparison
  /// 작음 (<)
  Lt(Box<UnifiedExpr>, Box<UnifiedExpr>),
  /// 큼 (>)
  Gt(Box<UnifiedExpr>, Box<UnifiedExpr>),
  /// 작거나 같음 (<=)
  Le(Box<UnifiedExpr>, Box<UnifiedExpr>),
  /// 크거나 같음 (>=)
  Ge(Box<UnifiedExpr>, Box<UnifiedExpr>),
  /// 같음 (==)
  Eq(Box<UnifiedExpr>, Box<UnifiedExpr>),
  /// 다름 (!=)
  Ne(Box<UnifiedExpr>, Box<UnifiedExpr>),

  // Logic
  /// 논리곱 (&&)
  And(Box<UnifiedExpr>, Box<UnifiedExpr>),
  /// 논리합 (||)
  Or(Box<UnifiedExpr>, Box<UnifiedExpr>),
  /// 논리부정 (!)
  Not(Box<UnifiedExpr>),

  // Control flow
  /// 조건문 (if-then-else)
  If {
    /// 조건 표현식
    cond: Box<UnifiedExpr>,
    /// 참일 때 표현식
    then_: Box<UnifiedExpr>,
    /// 거짓일 때 표현식
    else_: Box<UnifiedExpr>,
  },
  /// Y08b-2: 런타임 에러 발생 (non-exhaustive match 등)
  Throw(
    /// 에러 메시지
    String,
  ),

  // Let bindings
  /// Let 바인딩
  Let {
    /// 바인딩 이름
    name: String,
    /// 값 표현식
    value: Box<UnifiedExpr>,
    /// 본문 표현식
    body: Box<UnifiedExpr>,
  },

  // Function application
  /// 함수 호출
  Apply {
    /// 함수 이름
    func: String,
    /// 인자 표현식 목록
    args: Vec<UnifiedExpr>,
  },

  // FX block (시그널 정의)
  /// FX 블록 (시그널 정의)
  Fx(
    /// 내부 표현식
    Box<UnifiedExpr>,
  ),

  // Interop (다른 언어 임베딩)
  /// 언어 간 호출 (다른 언어 임베딩)
  Interop {
    /// 언어 이름 ("nix", "clj", "py" 등)
    lang: String,
    /// 원본 코드
    code: String,
  },

  // Derived (pattern-lifted ops)
  /// 파생 연산 (패턴 리프팅된 고수준 연산)
  Derived {
    /// 연산 ID
    op: MeaningOpId,
    /// 인자 표현식 목록
    args: Vec<UnifiedExpr>,
  },

  // Attr set (draw commands 등)
  /// AttrSet (속성 집합)
  AttrSet(
    /// 키-값 쌍 목록
    Vec<(String, UnifiedExpr)>,
  ),

  /// Y10c: AttrSet 병합 연산자 (a // b)
  /// 오른쪽 값이 우선순위를 가짐
  Merge(Box<UnifiedExpr>, Box<UnifiedExpr>),

  // List
  /// 리스트
  List(
    /// 요소 표현식 목록
    Vec<UnifiedExpr>,
  ),

  // Lambda (function definition)
  /// 람다 함수 정의
  Lambda {
    /// 파라미터 이름
    param: String,
    /// 본문 표현식
    body: Box<UnifiedExpr>,
  },

  // Null/Nil
  /// Null/Nil 값
  Null,

  /// ADT 값 생성자: Some(42), None, Ok(x), Err("msg")
  Construct {
    /// Variant 이름 (예: "Some", "None", "Ok", "Err")
    variant: String,
    /// 인자 목록 (None 같은 nullary 생성자는 빈 목록)
    args: Vec<UnifiedExpr>,
  },
}

impl UnifiedExpr {
  // Constructors
  pub fn int(v: i64) -> Self {
    UnifiedExpr::Int(v)
  }

  pub fn float(v: f64) -> Self {
    UnifiedExpr::Float(v)
  }

  pub fn bool(v: bool) -> Self {
    UnifiedExpr::Bool(v)
  }

  pub fn var(name: impl Into<String>) -> Self {
    UnifiedExpr::Var(name.into())
  }

  pub fn time() -> Self {
    UnifiedExpr::ParamTime
  }

  #[allow(clippy::should_implement_trait)]
  pub fn add(lhs: UnifiedExpr, rhs: UnifiedExpr) -> Self {
    UnifiedExpr::Add(Box::new(lhs), Box::new(rhs))
  }

  #[allow(clippy::should_implement_trait)]
  pub fn sub(lhs: UnifiedExpr, rhs: UnifiedExpr) -> Self {
    UnifiedExpr::Sub(Box::new(lhs), Box::new(rhs))
  }

  #[allow(clippy::should_implement_trait)]
  pub fn mul(lhs: UnifiedExpr, rhs: UnifiedExpr) -> Self {
    UnifiedExpr::Mul(Box::new(lhs), Box::new(rhs))
  }

  #[allow(clippy::should_implement_trait)]
  pub fn div(lhs: UnifiedExpr, rhs: UnifiedExpr) -> Self {
    UnifiedExpr::Div(Box::new(lhs), Box::new(rhs))
  }

  pub fn modulo(lhs: UnifiedExpr, rhs: UnifiedExpr) -> Self {
    UnifiedExpr::Mod(Box::new(lhs), Box::new(rhs))
  }

  pub fn floor(arg: UnifiedExpr) -> Self {
    UnifiedExpr::Floor(Box::new(arg))
  }

  pub fn sin(arg: UnifiedExpr) -> Self {
    UnifiedExpr::Sin(Box::new(arg))
  }

  pub fn cos(arg: UnifiedExpr) -> Self {
    UnifiedExpr::Cos(Box::new(arg))
  }

  pub fn exp(arg: UnifiedExpr) -> Self {
    UnifiedExpr::Exp(Box::new(arg))
  }

  pub fn ln(arg: UnifiedExpr) -> Self {
    UnifiedExpr::Ln(Box::new(arg))
  }

  pub fn pow(base: UnifiedExpr, exponent: UnifiedExpr) -> Self {
    UnifiedExpr::Pow(Box::new(base), Box::new(exponent))
  }

  pub fn if_then_else(cond: UnifiedExpr, then_: UnifiedExpr, else_: UnifiedExpr) -> Self {
    UnifiedExpr::If {
      cond: Box::new(cond),
      then_: Box::new(then_),
      else_: Box::new(else_),
    }
  }

  pub fn let_in(name: impl Into<String>, value: UnifiedExpr, body: UnifiedExpr) -> Self {
    UnifiedExpr::Let {
      name: name.into(),
      value: Box::new(value),
      body: Box::new(body),
    }
  }

  pub fn fx(body: UnifiedExpr) -> Self {
    UnifiedExpr::Fx(Box::new(body))
  }

  /// Create an ADT value constructor expression
  pub fn construct(variant: impl Into<String>, args: Vec<UnifiedExpr>) -> Self {
    UnifiedExpr::Construct {
      variant: variant.into(),
      args,
    }
  }
}

/// 실행 모드: 표현식의 실행 모드
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
  /// 순수 모드 (시간 독립적)
  Pure,
  /// 실시간 모드 (시간 의존적)
  Realtime,
}

/// 시그널 해결: UnifiedExpr의 시그널 참조를 해결
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn resolve_signals(
  expr: &UnifiedExpr,
  mode: ExecutionMode,
  allowlist: &[&str],
) -> Result<UnifiedExpr, PnixError> {
  match expr {
    UnifiedExpr::SignalVar(name) => match mode {
      ExecutionMode::Pure => Err(PnixError::lowering(format!(
        "SignalVar '{}' is not allowed in Pure context",
        name
      ))),
      ExecutionMode::Realtime => Ok(UnifiedExpr::SignalVar(name.clone())),
    },
    UnifiedExpr::ParamSignal(name) => match mode {
      ExecutionMode::Pure => Err(PnixError::lowering(format!(
        "ParamSignal '{}' is not allowed in Pure context",
        name
      ))),
      ExecutionMode::Realtime => {
        if is_allowed_signal(name, allowlist) {
          Ok(UnifiedExpr::SignalVar(name.clone()))
        } else {
          Err(PnixError::lowering(format!(
            "ParamSignal '{}' is not in allowlist",
            name
          )))
        }
      }
    },
    UnifiedExpr::Int(v) => Ok(UnifiedExpr::Int(*v)),
    UnifiedExpr::Float(v) => Ok(UnifiedExpr::Float(*v)),
    UnifiedExpr::Bool(v) => Ok(UnifiedExpr::Bool(*v)),
    UnifiedExpr::String(v) => Ok(UnifiedExpr::String(v.clone())),
    UnifiedExpr::Var(name) => Ok(UnifiedExpr::Var(name.clone())),
    UnifiedExpr::ParamTime => Ok(UnifiedExpr::ParamTime),
    UnifiedExpr::ParamDeltaTime => Ok(UnifiedExpr::ParamDeltaTime),

    UnifiedExpr::Add(lhs, rhs) => Ok(UnifiedExpr::Add(
      Box::new(resolve_signals(lhs, mode, allowlist)?),
      Box::new(resolve_signals(rhs, mode, allowlist)?),
    )),
    // Y-CLAUDE-6: ++ 연산자로 명시적 문자열 연결
    UnifiedExpr::Concat(lhs, rhs) => Ok(UnifiedExpr::Concat(
      Box::new(resolve_signals(lhs, mode, allowlist)?),
      Box::new(resolve_signals(rhs, mode, allowlist)?),
    )),
    // Y10c: // 병합 연산자: a // b → AttrSet 병합
    UnifiedExpr::Merge(lhs, rhs) => Ok(UnifiedExpr::Merge(
      Box::new(resolve_signals(lhs, mode, allowlist)?),
      Box::new(resolve_signals(rhs, mode, allowlist)?),
    )),
    UnifiedExpr::Sub(lhs, rhs) => Ok(UnifiedExpr::Sub(
      Box::new(resolve_signals(lhs, mode, allowlist)?),
      Box::new(resolve_signals(rhs, mode, allowlist)?),
    )),
    UnifiedExpr::Mul(lhs, rhs) => Ok(UnifiedExpr::Mul(
      Box::new(resolve_signals(lhs, mode, allowlist)?),
      Box::new(resolve_signals(rhs, mode, allowlist)?),
    )),
    UnifiedExpr::Div(lhs, rhs) => Ok(UnifiedExpr::Div(
      Box::new(resolve_signals(lhs, mode, allowlist)?),
      Box::new(resolve_signals(rhs, mode, allowlist)?),
    )),
    UnifiedExpr::Mod(lhs, rhs) => Ok(UnifiedExpr::Mod(
      Box::new(resolve_signals(lhs, mode, allowlist)?),
      Box::new(resolve_signals(rhs, mode, allowlist)?),
    )),
    UnifiedExpr::Neg(arg) => Ok(UnifiedExpr::Neg(Box::new(resolve_signals(
      arg, mode, allowlist,
    )?))),

    UnifiedExpr::Floor(arg) => Ok(UnifiedExpr::Floor(Box::new(resolve_signals(
      arg, mode, allowlist,
    )?))),
    UnifiedExpr::Ceil(arg) => Ok(UnifiedExpr::Ceil(Box::new(resolve_signals(
      arg, mode, allowlist,
    )?))),
    UnifiedExpr::Abs(arg) => Ok(UnifiedExpr::Abs(Box::new(resolve_signals(
      arg, mode, allowlist,
    )?))),
    UnifiedExpr::Sqrt(arg) => Ok(UnifiedExpr::Sqrt(Box::new(resolve_signals(
      arg, mode, allowlist,
    )?))),
    UnifiedExpr::Sin(arg) => Ok(UnifiedExpr::Sin(Box::new(resolve_signals(
      arg, mode, allowlist,
    )?))),
    UnifiedExpr::Cos(arg) => Ok(UnifiedExpr::Cos(Box::new(resolve_signals(
      arg, mode, allowlist,
    )?))),
    UnifiedExpr::Tan(arg) => Ok(UnifiedExpr::Tan(Box::new(resolve_signals(
      arg, mode, allowlist,
    )?))),
    UnifiedExpr::Exp(arg) => Ok(UnifiedExpr::Exp(Box::new(resolve_signals(
      arg, mode, allowlist,
    )?))),
    UnifiedExpr::Ln(arg) => Ok(UnifiedExpr::Ln(Box::new(resolve_signals(
      arg, mode, allowlist,
    )?))),
    UnifiedExpr::Pow(base, exponent) => Ok(UnifiedExpr::Pow(
      Box::new(resolve_signals(base, mode, allowlist)?),
      Box::new(resolve_signals(exponent, mode, allowlist)?),
    )),

    UnifiedExpr::Lt(lhs, rhs) => Ok(UnifiedExpr::Lt(
      Box::new(resolve_signals(lhs, mode, allowlist)?),
      Box::new(resolve_signals(rhs, mode, allowlist)?),
    )),
    UnifiedExpr::Gt(lhs, rhs) => Ok(UnifiedExpr::Gt(
      Box::new(resolve_signals(lhs, mode, allowlist)?),
      Box::new(resolve_signals(rhs, mode, allowlist)?),
    )),
    UnifiedExpr::Le(lhs, rhs) => Ok(UnifiedExpr::Le(
      Box::new(resolve_signals(lhs, mode, allowlist)?),
      Box::new(resolve_signals(rhs, mode, allowlist)?),
    )),
    UnifiedExpr::Ge(lhs, rhs) => Ok(UnifiedExpr::Ge(
      Box::new(resolve_signals(lhs, mode, allowlist)?),
      Box::new(resolve_signals(rhs, mode, allowlist)?),
    )),
    UnifiedExpr::Eq(lhs, rhs) => Ok(UnifiedExpr::Eq(
      Box::new(resolve_signals(lhs, mode, allowlist)?),
      Box::new(resolve_signals(rhs, mode, allowlist)?),
    )),
    UnifiedExpr::Ne(lhs, rhs) => Ok(UnifiedExpr::Ne(
      Box::new(resolve_signals(lhs, mode, allowlist)?),
      Box::new(resolve_signals(rhs, mode, allowlist)?),
    )),

    UnifiedExpr::And(lhs, rhs) => Ok(UnifiedExpr::And(
      Box::new(resolve_signals(lhs, mode, allowlist)?),
      Box::new(resolve_signals(rhs, mode, allowlist)?),
    )),
    UnifiedExpr::Or(lhs, rhs) => Ok(UnifiedExpr::Or(
      Box::new(resolve_signals(lhs, mode, allowlist)?),
      Box::new(resolve_signals(rhs, mode, allowlist)?),
    )),
    UnifiedExpr::Not(arg) => Ok(UnifiedExpr::Not(Box::new(resolve_signals(
      arg, mode, allowlist,
    )?))),

    UnifiedExpr::If { cond, then_, else_ } => Ok(UnifiedExpr::If {
      cond: Box::new(resolve_signals(cond, mode, allowlist)?),
      then_: Box::new(resolve_signals(then_, mode, allowlist)?),
      else_: Box::new(resolve_signals(else_, mode, allowlist)?),
    }),

    UnifiedExpr::Let { name, value, body } => Ok(UnifiedExpr::Let {
      name: name.clone(),
      value: Box::new(resolve_signals(value, mode, allowlist)?),
      body: Box::new(resolve_signals(body, mode, allowlist)?),
    }),

    UnifiedExpr::Apply { func, args } => {
      let mut lowered = Vec::with_capacity(args.len());
      for arg in args {
        lowered.push(resolve_signals(arg, mode, allowlist)?);
      }
      Ok(UnifiedExpr::Apply {
        func: func.clone(),
        args: lowered,
      })
    }

    UnifiedExpr::Fx(body) => Ok(UnifiedExpr::Fx(Box::new(resolve_signals(
      body, mode, allowlist,
    )?))),

    UnifiedExpr::Interop { lang, code } => Ok(UnifiedExpr::Interop {
      lang: lang.clone(),
      code: code.clone(),
    }),
    UnifiedExpr::Derived { op, args } => {
      let mut lowered = Vec::with_capacity(args.len());
      for arg in args {
        lowered.push(resolve_signals(arg, mode, allowlist)?);
      }
      Ok(UnifiedExpr::Derived {
        op: *op,
        args: lowered,
      })
    }
    // Y08b-2: Throw는 그대로 전달 (에러 메시지는 변경하지 않음)
    UnifiedExpr::Throw(msg) => Ok(UnifiedExpr::Throw(msg.clone())),

    UnifiedExpr::AttrSet(pairs) => {
      let mut lowered = Vec::with_capacity(pairs.len());
      for (k, v) in pairs {
        lowered.push((k.clone(), resolve_signals(v, mode, allowlist)?));
      }
      Ok(UnifiedExpr::AttrSet(lowered))
    }

    UnifiedExpr::List(items) => {
      let mut lowered = Vec::with_capacity(items.len());
      for item in items {
        lowered.push(resolve_signals(item, mode, allowlist)?);
      }
      Ok(UnifiedExpr::List(lowered))
    }

    UnifiedExpr::Lambda { param, body } => Ok(UnifiedExpr::Lambda {
      param: param.clone(),
      body: Box::new(resolve_signals(body, mode, allowlist)?),
    }),

    UnifiedExpr::Null => Ok(UnifiedExpr::Null),

    UnifiedExpr::Construct { variant, args } => {
      let mut lowered = Vec::with_capacity(args.len());
      for arg in args {
        lowered.push(resolve_signals(arg, mode, allowlist)?);
      }
      Ok(UnifiedExpr::Construct {
        variant: variant.clone(),
        args: lowered,
      })
    }
  }
}

fn is_allowed_signal(name: &str, allowlist: &[&str]) -> bool {
  allowlist.contains(&name)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_unified_expr_construction() {
    let expr = UnifiedExpr::add(
      UnifiedExpr::int(1),
      UnifiedExpr::mul(UnifiedExpr::int(2), UnifiedExpr::int(3)),
    );

    match expr {
      UnifiedExpr::Add(_, _) => {}
      _ => panic!("Expected Add"),
    }
  }

  #[test]
  fn test_unified_fx_block() {
    let expr = UnifiedExpr::fx(UnifiedExpr::modulo(
      UnifiedExpr::floor(UnifiedExpr::time()),
      UnifiedExpr::int(60),
    ));

    match expr {
      UnifiedExpr::Fx(_) => {}
      _ => panic!("Expected Fx"),
    }
  }

  #[test]
  fn test_resolve_signals_realtime() {
    let expr = UnifiedExpr::ParamSignal("time".to_string());
    let resolved = resolve_signals(&expr, ExecutionMode::Realtime, &["time"]).unwrap();
    assert!(matches!(
      resolved,
      UnifiedExpr::SignalVar(name) if name == "time"
    ));
  }

  #[test]
  fn test_resolve_signals_pure_rejects() {
    let expr = UnifiedExpr::ParamSignal("time".to_string());
    let err = resolve_signals(&expr, ExecutionMode::Pure, &["time"]).unwrap_err();
    assert!(matches!(err, PnixError::Lowering { message: msg, .. } if msg.contains("Pure")));
  }

  #[test]
  fn test_resolve_signals_pure_rejects_signalvar() {
    let expr = UnifiedExpr::SignalVar("time".to_string());
    let err = resolve_signals(&expr, ExecutionMode::Pure, &["time"]).unwrap_err();
    assert!(matches!(err, PnixError::Lowering { message: msg, .. } if msg.contains("Pure")));
  }
}
