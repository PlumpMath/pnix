//! Build IR types
//!
//! 그래프 - "무엇을 만들 것인가"만 표현
//! 실행 순서/절차 없음

use crate::core::FxCoreModule;
use serde::{Deserialize, Serialize};

/// Build IR versioning
pub const BUILD_IR_VERSION: &str = "1.0";

/// 표준 toolchain 이름 맵
///
/// 다양한 이름으로 표현될 수 있는 toolchain을 표준 이름으로 정규화합니다.
/// 결정성 보장: 동일한 toolchain이 항상 동일한 이름으로 표현됩니다.
fn normalize_toolchain_name(name: &str) -> String {
  match name.to_lowercase().as_str() {
    "rust" | "rustc" | "cargo" => "rustc".to_string(),
    "clang" | "clang++" | "clang-15" | "clang-16" | "clang-17" | "clang-18" => "clang".to_string(),
    "gcc" | "g++" => "gcc".to_string(),
    "jdk" | "java" | "javac" | "jdk21" | "jdk17" | "jdk11" => "jdk".to_string(),
    "clojure" | "clj" | "cljs" | "clojurescript" => "jdk".to_string(),
    "deno" | "deno-runtime" => "deno".to_string(),
    "nix" | "nix-build" | "nix-shell" => "nix".to_string(),
    "python" | "python3" | "py" | "cpython" => "python".to_string(),
    "node" | "nodejs" | "npm" => "node".to_string(),
    _ => name.to_string(),
  }
}

/// Build IR: 빌드 중간 표현
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildIr {
  /// Build IR 버전
  pub version: String,
  /// 빌드 스펙
  pub build: BuildSpec,
}

/// 빌드 스펙: 빌드 명세 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildSpec {
  /// 빌드 이름
  pub name: String,
  /// 타겟 트리플 (OS + 아키텍처)
  pub target: TargetTriple,
  /// 아티팩트 종류 (Binary/Lib/Service)
  pub artifact: ArtifactKind,
  /// 의존성 정보
  pub deps: Deps,
  /// 빌드 계약 (순수성, 재현 가능성)
  pub contract: Contract,
}

/// 타겟 트리플: 빌드 타겟의 OS와 아키텍처
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetTriple {
  /// 운영체제
  pub os: Os,
  /// 아키텍처
  pub arch: Arch,
}

/// Operating system
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Os {
  /// Linux 운영체제
  Linux,
  /// macOS (Darwin) 운영체제
  Darwin,
  /// Windows 운영체제
  Windows,
}

/// Architecture
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Arch {
  /// x86_64 아키텍처
  X86_64,
  /// ARM64 (aarch64) 아키텍처
  Aarch64,
}

/// Artifact kind
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArtifactKind {
  /// 바이너리 실행 파일
  Binary,
  /// 라이브러리
  Lib,
  /// 서비스
  Service,
}

/// 의존성: 빌드에 필요한 툴체인과 라이브러리
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Deps {
  /// 툴체인 목록 (예: ["rustc", "python"])
  pub toolchain: Vec<String>,
  /// 라이브러리 목록 (예: ["blas", "lapack"])
  pub lib: Vec<String>,
}

/// 빌드 계약: 빌드의 순수성과 재현 가능성 보장
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
  /// 순수 함수 여부 (부작용 없음)
  pub pure: bool,
  /// 재현 가능 여부 (동일 입력에 동일 출력)
  pub reproducible: bool,
}

impl BuildIr {
  /// FxCore에서 Build IR 생성 (target은 옵션으로 주입)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn from_fxcore(fx: &FxCoreModule, target_os: Os, target_arch: Arch) -> Self {
    let mut spec = BuildSpec {
      name: fx.name.clone(),
      target: TargetTriple {
        os: target_os,
        arch: target_arch,
      },
      artifact: ArtifactKind::Service,
      deps: Deps::default(),
      contract: Contract {
        pure: true,
        reproducible: true,
      },
    };

    // extern prefix 기반 최소 추론
    // 결정성 보장: morphisms를 이름 기준으로 정렬하여 순서 일관성 유지
    let mut sorted_morphisms: Vec<_> = fx.morphisms.iter().collect();
    sorted_morphisms.sort_by(|a, b| a.name.cmp(&b.name));

    for m in sorted_morphisms {
      let n = m.name.as_str();

      // clojure/clj/cljs.* → jdk toolchain 필요
      if n.starts_with("clojure.")
        || n.starts_with("clj.")
        || n.starts_with("cljs.")
        || n.starts_with("clojurescript.")
      {
        push_unique(&mut spec.deps.toolchain, "jdk21");
      }

      // py.* → python runner 필요
      if n.starts_with("py.") {
        push_unique(&mut spec.deps.toolchain, "python");
      }

      // deno.* → deno 필요
      if n.starts_with("deno.") || n.starts_with("js.") || n.starts_with("ts.") {
        push_unique(&mut spec.deps.toolchain, "deno");
      }

      // nix.* → nix toolchain 필요
      if n.starts_with("nix.") {
        push_unique(&mut spec.deps.toolchain, "nix");
      }

      // 선형대수 등 lib 힌트
      if n.contains("linear") || n.contains("linalg") {
        push_unique(&mut spec.deps.lib, "blas");
      }
    }

    // 결정성 보장: toolchain 이름 정규화 및 중복 제거
    let mut normalized_toolchains: Vec<String> = spec
      .deps
      .toolchain
      .iter()
      .map(|s| normalize_toolchain_name(s))
      .collect();
    normalized_toolchains.sort();
    normalized_toolchains.dedup();
    spec.deps.toolchain = normalized_toolchains;

    // 결정성 보장: lib 벡터 정렬 및 중복 제거
    spec.deps.lib.sort();
    spec.deps.lib.dedup();

    Self {
      version: BUILD_IR_VERSION.into(),
      build: spec,
    }
  }
}

fn push_unique(v: &mut Vec<String>, item: &str) {
  if !v.iter().any(|x| x == item) {
    v.push(item.to_string());
  }
}
