//! # Event Bus
//!
//! 구조화된 로그 이벤트 버스
//!
//! 모든 핵심 컴포넌트(`ProblemSolver`, `AutonomousAgent`, `m-pnix` 등)가
//! 자신의 상태/결정을 구조화된 로그 이벤트로 발행하고,
//! 구독자(콘솔 UI 등)가 이를 구독하여 실시간으로 표시할 수 있습니다.
//!
//! ## 이벤트 형식
//!
//! ```rust
//! use std::collections::HashMap;
//! use serde_json::json;
//! use pnix_core::event_bus::{LogEvent, LogLevel};
//!
//! let mut data = HashMap::new();
//! data.insert("goal".to_string(), json!("deploy-website"));
//! data.insert("plan_id".to_string(), json!("plan_123"));
//!
//! let _event = LogEvent {
//!     source: "ProblemSolver".to_string(),
//!     action: "create-plan".to_string(),
//!     data,
//!     timestamp: 1234567890,
//!     level: LogLevel::Info,
//!     task_id: None,
//! };
//! ```
//!
//! ## Clojure-like 렌더링
//!
//! 이벤트는 다음과 같이 렌더링됩니다:
//! `(problem-solver create-plan 'deploy-website' plan-id 'plan_123')`

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// 로그 이벤트: 구조화된 로그 이벤트 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
  /// 이벤트 소스 (컴포넌트 이름)
  pub source: String,
  /// 액션 이름
  pub action: String,
  /// 이벤트 데이터 (키-값 쌍)
  pub data: HashMap<String, serde_json::Value>,
  /// 타임스탬프 (단조 증가 카운터, executor에서 덮어쓸 수 있음)
  pub timestamp: u64,
  /// 이벤트 레벨 (info, debug, warn, error)
  #[serde(default)]
  pub level: LogLevel,
  /// L-11.3: 작업 ID (동시성 로그를 위한 task_id)
  /// 메인 에이전트 루프, JIT 컴파일러, KnowledgeAbsorber 등 각기 다른 스레드/태스크는 고유한 ID를 부여받습니다.
  #[serde(default)]
  pub task_id: Option<u64>,
}

/// 로그 레벨: 로그 이벤트의 심각도 레벨 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LogLevel {
  /// Debug 레벨 (디버깅 정보)
  Debug,
  /// Info 레벨 (일반 정보, 기본값)
  #[default]
  Info,
  /// Warn 레벨 (경고)
  Warn,
  /// Error 레벨 (에러)
  Error,
}

/// 이벤트 구독자 트레이트: 이벤트 버스에서 이벤트를 수신하는 구독자 트레이트
pub trait EventSubscriber: Send + Sync {
  /// 이벤트 수신 처리
  fn on_event(&self, event: &LogEvent);
}

/// 이벤트 버스: 구조화된 로그 이벤트를 발행하고 구독하는 이벤트 버스
#[derive(Clone)]
pub struct EventBus {
  /// 구독자 목록
  subscribers: Arc<Mutex<Vec<Arc<dyn EventSubscriber>>>>,
  /// 이벤트 버퍼 (최근 N개 이벤트 보관, VecDeque 사용으로 O(1) 삽입/삭제)
  buffer: Arc<Mutex<VecDeque<LogEvent>>>,
  /// 버퍼 최대 크기
  buffer_size: usize,
  /// I5.1 scaffold: last buffer index per source (batch/indexing lane)
  source_tail_index: Arc<Mutex<HashMap<String, usize>>>,
  /// 결정적 타임스탬프 카운터
  timestamp_counter: Arc<AtomicU64>,
}

impl EventBus {
  /// 새 이벤트 버스 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(buffer_size: usize) -> Self {
    Self {
      subscribers: Arc::new(Mutex::new(Vec::new())),
      buffer: Arc::new(Mutex::new(VecDeque::with_capacity(buffer_size))),
      buffer_size,
      source_tail_index: Arc::new(Mutex::new(HashMap::new())),
      timestamp_counter: Arc::new(AtomicU64::new(0)),
    }
  }

  /// 타임스탬프 카운터 증가 (내부 함수)
  fn next_timestamp(&self) -> u64 {
    self.timestamp_counter.fetch_add(1, Ordering::Relaxed)
  }

  /// L-11.3: 작업 ID를 지정하여 이벤트 발행
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn publish_with_task_id(
    &self,
    source: impl Into<String>,
    action: impl Into<String>,
    data: HashMap<String, serde_json::Value>,
    task_id: u64,
  ) {
    let timestamp = self.next_timestamp();

    let event = LogEvent {
      source: source.into(),
      action: action.into(),
      data,
      timestamp,
      level: LogLevel::Info,
      task_id: Some(task_id),
    };

    // 구독자에게 전달
    if let Ok(subscribers) = self.subscribers.lock() {
      for subscriber in subscribers.iter() {
        subscriber.on_event(&event);
      }
    }

    // 버퍼에 추가 (VecDeque 사용으로 O(1) 성능)
    if let Ok(mut buffer) = self.buffer.lock() {
      buffer.push_back(event.clone());
      let index = buffer.len().saturating_sub(1);
      self.record_source_tail(&event.source, index);
      // 버퍼 크기 초과 시 오래된 이벤트 제거 (O(1))
      if buffer.len() > self.buffer_size {
        buffer.pop_front();
      }
    }
  }

  /// 이벤트 발행
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn publish(
    &self,
    source: impl Into<String>,
    action: impl Into<String>,
    data: HashMap<String, serde_json::Value>,
  ) {
    let timestamp = self.next_timestamp();

    let event = LogEvent {
      source: source.into(),
      action: action.into(),
      data,
      timestamp,
      level: LogLevel::Info,
      task_id: None, // 기본값: task_id 없음
    };

    // 구독자에게 전달
    if let Ok(subscribers) = self.subscribers.lock() {
      for subscriber in subscribers.iter() {
        subscriber.on_event(&event);
      }
    }

    // 버퍼에 추가 (VecDeque 사용으로 O(1) 성능)
    if let Ok(mut buffer) = self.buffer.lock() {
      buffer.push_back(event.clone());
      let index = buffer.len().saturating_sub(1);
      self.record_source_tail(&event.source, index);
      // 버퍼 크기 초과 시 오래된 이벤트 제거 (O(1))
      if buffer.len() > self.buffer_size {
        buffer.pop_front();
      }
    }
  }

  fn record_source_tail(&self, source: &str, index: usize) {
    if let Ok(mut map) = self.source_tail_index.lock() {
      map.insert(source.to_string(), index);
    }
  }

  /// I5.4: publish multiple events with one subscribers lock + one buffer lock.
  pub fn publish_batch(&self, events: Vec<(String, String, HashMap<String, serde_json::Value>)>) {
    if events.is_empty() {
      return;
    }
    let mut built = Vec::with_capacity(events.len());
    for (source, action, data) in events {
      built.push(LogEvent {
        source,
        action,
        data,
        timestamp: self.next_timestamp(),
        level: LogLevel::Info,
        task_id: None,
      });
    }
    if let Ok(subscribers) = self.subscribers.lock() {
      for event in &built {
        for subscriber in subscribers.iter() {
          subscriber.on_event(event);
        }
      }
    }
    if let Ok(mut buffer) = self.buffer.lock() {
      for event in built {
        buffer.push_back(event.clone());
        let index = buffer.len().saturating_sub(1);
        self.record_source_tail(&event.source, index);
        if buffer.len() > self.buffer_size {
          buffer.pop_front();
        }
      }
    }
  }

  /// I5.1 scaffold: O(1) lookup of latest buffer index for a source.
  pub fn latest_source_buffer_index(&self, source: &str) -> Option<usize> {
    self
      .source_tail_index
      .lock()
      .ok()
      .and_then(|map| map.get(source).copied())
  }

  /// 레벨을 지정하여 이벤트 발행
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn publish_with_level(
    &self,
    source: impl Into<String>,
    action: impl Into<String>,
    data: HashMap<String, serde_json::Value>,
    level: LogLevel,
  ) {
    let timestamp = self.next_timestamp();

    let event = LogEvent {
      source: source.into(),
      action: action.into(),
      data,
      timestamp,
      level,
      task_id: None, // 기본값: task_id 없음
    };

    // 구독자에게 전달
    if let Ok(subscribers) = self.subscribers.lock() {
      for subscriber in subscribers.iter() {
        subscriber.on_event(&event);
      }
    }

    // 버퍼에 추가 (VecDeque 사용으로 O(1) 성능)
    if let Ok(mut buffer) = self.buffer.lock() {
      buffer.push_back(event.clone());
      // 버퍼 크기 초과 시 오래된 이벤트 제거 (O(1))
      if buffer.len() > self.buffer_size {
        buffer.pop_front();
      }
    }
  }

  /// 구독자 등록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn subscribe(&self, subscriber: Arc<dyn EventSubscriber>) {
    if let Ok(mut subscribers) = self.subscribers.lock() {
      subscribers.push(subscriber);
    }
  }

  /// 최근 이벤트 조회 (성능 최적화: 필요한 만큼만 복사)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn recent_events(&self, count: usize) -> Vec<LogEvent> {
    if let Ok(buffer) = self.buffer.lock() {
      let start = buffer.len().saturating_sub(count);
      buffer.iter().skip(start).cloned().collect()
    } else {
      Vec::new()
    }
  }

  /// 모든 이벤트 조회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn all_events(&self) -> Vec<LogEvent> {
    if let Ok(buffer) = self.buffer.lock() {
      buffer.iter().cloned().collect()
    } else {
      Vec::new()
    }
  }

  /// 버퍼 비우기
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 제거만, 값 계산 없음
  pub fn clear(&self) {
    if let Ok(mut buffer) = self.buffer.lock() {
      buffer.clear();
    }
  }
}

impl Default for EventBus {
  fn default() -> Self {
    Self::new(1000)
  }
}

/// 전역 이벤트 버스 (싱글톤)
static GLOBAL_EVENT_BUS: OnceLock<Arc<EventBus>> = OnceLock::new();

use std::sync::OnceLock;

/// 전역 이벤트 버스 초기화
///
/// 지정된 버퍼 크기로 전역 이벤트 버스를 초기화합니다.
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 생성만, 값 계산 없음
pub fn init_global_event_bus(buffer_size: usize) -> Arc<EventBus> {
  GLOBAL_EVENT_BUS
    .get_or_init(|| Arc::new(EventBus::new(buffer_size)))
    .clone()
}

/// 전역 이벤트 버스 가져오기
///
/// 전역 이벤트 버스를 반환합니다. 아직 초기화되지 않았다면 기본 설정으로 초기화합니다.
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 접근만, 값 계산 없음
pub fn global_event_bus() -> Arc<EventBus> {
  GLOBAL_EVENT_BUS
    .get_or_init(|| Arc::new(EventBus::default()))
    .clone()
}

/// 전역 이벤트 버스에 이벤트 발행 (편의 함수)
///
/// 전역 이벤트 버스에 이벤트를 발행하는 편의 함수입니다.
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 추가만, 값 계산 없음
pub fn publish_event(
  source: impl Into<String>,
  action: impl Into<String>,
  data: HashMap<String, serde_json::Value>,
) {
  global_event_bus().publish(source, action, data);
}

/// 전역 이벤트 버스에 레벨을 지정하여 이벤트 발행 (편의 함수)
///
/// 전역 이벤트 버스에 지정된 레벨로 이벤트를 발행하는 편의 함수입니다.
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 추가만, 값 계산 없음
pub fn publish_event_with_level(
  source: impl Into<String>,
  action: impl Into<String>,
  data: HashMap<String, serde_json::Value>,
  level: LogLevel,
) {
  global_event_bus().publish_with_level(source, action, data, level);
}

/// Clojure-like 문자열로 렌더링
///
/// 이벤트를 Clojure 스타일의 S-표현식으로 렌더링합니다.
/// 예: `(problem-solver create-plan 'deploy-website' plan-id 'plan_123')`
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 생성만, 파일 I/O 없음
pub fn render_clojure_like(event: &LogEvent) -> String {
  // CamelCase를 kebab-case로 변환하는 헬퍼 함수
  fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
      if c.is_uppercase() && i > 0 {
        result.push('-');
      }
      result.push(c.to_ascii_lowercase());
    }
    result.replace("_", "-")
  }

  let mut parts = vec![
    format!("({}", to_kebab_case(&event.source)),
    to_kebab_case(&event.action),
  ];

  // 데이터를 키-값 쌍으로 추가
  for (key, value) in &event.data {
    let key_str = key.to_lowercase().replace("_", "-");
    let value_str = match value {
      serde_json::Value::String(s) => format!("'{}'", s),
      serde_json::Value::Number(n) => n.to_string(),
      serde_json::Value::Bool(b) => b.to_string(),
      serde_json::Value::Null => "nil".to_string(),
      _ => format!("{:?}", value),
    };
    parts.push(format!("{} {}", key_str, value_str));
  }

  parts.join(" ") + ")"
}

fn run_i5_4_batch_hot_path_self_test() -> bool {
  let bus = EventBus::new(16);
  let batch = vec![
    ("jarvis".into(), "lift".into(), HashMap::new()),
    ("jarvis".into(), "plan".into(), HashMap::new()),
    ("jarvis".into(), "execute".into(), HashMap::new()),
  ];
  bus.publish_batch(batch);
  bus.recent_events(3).len() == 3 && bus.latest_source_buffer_index("jarvis") == Some(2)
}

static I5_4_SELF_TEST: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// I5.4: EventBus batch publish hot-path install verification for tier5 inventory.
pub fn i5_4_jarvis_batch_hot_path_install_verified() -> bool {
  *I5_4_SELF_TEST.get_or_init(run_i5_4_batch_hot_path_self_test)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_publish_batch_and_source_index() {
    let bus = EventBus::new(8);
    let batch = vec![
      ("A".into(), "one".into(), HashMap::new()),
      ("A".into(), "two".into(), HashMap::new()),
    ];
    bus.publish_batch(batch);
    assert_eq!(bus.recent_events(2).len(), 2);
    assert_eq!(bus.latest_source_buffer_index("A"), Some(1));
  }

  #[test]
  fn test_event_bus() {
    let bus = EventBus::new(10);
    let mut data = HashMap::new();
    data.insert(
      "goal".to_string(),
      serde_json::Value::String("test".to_string()),
    );

    bus.publish("ProblemSolver", "create-plan", data);

    let events = bus.recent_events(1);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].source, "ProblemSolver");
    assert_eq!(events[0].action, "create-plan");
  }

  #[test]
  fn test_clojure_rendering() {
    let mut data = HashMap::new();
    data.insert(
      "goal".to_string(),
      serde_json::Value::String("deploy-website".to_string()),
    );
    data.insert(
      "plan_id".to_string(),
      serde_json::Value::String("plan_123".to_string()),
    );

    let event = LogEvent {
      source: "ProblemSolver".to_string(),
      action: "create-plan".to_string(),
      data,
      timestamp: 1234567890,
      level: LogLevel::Info,
      task_id: None,
    };

    let rendered = render_clojure_like(&event);
    assert!(rendered.contains("problem-solver"));
    assert!(rendered.contains("create-plan"));
    assert!(rendered.contains("deploy-website"));
  }
}
