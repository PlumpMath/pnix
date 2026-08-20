//! Light 구조 정의
//!
//! pnix-old의 pnix_graphics/src/light.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 렌더링 실행 로직 제외

use serde::{Deserialize, Serialize};

/// Light type
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LightType {
  /// Directional light (sun)
  Directional {
    /// Direction vector (x, y, z)
    direction: [f32; 3],
  },
  /// Point light
  Point {
    /// Position (x, y, z)
    position: [f32; 3],
    /// Range
    range: f32,
  },
  /// Spot light
  Spot {
    /// Position (x, y, z)
    position: [f32; 3],
    /// Direction vector (x, y, z)
    direction: [f32; 3],
    /// Range
    range: f32,
    /// Inner angle (radians)
    inner_angle: f32,
    /// Outer angle (radians)
    outer_angle: f32,
  },
  /// Ambient light
  Ambient,
}

/// Light source
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Light {
  /// Light type and parameters
  pub light_type: LightType,
  /// Color (RGB)
  pub color: [f32; 3],
  /// Intensity
  pub intensity: f32,
  /// Cast shadows
  pub cast_shadows: bool,
}

impl Default for Light {
  fn default() -> Self {
    Self {
      light_type: LightType::Directional {
        direction: [1.0, -1.0, 1.0],
      },
      color: [1.0, 0.98, 0.95],
      intensity: 1.0,
      cast_shadows: true,
    }
  }
}

impl Light {
  /// 방향성 광원 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn directional(direction: [f32; 3]) -> Self {
    Self {
      light_type: LightType::Directional { direction },
      color: [1.0, 0.98, 0.95],
      intensity: 1.0,
      cast_shadows: true,
    }
  }

  /// 점 광원 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn point(position: [f32; 3], range: f32) -> Self {
    Self {
      light_type: LightType::Point { position, range },
      color: [1.0, 1.0, 1.0],
      intensity: 1.0,
      cast_shadows: false,
    }
  }

  /// 환경 광원 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn ambient(intensity: f32) -> Self {
    Self {
      light_type: LightType::Ambient,
      color: [0.15, 0.15, 0.2],
      intensity,
      cast_shadows: false,
    }
  }
}
