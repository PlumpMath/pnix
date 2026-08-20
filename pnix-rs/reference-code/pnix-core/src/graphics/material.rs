//! Material 구조 정의
//!
//! pnix-old의 pnix_graphics/src/material.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 렌더링 실행 로직 제외

use serde::{Deserialize, Serialize};

/// Material properties
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Material {
  /// Base color (RGBA)
  pub base_color: [f32; 4],
  /// Metallic factor (0.0 = dielectric, 1.0 = metallic)
  pub metallic: f32,
  /// Roughness factor (0.0 = smooth, 1.0 = rough)
  pub roughness: f32,
  /// Emissive color (RGBA)
  pub emissive: [f32; 4],
  /// Alpha mode
  pub alpha_mode: AlphaMode,
  /// Double-sided rendering
  pub double_sided: bool,
}

/// Alpha blending mode
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum AlphaMode {
  /// Opaque
  Opaque,
  /// Blend
  Blend,
  /// Mask with cutoff value
  Mask(f32),
}

impl Default for Material {
  fn default() -> Self {
    Self {
      base_color: [1.0, 1.0, 1.0, 1.0],
      metallic: 0.0,
      roughness: 0.5,
      emissive: [0.0, 0.0, 0.0, 0.0],
      alpha_mode: AlphaMode::Opaque,
      double_sided: false,
    }
  }
}

impl Material {
  /// 기본 색상으로 새 material 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(color: [f32; 4]) -> Self {
    Self {
      base_color: color,
      ..Default::default()
    }
  }

  /// Hex 색상 문자열에서 생성 (순수 함수)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn from_hex(hex: &str) -> Self {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(hex.get(0..2).unwrap_or("FF"), 16).unwrap_or(255) as f32 / 255.0;
    let g = u8::from_str_radix(hex.get(2..4).unwrap_or("FF"), 16).unwrap_or(255) as f32 / 255.0;
    let b = u8::from_str_radix(hex.get(4..6).unwrap_or("FF"), 16).unwrap_or(255) as f32 / 255.0;
    let a = if hex.len() > 6 {
      u8::from_str_radix(hex.get(6..8).unwrap_or("FF"), 16).unwrap_or(255) as f32 / 255.0
    } else {
      1.0
    };
    Self::new([r, g, b, a])
  }

  /// Preset: Red
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn red() -> Self {
    Self::from_hex("#FF6B6B")
  }

  /// Preset: Green
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn green() -> Self {
    Self::from_hex("#4ECDC4")
  }

  /// Preset: Blue
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn blue() -> Self {
    Self::from_hex("#45B7D1")
  }

  /// Preset: White
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn white() -> Self {
    Self::new([1.0, 1.0, 1.0, 1.0])
  }

  /// Preset: Black
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn black() -> Self {
    Self::new([0.0, 0.0, 0.0, 1.0])
  }
}
