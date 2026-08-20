//! Python 언어 지원 모듈 (내부/테스트/interop 전용)
//!
//! PythonNode → FxCoreExpr 변환. **표현식 subset만** 지원.
//!
//! ## 범위 (ADR-0010)
//!
//! - CLI에서 Python 텍스트 입력은 **지원하지 않음**
//! - statement-level 구문(def, if, for, while 등)은 `UnsupportedSyntax` 에러
//! - 외부 파서가 Python AST를 JSON으로 전달할 때 interop 지점으로 사용
//!
//! ## 헌법 준수
//!
//! core에 Python 텍스트 파서(I/O, 외부 의존성)를 두지 않음.

pub mod convert;
pub mod unified;

pub use convert::{convert_python_to_fx_core, PythonConvertError};
pub use unified::{python_node_to_unified, PythonUnifiedError};
