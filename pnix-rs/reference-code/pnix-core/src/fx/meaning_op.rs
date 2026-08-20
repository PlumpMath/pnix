//! MeaningOpId - 모든 연산의 의미론적 식별자
//!
//! pnix-old의 meaning_ops.rs에서 마이그레이션
//!
//! ## 헌법 준수 (P0-1)
//!
//! 순수 enum 정의, 값 계산 없음
//!
//! ## 사용 목적
//!
//! - CT/SSA/FRP가 공유하는 언어 독립적 연산 ID
//! - FxCoreExpr의 Unary/Binary/Derived 연산 식별
//! - EffectZone 분류 기준

use crate::effects::{EffectZone, TimeKind};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

/// FRP/CT/SSA 공통 연산 식별자
///
/// 모든 연산의 의미론적 ID. 언어 독립적.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeaningOpId {
  // ========== Pure Math ==========
  /// 덧셈 (+)
  Add,
  /// 뺄셈 (-)
  Sub,
  /// 곱셈 (*)
  Mul,
  /// 나눗셈 (/)
  Div,
  /// 나머지 (%)
  Mod,
  /// 부정 (-)
  Neg,

  // Math functions
  /// 버림 (floor)
  Floor,
  /// 올림 (ceil)
  Ceil,
  /// 절대값 (abs)
  Abs,
  /// 제곱근 (sqrt)
  Sqrt,
  /// 사인 (sin)
  Sin,
  /// 코사인 (cos)
  Cos,
  /// 탄젠트 (tan)
  Tan,
  /// 지수 (e^x)
  Exp,
  /// 자연로그 (ln)
  Ln,
  /// 거듭제곱 (x^y)
  Pow,

  // Comparison
  /// 작다 (<)
  Lt,
  /// 크다 (>)
  Gt,
  /// 작거나 같다 (<=)
  Le,
  /// 크거나 같다 (>=)
  Ge,
  /// 같다 (==)
  Eq,
  /// 다르다 (!=)
  Ne,

  // Logic
  /// 논리곱 (&&)
  And,
  /// 논리합 (||)
  Or,
  /// 논리부정 (!)
  Not,

  // ========== FRP / Time ==========
  /// 시스템 시간 (param.system_time)
  SysTime,
  /// 델타 시간 (param.dt)
  DeltaTime,

  // Derived time ops (pattern lifted)
  /// floor(t) % 60
  SecondsFromTime,
  /// floor(t / 60) % 60
  MinutesFromTime,
  /// floor(t / 3600) % 12
  HoursFromTime,

  // ========== Signal Transforms ==========
  /// seconds * 6 * DEG2RAD
  AngleFromSecond,
  /// minutes * 6 * DEG2RAD
  AngleFromMinute,
  /// (hours + minutes/60) * 30 * DEG2RAD
  AngleFromHour,
  /// center + radius * sin/cos(angle)
  PositionFromAngle,

  // ========== Control Flow ==========
  /// 조건문 (if-then-else)
  If,
  /// 선택 (select)
  Select,

  // ========== Data Structures ==========
  /// 리스트 생성
  List,
  /// 리스트 맵 (fmap)
  ListMap,
  /// 속성 집합 생성
  AttrSet,
  /// 리스트 cons (prepend)
  ListCons,
  /// 연결 (++)
  Concat,
  /// 속성 집합 병합 (//)
  AttrSetUpdate,

  // ========== Interop ==========
  /// 언어 간 호출
  InteropCall,
  /// Clojure 호출
  InteropClj,
  InteropCljCall,
  InteropCljFn,
  /// Python 호출
  InteropPy,
  InteropPyCall,
  InteropPyFn,
  /// Nix 호출
  InteropNix,
  /// JavaScript 호출
  InteropJs,
  InteropJsCall,
  InteropJsFn,
  /// TypeScript 호출
  InteropTs,
  InteropTsCall,
  InteropTsFn,

  // ========== Category Theory ==========
  /// Functor fmap (<$>)
  CtFmap,
  /// Applicative pure
  CtPure,
  /// Applicative ap (<*>)
  CtAp,
  /// Monad bind (>>=)
  CtBind,
  /// Monad return
  CtReturn,
  /// Kleisli composition (>=>)
  CtCompose,
  /// Natural transformation
  CtNatTrans,
  /// Identity morphism
  CtId,

  // ========== Core Language Constructs ==========
  /// 리터럴 값
  Literal,
  /// 변수 참조
  Var,
  /// 람다 추상화
  Lambda,
  /// 함수 적용
  Apply,
  /// Let 바인딩
  Let,
  /// 재귀 속성 집합
  RecAttrSet,
  /// 속성 선택 (.)
  SelectAttr,
  /// 속성 존재 확인 (?)
  HasAttr,
  /// With 표현식
  With,
  /// 필터
  Filter,
  /// 왼쪽 폴드
  Foldl,
  /// 길이
  Length,
  /// 첫 원소
  Head,
  /// 나머지 원소
  Tail,
  /// 인덱스 접근
  ElemAt,
  /// 속성 이름들
  AttrNames,
  /// 속성 값들
  AttrValues,
  /// 동적 속성 접근
  GetAttr,
  /// 문자열 변환
  ToString,
  /// 타입 조회
  TypeOf,

  // ========== Concurrency / STM ==========
  /// Atom 생성
  AtomNew,
  /// Atom 읽기
  AtomRead,
  /// Atom 쓰기
  AtomWrite,
  /// Atom swap
  AtomSwap,
  /// Ref 생성
  RefNew,
  /// Ref 읽기
  RefRead,
  /// Ref 쓰기
  RefWrite,
  /// 원자적 트랜잭션
  Atomically,

  // ========== Channels / Async ==========
  /// 채널 생성
  ChannelNew,
  /// 채널 전송
  ChannelSend,
  /// 채널 수신
  ChannelRecv,
  /// 채널 닫기
  ChannelClose,
  /// 비동기 표현식
  Async,
  /// Await
  Await,

  // ========== IO ==========
  /// IO 읽기
  IoRead,
  /// IO 쓰기
  IoWrite,
  /// 출력
  Print,

  // ========== FRP Signals ==========
  /// 신호 리프트
  FrpLift,
  /// 신호 스텝
  FrpStep,
  /// 신호 샘플
  FrpSample,
  /// 입력 신호
  FrpInput,
  /// 상수 신호
  FrpConst,
}

/// MeaningOp 메타데이터 (SETO 로드용)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, 값 계산 없음
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeaningOpSpec {
  /// 연산 식별자
  pub op: MeaningOpId,
  /// 효과 영역
  pub zone: EffectZone,
  /// 시간 성질
  pub time: TimeKind,
  /// IR 심볼 (문자열 표현)
  pub ir_symbol: String,
}

static MEANING_OP_META: OnceLock<HashMap<MeaningOpId, MeaningOpSpec>> = OnceLock::new();

/// SETO에서 로드된 MeaningOp 메타 등록
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 등록만, 값 계산 없음
pub fn load_meaning_ops_from_seto<I>(ops: I) -> usize
where
  I: IntoIterator<Item = MeaningOpSpec>,
{
  let mut map = HashMap::new();
  for entry in ops {
    map.insert(entry.op, entry);
  }
  if MEANING_OP_META.set(map).is_err() {
    return MEANING_OP_META.get().map(|m| m.len()).unwrap_or(0);
  }
  MEANING_OP_META.get().map(|m| m.len()).unwrap_or(0)
}

/// SETO에서 로드된 MeaningOp 메타 조회
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 조회만, 값 계산 없음
pub fn meaning_op_meta(op: MeaningOpId) -> Option<&'static MeaningOpSpec> {
  MEANING_OP_META.get().and_then(|map| map.get(&op))
}

impl MeaningOpId {
  /// EffectZone 분류 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn zone(&self) -> EffectZone {
    if let Some(meta) = meaning_op_meta(*self) {
      return meta.zone;
    }
    use MeaningOpId::*;

    match self {
      // Pure operations
      Add | Sub | Mul | Div | Mod | Neg | Floor | Ceil | Abs | Sqrt | Sin | Cos | Tan | Exp
      | Ln | Pow | Lt | Gt | Le | Ge | Eq | Ne | And | Or | Not | If | Select | List | ListMap
      | AttrSet | ListCons | Concat | AttrSetUpdate | SecondsFromTime | MinutesFromTime
      | HoursFromTime | AngleFromSecond | AngleFromMinute | AngleFromHour | PositionFromAngle
      | SysTime | DeltaTime => EffectZone::Pure,

      // CT / Fractal ops (all pure)
      CtFmap | CtPure | CtAp | CtBind | CtReturn | CtCompose | CtNatTrans | CtId => {
        EffectZone::Pure
      }

      // Core language constructs (pure)
      Literal | Var | Lambda | Apply | Let | RecAttrSet | SelectAttr | HasAttr | With | Filter
      | Foldl | Length | Head | Tail | ElemAt | AttrNames | AttrValues | GetAttr | ToString
      | TypeOf => EffectZone::Pure,

      // Interop ops
      InteropCall | InteropClj | InteropCljCall | InteropCljFn | InteropPy | InteropPyCall
      | InteropPyFn | InteropNix | InteropJs | InteropJsCall | InteropJsFn | InteropTs
      | InteropTsCall | InteropTsFn => EffectZone::Interop,

      // STM ops
      AtomNew | AtomRead | AtomWrite | AtomSwap | RefNew | RefRead | RefWrite | Atomically => {
        EffectZone::Stm
      }

      // Channel/Async/IO ops (World effects)
      ChannelNew | ChannelSend | ChannelRecv | ChannelClose | Async | Await | IoRead | IoWrite
      | Print => EffectZone::World,

      // FRP Signal ops (pure - signals are functional)
      FrpLift | FrpStep | FrpSample | FrpInput | FrpConst => EffectZone::Pure,
    }
  }

  /// IR symbol name (코드 생성용)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn ir_symbol(&self) -> &'static str {
    if let Some(meta) = meaning_op_meta(*self) {
      return meta.ir_symbol.as_str();
    }
    use MeaningOpId::*;

    match self {
      Add => "fx_add",
      Sub => "fx_sub",
      Mul => "fx_mul",
      Div => "fx_div",
      Mod => "fx_mod",
      Neg => "fx_neg",
      Floor => "fx_floor",
      Ceil => "fx_ceil",
      Abs => "fx_abs",
      Sqrt => "fx_sqrt",
      Sin => "fx_sin",
      Cos => "fx_cos",
      Tan => "fx_tan",
      Exp => "fx_exp",
      Ln => "fx_ln",
      Pow => "fx_pow",
      Lt => "fx_lt",
      Gt => "fx_gt",
      Le => "fx_le",
      Ge => "fx_ge",
      Eq => "fx_eq",
      Ne => "fx_ne",
      And => "fx_and",
      Or => "fx_or",
      Not => "fx_not",
      If => "fx_if",
      Select => "fx_select",
      List => "fx_list",
      ListMap => "fx_list_map",
      AttrSet => "fx_attrset",
      ListCons => "fx_list_cons",
      Concat => "fx_concat",
      AttrSetUpdate => "fx_attrset_update",
      SysTime => "fx_sys_time",
      DeltaTime => "fx_delta_time",
      SecondsFromTime => "fx_seconds_from_time",
      MinutesFromTime => "fx_minutes_from_time",
      HoursFromTime => "fx_hours_from_time",
      AngleFromSecond => "fx_angle_from_second",
      AngleFromMinute => "fx_angle_from_minute",
      AngleFromHour => "fx_angle_from_hour",
      PositionFromAngle => "fx_position_from_angle",
      InteropCall => "fx_interop_call",
      InteropClj => "fx_interop_clj",
      InteropCljCall => "fx_interop_clj_call",
      InteropCljFn => "fx_interop_clj_fn",
      InteropPy => "fx_interop_py",
      InteropPyCall => "fx_interop_py_call",
      InteropPyFn => "fx_interop_py_fn",
      InteropNix => "fx_interop_nix",
      InteropJs => "fx_interop_js",
      InteropJsCall => "fx_interop_js_call",
      InteropJsFn => "fx_interop_js_fn",
      InteropTs => "fx_interop_ts",
      InteropTsCall => "fx_interop_ts_call",
      InteropTsFn => "fx_interop_ts_fn",
      CtFmap => "ct_fmap",
      CtPure => "ct_pure",
      CtAp => "ct_ap",
      CtBind => "ct_bind",
      CtReturn => "ct_return",
      CtCompose => "ct_compose",
      CtNatTrans => "ct_nat_trans",
      CtId => "ct_id",
      Literal => "fx_literal",
      Var => "fx_var",
      Lambda => "fx_lambda",
      Apply => "fx_apply",
      Let => "fx_let",
      RecAttrSet => "fx_rec_attrset",
      SelectAttr => "fx_select_attr",
      HasAttr => "fx_has_attr",
      With => "fx_with",
      Filter => "fx_filter",
      Foldl => "fx_foldl",
      Length => "fx_length",
      Head => "fx_head",
      Tail => "fx_tail",
      ElemAt => "fx_elem_at",
      AttrNames => "fx_attr_names",
      AttrValues => "fx_attr_values",
      GetAttr => "fx_get_attr",
      ToString => "fx_to_string",
      TypeOf => "fx_type_of",
      AtomNew => "fx_atom_new",
      AtomRead => "fx_atom_read",
      AtomWrite => "fx_atom_write",
      AtomSwap => "fx_atom_swap",
      RefNew => "fx_ref_new",
      RefRead => "fx_ref_read",
      RefWrite => "fx_ref_write",
      Atomically => "fx_atomically",
      ChannelNew => "fx_channel_new",
      ChannelSend => "fx_channel_send",
      ChannelRecv => "fx_channel_recv",
      ChannelClose => "fx_channel_close",
      Async => "fx_async",
      Await => "fx_await",
      IoRead => "fx_io_read",
      IoWrite => "fx_io_write",
      Print => "fx_print",
      FrpLift => "fx_frp_lift",
      FrpStep => "fx_frp_step",
      FrpSample => "fx_frp_sample",
      FrpInput => "fx_frp_input",
      FrpConst => "fx_frp_const",
    }
  }

  /// CT 최적화 가능 여부
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_optimizable(&self) -> bool {
    self.zone() == EffectZone::Pure
  }

  /// 이항 연산인지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_binary(&self) -> bool {
    use MeaningOpId::*;
    matches!(
      self,
      Add
        | Sub
        | Mul
        | Div
        | Mod
        | Pow
        | Lt
        | Gt
        | Le
        | Ge
        | Eq
        | Ne
        | And
        | Or
        | ListCons
        | Concat
        | AttrSetUpdate
    )
  }

  /// 단항 연산인지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_unary(&self) -> bool {
    use MeaningOpId::*;
    matches!(
      self,
      Neg | Not | Floor | Ceil | Abs | Sqrt | Sin | Cos | Tan | Exp | Ln
    )
  }
}

impl std::fmt::Display for MeaningOpId {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.ir_symbol())
  }
}

/// Meaning 메타데이터
///
/// 연산의 의미론적 정보 (ID + zone + time)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, 값 계산 없음
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeaningMeta {
  /// 연산 식별자
  pub op: MeaningOpId,
  /// 효과 영역
  pub zone: EffectZone,
  /// 시간 성질
  pub time: TimeKind,
}

impl MeaningMeta {
  /// 순수 연산 메타 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn pure(op: MeaningOpId) -> Self {
    Self {
      op,
      zone: EffectZone::Pure,
      time: TimeKind::Static,
    }
  }

  /// 연속 시간 연산 메타 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn continuous(op: MeaningOpId) -> Self {
    Self {
      op,
      zone: EffectZone::Pure,
      time: TimeKind::Continuous,
    }
  }

  /// Interop 연산 메타 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn interop(op: MeaningOpId) -> Self {
    Self {
      op,
      zone: EffectZone::Interop,
      time: TimeKind::Static,
    }
  }

  /// 연산 ID에서 자동 추론
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn from_op(op: MeaningOpId) -> Self {
    if let Some(meta) = meaning_op_meta(op) {
      return Self {
        op,
        zone: meta.zone,
        time: meta.time,
      };
    }
    Self {
      zone: op.zone(),
      time: TimeKind::Static,
      op,
    }
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;
  use crate::fx::op_table::UnifiedMeaningOp;
  use std::collections::BTreeSet;

  fn all_ops() -> Vec<MeaningOpId> {
    UnifiedMeaningOp::all()
      .iter()
      .copied()
      .map(MeaningOpId::from)
      .collect()
  }

  #[test]
  fn test_meaning_op_zone() {
    assert_eq!(MeaningOpId::Add.zone(), EffectZone::Pure);
    assert_eq!(MeaningOpId::InteropClj.zone(), EffectZone::Interop);
    assert_eq!(MeaningOpId::AtomNew.zone(), EffectZone::Stm);
    assert_eq!(MeaningOpId::Print.zone(), EffectZone::World);
  }

  #[test]
  fn test_meaning_op_ir_symbol() {
    assert_eq!(MeaningOpId::Add.ir_symbol(), "fx_add");
    assert_eq!(MeaningOpId::CtFmap.ir_symbol(), "ct_fmap");
    assert_eq!(MeaningOpId::FrpLift.ir_symbol(), "fx_frp_lift");
  }

  #[test]
  fn test_meaning_op_is_optimizable() {
    assert!(MeaningOpId::Add.is_optimizable());
    assert!(MeaningOpId::Sin.is_optimizable());
    assert!(!MeaningOpId::Print.is_optimizable());
    assert!(!MeaningOpId::InteropClj.is_optimizable());
  }

  #[test]
  fn test_meaning_op_is_binary() {
    assert!(MeaningOpId::Add.is_binary());
    assert!(MeaningOpId::Mul.is_binary());
    assert!(!MeaningOpId::Neg.is_binary());
    assert!(!MeaningOpId::Sin.is_binary());
  }

  #[test]
  fn test_meaning_op_is_unary() {
    assert!(MeaningOpId::Neg.is_unary());
    assert!(MeaningOpId::Sin.is_unary());
    assert!(!MeaningOpId::Add.is_unary());
  }

  #[test]
  fn test_meaning_op_zone_all() {
    for op in all_ops() {
      let _ = op.zone();
    }
  }

  #[test]
  fn test_meaning_op_ir_symbol_all() {
    let mut seen = BTreeSet::new();
    for op in all_ops() {
      let symbol = op.ir_symbol();
      assert!(!symbol.trim().is_empty());
      assert!(seen.insert(symbol), "duplicate ir_symbol: {}", symbol);
      assert_eq!(format!("{}", op), symbol);
    }
  }

  #[test]
  fn test_meaning_op_unary_binary_invariants() {
    for op in all_ops() {
      assert!(!(op.is_unary() && op.is_binary()));
    }
  }

  #[test]
  fn test_meaning_meta_pure() {
    let meta = MeaningMeta::pure(MeaningOpId::Add);
    assert_eq!(meta.op, MeaningOpId::Add);
    assert_eq!(meta.zone, EffectZone::Pure);
    assert_eq!(meta.time, TimeKind::Static);
  }

  #[test]
  fn test_meaning_meta_continuous() {
    let meta = MeaningMeta::continuous(MeaningOpId::SecondsFromTime);
    assert_eq!(meta.time, TimeKind::Continuous);
  }

  #[test]
  fn test_meaning_meta_from_op() {
    let meta = MeaningMeta::from_op(MeaningOpId::InteropClj);
    assert_eq!(meta.zone, EffectZone::Interop);
  }

  #[test]
  fn test_meaning_op_display() {
    assert_eq!(format!("{}", MeaningOpId::Add), "fx_add");
    assert_eq!(format!("{}", MeaningOpId::CtBind), "ct_bind");
  }

  #[test]
  fn test_meaning_op_serde() {
    let op = MeaningOpId::Add;
    let json = serde_json::to_string(&op).unwrap();
    assert_eq!(json, "\"add\""); // snake_case

    let restored: MeaningOpId = serde_json::from_str(&json).unwrap();
    assert_eq!(op, restored);
  }

  #[test]
  fn test_ct_ops_are_pure() {
    assert!(MeaningOpId::CtFmap.is_optimizable());
    assert!(MeaningOpId::CtBind.is_optimizable());
    assert!(MeaningOpId::CtCompose.is_optimizable());
  }

  #[test]
  fn test_frp_ops_are_pure() {
    // FRP signals are functional/declarative
    assert!(MeaningOpId::FrpLift.is_optimizable());
    assert!(MeaningOpId::FrpConst.is_optimizable());
  }

  #[test]
  fn test_unified_mapping_roundtrip() {
    let ops = [
      MeaningOpId::Exp,
      MeaningOpId::Ln,
      MeaningOpId::Pow,
      MeaningOpId::ListCons,
      MeaningOpId::AttrSetUpdate,
      MeaningOpId::InteropCall,
    ];
    for op in ops {
      let unified = UnifiedMeaningOp::from(&op);
      let back = MeaningOpId::from(unified);
      assert_eq!(back, op);
    }
  }
}

// ============================================================
// UnifiedMeaningOp ↔ MeaningOpId 변환
// ============================================================

use crate::fx::op_table::UnifiedMeaningOp;

/// Convert legacy MeaningOpId to UnifiedMeaningOp
impl From<&MeaningOpId> for UnifiedMeaningOp {
  fn from(op: &MeaningOpId) -> Self {
    match op {
      // Arithmetic
      MeaningOpId::Add => UnifiedMeaningOp::Add,
      MeaningOpId::Sub => UnifiedMeaningOp::Sub,
      MeaningOpId::Mul => UnifiedMeaningOp::Mul,
      MeaningOpId::Div => UnifiedMeaningOp::Div,
      MeaningOpId::Mod => UnifiedMeaningOp::Mod,
      MeaningOpId::Neg => UnifiedMeaningOp::Neg,

      // Math functions
      MeaningOpId::Floor => UnifiedMeaningOp::Floor,
      MeaningOpId::Ceil => UnifiedMeaningOp::Ceil,
      MeaningOpId::Abs => UnifiedMeaningOp::Abs,
      MeaningOpId::Sqrt => UnifiedMeaningOp::Sqrt,
      MeaningOpId::Sin => UnifiedMeaningOp::Sin,
      MeaningOpId::Cos => UnifiedMeaningOp::Cos,
      MeaningOpId::Tan => UnifiedMeaningOp::Tan,
      MeaningOpId::Exp => UnifiedMeaningOp::Exp,
      MeaningOpId::Ln => UnifiedMeaningOp::Ln,
      MeaningOpId::Pow => UnifiedMeaningOp::Pow,

      // Comparison
      MeaningOpId::Lt => UnifiedMeaningOp::Lt,
      MeaningOpId::Gt => UnifiedMeaningOp::Gt,
      MeaningOpId::Le => UnifiedMeaningOp::Le,
      MeaningOpId::Ge => UnifiedMeaningOp::Ge,
      MeaningOpId::Eq => UnifiedMeaningOp::Eq,
      MeaningOpId::Ne => UnifiedMeaningOp::Ne,

      // Logic
      MeaningOpId::And => UnifiedMeaningOp::And,
      MeaningOpId::Or => UnifiedMeaningOp::Or,
      MeaningOpId::Not => UnifiedMeaningOp::Not,

      // Control flow
      MeaningOpId::If => UnifiedMeaningOp::If,
      MeaningOpId::Select => UnifiedMeaningOp::Select,

      // Data structures
      MeaningOpId::List => UnifiedMeaningOp::List,
      MeaningOpId::ListMap => UnifiedMeaningOp::Map,
      MeaningOpId::AttrSet => UnifiedMeaningOp::AttrSet,
      MeaningOpId::Concat => UnifiedMeaningOp::Concat,
      MeaningOpId::ListCons => UnifiedMeaningOp::ListCons,
      MeaningOpId::AttrSetUpdate => UnifiedMeaningOp::AttrSetUpdate,

      // FRP / Time
      MeaningOpId::SysTime => UnifiedMeaningOp::SysTime,
      MeaningOpId::DeltaTime => UnifiedMeaningOp::DeltaTime,
      MeaningOpId::SecondsFromTime => UnifiedMeaningOp::SecondsFromTime,
      MeaningOpId::MinutesFromTime => UnifiedMeaningOp::MinutesFromTime,
      MeaningOpId::HoursFromTime => UnifiedMeaningOp::HoursFromTime,
      MeaningOpId::AngleFromSecond => UnifiedMeaningOp::AngleFromSecond,
      MeaningOpId::AngleFromMinute => UnifiedMeaningOp::AngleFromMinute,
      MeaningOpId::AngleFromHour => UnifiedMeaningOp::AngleFromHour,
      MeaningOpId::PositionFromAngle => UnifiedMeaningOp::PositionFromAngle,

      // Interop
      MeaningOpId::InteropCall => UnifiedMeaningOp::InteropCall,
      MeaningOpId::InteropClj => UnifiedMeaningOp::InteropClj,
      MeaningOpId::InteropCljCall => UnifiedMeaningOp::InteropCljCall,
      MeaningOpId::InteropCljFn => UnifiedMeaningOp::InteropCljFn,
      MeaningOpId::InteropPy => UnifiedMeaningOp::InteropPy,
      MeaningOpId::InteropPyCall => UnifiedMeaningOp::InteropPyCall,
      MeaningOpId::InteropPyFn => UnifiedMeaningOp::InteropPyFn,
      MeaningOpId::InteropNix => UnifiedMeaningOp::InteropNix,
      MeaningOpId::InteropJs => UnifiedMeaningOp::InteropJS,
      MeaningOpId::InteropJsCall => UnifiedMeaningOp::InteropJSCall,
      MeaningOpId::InteropJsFn => UnifiedMeaningOp::InteropJSFn,
      MeaningOpId::InteropTs => UnifiedMeaningOp::InteropTS,
      MeaningOpId::InteropTsCall => UnifiedMeaningOp::InteropTSCall,
      MeaningOpId::InteropTsFn => UnifiedMeaningOp::InteropTSFn,

      // CT / Fractal ops
      MeaningOpId::CtFmap => UnifiedMeaningOp::CTFmap,
      MeaningOpId::CtPure => UnifiedMeaningOp::CTPure,
      MeaningOpId::CtAp => UnifiedMeaningOp::CTAp,
      MeaningOpId::CtBind => UnifiedMeaningOp::CTBind,
      MeaningOpId::CtReturn => UnifiedMeaningOp::CTReturn,
      MeaningOpId::CtCompose => UnifiedMeaningOp::CTCompose,
      MeaningOpId::CtNatTrans => UnifiedMeaningOp::CTNatTrans,
      MeaningOpId::CtId => UnifiedMeaningOp::CTId,

      // Core language constructs
      MeaningOpId::Literal => UnifiedMeaningOp::Literal,
      MeaningOpId::Var => UnifiedMeaningOp::Var,
      MeaningOpId::Lambda => UnifiedMeaningOp::Lambda,
      MeaningOpId::Apply => UnifiedMeaningOp::Apply,
      MeaningOpId::Let => UnifiedMeaningOp::Let,
      MeaningOpId::RecAttrSet => UnifiedMeaningOp::RecAttrSet,
      MeaningOpId::SelectAttr => UnifiedMeaningOp::SelectAttr,
      MeaningOpId::HasAttr => UnifiedMeaningOp::HasAttr,
      MeaningOpId::With => UnifiedMeaningOp::With,
      MeaningOpId::Filter => UnifiedMeaningOp::Filter,
      MeaningOpId::Foldl => UnifiedMeaningOp::Foldl,
      MeaningOpId::Length => UnifiedMeaningOp::Length,
      MeaningOpId::Head => UnifiedMeaningOp::Head,
      MeaningOpId::Tail => UnifiedMeaningOp::Tail,
      MeaningOpId::ElemAt => UnifiedMeaningOp::ElemAt,
      MeaningOpId::AttrNames => UnifiedMeaningOp::AttrNames,
      MeaningOpId::AttrValues => UnifiedMeaningOp::AttrValues,
      MeaningOpId::GetAttr => UnifiedMeaningOp::GetAttr,
      MeaningOpId::ToString => UnifiedMeaningOp::ToString,
      MeaningOpId::TypeOf => UnifiedMeaningOp::TypeOf,

      // STM ops
      MeaningOpId::AtomNew => UnifiedMeaningOp::AtomNew,
      MeaningOpId::AtomRead => UnifiedMeaningOp::AtomRead,
      MeaningOpId::AtomWrite => UnifiedMeaningOp::AtomWrite,
      MeaningOpId::AtomSwap => UnifiedMeaningOp::AtomSwap,
      MeaningOpId::RefNew => UnifiedMeaningOp::RefNew,
      MeaningOpId::RefRead => UnifiedMeaningOp::RefRead,
      MeaningOpId::RefWrite => UnifiedMeaningOp::RefWrite,
      MeaningOpId::Atomically => UnifiedMeaningOp::Atomically,

      // Channel/Async ops
      MeaningOpId::ChannelNew => UnifiedMeaningOp::ChannelNew,
      MeaningOpId::ChannelSend => UnifiedMeaningOp::ChannelSend,
      MeaningOpId::ChannelRecv => UnifiedMeaningOp::ChannelRecv,
      MeaningOpId::ChannelClose => UnifiedMeaningOp::ChannelClose,
      MeaningOpId::Async => UnifiedMeaningOp::Async,
      MeaningOpId::Await => UnifiedMeaningOp::Await,

      // IO ops
      MeaningOpId::IoRead => UnifiedMeaningOp::IORead,
      MeaningOpId::IoWrite => UnifiedMeaningOp::IOWrite,
      MeaningOpId::Print => UnifiedMeaningOp::Print,

      // FRP Signal ops
      MeaningOpId::FrpLift => UnifiedMeaningOp::FrpLift,
      MeaningOpId::FrpStep => UnifiedMeaningOp::FrpStep,
      MeaningOpId::FrpSample => UnifiedMeaningOp::FrpSample,
      MeaningOpId::FrpInput => UnifiedMeaningOp::FrpInput,
      MeaningOpId::FrpConst => UnifiedMeaningOp::FrpConst,
    }
  }
}

/// Convert UnifiedMeaningOp to legacy MeaningOpId
impl From<UnifiedMeaningOp> for MeaningOpId {
  fn from(op: UnifiedMeaningOp) -> Self {
    match op {
      // Arithmetic
      UnifiedMeaningOp::Add => MeaningOpId::Add,
      UnifiedMeaningOp::Sub => MeaningOpId::Sub,
      UnifiedMeaningOp::Mul => MeaningOpId::Mul,
      UnifiedMeaningOp::Div => MeaningOpId::Div,
      UnifiedMeaningOp::Mod => MeaningOpId::Mod,
      UnifiedMeaningOp::Neg => MeaningOpId::Neg,

      // Math functions
      UnifiedMeaningOp::Floor => MeaningOpId::Floor,
      UnifiedMeaningOp::Ceil => MeaningOpId::Ceil,
      UnifiedMeaningOp::Abs => MeaningOpId::Abs,
      UnifiedMeaningOp::Sqrt => MeaningOpId::Sqrt,
      UnifiedMeaningOp::Sin => MeaningOpId::Sin,
      UnifiedMeaningOp::Cos => MeaningOpId::Cos,
      UnifiedMeaningOp::Tan => MeaningOpId::Tan,
      UnifiedMeaningOp::Exp => MeaningOpId::Exp,
      UnifiedMeaningOp::Ln => MeaningOpId::Ln,
      UnifiedMeaningOp::Pow => MeaningOpId::Pow,

      // Comparison
      UnifiedMeaningOp::Lt => MeaningOpId::Lt,
      UnifiedMeaningOp::Gt => MeaningOpId::Gt,
      UnifiedMeaningOp::Le => MeaningOpId::Le,
      UnifiedMeaningOp::Ge => MeaningOpId::Ge,
      UnifiedMeaningOp::Eq => MeaningOpId::Eq,
      UnifiedMeaningOp::Ne => MeaningOpId::Ne,

      // Logic
      UnifiedMeaningOp::And => MeaningOpId::And,
      UnifiedMeaningOp::Or => MeaningOpId::Or,
      UnifiedMeaningOp::Not => MeaningOpId::Not,

      // Control flow
      UnifiedMeaningOp::If => MeaningOpId::If,
      UnifiedMeaningOp::Select => MeaningOpId::Select,

      // Data structures
      UnifiedMeaningOp::List => MeaningOpId::List,
      UnifiedMeaningOp::Map => MeaningOpId::ListMap,
      UnifiedMeaningOp::AttrSet => MeaningOpId::AttrSet,
      UnifiedMeaningOp::Concat => MeaningOpId::Concat,
      UnifiedMeaningOp::ListCons => MeaningOpId::ListCons,
      UnifiedMeaningOp::AttrSetUpdate => MeaningOpId::AttrSetUpdate,

      // FRP / Time
      UnifiedMeaningOp::SysTime => MeaningOpId::SysTime,
      UnifiedMeaningOp::DeltaTime => MeaningOpId::DeltaTime,
      UnifiedMeaningOp::SecondsFromTime => MeaningOpId::SecondsFromTime,
      UnifiedMeaningOp::MinutesFromTime => MeaningOpId::MinutesFromTime,
      UnifiedMeaningOp::HoursFromTime => MeaningOpId::HoursFromTime,
      UnifiedMeaningOp::AngleFromSecond => MeaningOpId::AngleFromSecond,
      UnifiedMeaningOp::AngleFromMinute => MeaningOpId::AngleFromMinute,
      UnifiedMeaningOp::AngleFromHour => MeaningOpId::AngleFromHour,
      UnifiedMeaningOp::PositionFromAngle => MeaningOpId::PositionFromAngle,

      // Interop
      UnifiedMeaningOp::InteropCall => MeaningOpId::InteropCall,
      UnifiedMeaningOp::InteropClj => MeaningOpId::InteropClj,
      UnifiedMeaningOp::InteropCljCall => MeaningOpId::InteropCljCall,
      UnifiedMeaningOp::InteropCljFn => MeaningOpId::InteropCljFn,
      UnifiedMeaningOp::InteropPy => MeaningOpId::InteropPy,
      UnifiedMeaningOp::InteropPyCall => MeaningOpId::InteropPyCall,
      UnifiedMeaningOp::InteropPyFn => MeaningOpId::InteropPyFn,
      UnifiedMeaningOp::InteropNix => MeaningOpId::InteropNix,
      UnifiedMeaningOp::InteropJS => MeaningOpId::InteropJs,
      UnifiedMeaningOp::InteropJSCall => MeaningOpId::InteropJsCall,
      UnifiedMeaningOp::InteropJSFn => MeaningOpId::InteropJsFn,
      UnifiedMeaningOp::InteropTS => MeaningOpId::InteropTs,
      UnifiedMeaningOp::InteropTSCall => MeaningOpId::InteropTsCall,
      UnifiedMeaningOp::InteropTSFn => MeaningOpId::InteropTsFn,

      // CT / Fractal ops
      UnifiedMeaningOp::CTFmap => MeaningOpId::CtFmap,
      UnifiedMeaningOp::CTPure => MeaningOpId::CtPure,
      UnifiedMeaningOp::CTAp => MeaningOpId::CtAp,
      UnifiedMeaningOp::CTBind => MeaningOpId::CtBind,
      UnifiedMeaningOp::CTReturn => MeaningOpId::CtReturn,
      UnifiedMeaningOp::CTCompose => MeaningOpId::CtCompose,
      UnifiedMeaningOp::CTNatTrans => MeaningOpId::CtNatTrans,
      UnifiedMeaningOp::CTId => MeaningOpId::CtId,

      // Core language constructs
      UnifiedMeaningOp::Literal => MeaningOpId::Literal,
      UnifiedMeaningOp::Var => MeaningOpId::Var,
      UnifiedMeaningOp::Lambda => MeaningOpId::Lambda,
      UnifiedMeaningOp::Apply => MeaningOpId::Apply,
      UnifiedMeaningOp::Let => MeaningOpId::Let,
      UnifiedMeaningOp::RecAttrSet => MeaningOpId::RecAttrSet,
      UnifiedMeaningOp::SelectAttr => MeaningOpId::SelectAttr,
      UnifiedMeaningOp::HasAttr => MeaningOpId::HasAttr,
      UnifiedMeaningOp::With => MeaningOpId::With,
      UnifiedMeaningOp::Filter => MeaningOpId::Filter,
      UnifiedMeaningOp::Foldl => MeaningOpId::Foldl,
      UnifiedMeaningOp::Length => MeaningOpId::Length,
      UnifiedMeaningOp::Head => MeaningOpId::Head,
      UnifiedMeaningOp::Tail => MeaningOpId::Tail,
      UnifiedMeaningOp::ElemAt => MeaningOpId::ElemAt,
      UnifiedMeaningOp::AttrNames => MeaningOpId::AttrNames,
      UnifiedMeaningOp::AttrValues => MeaningOpId::AttrValues,
      UnifiedMeaningOp::GetAttr => MeaningOpId::GetAttr,
      UnifiedMeaningOp::ToString => MeaningOpId::ToString,
      UnifiedMeaningOp::TypeOf => MeaningOpId::TypeOf,

      // STM / Concurrency
      UnifiedMeaningOp::AtomNew => MeaningOpId::AtomNew,
      UnifiedMeaningOp::AtomRead => MeaningOpId::AtomRead,
      UnifiedMeaningOp::AtomWrite => MeaningOpId::AtomWrite,
      UnifiedMeaningOp::AtomSwap => MeaningOpId::AtomSwap,
      UnifiedMeaningOp::RefNew => MeaningOpId::RefNew,
      UnifiedMeaningOp::RefRead => MeaningOpId::RefRead,
      UnifiedMeaningOp::RefWrite => MeaningOpId::RefWrite,
      UnifiedMeaningOp::Atomically => MeaningOpId::Atomically,

      // Channels / Async
      UnifiedMeaningOp::ChannelNew => MeaningOpId::ChannelNew,
      UnifiedMeaningOp::ChannelSend => MeaningOpId::ChannelSend,
      UnifiedMeaningOp::ChannelRecv => MeaningOpId::ChannelRecv,
      UnifiedMeaningOp::ChannelClose => MeaningOpId::ChannelClose,
      UnifiedMeaningOp::Async => MeaningOpId::Async,
      UnifiedMeaningOp::Await => MeaningOpId::Await,

      // IO
      UnifiedMeaningOp::IORead => MeaningOpId::IoRead,
      UnifiedMeaningOp::IOWrite => MeaningOpId::IoWrite,
      UnifiedMeaningOp::Print => MeaningOpId::Print,

      // FRP Signals
      UnifiedMeaningOp::FrpLift => MeaningOpId::FrpLift,
      UnifiedMeaningOp::FrpStep => MeaningOpId::FrpStep,
      UnifiedMeaningOp::FrpSample => MeaningOpId::FrpSample,
      UnifiedMeaningOp::FrpInput => MeaningOpId::FrpInput,
      UnifiedMeaningOp::FrpConst => MeaningOpId::FrpConst,
    }
  }
}

// ============================================================
// UnifiedMeta - Enhanced metadata with laws
// ============================================================

/// UnifiedMeaningOp를 위한 향상된 메타데이터
///
/// UnifiedMeaningOp를 위한 메타데이터 구조체.
/// MeaningMeta와 유사하지만 UnifiedMeaningOp를 사용합니다.
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, 값 계산 없음
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedMeta {
  /// 연산 식별자 (UnifiedMeaningOp)
  pub op: UnifiedMeaningOp,
  /// 효과 영역
  pub zone: EffectZone,
  /// 시간 성질
  pub time: TimeKind,
}

impl UnifiedMeta {
  /// 순수 연산 메타 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn pure(op: UnifiedMeaningOp) -> Self {
    Self {
      op,
      zone: op.zone(),
      time: TimeKind::Static,
    }
  }

  /// 연속 시간 연산 메타 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn continuous(op: UnifiedMeaningOp) -> Self {
    Self {
      op,
      zone: op.zone(),
      time: TimeKind::Continuous,
    }
  }

  /// 연산 ID에서 자동 추론
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn from_op(op: UnifiedMeaningOp) -> Self {
    Self {
      op,
      zone: op.zone(),
      time: TimeKind::Static,
    }
  }
}
