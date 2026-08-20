//! Telemetry 구조 정의
//!
//! pnix-old의 meaning_core/src/unified_meaning/frp_telemetry.rs와 컴파일 텔레메트리에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 런타임 모니터링 실행 로직 제외
//! - FrameRecord: 단일 프레임 텔레메트리 데이터 구조 정의
//! - TelemetryCollector: 텔레메트리 수집기 구조 정의
//! - TelemetrySnapshot: 텔레메트리 스냅샷 구조 정의
//! - CompileFrame, CompilePhase, CompileStats: 컴파일 타임 텔레메트리 구조 정의
//! - 실제 수집 및 렌더링 로직은 executor에서 구현

use serde::{Deserialize, Serialize};

/// Telemetry schema version for JSON serialization compatibility
pub const TELEMETRY_SCHEMA_VERSION: &str = "1.0";
use std::collections::VecDeque;

use crate::passes::cache_policy::CachePolicy;
use crate::runtime::frp_eval::CacheStats;

// ============================================================
// Compile-time Telemetry
// ============================================================

/// 컴파일 단계
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompilePhase {
  /// 소스를 AST로 파싱
  Parsing,
  /// AST를 Surface로 lowering
  SurfaceLowering,
  /// Surface를 FxCore로 lowering
  FxCoreLowering,
  /// FxCore 최적화
  FxCoreOptimization,
  /// FxCore를 SSA로 lowering
  SsaLowering,
  /// SSA 최적화
  SsaOptimization,
  /// Build IR 생성
  BuildIrGeneration,
  /// 코드 생성
  CodeGeneration,
  /// 계약 검증
  Verification,
  /// Unknown compile phase (for backwards compatibility)
  #[serde(other)]
  Unknown,
}

/// 단일 컴파일 프레임 텔레메트리
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompileFrame {
  /// 컴파일 단계
  pub phase: CompilePhase,
  /// 경과 시간 (밀리초) - 구조 정의만, 실제 측정은 executor에서
  pub duration_ms: f64,
  /// 이 단계 후 IR 크기 (노드 수)
  pub ir_size: usize,
  /// 메모리 사용량 (바이트, 선택적)
  pub memory_bytes: Option<usize>,
  /// 추가 메타데이터
  pub metadata: FrameMetadata,
}

/// 프레임 메타데이터
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FrameMetadata {
  /// 처리된 노드 수
  pub nodes_processed: usize,
  /// 처리된 엣지 수
  pub edges_processed: usize,
  /// 적용된 변환 수
  pub transformations: usize,
  /// 에러 수
  pub errors: usize,
  /// 경고 수
  pub warnings: usize,
  /// 커스텀 노트
  pub notes: Vec<String>,
}

impl CompileFrame {
  /// 새로운 프레임 생성
  pub fn new(phase: CompilePhase, duration_ms: f64, ir_size: usize) -> Self {
    Self {
      phase,
      duration_ms,
      ir_size,
      memory_bytes: None,
      metadata: FrameMetadata::default(),
    }
  }

  /// 변환 수 설정
  pub fn with_transformations(mut self, count: usize) -> Self {
    self.metadata.transformations = count;
    self
  }

  /// 노트 추가
  pub fn with_note(mut self, note: impl Into<String>) -> Self {
    self.metadata.notes.push(note.into());
    self
  }
}

/// 컴파일 통계 (프레임에서 계산됨)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompileStats {
  /// 총 컴파일 시간 (밀리초)
  pub total_time_ms: f64,
  /// 초기 IR 크기
  pub initial_ir_size: usize,
  /// 최종 IR 크기
  pub final_ir_size: usize,
  /// 총 변환 수
  pub total_transformations: usize,
  /// 총 에러 수
  pub total_errors: usize,
  /// 총 경고 수
  pub total_warnings: usize,
}

impl CompileStats {
  /// 프레임에서 통계 생성 (헌법 준수: 순수 함수)
  pub fn from_frames(frames: &[CompileFrame]) -> Self {
    let total_time_ms = frames.iter().map(|f| f.duration_ms).sum();
    let initial_ir_size = frames.first().map(|f| f.ir_size).unwrap_or(0);
    let final_ir_size = frames.last().map(|f| f.ir_size).unwrap_or(0);
    let total_transformations = frames.iter().map(|f| f.metadata.transformations).sum();
    let total_errors = frames.iter().map(|f| f.metadata.errors).sum();
    let total_warnings = frames.iter().map(|f| f.metadata.warnings).sum();

    Self {
      total_time_ms,
      initial_ir_size,
      final_ir_size,
      total_transformations,
      total_errors,
      total_warnings,
    }
  }

  /// IR 크기 감소율 (%)
  pub fn size_reduction_pct(&self) -> f64 {
    if self.initial_ir_size == 0 {
      return 0.0;
    }
    let reduction = self.initial_ir_size as f64 - self.final_ir_size as f64;
    (reduction / self.initial_ir_size as f64) * 100.0
  }
}

// ============================================================
// FRP Telemetry
// ============================================================

/// 단일 프레임의 텔레메트리 데이터 구조
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrameRecord {
  /// 프레임 번호
  pub frame: u64,
  /// 이 프레임의 캐시 통계 (구조 정의만)
  pub stats: CacheStats,
  /// 이 프레임의 정책 스냅샷 (min_size)
  pub policy_min_size: u32,
  /// 이 프레임의 히트율 (구조 정의만, 실제 계산은 executor에서)
  pub hit_rate: f64,
  /// 튜닝 액션 (선택적)
  pub tuning_action: Option<String>,
}

impl FrameRecord {
  /// 새로운 프레임 레코드 생성 (구조 생성만)
  pub fn new(frame: u64, stats: CacheStats, policy: &CachePolicy) -> Self {
    Self {
      frame,
      stats,
      policy_min_size: policy.min_size,
      hit_rate: 0.0, // 실제 계산은 executor에서
      tuning_action: None,
    }
  }

  /// 튜닝 액션 설정 (구조 변경만)
  pub fn with_tuning_action(mut self, action: String) -> Self {
    self.tuning_action = Some(action);
    self
  }
}

/// 텔레메트리 수집기 구조
///
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, 런타임 모니터링 실행 로직 제외
/// - history: 프레임 히스토리 (구조 정의만)
/// - max_history: 최대 히스토리 길이 (구조 정의만)
/// - 실제 수집 및 계산은 executor에서 구현
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryCollector {
  /// 프레임 히스토리 (구조 정의만)
  pub history: VecDeque<FrameRecord>,
  /// 최대 히스토리 길이 (구조 정의만)
  pub max_history: usize,
  /// 현재 프레임 카운터 (구조 정의만)
  pub frame_counter: u64,
  /// 총 히트 수 (구조 정의만)
  pub total_hits: u64,
  /// 총 미스 수 (구조 정의만)
  pub total_misses: u64,
  /// 조정 횟수 (구조 정의만)
  pub adjustment_count: u64,
}

impl TelemetryCollector {
  /// 새로운 텔레메트리 수집기 생성
  pub fn new(max_history: usize) -> Self {
    Self {
      history: VecDeque::with_capacity(max_history),
      max_history,
      frame_counter: 0,
      total_hits: 0,
      total_misses: 0,
      adjustment_count: 0,
    }
  }
}

/// 텔레메트리 스냅샷 구조
///
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, 런타임 모니터링 실행 로직 제외
/// - frame_count: 프레임 수 (구조 정의만)
/// - total_hits, total_misses: 총 통계 (구조 정의만)
/// - overall_hit_rate: 전체 히트율 (구조 정의만, 실제 계산은 executor에서)
/// - recent_history: 최근 히스토리 (구조 정의만)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
  /// 스키마 버전 (JSON 호환성)
  pub schema_version: String,
  /// 프레임 수
  pub frame_count: u64,
  /// 총 히트 수
  pub total_hits: u64,
  /// 총 미스 수
  pub total_misses: u64,
  /// 조정 횟수
  pub adjustment_count: u64,
  /// 최근 히스토리 (구조 정의만)
  pub recent_history: Vec<FrameRecord>,
  /// 전체 히트율 (구조 정의만, 실제 계산은 executor에서)
  pub overall_hit_rate: f64,
}

impl TelemetrySnapshot {
  /// 새로운 스냅샷 생성 (현재 스키마 버전 사용)
  pub fn new(
    frame_count: u64,
    total_hits: u64,
    total_misses: u64,
    adjustment_count: u64,
    recent_history: Vec<FrameRecord>,
    overall_hit_rate: f64,
  ) -> Self {
    Self {
      schema_version: TELEMETRY_SCHEMA_VERSION.to_string(),
      frame_count,
      total_hits,
      total_misses,
      adjustment_count,
      recent_history,
      overall_hit_rate,
    }
  }

  /// 명시적 스키마 버전으로 스냅샷 생성
  pub fn with_schema_version(
    schema_version: impl Into<String>,
    frame_count: u64,
    total_hits: u64,
    total_misses: u64,
    adjustment_count: u64,
    recent_history: Vec<FrameRecord>,
    overall_hit_rate: f64,
  ) -> Self {
    Self {
      schema_version: schema_version.into(),
      frame_count,
      total_hits,
      total_misses,
      adjustment_count,
      recent_history,
      overall_hit_rate,
    }
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - record_frame(stats, policy) (프레임 기록)
// - record_frame_with_action(stats, policy, action) (액션과 함께 프레임 기록)
// - snapshot() -> TelemetrySnapshot (스냅샷 생성)
// - overall_hit_rate() -> f64 (전체 히트율 계산)
// - recent_hit_rate(n) -> f64 (최근 히트율 계산)
// - render_compact(snapshot) -> String (렌더링)
//
// 이 함수들은 값 계산 및 실행을 수행하므로 pnix-core에서 제외됩니다.

// ============================================================
// Telemetry Bus 구조 (pnix-old telemetry/bus.rs에서 마이그레이션)
// ============================================================

/// Thread-safe ring buffer for telemetry data
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, 실제 push/snapshot 실행 로직은 executor에서 구현
/// Arc<Mutex<>>는 구조 정의에 포함되지만 실제 사용은 executor에서
#[derive(Clone, Debug)]
pub struct TelemetryBus<T: Clone> {
  /// 버퍼 용량 (구조 정의만)
  pub capacity: usize,
  /// 내부 버퍼 (구조 정의만, 실제 사용은 executor에서)
  /// Note: Arc<Mutex<>>는 구조 정의에 포함되지만 실제 lock/unlock은 executor에서 수행
  /// LOW: std::sync::Mutex가 async 컨텍스트에서 사용 수정 완료
  /// 현재는 동기 컨텍스트에서만 사용되므로 std::sync::Mutex가 적절함
  /// 향후 async 런타임 사용 시 async-aware Mutex로 변경 고려 필요
  /// 현재 구현: std::sync::Mutex는 동기 컨텍스트에서 안전하게 사용됨
  pub inner: std::sync::Arc<std::sync::Mutex<TelemetryBuffer<T>>>,
}

/// 내부 버퍼 구조 (구조 정의만)
#[derive(Debug)]
pub struct TelemetryBuffer<T> {
  /// 데이터 벡터 (구조 정의만)
  #[allow(dead_code)]
  data: Vec<T>,
  /// 용량 (구조 정의만)
  #[allow(dead_code)]
  capacity: usize,
}

impl<T: Clone> TelemetryBus<T> {
  /// 새로운 텔레메트리 버스 생성 (구조 생성만)
  ///
  /// 60fps 애니메이션의 경우 600을 사용하면 ~10초의 히스토리
  pub fn new(capacity: usize) -> Self {
    Self {
      capacity,
      inner: std::sync::Arc::new(std::sync::Mutex::new(TelemetryBuffer {
        data: Vec::with_capacity(capacity),
        capacity,
      })),
    }
  }

  /// 용량 반환 (구조 조회만, 허용)
  pub fn capacity(&self) -> usize {
    self.capacity
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 메서드들은 executor/runtime 계층에서 구현하세요:
// - push(frame: T) (프레임 추가, 상태 변경)
// - snapshot() -> Vec<T> (스냅샷 생성, 값 계산)
// - last_n(n: usize) -> Vec<T> (최근 N개 조회, 값 계산)
// - last() -> Option<T> (최근 프레임 조회, 값 계산)
// - len() -> usize (길이 조회, 값 계산)
// - is_empty() -> bool (빈 여부 조회, 값 계산)
// - clear() (버퍼 클리어, 상태 변경)
//
// 이 메서드들은 값 계산 및 상태 변경을 수행하므로 pnix-core에서 제외됩니다.

// ============================================================
// Console Telemetry 구조 (pnix-old telemetry/console.rs에서 마이그레이션)
// ============================================================

/// Console telemetry printer 구조
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, 실제 print/렌더링 실행 로직은 executor에서 구현
#[derive(Debug, Clone)]
pub struct ConsoleTelemetry {
  /// 색상 사용 여부 (ANSI escape codes)
  pub use_colors: bool,
  /// 표시할 최근 결정 수
  pub decision_history: usize,
}

impl Default for ConsoleTelemetry {
  fn default() -> Self {
    Self {
      use_colors: true,
      decision_history: 5,
    }
  }
}

impl ConsoleTelemetry {
  /// 새로운 콘솔 텔레메트리 생성
  pub fn new(use_colors: bool, decision_history: usize) -> Self {
    Self {
      use_colors,
      decision_history,
    }
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 메서드들은 executor/runtime 계층에서 구현하세요:
// - print_summary(bus: &TelemetryBus<FrpTelemetryFrame>) (stdout 출력, I/O)
// - print(bus: &TelemetryBus<FrpTelemetryFrame>) (stdout 출력, I/O)
// - print_line(frame: &FrpTelemetryFrame) (stdout 출력, I/O)
// - print_stats(bus: &TelemetryBus<FrpTelemetryFrame>) (stdout 출력, I/O)
// - header(text: &str) -> String (문자열 생성, 값 계산)
// - section(text: &str) -> String (문자열 생성, 값 계산)
// - progress_bar(value: f32, width: usize) -> String (문자열 생성, 값 계산)
// - mini_bar(value: f32, width: usize) -> String (문자열 생성, 값 계산)
// - hit_rate_color(hit_rate: f32, target: f32) -> String (문자열 생성, 값 계산)
// - memo_color(fill_pct: f32) -> String (문자열 생성, 값 계산)
// - colorize(text: &str, color: &str) -> String (문자열 생성, 값 계산)
//
// 이 메서드들은 값 계산 및 I/O를 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_telemetry_collector_creation() {
    let collector = TelemetryCollector::new(100);
    assert_eq!(collector.max_history, 100);
    assert_eq!(collector.frame_counter, 0);
  }

  #[test]
  fn test_frame_record_creation() {
    let stats = CacheStats {
      hits: 10,
      misses: 5,
      cached_keys_count: 0,
    };
    let policy = CachePolicy::default();
    let record = FrameRecord::new(1, stats, &policy);
    assert_eq!(record.frame, 1);
  }

  // =============================================================================
  // N24: Telemetry JSON Field Ordering Tests
  // =============================================================================

  #[test]
  fn test_n24_compile_frame_json_field_ordering() {
    // Verify JSON serialization field ordering is deterministic
    fn serialize_compile_frame() -> String {
      let frame = CompileFrame::new(CompilePhase::Parsing, 10.5, 100)
        .with_transformations(5)
        .with_note("test note");
      serde_json::to_string(&frame).unwrap()
    }

    let json1 = serialize_compile_frame();
    let json2 = serialize_compile_frame();
    let json3 = serialize_compile_frame();

    assert_eq!(json1, json2, "CompileFrame JSON must be deterministic");
    assert_eq!(json2, json3, "CompileFrame JSON must be deterministic");

    // Verify fields are present (order depends on serde's derive, which is stable)
    assert!(json1.contains("\"phase\""));
    assert!(json1.contains("\"duration_ms\""));
    assert!(json1.contains("\"ir_size\""));
  }

  #[test]
  fn test_n24_frame_metadata_json_field_ordering() {
    // Verify FrameMetadata JSON output is consistent
    fn serialize_metadata() -> String {
      let metadata = FrameMetadata {
        nodes_processed: 100,
        edges_processed: 50,
        transformations: 10,
        errors: 0,
        warnings: 2,
        notes: vec!["note1".to_string(), "note2".to_string()],
      };
      serde_json::to_string(&metadata).unwrap()
    }

    let json1 = serialize_metadata();
    let json2 = serialize_metadata();
    let json3 = serialize_metadata();

    assert_eq!(json1, json2, "FrameMetadata JSON must be deterministic");
    assert_eq!(json2, json3, "FrameMetadata JSON must be deterministic");
  }

  #[test]
  fn test_n24_compile_stats_json_field_ordering() {
    // Verify CompileStats JSON output is consistent
    fn serialize_stats() -> String {
      let stats = CompileStats {
        total_time_ms: 100.0,
        initial_ir_size: 10,
        final_ir_size: 150,
        total_transformations: 5,
        total_errors: 0,
        total_warnings: 2,
      };
      serde_json::to_string(&stats).unwrap()
    }

    let json1 = serialize_stats();
    let json2 = serialize_stats();
    let json3 = serialize_stats();

    assert_eq!(json1, json2, "CompileStats JSON must be deterministic");
    assert_eq!(json2, json3, "CompileStats JSON must be deterministic");
  }

  #[test]
  fn test_n24_frame_record_json_field_ordering() {
    // Verify FrameRecord JSON output is consistent
    fn serialize_record() -> String {
      let stats = CacheStats {
        hits: 10,
        misses: 5,
        cached_keys_count: 3,
      };
      let policy = CachePolicy::default();
      let record = FrameRecord::new(1, stats, &policy);
      serde_json::to_string(&record).unwrap()
    }

    let json1 = serialize_record();
    let json2 = serialize_record();
    let json3 = serialize_record();

    assert_eq!(json1, json2, "FrameRecord JSON must be deterministic");
    assert_eq!(json2, json3, "FrameRecord JSON must be deterministic");
  }

  #[test]
  fn test_n24_compile_phase_enum_serialization() {
    // Verify enum variant serialization is consistent
    let phases = [
      CompilePhase::Parsing,
      CompilePhase::SurfaceLowering,
      CompilePhase::FxCoreLowering,
      CompilePhase::FxCoreOptimization,
      CompilePhase::SsaLowering,
      CompilePhase::SsaOptimization,
      CompilePhase::BuildIrGeneration,
      CompilePhase::CodeGeneration,
      CompilePhase::Verification,
    ];

    // Serialize twice and compare
    let json1: Vec<String> = phases
      .iter()
      .map(|p| serde_json::to_string(p).unwrap())
      .collect();
    let json2: Vec<String> = phases
      .iter()
      .map(|p| serde_json::to_string(p).unwrap())
      .collect();

    assert_eq!(
      json1, json2,
      "CompilePhase enum serialization must be deterministic"
    );

    // Verify specific values
    assert_eq!(json1[0], "\"Parsing\"");
    assert_eq!(json1[1], "\"SurfaceLowering\"");
  }

  // =============================================================================
  // M24: Telemetry Buffer/Bus Capacity and Structure Tests
  // =============================================================================

  #[test]
  fn test_m24_telemetry_bus_capacity_stability() {
    // Verify TelemetryBus capacity is stable across multiple creations
    fn create_bus_and_get_capacity(capacity: usize) -> usize {
      let bus: TelemetryBus<CompileFrame> = TelemetryBus::new(capacity);
      bus.capacity()
    }

    // Test various capacity values
    for cap in [1, 10, 100, 600, 1000] {
      let cap1 = create_bus_and_get_capacity(cap);
      let cap2 = create_bus_and_get_capacity(cap);
      let cap3 = create_bus_and_get_capacity(cap);

      assert_eq!(cap1, cap, "Capacity must match requested value");
      assert_eq!(cap1, cap2, "Capacity must be stable");
      assert_eq!(cap2, cap3, "Capacity must be stable");
    }
  }

  #[test]
  fn test_m24_telemetry_bus_default_60fps_capacity() {
    // 60fps animation: 600 frames = ~10 seconds of history
    let bus: TelemetryBus<FrameRecord> = TelemetryBus::new(600);
    assert_eq!(bus.capacity(), 600);

    // Verify inner buffer is initialized with matching capacity
    let guard = bus.inner.lock().unwrap();
    assert_eq!(guard.capacity, 600);
  }

  #[test]
  fn test_m24_telemetry_bus_multiple_types_capacity() {
    // Test that capacity works correctly for different generic types
    let bus_frame: TelemetryBus<CompileFrame> = TelemetryBus::new(100);
    let bus_u64: TelemetryBus<u64> = TelemetryBus::new(100);
    let bus_string: TelemetryBus<String> = TelemetryBus::new(100);

    // All types should have consistent capacity behavior
    assert_eq!(bus_frame.capacity(), 100);
    assert_eq!(bus_u64.capacity(), 100);
    assert_eq!(bus_string.capacity(), 100);
  }

  #[test]
  fn test_m24_telemetry_collector_rolling_config() {
    // Test TelemetryCollector max_history configuration for rolling behavior
    fn create_collector_with_history(max: usize) -> (usize, usize) {
      let collector = TelemetryCollector::new(max);
      (collector.max_history, collector.history.capacity())
    }

    // Test various rolling window sizes
    let sizes = [10, 50, 100, 500, 1000];
    for &size in &sizes {
      let (max, capacity) = create_collector_with_history(size);
      assert_eq!(max, size, "max_history must match");
      assert!(capacity >= size, "capacity must be at least max_history");
    }

    // Verify determinism
    let (max1, _) = create_collector_with_history(100);
    let (max2, _) = create_collector_with_history(100);
    assert_eq!(max1, max2, "Rolling config must be deterministic");
  }

  #[test]
  fn test_m24_console_telemetry_flush_config() {
    // Test ConsoleTelemetry decision_history (affects flush display)
    fn create_console(history: usize) -> usize {
      let console = ConsoleTelemetry::new(true, history);
      console.decision_history
    }

    // Test various flush history sizes
    for history in [1, 5, 10, 20, 50] {
      let h1 = create_console(history);
      let h2 = create_console(history);
      let h3 = create_console(history);

      assert_eq!(h1, history, "decision_history must match");
      assert_eq!(h1, h2, "Config must be deterministic");
      assert_eq!(h2, h3, "Config must be deterministic");
    }

    // Test default
    let default = ConsoleTelemetry::default();
    assert_eq!(default.decision_history, 5, "Default history should be 5");
    assert!(default.use_colors, "Default should use colors");
  }

  // =============================================================================
  // P24: Telemetry JSON Schema Version Field Tests
  // =============================================================================

  #[test]
  fn test_p24_schema_version_constant() {
    // Verify schema version constant is properly defined
    assert_eq!(TELEMETRY_SCHEMA_VERSION, "1.0");
  }

  #[test]
  fn test_p24_snapshot_default_schema_version() {
    // Verify TelemetrySnapshot::new() uses default schema version
    let snapshot = TelemetrySnapshot::new(100, 50, 10, 5, vec![], 0.83);

    assert_eq!(
      snapshot.schema_version, TELEMETRY_SCHEMA_VERSION,
      "Default constructor must use TELEMETRY_SCHEMA_VERSION"
    );
    assert_eq!(snapshot.schema_version, "1.0");
  }

  #[test]
  fn test_p24_snapshot_custom_schema_version() {
    // Verify with_schema_version allows custom versions
    let snapshot = TelemetrySnapshot::with_schema_version("2.0-beta", 100, 50, 10, 5, vec![], 0.83);

    assert_eq!(snapshot.schema_version, "2.0-beta");
  }

  #[test]
  fn test_p24_snapshot_json_includes_version() {
    // Verify JSON serialization includes schema_version field
    let snapshot = TelemetrySnapshot::new(100, 50, 10, 5, vec![], 0.83);
    let json = serde_json::to_string(&snapshot).unwrap();

    assert!(
      json.contains("\"schema_version\""),
      "JSON must include schema_version field"
    );
    assert!(
      json.contains("\"1.0\""),
      "JSON must include version value 1.0"
    );
  }

  #[test]
  fn test_p24_snapshot_json_version_determinism() {
    // Verify schema_version is deterministic across multiple serializations
    fn serialize_snapshot() -> String {
      let snapshot = TelemetrySnapshot::new(100, 50, 10, 5, vec![], 0.83);
      serde_json::to_string(&snapshot).unwrap()
    }

    let json1 = serialize_snapshot();
    let json2 = serialize_snapshot();
    let json3 = serialize_snapshot();

    assert_eq!(json1, json2, "JSON must be deterministic");
    assert_eq!(json2, json3, "JSON must be deterministic");

    // Parse and verify version field
    let parsed: serde_json::Value = serde_json::from_str(&json1).unwrap();
    assert_eq!(
      parsed["schema_version"], "1.0",
      "Parsed version must be 1.0"
    );
  }

  #[test]
  fn test_p24_snapshot_json_deserialization_with_version() {
    // Verify JSON deserialization correctly reads schema_version
    let json_str = r#"{
            "schema_version": "1.0",
            "frame_count": 100,
            "total_hits": 50,
            "total_misses": 10,
            "adjustment_count": 5,
            "recent_history": [],
            "overall_hit_rate": 0.83
        }"#;

    let snapshot: TelemetrySnapshot = serde_json::from_str(json_str).unwrap();
    assert_eq!(snapshot.schema_version, "1.0");
    assert_eq!(snapshot.frame_count, 100);
  }

  #[test]
  fn test_p24_snapshot_version_migration_compatibility() {
    // Verify old JSON (hypothetically without version) fails gracefully
    // and new JSON with version works correctly
    let old_json = r#"{
            "frame_count": 100,
            "total_hits": 50,
            "total_misses": 10,
            "adjustment_count": 5,
            "recent_history": [],
            "overall_hit_rate": 0.83
        }"#;

    // Old JSON without schema_version should fail to parse
    let result: Result<TelemetrySnapshot, _> = serde_json::from_str(old_json);
    assert!(result.is_err(), "Old JSON without version should fail");

    // New JSON with schema_version should succeed
    let new_json = r#"{
            "schema_version": "1.0",
            "frame_count": 100,
            "total_hits": 50,
            "total_misses": 10,
            "adjustment_count": 5,
            "recent_history": [],
            "overall_hit_rate": 0.83
        }"#;
    let result: Result<TelemetrySnapshot, _> = serde_json::from_str(new_json);
    assert!(result.is_ok(), "New JSON with version should succeed");
  }
}
