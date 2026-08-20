//! # OWNER-LAW NOTE (2026-05-10)
//!
//! **pnix 는 LLM 없이 작동하는 self-sufficient deterministic AI 지능이다.**
//! 정본 헌법은 `CLAUDE.md` 의 `OWNER-LAW CONSTITUTION` section.
//!
//! 이 file 의 doc/comment 에 "LLM as X" / "LLM 이 ..." / "LLM 모드 / 연동 /
//! 호출 / 정제" 류 framing 이 남아있어도 모두 owner-law 위반으로
//! *superseded* 다. substrate 안에 LLM 자리 *없음*. 외부 LLM output 은
//! untrusted external prose 일 뿐 substrate enumeration category 가
//! 아님. 본문은 legacy framing 의 history retention.
//!
//! 대화 맥락 타입 정의 (순수 구조)
//!
//! pnix-old의 pnix_llm/src/context.rs에서 마이그레이션.
//! 순수 타입만 유지, system time 의존 타입(Message, ContextWindow, ConversationContext)은
//! executor에서 구현.

use serde::{Deserialize, Serialize};

/// 메시지 역할: 대화 메시지의 역할 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageRole {
  /// 사용자 메시지 (사용자가 입력한 메시지)
  User,
  /// 어시스턴트(LLM) 응답 (LLM이 생성한 응답)
  Assistant,
  /// 시스템 프롬프트 (시스템 레벨 지시사항)
  System,
}

impl MessageRole {
  /// 역할 이름 문자열 반환
  pub fn as_str(&self) -> &'static str {
    match self {
      MessageRole::User => "user",
      MessageRole::Assistant => "assistant",
      MessageRole::System => "system",
    }
  }

  /// 문자열에서 역할 파싱
  #[allow(clippy::should_implement_trait)]
  pub fn from_str(s: &str) -> Option<Self> {
    match s.to_lowercase().as_str() {
      "user" => Some(MessageRole::User),
      "assistant" => Some(MessageRole::Assistant),
      "system" => Some(MessageRole::System),
      _ => None,
    }
  }
}

impl std::fmt::Display for MessageRole {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.as_str())
  }
}

// NOTE: 런타임 의존 타입 (Message, ContextWindow, ConversationContext)은 executor에서 구현합니다.
// - Message: system time 의존 (timestamp 필드)
// - ContextWindow: 런타임 상태 관리 (VecDeque, token counting)
// - ConversationContext: system time 의존 (last_updated), EmotionalState/SetoEvolutionState 상태 관리
// (pnix-core는 실행/상태 머신을 포함하지 않음)

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_message_role_as_str() {
    assert_eq!(MessageRole::User.as_str(), "user");
    assert_eq!(MessageRole::Assistant.as_str(), "assistant");
    assert_eq!(MessageRole::System.as_str(), "system");
  }

  #[test]
  fn test_message_role_from_str() {
    assert_eq!(MessageRole::from_str("user"), Some(MessageRole::User));
    assert_eq!(MessageRole::from_str("USER"), Some(MessageRole::User));
    assert_eq!(
      MessageRole::from_str("assistant"),
      Some(MessageRole::Assistant)
    );
    assert_eq!(MessageRole::from_str("system"), Some(MessageRole::System));
    assert_eq!(MessageRole::from_str("unknown"), None);
  }

  #[test]
  fn test_message_role_display() {
    assert_eq!(format!("{}", MessageRole::User), "user");
    assert_eq!(format!("{}", MessageRole::Assistant), "assistant");
  }
}
