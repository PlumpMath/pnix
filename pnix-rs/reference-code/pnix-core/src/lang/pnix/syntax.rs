//! PNIX 표면 문법 AST
//!
//! `.sam/.px` (PNIX) 프로그램의 표준 표면 언어입니다.
//! 파싱/AST는 `pnix-core`에 위치 (실행 없음), 평가는 런타임에서 수행됩니다.

use crate::diagnostics::Span;
use std::sync::Arc;

/// 문자열 보간 부분: 문자열 보간에서 리터럴 또는 표현식 부분
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum StringInterpPart {
  /// 리터럴 문자열 부분 (일반 텍스트)
  Lit(
    /// 리터럴 문자열
    String,
  ),
  /// 임베디드 표현식 ${...} (평가될 표현식)
  Expr(
    /// 표현식
    Arc<PnixExpr>,
  ),
}

/// 경로 리터럴 타입: Pnix 경로 리터럴의 종류 (Y10b)
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PnixPath {
  /// 상대 경로: ./foo, ./bar/baz, ../parent
  Relative(
    /// 경로 문자열
    String,
  ),
  /// 절대 경로: /etc/nixos, /home/user
  Absolute(
    /// 경로 문자열
    String,
  ),
  /// 검색 경로: `<nixpkgs>`, `<nixpkgs/lib>`
  Search(
    /// 경로 문자열
    String,
  ),
  /// 홈 경로: ~/foo, ~/.config
  Home(
    /// 경로 문자열
    String,
  ),
  /// 보간된 경로: `./${foo}`, `./a-${foo}-b`, `/${foo}`,
  /// `./${foo + "/bar"}`, `~/${foo}`, `./bar/${foo}`,
  /// `/${foo}/${"bar"}`. evaluator 가 각 part 를 string 으로 coerce
  /// 한 뒤 base prefix 와 합쳐 최종 path 를 만든다.
  Interpolated {
    /// 경로 base 종류 (Relative `./` / Absolute `/` / Home `~/`).
    base: PnixPathBase,
    /// path 안의 literal/interp 부분 목록.
    parts: Vec<StringInterpPart>,
  },
}

/// 보간된 path 의 base prefix 종류.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PnixPathBase {
  /// `./` 시작 (현재 import base 기준 상대).
  Relative,
  /// `/` 시작 (절대 경로).
  Absolute,
  /// `~/` 시작 (홈 디렉토리 기준).
  Home,
}

/// PNIX 표현식 AST: PNIX 표현식의 추상 구문 트리 (runtime-legacy에서 사용하는 최소 Nix 부분집합)
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PnixExpr {
  /// 정수 리터럴 (정수 값)
  Int(
    /// 정수 값
    i64,
  ),
  /// 부동소수점 리터럴 (실수 값)
  Float(
    /// 실수 값
    f64,
  ),
  /// 불리언 리터럴 (불리언 값)
  Bool(
    /// 불리언 값
    bool,
  ),
  /// null 리터럴 (null 값)
  Null,
  /// 문자열 리터럴 (문자열 값)
  String(
    /// 문자열 값
    String,
  ),
  /// 문자열 보간: Y10a, "hello ${name}!" → [Lit("hello "), Expr(name), Lit("!")]
  StringInterp(
    /// 보간 부분 목록 (리터럴과 표현식 부분 목록)
    Vec<StringInterpPart>,
  ),
  /// 경로 리터럴: Y10b, `./foo`, `/abs/path`, `<nixpkgs>`, `~/home`
  Path(
    /// 경로 타입과 값
    PnixPath,
  ),
  /// 변수 참조 (변수 이름)
  Var(
    /// 변수 이름
    String,
  ),
  /// let 바인딩: let x = 1; y = 2; in x + y
  Let {
    /// 바인딩 목록 (let 바인딩 목록)
    bindings: Vec<PnixLetBinding>,
    /// 본문 표현식 (let 표현식의 본문)
    body: Arc<PnixExpr>,
  },
  /// 조건식: if cond then then_ else else_
  If {
    /// 조건 표현식 (if 조건)
    cond: Arc<PnixExpr>,
    /// then 절 (조건이 참일 때 평가할 표현식)
    then_: Arc<PnixExpr>,
    /// else 절 (조건이 거짓일 때 평가할 표현식)
    else_: Arc<PnixExpr>,
  },
  /// 람다 함수: x: x + 1
  Lambda {
    /// 파라미터 패턴 (함수 파라미터 패턴)
    param: PnixParamPattern,
    /// 본문 표현식 (함수 본문)
    body: Arc<PnixExpr>,
  },
  /// 함수 적용: f x
  Apply {
    /// 함수 표현식 (호출할 함수)
    func: Arc<PnixExpr>,
    /// 인자 표현식 (함수에 전달할 인자)
    arg: Arc<PnixExpr>,
  },
  /// 속성 집합: Attribute set, optionally recursive (rec { ... })
  AttrSet {
    /// 속성 항목 목록 (속성 집합의 항목 목록)
    items: Vec<PnixAttrItem>,
    /// 재귀 여부 (true이면 바인딩들이 서로 참조 가능, 예: rec { x = 1; y = x + 1; })
    recursive: bool,
  },
  /// 리스트 리터럴: [1, 2, 3]
  List(
    /// 요소 표현식 목록 (리스트 요소 목록)
    Vec<PnixExpr>,
  ),
  /// 속성 선택: x.y
  Select {
    /// 기준 표현식 (속성을 선택할 대상 표현식)
    base: Arc<PnixExpr>,
    /// 속성 이름 (선택할 속성 이름)
    attr: String,
  },
  /// 속성 선택 with 기본값: x.y or default
  /// x.y가 존재하지 않으면 default를 평가함
  SelectOrDefault {
    /// 기준 표현식 (속성을 선택할 대상 표현식)
    base: Arc<PnixExpr>,
    /// 속성 이름 (선택할 속성 이름)
    attr: String,
    /// 기본값 표현식 (속성이 없을 때 사용할 기본값)
    default: Arc<PnixExpr>,
  },
  /// 리스트 인덱싱: `list[0]`, `list[index]`
  /// Lowering 중에 builtins.elemAt(list, index)로 변환됨
  Index {
    /// 기준 리스트 표현식 (인덱싱할 리스트)
    base: Arc<PnixExpr>,
    /// 인덱스 표현식 (인덱스 값)
    index: Arc<PnixExpr>,
  },
  /// 이항 연산: +, -, *, /, ==, !=, <, >, <=, >=, &&, || 등
  Binary {
    /// 연산자 (연산자 문자열). pre-parse embedding (2026-05-16) 위해
    /// `&'static str` 에서 `Box<str>` 로 전환: serde 가 recursive AST
    /// 의 임의 `'de` lifetime 으로 deserialize 가능하게.
    op: Box<str>,
    /// 왼쪽 피연산자 (왼쪽 표현식)
    lhs: Arc<PnixExpr>,
    /// 오른쪽 피연산자 (오른쪽 표현식)
    rhs: Arc<PnixExpr>,
  },
  /// 단항 연산: -x (부정), !x (논리 부정)
  Unary {
    /// 연산자 ("-" for negate, "!" for not). See Binary::op note.
    op: Box<str>,
    /// 피연산자 (단항 연산을 적용할 표현식)
    arg: Arc<PnixExpr>,
  },
  /// ADT 값 생성자: Some(42), None, Ok(x), Err("msg")
  Construct {
    /// Variant 이름 (예: "Some", "None", "Ok", "Err")
    variant: String,
    /// 인자 목록 (None 같은 nullary constructor는 빈 목록)
    args: Vec<PnixExpr>,
  },
  /// 패턴 매칭 표현식: match expr with | Pat1 => e1 | Pat2 => e2
  Match {
    /// 매칭 대상 표현식 (매칭할 표현식)
    scrutinee: Arc<PnixExpr>,
    /// 매칭 암 목록 (패턴 => 표현식 쌍 목록)
    arms: Vec<PnixMatchArm>,
  },
  /// import 표현식: N01a, import ./file.nix or import "path"
  Import {
    /// 경로 표현식 (일반적으로 문자열 또는 경로 리터럴)
    path: Arc<PnixExpr>,
  },
  /// with 표현식: `with pkgs; [ gcc make ]` → scope extension
  /// `env`의 모든 속성을 `body`에서 변수로 사용 가능하게 함
  With {
    /// 스코프에 가져올 속성 집합 (스코프에 추가할 속성 집합)
    env: Arc<PnixExpr>,
    /// 본문 표현식 (env의 속성들이 접근 가능한 본문)
    body: Arc<PnixExpr>,
  },
  /// assert 표현식: Y10e, `assert cond; body` → cond가 참이면 body 평가, 거짓이면 throw
  Assert {
    /// 조건 표현식 (참이어야 하는 조건)
    cond: Arc<PnixExpr>,
    /// 본문 표현식 (cond가 참일 때 평가할 본문)
    body: Arc<PnixExpr>,
  },
  /// HasAttr 표현식: N00e, `x ? a` → x에 속성 `a`가 있는지 확인
  /// x에 속성 a가 있으면 true, 없으면 false 반환
  HasAttr {
    /// 확인할 속성 집합 (속성 존재 여부를 확인할 대상)
    base: Arc<PnixExpr>,
    /// 확인할 속성 이름 (경로 가능, 예: "a.b.c")
    attr: String,
  },
  /// 동적 HasAttr 표현식: N00p, `x ? ${expr}` or `x ? "string"`
  /// 속성 이름이 런타임에 계산됨
  DynamicHasAttr {
    /// 확인할 속성 집합 (속성 존재 여부를 확인할 대상)
    base: Arc<PnixExpr>,
    /// 속성 이름으로 평가되는 표현식 (속성 이름을 계산하는 표현식)
    attr_expr: Arc<PnixExpr>,
  },
  /// 동적 속성 접근: N00f, `attrset.${expr}` → getAttr(expr, attrset)
  /// 속성 이름이 런타임에 expr로부터 계산됨
  DynamicSelect {
    /// 접근할 속성 집합 (속성을 가져올 대상)
    base: Arc<PnixExpr>,
    /// 속성 이름으로 평가되는 표현식 (속성 이름을 계산하는 표현식)
    attr_expr: Arc<PnixExpr>,
  },
  /// 동적 속성 접근 with 기본값: N00m, `attrset.${expr} or default`
  /// 속성이 존재하지 않으면 default를 평가함
  DynamicSelectOrDefault {
    /// 접근할 속성 집합 (속성을 가져올 대상)
    base: Arc<PnixExpr>,
    /// 속성 이름으로 평가되는 표현식 (속성 이름을 계산하는 표현식)
    attr_expr: Arc<PnixExpr>,
    /// 기본값 표현식 (속성이 없을 때 사용할 기본값)
    default: Arc<PnixExpr>,
  },
}

/// PNIX 파라미터 패턴: 함수 파라미터 패턴 타입
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PnixParamPattern {
  /// 식별자 패턴: `x` (단순 변수 패턴)
  Ident(
    /// 변수 이름
    String,
  ),
  /// AttrSet 패턴: `{ x, y }` or `{ x, y, ... }`
  /// ellipsis가 true이면 추가 필드 허용
  AttrSet {
    /// 필드 목록 (패턴 필드 목록)
    fields: Vec<PnixPatternField>,
    /// 나머지 필드 허용 여부 (ellipsis 여부)
    ellipsis: bool,
  },
  /// 리스트 패턴: `[x, y, z]`
  List(
    /// 리스트 패턴 (리스트 패턴 구조)
    PnixListPattern,
  ),
  /// AttrSet 패턴 with 바인딩: `args@{x, y}` or `{x, y}@args`
  /// 전체 attrset를 `bind_name`에 바인딩하고 구조 분해함
  AttrSetWithBind {
    /// 바인딩 이름 (전체 attrset를 바인딩할 이름)
    bind_name: String,
    /// 필드 목록 (패턴 필드 목록)
    fields: Vec<PnixPatternField>,
    /// 나머지 필드 허용 여부 (ellipsis 여부)
    ellipsis: bool,
  },
}

/// PNIX 패턴 필드: 속성 집합 패턴의 필드 정보
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PnixPatternField {
  /// 필드 이름 (패턴 필드의 이름)
  pub name: String,
  /// 기본값 (선택적, 필드의 기본값 표현식)
  pub default: Option<PnixExpr>,
}

/// PNIX 리스트 패턴: 리스트 패턴의 구조 정보
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PnixListPattern {
  /// 항목 목록 (리스트 패턴의 항목 이름 목록)
  pub items: Vec<String>,
  /// 나머지 항목 바인딩 (선택적, 나머지 항목을 바인딩할 이름)
  pub tail: Option<String>,
}

/// Let 바인딩: N00b, 패턴 = 값 바인딩 또는 inherit 문
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PnixLetBinding {
  /// 일반 바인딩: pattern = value;
  Binding {
    /// 패턴 (바인딩 패턴)
    pattern: PnixParamPattern,
    /// 값 표현식 (바인딩할 값)
    value: Arc<PnixExpr>,
  },
  /// Inherit 문: inherit vars; 또는 inherit (scope) vars;
  Inherit {
    /// 스코프 표현식 (옵셔널): inherit (scope) vars;
    from: Option<Arc<PnixExpr>>,
    /// 상속할 변수 이름 목록 (상속할 변수 이름들)
    names: Vec<String>,
  },
}

/// 속성 키 경로 세그먼트: 속성 키 경로의 한 세그먼트
/// 정적 이름과 동적 표현식 모두 지원: { a.${b}.c = value; }
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AttrKeySegment {
  /// 정적 속성 이름: a, "quoted-name"
  Static(
    /// 속성 이름 (정적 속성 이름)
    String,
  ),
  /// 동적 속성 이름: ${expr}
  Dynamic(
    /// 속성 이름으로 평가되는 표현식
    Arc<PnixExpr>,
  ),
}

/// PNIX 속성 항목: 속성 집합의 항목 타입
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PnixAttrItem {
  /// 정적 속성 할당: { a = 1; } 또는 { a.b.c = 1; }
  /// key_path는 각 경로 세그먼트를 포함. "a.b" 같은 따옴표 키는 단일 세그먼트가 됨.
  /// - `a.b.c = 1;` → `key_path: vec!["a", "b", "c"]`
  /// - `"a.b" = 1;` → `key_path: vec!["a.b"]` (단일 세그먼트, 중첩 없음)
  Assign {
    /// 키 경로 (각 세그먼트, 속성 키 경로의 각 세그먼트)
    key_path: Vec<String>,
    /// 값 표현식 (할당할 값)
    value: Arc<PnixExpr>,
    /// 소스 위치 (소스 코드 위치)
    span: Span,
  },
  /// 동적 속성 할당: { ${expr} = value; } 또는 { a.${b}.c = value; }
  /// 키 경로는 정적 및 동적 세그먼트를 모두 포함할 수 있음
  /// N00k: Dynamic attribute assignment: { ${expr} = value; } or { a.${b}.c = value; }
  DynamicAssign {
    /// 속성 키 경로 (하나 이상의 세그먼트, 최소 하나는 동적이어야 함)
    key_path: Vec<AttrKeySegment>,
    /// 값 표현식 (할당할 값)
    value: Arc<PnixExpr>,
    /// 소스 위치 (소스 코드 위치)
    span: Span,
  },
  /// Inherit 문: inherit vars; 또는 inherit (scope) vars;
  Inherit {
    /// 스코프 표현식 (옵셔널, 상속할 스코프 표현식)
    from: Option<Arc<PnixExpr>>,
    /// 상속할 변수 이름 목록 (상속할 변수 이름들)
    names: Vec<String>,
    /// 소스 위치 (소스 코드 위치)
    span: Span,
  },
}

/// Match arm: match 표현식의 한 가지 경우 (패턴 => 표현식 또는 패턴 if 가드 => 표현식)
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PnixMatchArm {
  /// 패턴 (매칭할 패턴)
  pub pattern: PnixPattern,
  /// 가드 조건 (옵셔널, 예: `if n > 0`)
  /// Y08c: Optional guard condition (e.g., `if n > 0`)
  pub guard: Option<Arc<PnixExpr>>,
  /// 본문 표현식 (매칭 성공 시 평가할 표현식). `Arc<PnixExpr>` 로
  /// wrap (2026-05-16 memory-opt): inline `PnixExpr` 는 64 bytes,
  /// `Arc<PnixExpr>` 는 8 bytes. 매 match arm 당 56 bytes 절감.
  pub body: Arc<PnixExpr>,
}

/// Match 표현식 패턴: match 표현식에서 사용하는 패턴 타입
// LOW: Match에서 As 패턴(@) 미지원 수정 완료
// As 패턴은 lambda 파라미터에서만 지원되며, match 표현식에서는 미지원
// 이는 의도된 설계: match 표현식의 패턴 매칭은 구조 분해에 집중하며, As 패턴은 lambda에서만 필요
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PnixPattern {
  /// Wildcard 패턴: _ (모든 것과 매칭, 바인딩 없음)
  Wildcard,
  /// 변수 패턴: x (모든 것과 매칭, x에 바인딩)
  Var(
    /// 변수 이름
    String,
  ),
  /// 리터럴 패턴: 42, true, "hello"
  Literal(
    /// 리터럴 패턴 (리터럴 값 패턴)
    PnixLiteralPattern,
  ),
  /// Attrset 패턴: { a, b, ... }
  AttrSet {
    /// 필드 목록 (매칭할 필드 목록)
    fields: Vec<PnixMatchAttrField>,
    /// 나머지 필드 허용 여부 (ellipsis 여부)
    ellipsis: bool,
  },
  /// 리스트 패턴: [a, b, ...rest]
  List(
    /// 리스트 패턴 (리스트 패턴 구조)
    PnixMatchListPattern,
  ),
  /// 생성자 패턴: Some(x), None, Ok(a), Err(e)
  Constructor {
    /// Variant 이름 (생성자 variant 이름)
    variant: String,
    /// 인자 패턴 목록 (생성자 인자 패턴 목록)
    args: Vec<PnixPattern>,
  },
}

/// Match 속성 집합 패턴 필드: match 표현식의 속성 집합 패턴 필드 정보
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PnixMatchAttrField {
  /// 필드 이름 (매칭할 필드 이름)
  pub name: String,
  /// 중첩 패턴 (선택적, 필드 값의 중첩 패턴)
  pub pattern: Option<Arc<PnixPattern>>,
}

/// Match 리스트 패턴: match 표현식의 리스트 패턴 구조 (중첩 패턴 지원)
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PnixMatchListPattern {
  /// 항목 패턴 목록 (리스트 항목 패턴 목록)
  pub items: Vec<PnixPattern>,
  /// 나머지 항목 바인딩 (선택적, 나머지 항목을 바인딩할 이름)
  pub tail: Option<String>,
}

/// 리터럴 패턴: 패턴에서 사용할 수 있는 리터럴 값 타입
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PnixLiteralPattern {
  /// 정수 리터럴 (정수 값)
  Int(
    /// 정수 값
    i64,
  ),
  /// 부동소수점 리터럴 (실수 값)
  Float(
    /// 실수 값
    f64,
  ),
  /// 불리언 리터럴 (불리언 값)
  Bool(
    /// 불리언 값
    bool,
  ),
  /// 문자열 리터럴 (문자열 값)
  String(
    /// 문자열 값
    String,
  ),
  /// null 리터럴 (null 값)
  Null,
}

// Backward-compatible aliases (keep older code readable, but prefer `Pnix*` names).
/// 표현식 타입 별칭
pub type Expr = PnixExpr;
/// 파라미터 패턴 타입 별칭
pub type ParamPattern = PnixParamPattern;
/// 패턴 필드 타입 별칭
pub type PatternField = PnixPatternField;
/// 리스트 패턴 타입 별칭
pub type ListPattern = PnixListPattern;
/// Let 바인딩 타입 별칭
pub type LetBinding = PnixLetBinding;
/// 속성 항목 타입 별칭
pub type AttrItem = PnixAttrItem;
/// Match arm 타입 별칭
pub type MatchArm = PnixMatchArm;
/// 패턴 타입 별칭
pub type Pattern = PnixPattern;
/// 리터럴 패턴 타입 별칭
pub type LiteralPattern = PnixLiteralPattern;
/// Match 속성 필드 타입 별칭
pub type MatchAttrField = PnixMatchAttrField;
/// Match 리스트 패턴 타입 별칭
pub type MatchListPattern = PnixMatchListPattern;
