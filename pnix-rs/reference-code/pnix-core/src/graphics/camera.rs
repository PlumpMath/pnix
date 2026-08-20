//! Camera 구조 정의
//!
//! pnix-old의 pnix_graphics/src/camera.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 카메라 업데이트/행렬 계산 로직 제외
//! nalgebra 의존성 제거, 기본 타입만 사용

use serde::{Deserialize, Serialize};

/// Camera projection mode
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ProjectionMode {
  /// Perspective projection
  Perspective {
    /// Field of view in radians
    fov: f32,
    /// Near plane
    near: f32,
    /// Far plane
    far: f32,
  },
  /// Orthographic projection
  Orthographic {
    /// Orthographic scale
    scale: f32,
    /// Near plane
    near: f32,
    /// Far plane
    far: f32,
  },
}

impl Default for ProjectionMode {
  fn default() -> Self {
    Self::Perspective {
      fov: std::f32::consts::FRAC_PI_4, // 45 degrees
      near: 0.1,
      far: 1000.0,
    }
  }
}

/// Camera controller type
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum CameraController {
  /// Orbit around a target point
  Orbit {
    /// Target point (x, y, z)
    target: [f32; 3],
    /// Distance from target
    distance: f32,
    /// Horizontal rotation (yaw)
    yaw: f32,
    /// Vertical rotation (pitch)
    pitch: f32,
  },
  /// Free-fly camera
  Fly {
    /// Horizontal rotation (yaw)
    yaw: f32,
    /// Vertical rotation (pitch)
    pitch: f32,
  },
  /// First-person style
  FirstPerson {
    /// Horizontal rotation (yaw)
    yaw: f32,
    /// Vertical rotation (pitch)
    pitch: f32,
    /// Height above ground
    height: f32,
  },
}

impl Default for CameraController {
  fn default() -> Self {
    Self::Orbit {
      target: [0.0, 0.0, 0.0],
      distance: 10.0,
      yaw: 0.0,
      pitch: 0.3,
    }
  }
}

/// 3D Camera 구조
///
/// **주의**: 카메라 업데이트/행렬 계산 로직은 executor에서 구현합니다.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Camera {
  /// Camera position in world space (x, y, z)
  pub position: [f32; 3],
  /// Up direction (x, y, z)
  pub up: [f32; 3],
  /// Projection mode
  pub projection: ProjectionMode,
  /// Controller
  pub controller: CameraController,
  /// Aspect ratio (width / height)
  pub aspect: f32,
  /// Movement speed
  pub speed: f32,
  /// Mouse sensitivity
  pub sensitivity: f32,
}

impl Default for Camera {
  fn default() -> Self {
    Self {
      position: [0.0, 5.0, 10.0],
      up: [0.0, 1.0, 0.0],
      projection: ProjectionMode::default(),
      controller: CameraController::default(),
      aspect: 16.0 / 9.0,
      speed: 5.0,
      sensitivity: 0.005,
    }
  }
}
