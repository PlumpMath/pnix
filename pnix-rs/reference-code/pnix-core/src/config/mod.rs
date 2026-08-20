//! 설정 관련 타입 및 문서화
//!
//! pnix-old의 pnix_config에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1, C1)
//!
//! 구조 정의 및 텍스트 생성만, 파일 I/O 없음
//!
//! ## 모듈 구성
//!
//! - `types`: 설정 데이터 타입 (ReplMode, LlmConfig, SymbolicConfig 등)
//! - `schema`: 설정 스키마 문서화 (Markdown, JSON Schema, HTML)
//! - `migration`: 설정 버전 마이그레이션 구조

pub mod migration;
pub mod schema;
pub mod types;

pub use migration::{
  BackupMetadata, BackupSchedule, BackupType, ConfigVersion, FieldChangeType, MigratedField,
  MigrationReport, VersionedConfig,
};
pub use schema::{ConfigSchema, FieldInfo, SchemaDocumenter, SectionInfo};
pub use types::{
  LlmConfig, LlmProvider, PnixConfig, PromptStyle, ReplConfig, ReplMode, SymbolicConfig,
};
