//! 보안 관련 타입 정의 (순수 구조)
//!
//! pnix-old의 pnix_utils/src/auth.rs, crypto.rs에서 마이그레이션.
//! 런타임 로직(토큰 생성/검증, 해싱, 암호화)은 executor에서 구현.

use core::fmt;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// ============================================================================
// Auth Types
// ============================================================================

/// 인증 에러: 인증/권한 관련 에러 타입
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthError {
  /// 잘못된 자격 증명
  InvalidCredentials,
  /// 토큰 만료
  TokenExpired,
  /// 토큰 검증 실패
  InvalidToken(
    /// 실패 이유
    String,
  ),
  /// 권한 없음
  PermissionDenied,
  /// 세션 없음
  SessionNotFound,
  /// 세션 만료
  SessionExpired,
  /// 사용자 없음
  UserNotFound,
  /// 내부 에러
  InternalError(
    /// 에러 메시지
    String,
  ),
}

impl fmt::Display for AuthError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      AuthError::InvalidCredentials => write!(f, "Invalid credentials"),
      AuthError::TokenExpired => write!(f, "Token expired"),
      AuthError::InvalidToken(msg) => write!(f, "Invalid token: {}", msg),
      AuthError::PermissionDenied => write!(f, "Permission denied"),
      AuthError::SessionNotFound => write!(f, "Session not found"),
      AuthError::SessionExpired => write!(f, "Session expired"),
      AuthError::UserNotFound => write!(f, "User not found"),
      AuthError::InternalError(msg) => write!(f, "Internal error: {}", msg),
    }
  }
}

impl std::error::Error for AuthError {}

/// 토큰 클레임: JWT-like 토큰의 클레임 구조
///
/// 생성 시 시간 설정은 executor에서 수행합니다.
/// 헌법 P0-1 준수: 구조 정의만, 값 계산 없음
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenClaims {
  /// 주체 (사용자 ID)
  pub subject: String,
  /// 발급 시간 (Unix timestamp)
  pub issued_at: u64,
  /// 만료 시간 (Unix timestamp)
  pub expires_at: u64,
  /// 발급자
  pub issuer: Option<String>,
  /// 추가 클레임
  pub custom_claims: HashMap<String, String>,
}

impl TokenClaims {
  /// 직접 값으로 클레임 생성 (시간 계산 없음)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn with_times(subject: impl Into<String>, issued_at: u64, expires_at: u64) -> Self {
    Self {
      subject: subject.into(),
      issued_at,
      expires_at,
      issuer: None,
      custom_claims: HashMap::new(),
    }
  }

  /// 발급자 설정
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 설정만, 값 계산 없음
  pub fn with_issuer(mut self, issuer: impl Into<String>) -> Self {
    self.issuer = Some(issuer.into());
    self
  }

  /// 커스텀 클레임 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn with_claim(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
    self.custom_claims.insert(key.into(), value.into());
    self
  }

  /// 만료 여부 확인 (현재 시간을 외부에서 제공)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_expired(&self, current_time: u64) -> bool {
    current_time >= self.expires_at
  }
}

/// 비밀번호 강도: 비밀번호의 강도 레벨 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PasswordStrength {
  /// 매우 약함
  VeryWeak,
  /// 약함
  Weak,
  /// 보통
  Fair,
  /// 강함
  Strong,
  /// 매우 강함
  VeryStrong,
}

/// 권한: 리소스에 대한 액션 권한 구조
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Permission {
  /// 리소스 이름
  pub resource: String,
  /// 액션 이름
  pub action: String,
}

impl Permission {
  /// 새 권한 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(resource: impl Into<String>, action: impl Into<String>) -> Self {
    Self {
      resource: resource.into(),
      action: action.into(),
    }
  }

  /// 문자열에서 권한 파싱 ("resource:action")
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 파싱만, 값 계산 없음
  #[allow(clippy::should_implement_trait)]
  pub fn from_str(s: &str) -> Option<Self> {
    let parts: Vec<&str> = s.splitn(2, ':').collect();
    if parts.len() == 2 {
      Some(Self::new(parts[0], parts[1]))
    } else {
      None
    }
  }
}

impl fmt::Display for Permission {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}:{}", self.resource, self.action)
  }
}

/// 역할: 권한 집합을 가진 역할 구조
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
  /// 역할 이름
  pub name: String,
  /// 권한 목록
  pub permissions: HashSet<Permission>,
}

impl Role {
  /// 새 역할 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      permissions: HashSet::new(),
    }
  }

  /// 권한 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn add_permission(&mut self, permission: Permission) {
    self.permissions.insert(permission);
  }

  /// 권한 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn has_permission(&self, resource: &str, action: &str) -> bool {
    self
      .permissions
      .iter()
      .any(|p| p.resource == resource && p.action == action)
  }
}

// ============================================================================
// Crypto Types
// ============================================================================

/// 암호화 에러: 암호화/복호화 관련 에러 타입
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CryptoError {
  /// 잘못된 입력
  InvalidInput(
    /// 에러 메시지
    String,
  ),
  /// 키 크기 오류
  InvalidKeySize,
  /// 논스 크기 오류
  InvalidNonceSize,
  /// 복호화 실패
  DecryptionFailed,
}

impl fmt::Display for CryptoError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      CryptoError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
      CryptoError::InvalidKeySize => write!(f, "Invalid key size"),
      CryptoError::InvalidNonceSize => write!(f, "Invalid nonce size"),
      CryptoError::DecryptionFailed => write!(f, "Decryption failed"),
    }
  }
}

impl std::error::Error for CryptoError {}

// NOTE: 런타임 로직 (TokenManager, SessionStore, PermissionManager, 해싱, 암호화 함수 등)은
// executor/runtime 계층에서 구현합니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_auth_error_display() {
    assert_eq!(
      format!("{}", AuthError::InvalidCredentials),
      "Invalid credentials"
    );
    assert_eq!(format!("{}", AuthError::TokenExpired), "Token expired");
  }

  #[test]
  fn test_token_claims() {
    let claims = TokenClaims::with_times("user123", 1000, 2000)
      .with_issuer("pnix")
      .with_claim("role", "admin");

    assert_eq!(claims.subject, "user123");
    assert_eq!(claims.issuer, Some("pnix".to_string()));
    assert!(!claims.is_expired(1500));
    assert!(claims.is_expired(2000));
  }

  #[test]
  fn test_permission() {
    let perm = Permission::new("users", "read");
    assert_eq!(format!("{}", perm), "users:read");

    let parsed = Permission::from_str("users:write").unwrap();
    assert_eq!(parsed.resource, "users");
    assert_eq!(parsed.action, "write");
  }

  #[test]
  fn test_role() {
    let mut role = Role::new("admin");
    role.add_permission(Permission::new("users", "read"));
    role.add_permission(Permission::new("users", "write"));

    assert!(role.has_permission("users", "read"));
    assert!(!role.has_permission("users", "delete"));
  }

  #[test]
  fn test_crypto_error_display() {
    assert_eq!(
      format!("{}", CryptoError::InvalidKeySize),
      "Invalid key size"
    );
    assert_eq!(
      format!("{}", CryptoError::DecryptionFailed),
      "Decryption failed"
    );
  }
}
