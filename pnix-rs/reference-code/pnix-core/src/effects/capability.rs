//! Capability: Effect permission set
//!
//! Minimal scaffolding for Phase 2 (e-layer) capability checks.

use crate::effects::EffectZone;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::sync::OnceLock;

/// 권한: Effect 권한 집합 타입
///
/// Phase 2 (e-layer) 권한 검사를 위한 최소 스캐폴딩
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
  /// 읽기 권한
  Read,
  /// 쓰기 권한
  Write,
  /// 실행 권한
  Execute,
  /// 네트워크 접근 권한
  NetworkAccess,
  /// 프로세스 생성 권한
  SpawnProcess,
  /// 파일 시스템 접근 권한
  FileSystem,
}

impl Capability {
  /// 권한 이름 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn name(self) -> &'static str {
    match self {
      Capability::Read => "read",
      Capability::Write => "write",
      Capability::Execute => "execute",
      Capability::NetworkAccess => "network_access",
      Capability::SpawnProcess => "spawn_process",
      Capability::FileSystem => "file_system",
    }
  }

  /// 문자열에서 파싱
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn parse(s: &str) -> Option<Self> {
    match s.trim().to_lowercase().as_str() {
      "read" => Some(Capability::Read),
      "write" => Some(Capability::Write),
      "execute" => Some(Capability::Execute),
      "network_access" => Some(Capability::NetworkAccess),
      "spawn_process" => Some(Capability::SpawnProcess),
      "file_system" => Some(Capability::FileSystem),
      _ => None,
    }
  }
}

/// 권한 집합: 여러 권한을 관리하는 집합
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CapabilitySet {
  inner: BTreeSet<Capability>,
}

impl CapabilitySet {
  pub fn new() -> Self {
    Self::default()
  }

  /// 권한 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn insert(&mut self, cap: Capability) {
    self.inner.insert(cap);
  }

  /// 권한 포함 여부 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn contains(&self, cap: Capability) -> bool {
    self.inner.contains(&cap)
  }

  /// 빈 집합 여부 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn is_empty(&self) -> bool {
    self.inner.is_empty()
  }

  /// 부분집합 여부 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 비교만, 값 계산 없음
  pub fn is_subset_of(&self, other: &CapabilitySet) -> bool {
    self.inner.is_subset(&other.inner)
  }

  /// 합집합 계산
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 병합만, 값 계산 없음
  pub fn union(&self, other: &CapabilitySet) -> CapabilitySet {
    let mut merged = self.inner.clone();
    merged.extend(other.inner.iter().copied());
    CapabilitySet { inner: merged }
  }

  /// 차집합 계산
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 계산만, 값 계산 없음
  pub fn difference(&self, other: &CapabilitySet) -> CapabilitySet {
    let diff: BTreeSet<Capability> = self.inner.difference(&other.inner).copied().collect();
    CapabilitySet { inner: diff }
  }

  /// 권한 순회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn iter(&self) -> impl Iterator<Item = &Capability> {
    self.inner.iter()
  }
}

impl FromIterator<Capability> for CapabilitySet {
  fn from_iter<I: IntoIterator<Item = Capability>>(iter: I) -> Self {
    let mut set = CapabilitySet::new();
    for cap in iter {
      set.insert(cap);
    }
    set
  }
}

/// Zone별 권한: Effect Zone별로 권한을 관리하는 구조
#[derive(Debug, Default, Clone)]
pub struct ZoneCapabilities {
  by_zone: HashMap<EffectZone, CapabilitySet>,
}

impl ZoneCapabilities {
  pub fn new() -> Self {
    Self::default()
  }

  /// 기본 Zone별 권한 생성 (seto에서 로드)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn from_seto_default() -> Self {
    default_zone_capabilities().clone()
  }

  /// Zone에 권한 설정
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn set(&mut self, zone: EffectZone, caps: CapabilitySet) {
    self.by_zone.insert(zone, caps);
  }

  /// Zone의 권한 조회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn for_zone(&self, zone: EffectZone) -> CapabilitySet {
    self.by_zone.get(&zone).cloned().unwrap_or_default()
  }

  /// Zone에서 권한 허용 여부 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 비교만, 값 계산 없음
  pub fn allows(&self, zone: EffectZone, required: &CapabilitySet) -> bool {
    required.is_subset_of(&self.for_zone(zone))
  }
}

static ZONE_CAPABILITIES: OnceLock<ZoneCapabilities> = OnceLock::new();

fn default_zone_capabilities() -> &'static ZoneCapabilities {
  ZONE_CAPABILITIES.get_or_init(load_zone_capabilities_from_seto)
}

fn load_zone_capabilities_from_seto() -> ZoneCapabilities {
  let content = include_str!("../../../../data/seto/effect_layer.seto.toml");
  load_zone_capabilities_from_str(content)
}

fn load_zone_capabilities_from_str(src: &str) -> ZoneCapabilities {
  let mut policy = ZoneCapabilities::new();
  let mut in_zone_block = false;
  let mut current_zone: Option<String> = None;
  let mut current_caps: Vec<String> = Vec::new();

  let flush = |policy: &mut ZoneCapabilities, zone: &mut Option<String>, caps: &mut Vec<String>| {
    let Some(zone_name) = zone.take() else {
      caps.clear();
      return;
    };
    let Some(zone) = EffectZone::parse(&zone_name) else {
      caps.clear();
      return;
    };
    let mut set = CapabilitySet::new();
    for cap in caps.drain(..) {
      if let Some(cap) = Capability::parse(&cap) {
        set.insert(cap);
      }
    }
    policy.set(zone, set);
  };

  for line in src.lines() {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
      continue;
    }
    if line.starts_with("[[") {
      if in_zone_block {
        flush(&mut policy, &mut current_zone, &mut current_caps);
      }
      in_zone_block = line.contains("zone_capability");
      continue;
    }
    if !in_zone_block {
      continue;
    }
    if line.starts_with("zone") {
      current_zone = parse_toml_string(line);
    } else if line.starts_with("capabilities") {
      current_caps = parse_toml_string_list(line);
    }
  }
  if in_zone_block {
    flush(&mut policy, &mut current_zone, &mut current_caps);
  }
  policy
}

fn parse_toml_string(line: &str) -> Option<String> {
  let start = line.find('"')?;
  let end = line[start + 1..].find('"')? + start + 1;
  Some(line[start + 1..end].to_string())
}

fn parse_toml_string_list(line: &str) -> Vec<String> {
  let start = match line.find('[') {
    Some(idx) => idx + 1,
    None => return Vec::new(),
  };
  let end = match line[start..].find(']') {
    Some(idx) => start + idx,
    None => return Vec::new(),
  };
  let mut out = Vec::new();
  for item in line[start..end].split(',') {
    let trimmed = item.trim();
    if trimmed.is_empty() {
      continue;
    }
    let value = trimmed.trim_matches('"').trim_matches('\'').trim();
    if !value.is_empty() {
      out.push(value.to_string());
    }
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn capability_set_union_and_subset() {
    let a: CapabilitySet = [Capability::Read, Capability::Write].into_iter().collect();
    let b: CapabilitySet = [Capability::Write, Capability::Execute]
      .into_iter()
      .collect();
    let merged = a.union(&b);
    assert!(merged.contains(Capability::Read));
    assert!(merged.contains(Capability::Write));
    assert!(merged.contains(Capability::Execute));
    assert!(a.is_subset_of(&merged));
  }

  #[test]
  fn zone_capabilities_allows() {
    let mut policy = ZoneCapabilities::new();
    let caps: CapabilitySet = [Capability::Read, Capability::Write].into_iter().collect();
    policy.set(EffectZone::Pure, caps);

    let required: CapabilitySet = [Capability::Read].into_iter().collect();
    assert!(policy.allows(EffectZone::Pure, &required));
    let required: CapabilitySet = [Capability::Execute].into_iter().collect();
    assert!(!policy.allows(EffectZone::Pure, &required));
  }

  #[test]
  fn load_zone_capabilities_from_str_parses() {
    let src = r#"
      [[zone_capability]]
      zone = "Pure"
      capabilities = ["read", "write"]
    "#;
    let policy = load_zone_capabilities_from_str(src);
    let allowed = policy.for_zone(EffectZone::Pure);
    assert!(allowed.contains(Capability::Read));
    assert!(allowed.contains(Capability::Write));
  }
}
