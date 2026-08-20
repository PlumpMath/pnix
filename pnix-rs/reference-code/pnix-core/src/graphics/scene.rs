//! Scene 구조 정의
//!
//! pnix-old의 pnix_graphics/src/scene.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, GPU 버퍼 업로드/렌더링 실행 로직 제외
//! nalgebra 의존성 제거, 기본 타입만 사용

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Scene node transform
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transform {
  /// Position (x, y, z)
  pub position: [f32; 3],
  /// Rotation as quaternion (w, x, y, z)
  pub rotation: [f32; 4],
  /// Scale (x, y, z)
  pub scale: [f32; 3],
}

impl Default for Transform {
  fn default() -> Self {
    Self {
      position: [0.0, 0.0, 0.0],
      rotation: [1.0, 0.0, 0.0, 0.0], // Identity quaternion
      scale: [1.0, 1.0, 1.0],
    }
  }
}

impl Transform {
  /// 위치에서 새 transform 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn at(x: f32, y: f32, z: f32) -> Self {
    Self {
      position: [x, y, z],
      ..Default::default()
    }
  }

  /// 균일한 스케일 설정
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn with_scale(mut self, scale: f32) -> Self {
    self.scale = [scale, scale, scale];
    self
  }

  /// 비균일 스케일 설정
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn with_scale_xyz(mut self, x: f32, y: f32, z: f32) -> Self {
    self.scale = [x, y, z];
    self
  }
}

/// Scene node
///
/// **주의**: GPU 버퍼 업로드/렌더링 로직은 executor에서 구현합니다.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneNode {
  /// Node name
  pub name: String,
  /// Transform
  pub transform: Transform,
  /// Material ID (executor에서 실제 Material 참조)
  pub material_id: Option<String>,
  /// Children
  pub children: Vec<SceneNode>,
  /// Visible
  pub visible: bool,
  /// Optional node metadata bag for runtime/projection surfaces
  #[serde(default)]
  pub metadata: BTreeMap<String, String>,
}

impl SceneNode {
  /// 새 빈 노드 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(name: &str) -> Self {
    Self {
      name: name.to_string(),
      transform: Transform::default(),
      material_id: None,
      children: Vec::new(),
      visible: true,
      metadata: BTreeMap::new(),
    }
  }
}
