//! Render - Expression rendering to various formats
//!
//! pnix-old의 LaTeX 렌더링을 pnix-new에 마이그레이션
//!
//! ## 지원 포맷
//!
//! - LaTeX: 수학 표현식 렌더링 (교육용)
//! - Plain: 디버그용 텍스트 출력
//!
//! ## 헌법 준수 (P0-1)
//!
//! 순수 텍스트 변환, 실행 없음

mod latex;
mod plain;

pub use latex::to_latex;
pub use plain::to_plain;
