//! Utility Functions
//!
//! pnix-old의 pnix_utils/symbolic_core에서 마이그레이션된 유틸리티 함수들
//!
//! ## 모듈
//!
//! - `string_helpers`: 문자열 처리 유틸리티
//! - `json_escape`: JSON 이스케이프 함수
//! - `nonempty`: NonEmptyVec 컬렉션 타입
//! - `precision`: 정밀도 추적 타입
//! - `statistics`: 통계 타입 (계산 함수는 executor)
//! - `uuid`: UUID 타입 (생성 함수는 executor)
//! - `security`: 인증/권한/암호화 타입 (런타임 로직은 executor)
//! - `compression`: 압축 타입 (압축/해제 함수는 executor)

pub mod compression;
pub mod fast_parse;
pub mod json_escape;
pub mod json_safe;
pub mod log_level;
pub mod nonempty;
pub mod path_security;
pub mod petgraph;
pub mod precision;
pub mod profiling;
pub mod security;
pub mod statistics;
pub mod string_helpers;
pub mod test;
pub mod uuid;

pub use compression::{CompressionError, CompressionFormat, CompressionLevel, CompressionStats};
pub use fast_parse::*;
pub use json_escape::escape_json_for_string;
pub use log_level::LogLevel;
pub use nonempty::{NonEmptyIter, NonEmptySlice, NonEmptyVec};
pub use path_security::{contains_path_traversal, verify_path_within_base};
pub use petgraph::{PnixGraph, PoseEdge, PoseNode};
pub use precision::{
  Approx, ApproxReason, Exact, PrecisionInfo, PrecisionResult, SymbolicPrecision,
};
pub use security::{AuthError, CryptoError, PasswordStrength, Permission, Role, TokenClaims};
pub use statistics::{DescriptiveStats, StatisticsError};
pub use string_helpers::*;
pub use uuid::{Uuid, UuidError, UuidVariant};
