//! Event Loop 구조 정의
//!
//! pnix-old의 pnix_event_loop/src/lib.rs, dispatcher.rs, handler.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! - EventLoopConfig: 이벤트 루프 설정 구조 정의
//! - InputState: 입력 상태 구조 정의
//! - Event: 이벤트 타입 enum 정의
//! - EventHandler: 이벤트 핸들러 트레이트 정의 (구조만)
//! - HandlerPriority: 핸들러 우선순위 enum 정의
//! - HandlerId: 핸들러 ID 타입 정의
//! - HandlerInfo: 핸들러 정보 구조 정의
//! - 실제 이벤트 루프 실행, 디스패치, 핸들러 등록/실행 로직은 executor에서 구현

use serde::{Deserialize, Serialize};

/// 이벤트 루프 설정 (순수 데이터)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventLoopConfig {
  /// 목표 프레임률 (0 = 무제한)
  pub target_fps: u32,
  /// 물리 시뮬레이션 고정 타임스텝 (초)
  pub fixed_timestep: f64,
  /// 최대 프레임 스킵 (캐치업 제한)
  pub max_frame_skip: u32,
  /// Vsync 활성화
  pub vsync: bool,
  /// 지연 평가 활성화 (Houdini cook 스타일)
  pub lazy_eval: bool,
}

impl Default for EventLoopConfig {
  fn default() -> Self {
    Self {
      target_fps: 60,
      fixed_timestep: 1.0 / 60.0,
      max_frame_skip: 5,
      vsync: true,
      lazy_eval: true,
    }
  }
}

/// 입력 상태 (순수 데이터)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InputState {
  /// 마우스 X 위치
  pub mouse_x: f64,
  /// 마우스 Y 위치
  pub mouse_y: f64,
  /// 마우스 왼쪽 버튼
  pub mouse_left: bool,
  /// 마우스 오른쪽 버튼
  pub mouse_right: bool,
  /// 마우스 가운데 버튼
  pub mouse_middle: bool,
  /// 마우스 스크롤 델타
  pub scroll_delta: f64,
  /// 현재 눌린 키들
  pub keys_pressed: Vec<String>,
  /// 이번 프레임에 눌린 키들
  pub keys_just_pressed: Vec<String>,
  /// 이번 프레임에 떼어진 키들
  pub keys_just_released: Vec<String>,
}

/// 이벤트 타입
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Event {
  /// 키보드 이벤트
  KeyPress { key: String },
  /// 키보드 릴리스 이벤트
  KeyRelease { key: String },
  /// 마우스 이동 이벤트
  MouseMove { x: f64, y: f64 },
  /// 마우스 클릭 이벤트
  MouseClick { button: MouseButton, x: f64, y: f64 },
  /// 마우스 릴리스 이벤트
  MouseRelease { button: MouseButton, x: f64, y: f64 },
  /// 마우스 스크롤 이벤트
  MouseScroll { delta: f64 },
  /// 윈도우 리사이즈 이벤트
  WindowResize { width: u32, height: u32 },
  /// 프레임 업데이트 이벤트
  FrameUpdate { delta_time: f64 },
  /// 커스텀 이벤트
  Custom { name: String, data: String },
}

/// 마우스 버튼
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MouseButton {
  /// 왼쪽 버튼
  Left,
  /// 오른쪽 버튼
  Right,
  /// 가운데 버튼
  Middle,
}

/// 핸들러 우선순위
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum HandlerPriority {
  /// 최우선 (시스템 핸들러)
  Critical = 0,
  /// 높음 (UI 핸들러)
  High = 1,
  /// 보통 (기본)
  Normal = 2,
  /// 낮음 (백그라운드)
  Low = 3,
}

impl Default for HandlerPriority {
  fn default() -> Self {
    Self::Normal
  }
}

/// 핸들러 ID 타입
pub type HandlerId = u64;

/// 핸들러 정보 구조 (순수 데이터)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandlerInfo {
  /// 핸들러 ID
  pub id: HandlerId,
  /// 핸들러 이름
  pub name: String,
  /// 핸들러 우선순위
  pub priority: HandlerPriority,
  /// 핸들러 활성화 여부
  pub enabled: bool,
  /// 핸들러가 처리하는 이벤트 타입
  pub event_types: Vec<String>,
}

impl HandlerInfo {
  /// 새로운 핸들러 정보 생성
  pub fn new(id: HandlerId, name: impl Into<String>) -> Self {
    Self {
      id,
      name: name.into(),
      priority: HandlerPriority::default(),
      enabled: true,
      event_types: Vec::new(),
    }
  }

  /// 우선순위 설정 (구조 변경만)
  pub fn with_priority(mut self, priority: HandlerPriority) -> Self {
    self.priority = priority;
    self
  }

  /// 이벤트 타입 추가 (구조 변경만)
  pub fn with_event_type(mut self, event_type: impl Into<String>) -> Self {
    self.event_types.push(event_type.into());
    self
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 구조체와 메서드들은 executor/runtime 계층에서 구현하세요:
// - EventDispatcher 구조체 및 메서드들 (dispatch, register_handler, unregister_handler 등)
// - HandlerManager 구조체 및 메서드들 (register, unregister, get_handler 등)
// - World 구조체 및 메서드들 (step, update, render 등)
// - InputState::update_params() (런타임 상태 업데이트)
// - InputState::end_frame() (프레임 종료 시 상태 정리)
// - EventHandler 트레이트 구현 (handle 메서드)
//
// 이 함수들은 이벤트 루프 실행, 상태 관리, 또는 값 계산을 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_event_loop_config_default() {
    let config = EventLoopConfig::default();
    assert_eq!(config.target_fps, 60);
    assert_eq!(config.fixed_timestep, 1.0 / 60.0);
  }

  #[test]
  fn test_input_state_default() {
    let state = InputState::default();
    assert_eq!(state.mouse_x, 0.0);
    assert_eq!(state.mouse_y, 0.0);
  }

  #[test]
  fn test_handler_info_creation() {
    let info = HandlerInfo::new(1, "test_handler")
      .with_priority(HandlerPriority::High)
      .with_event_type("KeyPress");
    assert_eq!(info.id, 1);
    assert_eq!(info.name, "test_handler");
    assert_eq!(info.priority, HandlerPriority::High);
  }
}
