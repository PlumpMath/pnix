//! Layer 엔진: SETO 스타일 패키지를 위한 레이어 엔진 스텁
//!
//! 데이터만: 파싱/검증 헬퍼, 실행 로직 없음
#![allow(clippy::items_after_test_module)]

use crate::lang::pnix::parse_expr;
use crate::lang::pnix::{PnixAttrItem, PnixExpr};
use crate::spec::operators::{
  OperatorCatalog, OperatorFixity, OperatorLawEntry, OperatorLawRef, OperatorSpec,
};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;

/// 레이어 파일 스펙: 레이어 파일의 종류와 경로 정보
#[derive(Debug, Clone)]
pub struct LayerFileSpec {
  /// 파일 종류 (파일 종류 문자열, 예: "grammar", "meaning", "laws")
  pub kind: String,
  /// 파일 경로 (파일 경로 문자열)
  pub file: String,
}

/// 레이어 패키지: 레이어 패키지의 정의 정보
#[derive(Debug, Clone)]
pub struct LayerPackage {
  /// 패키지 이름 (레이어 패키지의 이름)
  pub name: String,
  /// 패키지 버전 (레이어 패키지의 버전 문자열)
  pub version: String,
  /// 연산자 카탈로그 (레이어의 연산자 카탈로그)
  pub operators: OperatorCatalog,
  /// 연산자 법칙 집합 (레이어의 연산자 법칙 집합)
  pub operator_laws: LayerLawSet,
  /// 의존성 목록 (의존하는 다른 레이어 패키지 이름 목록)
  pub depends: Vec<String>,
  /// 효과 목록 (레이어가 제공하는 효과 목록)
  pub effects: Vec<String>,
  /// 문법 파일 스펙 (선택적, 문법 파일 정보)
  pub grammar: Option<LayerFileSpec>,
  /// 의미 파일 스펙 (선택적, 의미 파일 정보)
  pub meaning: Option<LayerFileSpec>,
  /// 법칙 파일 스펙 (선택적, 법칙 파일 정보)
  pub laws: Option<LayerFileSpec>,
  /// 검증 리포트 파일 스펙 (선택적, 검증 리포트 파일 정보)
  pub verification_report: Option<LayerFileSpec>,
  /// 능력 맵 파일 스펙 (선택적, 능력 맵 파일 정보)
  pub capability_map: Option<LayerFileSpec>,
}

/// 레이어 리포트: 레이어 처리 결과 리포트
#[derive(Debug, Clone, Default)]
pub struct LayerReport {
  /// 에러 목록 (발생한 에러 메시지 목록)
  pub errors: Vec<String>,
  /// 경고 목록 (발생한 경고 메시지 목록)
  pub warnings: Vec<String>,
}

impl LayerReport {
  /// 리포트가 성공인지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 검증만, 값 계산 없음
  pub fn is_ok(&self) -> bool {
    self.errors.is_empty()
  }
}

/// 레이어 준비 결과: 레이어 파이프라인 준비 결과 정보
#[derive(Debug, Clone, Default)]
pub struct LayerPrepared {
  /// 디슈거 맵 (규칙 이름 → 규칙 본문 매핑)
  pub desugar_map: BTreeMap<String, String>,
  /// 타이핑 맵 (규칙 이름 → 규칙 본문 매핑)
  pub typing_map: BTreeMap<String, String>,
  /// 정규화 맵 (규칙 이름 → 규칙 본문 매핑)
  pub normalize_map: BTreeMap<String, String>,
  /// 리포트 (레이어 처리 결과 리포트)
  pub report: LayerReport,
}

/// 레이어 디슈거 계획: 디슈거 단계의 규칙 계획
#[derive(Debug, Clone, Default)]
pub struct LayerDesugarPlan {
  /// 규칙 맵 (규칙 이름 → 규칙 본문 매핑)
  pub rules: BTreeMap<String, String>,
}

/// 레이어 타이핑 계획: 타이핑 단계의 규칙 계획
#[derive(Debug, Clone, Default)]
pub struct LayerTypePlan {
  /// 규칙 맵 (규칙 이름 → 규칙 본문 매핑)
  pub rules: BTreeMap<String, String>,
}

/// 레이어 정규화 계획: 정규화 단계의 규칙 계획
#[derive(Debug, Clone, Default)]
pub struct LayerNormalizePlan {
  /// 규칙 맵 (규칙 이름 → 규칙 본문 매핑)
  pub rules: BTreeMap<String, String>,
}

/// 레이어 파이프라인: 레이어 처리 파이프라인의 구성 정보
#[derive(Debug, Clone, Default)]
pub struct LayerPipeline {
  /// 디슈거 계획 (디슈거 단계 계획)
  pub desugar: LayerDesugarPlan,
  /// 타이핑 계획 (타이핑 단계 계획)
  pub typing: LayerTypePlan,
  /// 정규화 계획 (정규화 단계 계획)
  pub normalize: LayerNormalizePlan,
  /// 리포트 (레이어 처리 결과 리포트)
  pub report: LayerReport,
}

/// 레이어 법칙 집합: 레이어 법칙 이름의 집합
#[derive(Debug, Clone, Default)]
pub struct LayerLawSet {
  /// 법칙 이름 집합 (법칙 이름들의 집합)
  pub names: BTreeSet<String>,
}

impl LayerLawSet {
  /// 법칙 이름 포함 여부 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 검색만, 값 계산 없음
  pub fn contains(&self, name: &str) -> bool {
    self.names.contains(name)
  }

  /// 다른 법칙 집합 병합
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 병합만, 값 계산 없음
  pub fn merge(&mut self, other: &LayerLawSet) {
    self.names.extend(other.names.iter().cloned());
  }
}

/// 레이어 검증 리포트: 레이어 검증 결과 리포트 정보
#[derive(Debug, Clone)]
pub struct LayerVerificationReport {
  /// 오버레이 ID (검증 대상 오버레이의 ID)
  pub overlay_id: String,
  /// 상태 (검증 상태 문자열, 예: "passed", "failed")
  pub status: String,
  /// 통과한 법칙 이름 집합 (검증을 통과한 법칙 이름 집합)
  pub passed: BTreeSet<String>,
  /// 실패한 법칙 이름 집합 (검증에 실패한 법칙 이름 집합)
  pub failed: BTreeSet<String>,
  /// 경고 목록 (검증 중 발생한 경고 메시지 목록)
  pub warnings: Vec<String>,
  /// 생성 시간 (선택적, 리포트 생성 시간 문자열)
  pub generated_at: Option<String>,
}

/// 레이어 능력 맵: 레이어의 능력과 커버리지 정보
#[derive(Debug, Clone)]
pub struct LayerCapabilityMap {
  /// 레이어 이름 (레이어의 이름)
  pub layer: String,
  /// 연산자 목록 (레이어가 제공하는 연산자 이름 목록)
  pub operators: Vec<String>,
  /// 법칙 목록 (레이어가 제공하는 법칙 이름 목록)
  pub laws: Vec<String>,
  /// 정규형 목록 (레이어가 지원하는 정규형 이름 목록)
  pub normal_forms: Vec<String>,
  /// 커버리지 (0.0 ~ 1.0, 레이어의 커버리지 점수)
  pub coverage: f64,
  /// 갭 목록 (커버되지 않은 항목들, 미구현 항목 목록)
  pub gaps: Vec<String>,
  /// 마지막 검증 시간 (선택적, 마지막 검증 시간 문자열)
  pub last_verified: Option<String>,
  /// 의존성 목록 (레이어의 의존성 목록)
  pub dependencies: Vec<String>,
  /// 제약 조건 목록 (레이어의 제약 조건 목록)
  pub constraints: Vec<String>,
}

/// Pnix 소스에서 검증 리포트 로드
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 파싱만, 파일 I/O 없음
pub fn load_verification_report_pnix(
  src: &str,
) -> Result<LayerVerificationReport, LayerParseError> {
  let expr = parse_expr(src).map_err(|err| LayerParseError::Parse(err.to_string()))?;
  let root = expect_attrset(&expr, "report")?;
  let report_expr = expect_value(root.get("report"), "report", "report")?;
  let report_map = expect_attrset(report_expr, "report.report")?;
  let overlay_id = expect_string(report_map.get("overlay_id"), "report.report", "overlay_id")?;
  let status = expect_string(report_map.get("status"), "report.report", "status")?;
  let passed_expr = expect_value(report_map.get("passed"), "report.report", "passed")?;
  let failed_expr = expect_value(report_map.get("failed"), "report.report", "failed")?;
  let warnings_expr = report_map.get("warnings");
  let generated_at = match report_map.get("generated_at") {
    Some(expr) => Some(expect_string(Some(expr), "report.report", "generated_at")?),
    None => None,
  };

  let passed = parse_string_set(passed_expr, "report.report.passed")?;
  let failed = parse_failed_laws(failed_expr, "report.report.failed")?;
  let warnings = match warnings_expr {
    Some(expr) => validate_string_list(expr, "report.report.warnings")?,
    None => Vec::new(),
  };

  Ok(LayerVerificationReport {
    overlay_id,
    status,
    passed,
    failed,
    warnings,
    generated_at,
  })
}

/// 능력 맵 생성: 레이어 패키지, 법칙, 검증 리포트로부터 능력 맵 생성
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 생성만, 값 계산 없음
pub fn build_capability_map(
  package: &LayerPackage,
  laws: &LayerLawSet,
  report: Option<&LayerVerificationReport>,
) -> LayerCapabilityMap {
  let mut operators: Vec<String> = package.operators.operators.keys().cloned().collect();
  operators.sort();
  let mut law_list: Vec<String> = laws.names.iter().cloned().collect();
  law_list.sort();

  let (passed, last_verified) = match report {
    Some(report) => (report.passed.clone(), report.generated_at.clone()),
    None => (laws.names.clone(), None),
  };

  let mut passed_count = 0usize;
  for name in &law_list {
    if passed.contains(name) {
      passed_count += 1;
    }
  }
  let total = law_list.len();
  let coverage = if total == 0 {
    1.0
  } else {
    passed_count as f64 / total as f64
  };

  let mut gaps = Vec::new();
  for name in &law_list {
    if !passed.contains(name) {
      gaps.push(name.clone());
    }
  }

  let mut normal_forms = Vec::new();
  for name in &law_list {
    if name.contains("normal_form") || name.contains("normalize") {
      normal_forms.push(name.clone());
    }
  }

  LayerCapabilityMap {
    layer: package.name.clone(),
    operators,
    laws: law_list,
    normal_forms,
    coverage,
    gaps,
    last_verified,
    dependencies: package.depends.clone(),
    constraints: package.effects.clone(),
  }
}

/// 능력 맵을 문자열로 렌더링
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 생성만, 파일 I/O 없음
pub fn render_capability_map(map: &LayerCapabilityMap) -> String {
  fn render_list(items: &[String]) -> String {
    if items.is_empty() {
      return "[ ]".to_string();
    }
    let mut out = String::from("[ ");
    for (idx, item) in items.iter().enumerate() {
      if idx > 0 {
        out.push(' ');
      }
      out.push('"');
      out.push_str(item);
      out.push('"');
    }
    out.push_str(" ]");
    out
  }

  let last_verified = map
    .last_verified
    .clone()
    .unwrap_or_else(|| "unknown".to_string());

  format!(
    "{{\n  capability = {{\n    layer = \"{}\";\n    operators = {};\n    laws = {};\n    normal_forms = {};\n    coverage = {};\n    gaps = {};\n    last_verified = \"{}\";\n    dependencies = {};\n    constraints = {};\n  }};\n}}\n",
    map.layer,
    render_list(&map.operators),
    render_list(&map.laws),
    render_list(&map.normal_forms),
    map.coverage,
    render_list(&map.gaps),
    last_verified,
    render_list(&map.dependencies),
    render_list(&map.constraints),
  )
}

/// 능력 맵을 파일에 쓰기 (write 콜백 사용)
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 생성만, 파일 I/O는 write 콜백에서 처리
pub fn write_capability_map_with(
  map: &LayerCapabilityMap,
  path: &Path,
  write: &dyn Fn(&Path, &str) -> Result<(), LayerParseError>,
) -> Result<(), LayerParseError> {
  let src = render_capability_map(map);
  write(path, &src)
}

/// 패키지의 능력 맵을 파일에 쓰기 (write 콜백 사용)
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 생성만, 파일 I/O는 write 콜백에서 처리
pub fn write_capability_map_for_package_with(
  package: &LayerPackage,
  laws: &LayerLawSet,
  report: Option<&LayerVerificationReport>,
  path: &Path,
  write: &dyn Fn(&Path, &str) -> Result<(), LayerParseError>,
) -> Result<(), LayerParseError> {
  let map = build_capability_map(package, laws, report);
  write_capability_map_with(&map, path, write)
}

/// 패키지의 능력 맵 파일 경로 조회
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn capability_map_path_for_package(
  package: &LayerPackage,
  base_dir: &Path,
) -> Result<Option<std::path::PathBuf>, LayerParseError> {
  let Some(spec) = package.capability_map.as_ref() else {
    return Ok(None);
  };
  if spec.kind != "pnix" {
    return Err(LayerParseError::Unsupported(format!(
      "unsupported capability_map kind: {}",
      spec.kind
    )));
  }
  Ok(Some(base_dir.join(&spec.file)))
}

/// 패키지의 능력 맵을 설정된 경로에 쓰기 (write 콜백 사용)
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 생성만, 파일 I/O는 write 콜백에서 처리
pub fn write_capability_map_for_package_configured_with(
  package: &LayerPackage,
  base_dir: &Path,
  laws: &LayerLawSet,
  report: Option<&LayerVerificationReport>,
  write: &dyn Fn(&Path, &str) -> Result<(), LayerParseError>,
) -> Result<Option<std::path::PathBuf>, LayerParseError> {
  let Some(path) = capability_map_path_for_package(package, base_dir)? else {
    return Ok(None);
  };
  write_capability_map_for_package_with(package, laws, report, &path, write)?;
  Ok(Some(path))
}

/// 스키마로부터 능력 맵 업데이트 (read/write 콜백 사용)
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 생성만, 파일 I/O는 read/write 콜백에서 처리
pub fn update_capability_map_from_schema_with(
  package: &LayerPackage,
  base_dir: &Path,
  laws: &LayerLawSet,
  read: &dyn Fn(&Path) -> Result<String, LayerParseError>,
  write: &dyn Fn(&Path, &str) -> Result<(), LayerParseError>,
) -> Result<Option<std::path::PathBuf>, LayerParseError> {
  let report = load_layer_verification_report_for_package_with(package, base_dir, read)?;
  write_capability_map_for_package_configured_with(package, base_dir, laws, report.as_ref(), write)
}

/// 레이어 엔진: 레이어 처리 엔진 (데이터만, 실행 로직 없음)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, 실행 로직 제외
#[derive(Debug, Default)]
pub struct LayerEngine;

impl LayerEngine {
  /// 새 레이어 엔진 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self
  }

  /// 레이어 패키지 준비
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn prepare(&self, package: &LayerPackage) -> LayerPrepared {
    self.prepare_with_laws(package, None)
  }

  /// 레이어 패키지 준비 (법칙 집합 지정)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn prepare_with_laws(
    &self,
    package: &LayerPackage,
    laws: Option<&LayerLawSet>,
  ) -> LayerPrepared {
    let mut report = LayerReport::default();
    let mut desugar_map = BTreeMap::new();
    let mut typing_map = BTreeMap::new();
    let mut normalize_map = BTreeMap::new();

    for (token, spec) in &package.operators.operators {
      self.validate_operator(token, spec, &mut report);
      if desugar_map
        .insert(token.clone(), spec.desugar.clone())
        .is_some()
      {
        report
          .errors
          .push(format!("duplicate operator token: {}", token));
      }
      if typing_map
        .insert(token.clone(), spec.typing.clone())
        .is_some()
      {
        report
          .errors
          .push(format!("duplicate operator typing for token: {}", token));
      }
      if let Some(rule) = &spec.normalize {
        if normalize_map.insert(token.clone(), rule.clone()).is_some() {
          report
            .errors
            .push(format!("duplicate operator normalize for token: {}", token));
        }
      }
      self.validate_operator_laws(token, spec, laws, &mut report);
    }

    LayerPrepared {
      desugar_map,
      typing_map,
      normalize_map,
      report,
    }
  }

  /// 레이어 패키지 준비 (법칙 집합 및 검증 리포트 지정)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn prepare_with_laws_and_report(
    &self,
    package: &LayerPackage,
    laws: &LayerLawSet,
    report: &LayerVerificationReport,
  ) -> LayerPrepared {
    let mut prepared = self.prepare_with_laws(package, Some(laws));
    let optional = collect_optional_laws(package);
    apply_verification_report(report, laws, &optional, &mut prepared.report);
    prepared
  }

  /// 레이어 파이프라인 구축
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn build_pipeline(&self, prepared: &LayerPrepared) -> LayerPipeline {
    let mut report = LayerReport::default();
    self.validate_typing_plan(&prepared.typing_map, &mut report);
    self.validate_normalize_plan(&prepared.normalize_map, &mut report);

    LayerPipeline {
      desugar: LayerDesugarPlan {
        rules: prepared.desugar_map.clone(),
      },
      typing: LayerTypePlan {
        rules: prepared.typing_map.clone(),
      },
      normalize: LayerNormalizePlan {
        rules: prepared.normalize_map.clone(),
      },
      report,
    }
  }

  fn validate_operator(&self, token: &str, spec: &OperatorSpec, report: &mut LayerReport) {
    if token.trim().is_empty() {
      report.errors.push("operator token is empty".to_string());
    }
    if spec.desugar.trim().is_empty() {
      report
        .errors
        .push(format!("operator {} has empty desugar", token));
    }
    if spec.typing.trim().is_empty() {
      report
        .errors
        .push(format!("operator {} has empty typing", token));
    }
    if spec.laws.is_empty() {
      report
        .errors
        .push(format!("operator {} requires at least one law", token));
    }
    for law in &spec.laws {
      if law.name().trim().is_empty() {
        report
          .errors
          .push(format!("operator {} has empty law name", token));
      }
    }
  }

  fn validate_operator_laws(
    &self,
    token: &str,
    spec: &OperatorSpec,
    laws: Option<&LayerLawSet>,
    report: &mut LayerReport,
  ) {
    let Some(laws) = laws else {
      return;
    };
    for law in &spec.laws {
      if laws.contains(law.name()) {
        continue;
      }
      if law.optional() {
        report.warnings.push(format!(
          "operator {} references optional law '{}' not found",
          token,
          law.name()
        ));
      } else {
        report.errors.push(format!(
          "operator {} references missing law '{}'",
          token,
          law.name()
        ));
      }
    }
  }

  fn validate_typing_plan(&self, typing: &BTreeMap<String, String>, report: &mut LayerReport) {
    for (token, rule) in typing {
      if !rule.contains("->") {
        report.warnings.push(format!(
          "operator {} typing rule does not contain '->'",
          token
        ));
      }
    }
  }

  fn validate_normalize_plan(
    &self,
    normalize: &BTreeMap<String, String>,
    report: &mut LayerReport,
  ) {
    for (token, rule) in normalize {
      if rule.trim().is_empty() {
        report
          .warnings
          .push(format!("operator {} has empty normalize rule", token));
      }
    }
  }
}

fn collect_optional_laws(package: &LayerPackage) -> BTreeSet<String> {
  let mut out = BTreeSet::new();
  for spec in package.operators.operators.values() {
    for law in &spec.laws {
      if law.optional() {
        out.insert(law.name().to_string());
      }
    }
  }
  out
}

fn apply_verification_report(
  report: &LayerVerificationReport,
  laws: &LayerLawSet,
  optional: &BTreeSet<String>,
  output: &mut LayerReport,
) {
  let has_required_failure = report.failed.iter().any(|name| !optional.contains(name));

  if report.status != "verified" {
    if report.status == "failed" && !has_required_failure {
      output.warnings.push(format!(
        "verification status is '{}' (optional law failures only)",
        report.status
      ));
    } else {
      output
        .errors
        .push(format!("verification status is '{}'", report.status));
    }
  }

  for warning in &report.warnings {
    output
      .warnings
      .push(format!("verification warning: {}", warning));
  }

  for name in &report.failed {
    if optional.contains(name) {
      output
        .warnings
        .push(format!("optional law '{}' failed verification", name));
    } else if laws.contains(name) {
      output
        .errors
        .push(format!("verification failed for law '{}'", name));
    } else {
      output
        .warnings
        .push(format!("verification failed for unknown law '{}'", name));
    }
  }

  for name in &report.passed {
    if !laws.contains(name) {
      output
        .warnings
        .push(format!("verification passed unknown law '{}'", name));
    }
  }

  for name in &laws.names {
    if !report.passed.contains(name) && !report.failed.contains(name) {
      output.warnings.push(format!("law '{}' not verified", name));
    }
  }
}

/// 레이어 레지스트리: 레이어 패키지 레지스트리
#[derive(Debug, Clone)]
pub struct LayerRegistry {
  /// 패키지 맵 (패키지 이름 → 패키지)
  pub packages: BTreeMap<String, LayerPackage>,
}

/// 레이어 에러: 레이어 처리 중 발생하는 에러 타입
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerError {
  /// 중복된 레이어 이름: 같은 이름의 레이어가 이미 등록됨
  DuplicateLayer(
    /// 레이어 이름 (중복된 레이어 이름)
    String,
  ),
}

impl LayerRegistry {
  /// 새 레이어 레지스트리 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self {
      packages: BTreeMap::new(),
    }
  }

  /// 레이어 패키지 등록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn register(&mut self, package: LayerPackage) -> Result<(), LayerError> {
    if self.packages.contains_key(&package.name) {
      return Err(LayerError::DuplicateLayer(package.name));
    }
    self.packages.insert(package.name.clone(), package);
    Ok(())
  }
}

impl Default for LayerRegistry {
  fn default() -> Self {
    Self::new()
  }
}

/// 레이어 파싱 에러: 레이어 파싱 중 발생하는 에러 타입
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerParseError {
  /// 파싱 에러: 레이어 파일 파싱 중 발생한 에러
  Parse(
    /// 에러 메시지
    String,
  ),
  /// IO 에러: 파일 I/O 중 발생한 에러
  Io {
    /// 파일 경로 (에러가 발생한 파일 경로)
    path: String,
    /// 에러 메시지
    message: String,
  },
  /// 필수 필드 누락: 필수 필드가 누락됨
  MissingField {
    /// 컨텍스트
    ctx: String,
    /// 필드 이름
    field: String,
  },
  /// 잘못된 필드
  InvalidField {
    /// 컨텍스트
    ctx: String,
    /// 필드 이름
    field: String,
    /// 예상 타입
    expected: String,
  },
  /// 지원하지 않는 기능
  Unsupported(
    /// 메시지
    String,
  ),
}

type LayerReadFn = dyn Fn(&Path) -> Result<String, LayerParseError>;

/// Pnix 소스에서 레이어 스키마 로드
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 파싱만, 파일 I/O 없음
pub fn load_layer_schema_pnix(src: &str) -> Result<LayerRegistry, LayerParseError> {
  load_layer_schema_pnix_with_base(src, None, None)
}

/// Pnix 소스에서 레이어 스키마 로드 (base_dir 및 read 콜백 사용)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 파싱만, 파일 I/O는 read 콜백에서 처리
pub fn load_layer_schema_pnix_with_base(
  src: &str,
  base_dir: Option<&Path>,
  read: Option<&LayerReadFn>,
) -> Result<LayerRegistry, LayerParseError> {
  let expr = parse_expr(src).map_err(|err| LayerParseError::Parse(err.to_string()))?;
  let root = expect_attrset(&expr, "schema")?;
  let seto_layers = match root.get("seto_layers") {
    Some(expr) => expr,
    None => {
      return Err(LayerParseError::MissingField {
        ctx: "schema".to_string(),
        field: "seto_layers".to_string(),
      });
    }
  };
  let layers = expect_attrset(seto_layers, "schema.seto_layers")?;
  let operator_packs =
    parse_operator_pack_index(root.get("operator_packs"), "schema.operator_packs")?;
  let mut registry = LayerRegistry::new();

  for (name, layer_expr) in layers {
    let ctx = format!("schema.seto_layers.{}", name);
    let layer_map = expect_attrset(&layer_expr, &ctx)?;
    let version = expect_string(layer_map.get("version"), &ctx, "version")?;
    let operators_expr = expect_value(layer_map.get("operators"), &ctx, "operators")?;
    let operator_pack = parse_operator_catalog(
      operators_expr,
      &format!("{ctx}.operators"),
      base_dir,
      &operator_packs,
      read,
    )?;
    let depends = parse_string_list(layer_map.get("depends"), &format!("{ctx}.depends"))?;
    let effects = parse_string_list(layer_map.get("effects"), &format!("{ctx}.effects"))?;
    let grammar = parse_file_spec(layer_map.get("grammar"), &ctx, "grammar")?;
    let meaning = parse_file_spec(layer_map.get("meaning"), &ctx, "meaning")?;
    let laws = parse_file_spec(layer_map.get("laws"), &ctx, "laws")?;
    let verification_report = parse_file_spec(
      layer_map.get("verification_report"),
      &ctx,
      "verification_report",
    )?;
    let capability_map = parse_file_spec(layer_map.get("capability_map"), &ctx, "capability_map")?;

    let package = LayerPackage {
      name: name.clone(),
      version,
      operators: operator_pack.catalog,
      operator_laws: operator_pack.laws,
      depends,
      effects,
      grammar,
      meaning,
      laws,
      verification_report,
      capability_map,
    };
    registry
      .register(package)
      .map_err(|err| LayerParseError::Unsupported(format!("{:?}", err)))?;
  }

  Ok(registry)
}

/// 패키지의 레이어 법칙 로드
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 파싱만, 파일 I/O는 read 콜백에서 처리
pub fn load_layer_laws_for_package_with(
  package: &LayerPackage,
  base_dir: &Path,
  read: &dyn Fn(&Path) -> Result<String, LayerParseError>,
) -> Result<LayerLawSet, LayerParseError> {
  let Some(spec) = package.laws.as_ref() else {
    if package.operator_laws.names.is_empty() && package.operators.operators.is_empty() {
      return Ok(LayerLawSet::default());
    }
    return Ok(package.operator_laws.clone());
  };
  if spec.kind != "pnix" {
    return Err(LayerParseError::Unsupported(format!(
      "unsupported laws kind: {}",
      spec.kind
    )));
  }
  let path = base_dir.join(&spec.file);
  let src = read(&path)?;
  let mut laws = load_laws_from_layer_pnix(&src)?;
  laws.merge(&package.operator_laws);
  Ok(laws)
}

/// 패키지의 레이어 문법 로드
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 파싱만, 파일 I/O는 read 콜백에서 처리
pub fn load_layer_grammar_for_package_with(
  package: &LayerPackage,
  base_dir: &Path,
  read: &dyn Fn(&Path) -> Result<String, LayerParseError>,
) -> Result<PnixExpr, LayerParseError> {
  load_layer_doc_for_package(package, base_dir, read, "grammar", "grammar")
}

/// 패키지의 레이어 의미 로드
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 파싱만, 파일 I/O는 read 콜백에서 처리
pub fn load_layer_meaning_for_package_with(
  package: &LayerPackage,
  base_dir: &Path,
  read: &dyn Fn(&Path) -> Result<String, LayerParseError>,
) -> Result<PnixExpr, LayerParseError> {
  load_layer_doc_for_package(package, base_dir, read, "meaning", "meaning")
}

/// 패키지의 레이어 검증 리포트 로드
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 파싱만, 파일 I/O는 read 콜백에서 처리
pub fn load_layer_verification_report_for_package_with(
  package: &LayerPackage,
  base_dir: &Path,
  read: &dyn Fn(&Path) -> Result<String, LayerParseError>,
) -> Result<Option<LayerVerificationReport>, LayerParseError> {
  let Some(spec) = package.verification_report.as_ref() else {
    return Ok(None);
  };
  if spec.kind != "pnix" {
    return Err(LayerParseError::Unsupported(format!(
      "unsupported verification_report kind: {}",
      spec.kind
    )));
  }
  let path = base_dir.join(&spec.file);
  let src = read(&path)?;
  let report = load_verification_report_pnix(&src)?;
  Ok(Some(report))
}

/// Pnix 소스에서 레이어 법칙 로드
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 파싱만, 파일 I/O 없음
pub fn load_laws_from_layer_pnix(src: &str) -> Result<LayerLawSet, LayerParseError> {
  let expr = parse_expr(src).map_err(|err| LayerParseError::Parse(err.to_string()))?;
  let root = expect_attrset(&expr, "layer")?;
  if let Some(layer_expr) = root.get("layer") {
    let layer_map = expect_attrset(layer_expr, "layer.layer")?;
    let laws_expr = expect_value(layer_map.get("laws"), "layer.layer", "laws")?;
    return parse_law_names(laws_expr, "layer.layer.laws");
  }
  let laws_expr = expect_value(root.get("laws"), "layer", "laws")?;
  parse_law_names(laws_expr, "layer.laws")
}

fn load_layer_doc_for_package(
  package: &LayerPackage,
  base_dir: &Path,
  read: &dyn Fn(&Path) -> Result<String, LayerParseError>,
  field: &str,
  required_key: &str,
) -> Result<PnixExpr, LayerParseError> {
  let spec = match field {
    "grammar" => package.grammar.as_ref(),
    "meaning" => package.meaning.as_ref(),
    _ => None,
  };
  let Some(spec) = spec else {
    return Err(LayerParseError::MissingField {
      ctx: package.name.clone(),
      field: field.to_string(),
    });
  };
  if spec.kind != "pnix" {
    return Err(LayerParseError::Unsupported(format!(
      "unsupported {} kind: {}",
      field, spec.kind
    )));
  }
  let path = base_dir.join(&spec.file);
  let src = read(&path)?;
  let expr = parse_expr(&src).map_err(|err| LayerParseError::Parse(err.to_string()))?;
  let root = expect_attrset(&expr, &format!("layer.{field}"))?;
  if !root.contains_key(required_key) {
    return Err(LayerParseError::MissingField {
      ctx: format!("layer.{}", field),
      field: required_key.to_string(),
    });
  }
  match field {
    "grammar" => validate_grammar_doc(&expr)?,
    "meaning" => validate_meaning_doc(&expr)?,
    _ => {}
  }
  Ok(expr)
}

#[derive(Debug, Clone, Default)]
struct OperatorPack {
  catalog: OperatorCatalog,
  laws: LayerLawSet,
}

fn parse_operator_list(expr: &PnixExpr, ctx: &str) -> Result<OperatorPack, LayerParseError> {
  let list = expect_list(expr, ctx)?;
  let mut pack = OperatorPack::default();
  for (idx, item) in list.iter().enumerate() {
    let entry_ctx = format!("{ctx}[{idx}]");
    let map = expect_attrset(item, &entry_ctx)?;
    let token = expect_string(map.get("token"), &entry_ctx, "token")?;
    let fixity_s = expect_string(map.get("fixity"), &entry_ctx, "fixity")?;
    let fixity = parse_fixity(&fixity_s, &entry_ctx)?;
    let precedence = expect_int(map.get("precedence"), &entry_ctx, "precedence")?;
    let arity = match map.get("arity") {
      Some(v) => expect_int(Some(v), &entry_ctx, "arity")? as usize,
      None => 2,
    };
    let desugar = expect_string(map.get("desugar"), &entry_ctx, "desugar")?;
    let typing = expect_string(map.get("typing"), &entry_ctx, "typing")?;
    let laws_expr = expect_value(map.get("laws"), &entry_ctx, "laws")?;
    let laws = parse_operator_laws(laws_expr, &entry_ctx)?;
    let normalize = match map.get("normalize") {
      Some(v) => Some(expect_string(Some(v), &entry_ctx, "normalize")?),
      None => None,
    };

    pack.catalog.register(OperatorSpec {
      token,
      fixity,
      precedence,
      arity,
      desugar,
      typing,
      laws,
      normalize,
    });
  }
  Ok(pack)
}

fn parse_operator_pack_index(
  expr: Option<&PnixExpr>,
  ctx: &str,
) -> Result<BTreeMap<String, String>, LayerParseError> {
  let mut out = BTreeMap::new();
  let Some(expr) = expr else {
    return Ok(out);
  };
  let map = expect_attrset(expr, ctx)?;
  for (name, value) in map {
    let path = expect_string(Some(&value), ctx, "operator_pack")?;
    if out.insert(name.clone(), path).is_some() {
      return Err(LayerParseError::InvalidField {
        ctx: ctx.to_string(),
        field: name,
        expected: "unique operator pack key".to_string(),
      });
    }
  }
  Ok(out)
}

fn parse_operator_catalog(
  expr: &PnixExpr,
  ctx: &str,
  base_dir: Option<&Path>,
  pack_index: &BTreeMap<String, String>,
  read: Option<&LayerReadFn>,
) -> Result<OperatorPack, LayerParseError> {
  match expr {
    PnixExpr::List(_) => parse_operator_list(expr, ctx),
    PnixExpr::AttrSet { .. } => {
      let map = expect_attrset(expr, ctx)?;
      let imports = match map.get("imports") {
        Some(expr) => parse_string_list(Some(expr), &format!("{ctx}.imports"))?,
        None => Vec::new(),
      };
      let file = match map.get("file") {
        Some(expr) => Some(expect_string(Some(expr), ctx, "file")?),
        None => None,
      };
      let mut pack = OperatorPack::default();
      let base_dir = base_dir.ok_or_else(|| LayerParseError::InvalidField {
        ctx: ctx.to_string(),
        field: "operators".to_string(),
        expected: "schema path for resolving operator packs".to_string(),
      })?;
      let read = read.ok_or_else(|| LayerParseError::InvalidField {
        ctx: ctx.to_string(),
        field: "operators".to_string(),
        expected: "reader for operator pack files".to_string(),
      })?;

      for (idx, import) in imports.iter().enumerate() {
        let pack_path = resolve_operator_pack_path(import, pack_index, base_dir, ctx)?;
        let loaded =
          load_operator_pack_from_path(&pack_path, &format!("{ctx}.imports[{idx}]"), read)?;
        merge_operator_pack(&mut pack, loaded, ctx)?;
      }

      if let Some(file) = file {
        let file_path = resolve_operator_pack_path(&file, pack_index, base_dir, ctx)?;
        let loaded = load_operator_pack_from_path(&file_path, &format!("{ctx}.file"), read)?;
        merge_operator_pack(&mut pack, loaded, ctx)?;
      }

      Ok(pack)
    }
    _ => Err(LayerParseError::InvalidField {
      ctx: ctx.to_string(),
      field: "operators".to_string(),
      expected: "List or AttrSet".to_string(),
    }),
  }
}

fn resolve_operator_pack_path(
  name: &str,
  pack_index: &BTreeMap<String, String>,
  base_dir: &Path,
  ctx: &str,
) -> Result<std::path::PathBuf, LayerParseError> {
  let is_path = name.contains('/') || name.ends_with(".px") || name.starts_with("./");
  let relative = if is_path {
    name.to_string()
  } else {
    pack_index
      .get(name)
      .cloned()
      .ok_or_else(|| LayerParseError::MissingField {
        ctx: ctx.to_string(),
        field: format!("operator_packs.{}", name),
      })?
  };
  Ok(base_dir.join(relative))
}

fn load_operator_pack_from_path(
  path: &Path,
  ctx: &str,
  read: &dyn Fn(&Path) -> Result<String, LayerParseError>,
) -> Result<OperatorPack, LayerParseError> {
  let src = read(path)?;
  let expr = parse_expr(&src).map_err(|err| LayerParseError::Parse(err.to_string()))?;
  match &expr {
    PnixExpr::List(_) => parse_operator_list(&expr, ctx),
    PnixExpr::AttrSet { .. } => {
      let map = expect_attrset(&expr, ctx)?;
      let ops_expr = expect_value(map.get("operators"), ctx, "operators")?;
      let mut pack = parse_operator_list(ops_expr, &format!("{ctx}.operators"))?;
      if let Some(laws_expr) = map.get("laws") {
        let laws = parse_law_names(laws_expr, &format!("{ctx}.laws"))?;
        pack.laws.merge(&laws);
      }
      Ok(pack)
    }
    _ => Err(LayerParseError::InvalidField {
      ctx: ctx.to_string(),
      field: "operators".to_string(),
      expected: "List or AttrSet".to_string(),
    }),
  }
}

fn merge_operator_pack(
  target: &mut OperatorPack,
  source: OperatorPack,
  ctx: &str,
) -> Result<(), LayerParseError> {
  for (token, spec) in source.catalog.operators {
    if target.catalog.contains(&token) {
      return Err(LayerParseError::InvalidField {
        ctx: ctx.to_string(),
        field: "operators".to_string(),
        expected: format!("unique operator token: {}", token),
      });
    }
    target.catalog.register(spec);
  }
  target.laws.merge(&source.laws);
  Ok(())
}

fn parse_operator_laws(expr: &PnixExpr, ctx: &str) -> Result<Vec<OperatorLawRef>, LayerParseError> {
  let list = expect_list(expr, &format!("{ctx}.laws"))?;
  let mut laws = Vec::new();
  for (idx, item) in list.iter().enumerate() {
    let entry_ctx = format!("{ctx}.laws[{idx}]");
    match item {
      PnixExpr::String(name) => laws.push(OperatorLawRef::Name(name.clone())),
      PnixExpr::AttrSet { .. } => {
        let map = expect_attrset(item, &entry_ctx)?;
        let name = expect_string(map.get("name"), &entry_ctx, "name")?;
        let optional = match map.get("optional") {
          Some(PnixExpr::Bool(val)) => *val,
          Some(_) => {
            return Err(LayerParseError::InvalidField {
              ctx: entry_ctx,
              field: "optional".to_string(),
              expected: "Bool".to_string(),
            })
          }
          None => false,
        };
        laws.push(OperatorLawRef::Entry(OperatorLawEntry { name, optional }));
      }
      _ => {
        return Err(LayerParseError::InvalidField {
          ctx: entry_ctx,
          field: "law".to_string(),
          expected: "String or AttrSet".to_string(),
        });
      }
    }
  }
  Ok(laws)
}

fn parse_law_names(expr: &PnixExpr, ctx: &str) -> Result<LayerLawSet, LayerParseError> {
  let list = expect_list(expr, ctx)?;
  let mut names = BTreeSet::new();
  for (idx, item) in list.iter().enumerate() {
    let entry_ctx = format!("{ctx}[{idx}]");
    match item {
      PnixExpr::String(name) => {
        names.insert(name.clone());
      }
      PnixExpr::AttrSet { .. } => {
        let map = expect_attrset(item, &entry_ctx)?;
        let name = expect_string(map.get("name"), &entry_ctx, "name")?;
        names.insert(name);
      }
      _ => {
        return Err(LayerParseError::InvalidField {
          ctx: entry_ctx,
          field: "law".to_string(),
          expected: "String or AttrSet".to_string(),
        });
      }
    }
  }
  Ok(LayerLawSet { names })
}

fn parse_fixity(value: &str, ctx: &str) -> Result<OperatorFixity, LayerParseError> {
  match value {
    "infixl" => Ok(OperatorFixity::Infixl),
    "infixr" => Ok(OperatorFixity::Infixr),
    "infix" => Ok(OperatorFixity::Infix),
    "prefix" => Ok(OperatorFixity::Prefix),
    "postfix" => Ok(OperatorFixity::Postfix),
    _ => Err(LayerParseError::InvalidField {
      ctx: ctx.to_string(),
      field: "fixity".to_string(),
      expected: "infixl|infixr|infix|prefix|postfix".to_string(),
    }),
  }
}

fn parse_string_list(expr: Option<&PnixExpr>, ctx: &str) -> Result<Vec<String>, LayerParseError> {
  let Some(expr) = expr else {
    return Ok(Vec::new());
  };
  let list = expect_list(expr, ctx)?;
  let mut out = Vec::new();
  for (idx, item) in list.iter().enumerate() {
    let entry_ctx = format!("{ctx}[{idx}]");
    match item {
      PnixExpr::String(value) => out.push(value.clone()),
      _ => {
        return Err(LayerParseError::InvalidField {
          ctx: entry_ctx,
          field: "item".to_string(),
          expected: "String".to_string(),
        })
      }
    }
  }
  Ok(out)
}

fn parse_string_set(expr: &PnixExpr, ctx: &str) -> Result<BTreeSet<String>, LayerParseError> {
  let list = expect_list(expr, ctx)?;
  let mut out = BTreeSet::new();
  for (idx, item) in list.iter().enumerate() {
    let entry_ctx = format!("{ctx}[{idx}]");
    match item {
      PnixExpr::String(value) => {
        if value.trim().is_empty() {
          return Err(LayerParseError::InvalidField {
            ctx: entry_ctx,
            field: "item".to_string(),
            expected: "non-empty String".to_string(),
          });
        }
        out.insert(value.clone());
      }
      _ => {
        return Err(LayerParseError::InvalidField {
          ctx: entry_ctx,
          field: "item".to_string(),
          expected: "String".to_string(),
        })
      }
    }
  }
  Ok(out)
}

fn parse_failed_laws(expr: &PnixExpr, ctx: &str) -> Result<BTreeSet<String>, LayerParseError> {
  let list = expect_list(expr, ctx)?;
  let mut out = BTreeSet::new();
  for (idx, item) in list.iter().enumerate() {
    let entry_ctx = format!("{ctx}[{idx}]");
    match item {
      PnixExpr::String(value) => {
        if value.trim().is_empty() {
          return Err(LayerParseError::InvalidField {
            ctx: entry_ctx,
            field: "item".to_string(),
            expected: "non-empty String".to_string(),
          });
        }
        out.insert(value.clone());
      }
      PnixExpr::AttrSet { .. } => {
        let map = expect_attrset(item, &entry_ctx)?;
        let name = expect_string(map.get("law"), &entry_ctx, "law")?;
        if name.trim().is_empty() {
          return Err(LayerParseError::InvalidField {
            ctx: entry_ctx,
            field: "law".to_string(),
            expected: "non-empty String".to_string(),
          });
        }
        out.insert(name);
      }
      _ => {
        return Err(LayerParseError::InvalidField {
          ctx: entry_ctx,
          field: "item".to_string(),
          expected: "String or AttrSet".to_string(),
        })
      }
    }
  }
  Ok(out)
}

fn validate_grammar_doc(expr: &PnixExpr) -> Result<(), LayerParseError> {
  let root = expect_attrset(expr, "layer.grammar")?;
  let grammar_expr = expect_value(root.get("grammar"), "layer.grammar", "grammar")?;
  let grammar_map = expect_attrset(grammar_expr, "layer.grammar.grammar")?;
  if let Some(entry_expr) = grammar_map.get("entry") {
    let entry = expect_string(Some(entry_expr), "layer.grammar.grammar", "entry")?;
    if entry.trim().is_empty() {
      return Err(LayerParseError::InvalidField {
        ctx: "layer.grammar.grammar".to_string(),
        field: "entry".to_string(),
        expected: "non-empty String".to_string(),
      });
    }
  }
  if let Some(reserved_expr) = grammar_map.get("reserved") {
    let reserved = validate_string_list(reserved_expr, "layer.grammar.grammar.reserved")?;
    let mut seen = BTreeSet::new();
    for token in reserved {
      if token.trim().is_empty() {
        return Err(LayerParseError::InvalidField {
          ctx: "layer.grammar.grammar.reserved".to_string(),
          field: "item".to_string(),
          expected: "non-empty String".to_string(),
        });
      }
      if !seen.insert(token) {
        return Err(LayerParseError::InvalidField {
          ctx: "layer.grammar.grammar.reserved".to_string(),
          field: "item".to_string(),
          expected: "unique String".to_string(),
        });
      }
    }
  }
  let rules_expr = expect_value(grammar_map.get("rules"), "layer.grammar.grammar", "rules")?;
  let rules = validate_string_list(rules_expr, "layer.grammar.grammar.rules")?;
  validate_grammar_rules(&rules, "layer.grammar.grammar.rules")
}

fn validate_meaning_doc(expr: &PnixExpr) -> Result<(), LayerParseError> {
  let root = expect_attrset(expr, "layer.meaning")?;
  let meaning_expr = expect_value(root.get("meaning"), "layer.meaning", "meaning")?;
  let meaning_map = expect_attrset(meaning_expr, "layer.meaning.meaning")?;
  let exports_expr = expect_value(
    meaning_map.get("exports"),
    "layer.meaning.meaning",
    "exports",
  )?;
  let exports = validate_string_list(exports_expr, "layer.meaning.meaning.exports")?;
  if exports.is_empty() {
    return Err(LayerParseError::InvalidField {
      ctx: "layer.meaning.meaning.exports".to_string(),
      field: "exports".to_string(),
      expected: "non-empty list".to_string(),
    });
  }
  if let Some(normalize_expr) = meaning_map.get("normalize") {
    let normalize = expect_string(Some(normalize_expr), "layer.meaning.meaning", "normalize")?;
    if normalize.trim().is_empty() {
      return Err(LayerParseError::InvalidField {
        ctx: "layer.meaning.meaning".to_string(),
        field: "normalize".to_string(),
        expected: "non-empty String".to_string(),
      });
    }
  }
  if let Some(eq_expr) = meaning_map.get("eq") {
    let eq = expect_string(Some(eq_expr), "layer.meaning.meaning", "eq")?;
    if eq.trim().is_empty() {
      return Err(LayerParseError::InvalidField {
        ctx: "layer.meaning.meaning".to_string(),
        field: "eq".to_string(),
        expected: "non-empty String".to_string(),
      });
    }
  }
  Ok(())
}

fn validate_string_list(expr: &PnixExpr, ctx: &str) -> Result<Vec<String>, LayerParseError> {
  let list = expect_list(expr, ctx)?;
  let mut out = Vec::new();
  for (idx, item) in list.iter().enumerate() {
    let entry_ctx = format!("{ctx}[{idx}]");
    match item {
      PnixExpr::String(value) => out.push(value.clone()),
      _ => {
        return Err(LayerParseError::InvalidField {
          ctx: entry_ctx,
          field: "item".to_string(),
          expected: "String".to_string(),
        })
      }
    }
  }
  Ok(out)
}

fn validate_grammar_rules(rules: &[String], ctx: &str) -> Result<(), LayerParseError> {
  for (idx, rule) in rules.iter().enumerate() {
    let entry_ctx = format!("{ctx}[{idx}]");
    let trimmed = rule.trim();
    if trimmed.is_empty() {
      return Err(LayerParseError::InvalidField {
        ctx: entry_ctx,
        field: "rule".to_string(),
        expected: "non-empty rule".to_string(),
      });
    }
    let mut parts = trimmed.splitn(2, '=');
    let lhs = parts.next().unwrap_or("").trim();
    let rhs = parts.next().unwrap_or("").trim();
    if lhs.is_empty() || rhs.is_empty() {
      return Err(LayerParseError::InvalidField {
        ctx: entry_ctx,
        field: "rule".to_string(),
        expected: "lhs = rhs".to_string(),
      });
    }
    if !is_ident_token(lhs) {
      return Err(LayerParseError::InvalidField {
        ctx: entry_ctx,
        field: "lhs".to_string(),
        expected: "IDENT".to_string(),
      });
    }
    for token in rhs.split_whitespace() {
      if !is_allowed_rule_token(token) {
        return Err(LayerParseError::InvalidField {
          ctx: entry_ctx,
          field: "rhs token".to_string(),
          expected: "IDENT | INT | ( | ) | | | + | - | * | / | , | -> | :: | ? | quoted"
            .to_string(),
        });
      }
    }
  }
  Ok(())
}

fn is_allowed_rule_token(token: &str) -> bool {
  matches!(
    token,
    "|" | "(" | ")" | "+" | "-" | "*" | "/" | "," | "->" | "::" | "?"
  ) || is_ident_token(token)
    || is_quoted_literal_token(token)
}

fn is_quoted_literal_token(token: &str) -> bool {
  if token.len() < 2 {
    return false;
  }
  let bytes = token.as_bytes();
  let first = bytes[0] as char;
  let last = bytes[bytes.len() - 1] as char;
  (first == '\'' && last == '\'') || (first == '"' && last == '"')
}

fn is_ident_token(token: &str) -> bool {
  let mut segments = token.split('.');
  let Some(first) = segments.next() else {
    return false;
  };
  if !is_ident_segment(first) {
    return false;
  }
  for segment in segments {
    if !is_ident_segment(segment) {
      return false;
    }
  }
  true
}

fn is_ident_segment(segment: &str) -> bool {
  let mut chars = segment.chars();
  let Some(first) = chars.next() else {
    return false;
  };
  if !is_ident_start(first) {
    return false;
  }
  chars.all(is_ident_continue)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn reject_invalid_grammar_token() {
    let src = "{ grammar = { rules = [ \"expr = @bad\" ]; }; }";
    let expr = parse_expr(src).unwrap();
    let err = validate_grammar_doc(&expr).unwrap_err();
    assert!(matches!(err, LayerParseError::InvalidField { .. }));
  }

  #[test]
  fn accept_arrow_and_quoted_tokens() {
    let src = r#"{ grammar = { rules = [ "expr = a -> b" "term = '(' expr ')'" "ann = a :: b" "opt = a ? b" ]; }; }"#;
    let expr = parse_expr(src).unwrap();
    assert!(validate_grammar_doc(&expr).is_ok());
  }

  #[test]
  fn parse_fixity_accepts_valid() {
    assert!(matches!(
      parse_fixity("infixl", "fixity"),
      Ok(OperatorFixity::Infixl)
    ));
    assert!(matches!(
      parse_fixity("prefix", "fixity"),
      Ok(OperatorFixity::Prefix)
    ));
  }

  #[test]
  fn parse_fixity_rejects_invalid() {
    let err = parse_fixity("invalid", "fixity").unwrap_err();
    assert!(matches!(err, LayerParseError::InvalidField { .. }));
  }

  #[test]
  fn accept_pipe_and_ident_tokens() {
    let src = r#"{ grammar = { rules = [ "expr = a | b" "term = alpha.beta" ]; }; }"#;
    let expr = parse_expr(src).unwrap();
    assert!(validate_grammar_doc(&expr).is_ok());
  }
}

fn is_ident_start(ch: char) -> bool {
  ch.is_ascii_alphabetic() || ch == '_'
}

fn is_ident_continue(ch: char) -> bool {
  ch.is_ascii_alphanumeric() || ch == '_'
}

fn parse_file_spec(
  expr: Option<&PnixExpr>,
  ctx: &str,
  field: &str,
) -> Result<Option<LayerFileSpec>, LayerParseError> {
  let Some(expr) = expr else {
    return Ok(None);
  };
  let spec_ctx = format!("{}.{}", ctx, field);
  let map = expect_attrset(expr, &spec_ctx)?;
  let kind = expect_string(map.get("kind"), &spec_ctx, "kind")?;
  let file = expect_string(map.get("file"), &spec_ctx, "file")?;
  Ok(Some(LayerFileSpec { kind, file }))
}

fn expect_attrset(
  expr: &PnixExpr,
  ctx: &str,
) -> Result<BTreeMap<String, PnixExpr>, LayerParseError> {
  match expr {
    PnixExpr::AttrSet { items, .. } => attrset_to_map(items, ctx),
    _ => Err(LayerParseError::InvalidField {
      ctx: ctx.to_string(),
      field: "attrset".to_string(),
      expected: "AttrSet".to_string(),
    }),
  }
}

fn attrset_to_map(
  items: &[PnixAttrItem],
  ctx: &str,
) -> Result<BTreeMap<String, PnixExpr>, LayerParseError> {
  let mut map = BTreeMap::new();
  for item in items {
    match item {
      PnixAttrItem::Assign {
        key_path, value, ..
      } => {
        if key_path.len() != 1 {
          return Err(LayerParseError::InvalidField {
            ctx: ctx.to_string(),
            field: "key_path".to_string(),
            expected: "single segment".to_string(),
          });
        }
        let key = key_path[0].clone();
        if map.insert(key, (**value).clone()).is_some() {
          return Err(LayerParseError::Unsupported(format!(
            "duplicate key in {}",
            ctx
          )));
        }
      }
      _ => {
        return Err(LayerParseError::Unsupported(
          "dynamic/inherit attrset entries are not supported".to_string(),
        ))
      }
    }
  }
  Ok(map)
}

fn expect_value<'a>(
  value: Option<&'a PnixExpr>,
  ctx: &str,
  field: &str,
) -> Result<&'a PnixExpr, LayerParseError> {
  value.ok_or_else(|| LayerParseError::MissingField {
    ctx: ctx.to_string(),
    field: field.to_string(),
  })
}

fn expect_string(
  value: Option<&PnixExpr>,
  ctx: &str,
  field: &str,
) -> Result<String, LayerParseError> {
  let value = expect_value(value, ctx, field)?;
  match value {
    PnixExpr::String(s) => Ok(s.clone()),
    _ => Err(LayerParseError::InvalidField {
      ctx: ctx.to_string(),
      field: field.to_string(),
      expected: "String".to_string(),
    }),
  }
}

fn expect_int(value: Option<&PnixExpr>, ctx: &str, field: &str) -> Result<u8, LayerParseError> {
  let value = expect_value(value, ctx, field)?;
  match value {
    PnixExpr::Int(v) => (*v).try_into().map_err(|_| LayerParseError::InvalidField {
      ctx: ctx.to_string(),
      field: field.to_string(),
      expected: "Int within u8 range".to_string(),
    }),
    _ => Err(LayerParseError::InvalidField {
      ctx: ctx.to_string(),
      field: field.to_string(),
      expected: "Int".to_string(),
    }),
  }
}

fn expect_list<'a>(expr: &'a PnixExpr, ctx: &str) -> Result<&'a [PnixExpr], LayerParseError> {
  match expr {
    PnixExpr::List(items) => Ok(items),
    _ => Err(LayerParseError::InvalidField {
      ctx: ctx.to_string(),
      field: "list".to_string(),
      expected: "List".to_string(),
    }),
  }
}
