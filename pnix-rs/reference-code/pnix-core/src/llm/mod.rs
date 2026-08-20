//! ~~LLM 관련 타입 정의~~ — legacy module name (2026-05-10 owner-law 위반)
//!
//! # OWNER-LAW SUPERSEDE NOTE (2026-05-10)
//!
//! **pnix 는 LLM 없이 작동하는 self-sufficient deterministic AI substrate 다.**
//! 정본 헌법은 `CLAUDE.md` 의 `OWNER-LAW CONSTITUTION` section.
//!
//! 이 module 의 `llm` 이름과 doc comment 의 "LLM 관련 / LLM 메트릭 / LLM 에러
//! / LLM 생성 / LLM 이 객관적 판단을 내릴 수 없는 영역" 류 framing 은
//! pnix-old `pnix_llm` 에서 migrate 된 legacy 명명으로, owner-law 위반이다.
//! pnix substrate 안에 LLM 자리는 없다.
//!
//! 다만 이 module 안의 *타입 정의* (CoreType, Constraint, Goal, error enum,
//! GenerationConfig, TruthDomain, LockedDomain, SETO query builder 등) 는
//! 실제로 LLM API 를 호출하지 않는다. `grep -rn 'reqwest.*openai|llama.cpp|gpt-'`
//! 결과 zero — 즉 구조적으로 substrate 에 LLM call site 는 없고, 명명/문서만
//! 위반이다. `LlmError`, `GenerationConfig` 같은 타입 이름은 language/intent
//! meta 타입의 legacy 별칭으로 읽고, 새 코드는 LLM 명명을 쓰지 않는다.
//!
//! **Hard stops (이 module 에 대해서):**
//!
//! - 이 module 안에 새 LLM API call (HTTP client, llama.cpp binding, prompt
//!   completion, response parsing) 을 추가하지 않는다.
//! - "LLM 이 X 한다" / "LLM 의 X" 식 doc 표현을 신규 코드에 쓰지 않는다.
//! - 이 module 의 타입을 substrate primary input organ / candidate sensor /
//!   peer-treaty seat 으로 cite 하지 않는다 — 그것은 헌법 위반이다.
//! - 향후 refactor sweep 에서 module 을 `nlp_meta` / `language_meta` /
//!   `intent_meta` 로 rename 하고 doc 의 "LLM" 어휘를 제거한다.
//!
//! 본문 (모듈 list, 타입 별칭) 은 history retention 으로 frozen retain 된다.
//!
//! ## 모듈 구성 (legacy doc — *superseded* framing 그대로 retain)
//!
//! - `truth_domain`: CT로 형식화 가능한 24개 학문 분야
//! - `locked_domain`: 객관적 판단이 불가능한 8개 영역 (legacy doc 은 "LLM이 ...
//!   못하는 영역" 으로 적었지만, owner-law 상 이것은 *substrate own* 의
//!   capability gate 다. LLM 과 무관)
//! - `unlock`: SETO 진화 상태 및 잠금 해제 요구사항
//! - `knowledge`: 지식 그래프 쿼리 인터페이스
//! - `error`: language/intent meta 에러 타입 (legacy 이름 `LlmError`)
//! - `config`: language/intent meta generation 설정 (legacy 이름 `GenerationConfig`)
//! - `filter`: 응답 필터
//! - `prompt`: 프롬프트 템플릿
//! - `emotion`: PAD 기반 감정 상태
//! - `verification`: CT 검증 요청/응답
//! - `query_builder`: SETO 쿼리 빌더 (Fluent API)
//! - `tokenizer`: 토큰 카운터 유틸리티
//! - `metrics`: language/intent meta 메트릭 타입 (legacy 이름 "LLM 메트릭")
//! - `context`: 대화 맥락 타입
//! - `intention`: 의도/목표 시스템 (White Box AGI)

pub mod context;
pub mod error;
pub mod intention;
pub mod truth_domain;
pub mod verification;

pub use context::MessageRole;
pub use error::{LlmError, LlmResult};
pub use intention::{Constraint, ConstraintType, Goal, GoalType, SystemDiagnosticState};

pub use truth_domain::TruthDomain;
pub use verification::{
  ProofStep, SetoNodeId, TruthVerification, VerificationRequest, VerificationResponse,
};
