//! Graphics 구조 정의
//!
//! pnix-old의 pnix_graphics에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 타입 정의만, GPU 버퍼 업로드/렌더링 실행 로직 제외
//! nalgebra/wgpu 의존성 제거, 기본 타입만 사용

pub mod camera;
pub mod light;
pub mod material;
pub mod mesh;
pub mod scene;

pub use camera::{Camera, CameraController, ProjectionMode};
pub use light::{Light, LightType};
pub use material::{AlphaMode, Material};
pub use mesh::{Mesh, Vertex};
pub use scene::{SceneNode, Transform};
