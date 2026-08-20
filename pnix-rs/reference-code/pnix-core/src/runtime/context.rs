//! Runtime 컨텍스트 구조 정의
//!
//! pnix-old의 pnix_runtime/src/context.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, values HashMap은 런타임 상태이므로 executor로 이관

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 런타임 컨텍스트 구조 (순수 부분만)
///
/// **주의**: `values` HashMap은 런타임 상태이므로 executor에서 관리합니다.
/// pnix-core에는 units, categories, index_spaces, constants만 포함합니다.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RuntimeContext {
  /// 변수별 단위 (CT 태그용)
  pub units: HashMap<String, String>,
  /// 변수별 카테고리
  pub categories: HashMap<String, String>,
  /// 텐서 인덱스별 공간
  pub index_spaces: HashMap<String, String>,
  /// 물리 상수 (value, unit)
  pub constants: HashMap<String, (f64, String)>,
}

impl RuntimeContext {
  /// 빈 컨텍스트 생성
  pub fn new() -> Self {
    Self::default()
  }

  /// 물리 교육용 기본 컨텍스트 (순수 함수)
  pub fn physics_default() -> Self {
    let mut ctx = Self::new();

    // 기본 물리량 단위
    ctx.units.insert("x".to_string(), "m".to_string());
    ctx.units.insert("y".to_string(), "m".to_string());
    ctx.units.insert("z".to_string(), "m".to_string());
    ctx.units.insert("v".to_string(), "m/s".to_string());
    ctx.units.insert("v0".to_string(), "m/s".to_string());
    ctx.units.insert("vx".to_string(), "m/s".to_string());
    ctx.units.insert("vy".to_string(), "m/s".to_string());
    ctx.units.insert("a".to_string(), "m/s^2".to_string());
    ctx.units.insert("t".to_string(), "s".to_string());
    ctx.units.insert("m".to_string(), "kg".to_string());
    ctx.units.insert("F".to_string(), "N".to_string());
    ctx.units.insert("E".to_string(), "J".to_string());
    ctx.units.insert("p".to_string(), "kg*m/s".to_string());

    // 물리 상수
    ctx
      .constants
      .insert("g".to_string(), (9.80665, "m/s^2".to_string()));
    ctx
      .constants
      .insert("c".to_string(), (299792458.0, "m/s".to_string()));
    ctx
      .constants
      .insert("pi".to_string(), (std::f64::consts::PI, "".to_string()));
    ctx
      .constants
      .insert("e".to_string(), (std::f64::consts::E, "".to_string()));

    ctx
  }

  /// GR/텐서용 기본 컨텍스트 (순수 함수)
  pub fn gr_default() -> Self {
    let mut ctx = Self::physics_default();

    // 시공간 인덱스
    ctx
      .index_spaces
      .insert("μ".to_string(), "spacetime".to_string());
    ctx
      .index_spaces
      .insert("ν".to_string(), "spacetime".to_string());
    ctx
      .index_spaces
      .insert("ρ".to_string(), "spacetime".to_string());
    ctx
      .index_spaces
      .insert("σ".to_string(), "spacetime".to_string());
    ctx
      .index_spaces
      .insert("α".to_string(), "spacetime".to_string());
    ctx
      .index_spaces
      .insert("β".to_string(), "spacetime".to_string());

    // 공간 인덱스
    ctx
      .index_spaces
      .insert("i".to_string(), "space".to_string());
    ctx
      .index_spaces
      .insert("j".to_string(), "space".to_string());
    ctx
      .index_spaces
      .insert("k".to_string(), "space".to_string());

    ctx
  }
}
