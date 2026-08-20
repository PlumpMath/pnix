//! Mesh 구조 정의
//!
//! pnix-old의 pnix_graphics/src/mesh.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, GPU 버퍼 생성/업로드 로직 제외
//! wgpu 의존성 제거, 기본 타입만 사용

use serde::{Deserialize, Serialize};

/// Vertex data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Vertex {
  /// Position (x, y, z)
  pub position: [f32; 3],
  /// Normal (x, y, z)
  pub normal: [f32; 3],
  /// UV coordinates (u, v)
  pub uv: [f32; 2],
  /// Color (RGBA)
  pub color: [f32; 4],
}

impl Vertex {
  /// 새 vertex 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(position: [f32; 3], normal: [f32; 3], uv: [f32; 2], color: [f32; 4]) -> Self {
    Self {
      position,
      normal,
      uv,
      color,
    }
  }
}

/// 3D Mesh
///
/// **주의**: GPU 버퍼 생성/업로드 로직은 executor에서 구현합니다.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mesh {
  /// Vertices
  pub vertices: Vec<Vertex>,
  /// Indices
  pub indices: Vec<u32>,
}

impl Mesh {
  /// 새 mesh 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(vertices: Vec<Vertex>, indices: Vec<u32>) -> Self {
    Self { vertices, indices }
  }
}
