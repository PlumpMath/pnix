//! AST types - 실행 의미 없는 선언적 구조만

use crate::diagnostics::Span;
use serde::{Deserialize, Serialize};

/// AST 모듈: AST의 최상위 모듈 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstModule {
  /// 모듈 이름
  pub name: String,
  /// 모듈 항목 목록
  pub items: Vec<AstItem>,
}

/// 포트 AST: Stage-2에서 포트의 이름과 타입
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortAst {
  /// 포트 이름
  pub name: String,
  /// 포트 타입
  pub ty: String,
}

/// 시그니처 AST: Stage-2에서 포트 기반 시그니처
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigAst {
  /// Stage-1 호환: 단일 입력 타입 (포트가 없으면 이 값을 사용)
  pub input: String,
  /// Stage-1 호환: 단일 출력 타입
  pub output: String,
  /// Stage-2: 입력 포트 목록
  #[serde(default)]
  pub inputs: Vec<PortAst>,
  /// Stage-2: 출력 포트 목록
  #[serde(default)]
  pub outputs: Vec<PortAst>,
}

impl SigAst {
  /// Stage-1 호환 생성자
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn simple(input: String, output: String) -> Self {
    Self {
      input: input.clone(),
      output: output.clone(),
      inputs: vec![PortAst {
        name: "in".into(),
        ty: input,
      }],
      outputs: vec![PortAst {
        name: "out".into(),
        ty: output,
      }],
    }
  }

  /// Stage-2 포트 기반 생성자
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn ported(inputs: Vec<PortAst>, outputs: Vec<PortAst>) -> Self {
    let input = inputs.first().map(|p| p.ty.clone()).unwrap_or_default();
    let output = outputs.first().map(|p| p.ty.clone()).unwrap_or_default();
    Self {
      input,
      output,
      inputs,
      outputs,
    }
  }
}

/// 엣지 소스 AST: Stage-2에서 엣지의 출발점 (input 또는 node.port)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EdgeSource {
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

/// 엣지 타겟 AST: Stage-2에서 엣지의 도착점 (node.port)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeTarget {
  /// 노드 이름
  pub node: String,
  /// 포트 이름 (None이면 기본 포트)
  pub port: Option<String>,
}

/// 엣지 엔드포인트 AST: Stage-1 호환을 위한 엔드포인트 표현 (노드.포트)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeEndpoint {
  /// 노드 이름
  pub node: String,
  /// 포트 이름 (None이면 기본 포트, Stage-1 호환)
  pub port: Option<String>,
}

/// AST 항목 - 실행 의미 없는 선언만
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AstItem {
  /// 타입 선언 (Stage-1) - 단순 타입 이름
  TypeDecl { name: String, span: Span },

  /// ADT 타입 선언 - enum/Option/Result 등
  /// 문법: `type Option a = None | Some a`
  AdtTypeDecl(AdtTypeDecl),

  /// 입력 선언 (Stage-2)
  InputDecl {
    name: String,
    ty: String,
    span: Span,
  },

  /// 외부 선언 (시그니처 포함)
  ExternDecl {
    name: String,
    sig: SigAst,
    span: Span,
  },

  /// 노드 선언 (Stage-3/3.1/4.1/4.2: 수정자 지원)
  /// 문법: `node <Name> uses <Extern> [gate] [optional] [scope S] [cost C] [priority P]`
  NodeDecl {
    name: String,
    uses: String,
    /// Stage-3: "gate" 가능
    #[serde(default)]
    kind: Option<String>,
    /// Stage-3.1: optional 노드
    #[serde(default)]
    optional: bool,
    /// Stage-4.1: scope 이름
    #[serde(default)]
    scope: Option<String>,
    /// Stage-4.2: cost hint ("tiny", "light", "medium", "heavy", "xheavy")
    #[serde(default)]
    cost: Option<String>,
    /// Stage-4.2: priority
    #[serde(default)]
    priority: Option<i32>,
    span: Span,
  },

  /// 엣지 선언 (Stage-2: 포트/입력 지원, Stage-3.2: 조건)
  /// 문법: `edge <From> -> <To> [when G] [unless G] [onfail N]`
  EdgeDecl {
    from: EdgeSource,
    to: EdgeTarget,
    /// Stage-3.2/4: 조건 (when/unless/onfail)
    #[serde(default)]
    cond: Option<EdgeCondAst>,
    span: Span,
  },

  /// Scope 선언 (Stage-4.1)
  /// 문법: `scope <Name> policy <Policy>`
  ScopeDecl {
    name: String,
    /// "failfast", "isolate", "best_effort"
    policy: String,
    span: Span,
  },
  /// Import 선언: `imports = [ "./module.px", ... ]`
  ImportDecl {
    /// Import 경로 (상대 또는 절대)
    path: String,
    span: Span,
  },

  /// 테스트 선언 (Y11a): `test <Name> = <Expr>`
  /// 또는 `@test node <Name> uses <Extern>`
  TestDecl {
    /// 테스트 이름
    name: String,
    /// 테스트 표현식 (Pnix 표현식 문자열 또는 node 선언)
    expr: String,
    span: Span,
  },
}

/// 엣지 조건 AST: Stage-3.2/4에서 엣지 활성화 조건
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EdgeCondAst {
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

// ============================================================
// Pattern AST (Destructuring)
// ============================================================

/// 패턴 AST: 구조 분해/매칭에 사용되는 패턴 표현
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternAst {
  /// 식별자 패턴: 변수 이름
  Ident(
    /// 변수 이름
    String,
  ),
  /// 속성 집합 패턴: { field1, field2, ... }
  AttrSet(
    /// 필드 목록
    Vec<PatternFieldAst>,
  ),
  /// 리스트 패턴: [item1, item2, ...tail]
  List(
    /// 리스트 패턴 구조
    PatternListAst,
  ),
}

/// 패턴 필드 AST: 속성 집합 패턴의 필드
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternFieldAst {
  /// 필드 이름
  pub name: String,
  /// 기본값 (선택적, 패턴 매칭 실패 시 사용)
  #[serde(default)]
  pub default: Option<String>,
}

/// 패턴 리스트 AST: 리스트 패턴 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternListAst {
  /// 항목 패턴 목록
  #[serde(default)]
  pub items: Vec<String>,
  /// 나머지 항목 바인딩 (선택적, tail 패턴)
  #[serde(default)]
  pub tail: Option<String>,
}

// ============================================================
// ADT (Algebraic Data Types) Support
// ============================================================

/// ADT 변이 정의: ADT의 한 가지 경우
/// 예: `Some a` → AdtVariant { name: "Some", fields: ["a"] }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdtVariant {
  /// 변이 이름 (예: "Some", "None", "Ok", "Err")
  pub name: String,
  /// 변이가 포함하는 필드 타입 목록 (타입 변수 또는 구체적 타입, nullary variant는 빈 목록)
  #[serde(default)]
  pub fields: Vec<String>,
}

/// ADT 타입 선언 AST: ADT 타입 정의
/// 예: `type Option a = None | Some a`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdtTypeDecl {
  /// 타입 이름 (예: "Option", "Result", "List")
  pub name: String,
  /// 타입 파라미터 목록 (예: ["a"] for Option, ["a", "e"] for Result)
  #[serde(default)]
  pub params: Vec<String>,
  /// 변이 목록 (예: [None, Some a])
  pub variants: Vec<AdtVariant>,
  /// 소스 위치
  pub span: Span,
}
