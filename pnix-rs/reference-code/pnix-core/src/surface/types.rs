//! Surface IR types

use serde::{Deserialize, Serialize};

/// Surface 모듈: Surface IR의 최상위 모듈 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceModule {
  /// 모듈 이름
  pub name: String,
  /// 타입 이름 목록 (Stage-1 호환)
  pub types: Vec<String>,
  /// Y09: ADT 타입 선언 (enum/Option/Result)
  #[serde(default)]
  pub adt_types: Vec<SurfaceAdtType>,
  /// Stage-2: 외부 입력 선언 목록
  #[serde(default)]
  pub inputs: Vec<SurfaceInput>,
  /// 외부 선언 목록 (Extern)
  pub decls: Vec<SurfaceDecl>,
  /// 노드 선언 목록
  pub nodes: Vec<SurfaceNode>,
  /// 엣지 선언 목록
  pub edges: Vec<SurfaceEdge>,
  /// Stage-4.1: scope 정의 목록
  #[serde(default)]
  pub scopes: Vec<SurfaceScope>,
}

/// Surface 외부 입력: Stage-2에서 외부로부터 입력받는 값
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceInput {
  /// 입력 이름
  pub name: String,
  /// 입력 타입
  pub ty: String,
}

/// Y09: ADT variant: ADT의 한 가지 경우 (예: Some a, None, Ok a, Err e)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceAdtVariant {
  /// Variant 이름
  pub name: String,
  /// 필드 타입 목록 (nullary variant는 빈 목록)
  #[serde(default)]
  pub fields: Vec<String>,
}

/// Y09: ADT 타입 선언: ADT 타입 정의 (예: Option a = None | Some a)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceAdtType {
  /// ADT 타입 이름
  pub name: String,
  /// 타입 파라미터 목록 (예: ["a"] for Option a)
  #[serde(default)]
  pub params: Vec<String>,
  /// Variant 목록
  pub variants: Vec<SurfaceAdtVariant>,
}

/// Surface 포트: Stage-2에서 노드의 입력/출력 포트
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfacePort {
  /// 포트 이름
  pub name: String,
  /// 포트 타입
  pub ty: String,
}

/// Surface 선언: Surface IR의 선언 타입
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SurfaceDecl {
  /// 외부 선언 (Extern)
  Extern {
    /// Extern 이름
    name: String,
    /// Stage-1 호환: 단일 입력 타입
    input: String,
    /// Stage-1 호환: 단일 출력 타입
    output: String,
    /// Stage-2: 입력 포트 목록
    #[serde(default)]
    inputs: Vec<SurfacePort>,
    /// Stage-2: 출력 포트 목록
    #[serde(default)]
    outputs: Vec<SurfacePort>,
  },
}

/// Surface 노드: Stage-3/3.1/4.1/4.2에서 수정자를 지원하는 노드 선언
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceNode {
  /// 노드 이름
  pub name: String,
  /// 사용하는 Extern 이름
  pub uses: String,
  /// Stage-3: 노드 종류 ("gate" 등)
  #[serde(default)]
  pub kind: Option<String>,
  /// Stage-3.1: optional 노드 여부
  #[serde(default)]
  pub optional: bool,
  /// Stage-4.1: scope 이름 (노드가 속한 scope)
  #[serde(default)]
  pub scope: Option<String>,
  /// Stage-4.2: 비용 힌트 ("tiny", "light", "medium", "heavy", "xheavy")
  #[serde(default)]
  pub cost: Option<String>,
  /// Stage-4.2: 우선순위 (숫자가 클수록 높은 우선순위)
  #[serde(default)]
  pub priority: Option<i32>,
}

/// Surface scope 정의: Stage-4.1에서 scope의 정책 정의
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceScope {
  /// Scope 이름
  pub name: String,
  /// 정책 이름 ("failfast", "isolate", "best_effort")
  pub policy: String,
}

/// Surface 엣지 소스: Stage-2에서 엣지의 출발점
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SurfaceEdgeSource {
  /// 외부 입력 (예: input.M1)
  Input {
    /// 입력 이름
    name: String,
  },
  /// 노드 출력 (예: node.port)
  Node {
    /// 노드 이름
    node: String,
    /// 포트 이름 (None이면 기본 포트)
    port: Option<String>,
  },
}

/// Surface 엣지 타겟: Stage-2에서 엣지의 도착점
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceEdgeTarget {
  /// 노드 이름
  pub node: String,
  /// 포트 이름 (None이면 기본 포트)
  pub port: Option<String>,
}

/// Surface 엣지 엔드포인트: Stage-1 호환을 위한 엔드포인트 표현
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceEndpoint {
  /// 노드 이름
  pub node: String,
  /// 포트 이름 (None이면 기본 포트, Stage-1 호환)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub port: Option<String>,
}

/// Surface 엣지 조건: Stage-3.2/4에서 엣지 활성화 조건
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SurfaceEdgeCond {
  /// when 조건: gate가 true일 때 활성화
  When(
    /// gate 이름
    String,
  ),
  /// unless 조건: gate가 false일 때 활성화
  Unless(
    /// gate 이름
    String,
  ),
  /// onfail 조건: 노드가 실패했을 때 활성화
  OnFail(
    /// 노드 이름
    String,
  ),
  /// 복합 조건: when gate가 true이고 unless gate가 false일 때 활성화
  WhenUnless {
    /// when gate 이름
    when: String,
    /// unless gate 이름
    unless: String,
  },
  /// 복합 조건: 나열된 모든 gate가 true일 때 활성화
  AllWhen(
    /// gate 이름 목록
    Vec<String>,
  ),
  /// 복합 조건: 나열된 모든 gate가 false일 때 활성화
  AllUnless(
    /// gate 이름 목록
    Vec<String>,
  ),
}

/// Surface 엣지: Stage-2에서 포트/입력 지원, Stage-3.2에서 조건 지원
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceEdge {
  /// Stage-1 호환: 출발점 (포트 없는 단순 노드명, input인 경우 "input")
  pub from: String,
  /// Stage-1 호환: 도착점 (포트 없는 단순 노드명)
  pub to: String,
  /// Stage-2: 포트 포함 출발점 엔드포인트
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub from_endpoint: Option<SurfaceEndpoint>,
  /// Stage-2: 포트 포함 도착점 엔드포인트
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub to_endpoint: Option<SurfaceEndpoint>,
  /// Stage-2: 입력 이름 (from이 input인 경우)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub from_input: Option<String>,
  /// Stage-3.2/4: 엣지 활성화 조건 (when/unless/onfail)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub cond: Option<SurfaceEdgeCond>,
}

impl SurfaceEdge {
  /// Stage-1 단순 엣지 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn simple(from: String, to: String) -> Self {
    Self {
      from: from.clone(),
      to: to.clone(),
      from_endpoint: Some(SurfaceEndpoint {
        node: from,
        port: None,
      }),
      to_endpoint: Some(SurfaceEndpoint {
        node: to,
        port: None,
      }),
      from_input: None,
      cond: None,
    }
  }

  /// Stage-2 포트 엣지 생성 (node.port -> node.port)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn ported(
    from_node: String,
    from_port: Option<String>,
    to_node: String,
    to_port: Option<String>,
  ) -> Self {
    Self {
      from: from_node.clone(),
      to: to_node.clone(),
      from_endpoint: Some(SurfaceEndpoint {
        node: from_node,
        port: from_port,
      }),
      to_endpoint: Some(SurfaceEndpoint {
        node: to_node,
        port: to_port,
      }),
      from_input: None,
      cond: None,
    }
  }

  /// Stage-2 입력 엣지 생성 (input.X -> node.port)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn from_input(input_name: String, to_node: String, to_port: Option<String>) -> Self {
    Self {
      from: "input".to_string(),
      to: to_node.clone(),
      from_endpoint: None,
      to_endpoint: Some(SurfaceEndpoint {
        node: to_node,
        port: to_port,
      }),
      from_input: Some(input_name),
      cond: None,
    }
  }

  /// Stage-3.2 조건 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn with_cond(mut self, cond: SurfaceEdgeCond) -> Self {
    self.cond = Some(cond);
    self
  }

  /// 입력 소스인지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn is_input_source(&self) -> bool {
    self.from_input.is_some()
  }
}
