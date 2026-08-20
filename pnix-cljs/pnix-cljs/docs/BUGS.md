# pnix-cljs BUGS

목적: 알려진 버그·한계, 그리고 **의도적으로 안 고치는** 항목을 적는다.
의도적으로 안 고치는 항목은 아래처럼 "이건 버그 아니라 의도된 제한"이라고
명시해서, 나중에 누가 실수로 "고치려고" 손대지 않게 한다. (구
`SCOPE_LOCK.md`의 "이 seed에서 제외" 목록이 정확히 이런 종류의 내용이라
2026-08-20에 여기로 옮겨왔다.)

## 의도된 제한 (버그 아님)

`pnix-cljs`는 ClojureScript/JavaScript 프로젝션 메커니즘만 소유한다(제품
소유 범위 정의는 [`IMPLEMENTATION.md`](IMPLEMENTATION.md) §2 참고). 아래
항목들은 "아직 구현 안 됨"이 아니라 **애초에 이 seed의 스코프 밖으로
못박아둔 것**이다 — 이건 버그 아니라 의도된 제한:

- service policy 및 admission status
- evaluator fallback
- proof-receipt-gated execution
- JVM/Java/ASM 구현 코드
- retained effects 및 filesystem execution
- automatic application code generation
- authoritative string-encoded types
- 복사된 `stdlib`, `pnixc-pnix`, `pnix-mirror-runtime`, 또는 domain-content roots

같은 맥락에서: 이 저장소에는 이식 가능한 언어 의미를 소유하는 별도
저장소-수준 트리가 없다 — 이 호스트는 복사된 Clojure/JVM 런타임 트리를
유지하지 않고 자체 네이티브 seed로 파싱/평가한다. 네이티브 seed는 공유
적합 코퍼스가 연결되고 all-host gate로 비교되기 전까지 정규 크로스호스트
패리티를 주장할 수 없다 — 이것도 버그가 아니라 지금 단계에서 그렇게
정해둔 상태다.

## 알려진 한계 (구조적, 버그로 취급하지 말 것)

`import`/`scopedImport`가 파서 단계에서 리터럴 경로 토큰만 받고 동적으로
계산된 경로 식은 못 받음(`import (if c then ./a.px else ./b.px)` 불가) —
이런 구조적 차이는 [`IMPLEMENTATION.md`](IMPLEMENTATION.md) §5 "다른
호스트와 알려진 차이점"에 상세히 정리돼 있다(여기서 중복하지 않음).
(2026-08-20 갱신: "경로가 일반 값이 아님" 항목은 해소됐다 — `PathValue`
도입 경위는 [`IMPLEMENTATION.md`](IMPLEMENTATION.md) §9, 그 작업이 남긴
시뮬레이션 한계는 아래 새 절 참고.)

`builtins.unsafeGetAttrPos`는 2026-08-20에 구현됐다. 리터럴 attrset
바인딩은 hy/Nix와 같은 `{ file; line; column }`을 돌려주고, 생성된
attrset(`listToAttrs` 등)과 없는 속성은 `null`이다. 인라인 평가의
`file`은 `"<pnix-px>"`. 파서가 아직 line/column을 토큰에 싣지 않던
시절의 죽은 등록이 아니다.

## 문자열 컨텍스트(string context) / `derivation` 시뮬레이션 한계 (2026-08-20, 버그 아님)

[`IMPLEMENTATION.md`](IMPLEMENTATION.md) §8에서 이식한 문자열 컨텍스트
추적 + `derivation`/`derivationStrict`는 **순수 시뮬레이션**이다 — 실제
Nix 빌드 시스템과 바이트 단위로 호환되게 만들려는 시도가 아니다. 참고로
삼은 다른 pnix 호스트(문자열 컨텍스트를 이미 갖춘 유일한 호스트)도 같은
스코프의 시뮬레이션이고, 이 호스트는 그 설계를 그대로 이식했다 — 아래
항목은 전부 의도된 한계지 고쳐야 할 버그가 아니다:

- **의사(pseudo) 해시, 진짜 Nix 해시 아님.** `derivation-paths`가 만드는
  `/nix/store/<hash>-<name>` 경로는 forced 입력 attrs의 canonical JSON
  텍스트를 sha256한 것 — 실제 `nix-instantiate`가 만드는 `.drv` 파일
  구조를 해시하는 진짜 Nix 알고리즘과 다르다. 같은 pnix 프로그램은
  항상 같은 경로를 내지만(결정적), 그 경로 문자열 자체는 실제 Nix가 낼
  경로와 절대 같지 않다.
- **`d.out == d` 자기참조가 표현 안 됨.** 실제 Nix에서 `derivation {...}`가
  낸 attrset `d`는 `d.out`이 `d` 자기 자신(첫 출력이 "out"일 때)이지만,
  이 호스트(그리고 참고로 삼은 호스트)의 값 모델은 순수 레코드/맵이라
  진짜 순환 참조를 만들 수 없다 — `d.out`은 `d`와 같은 타입/모양이지만
  별개의, 축소된(`type`/`name`/`drvPath`/`outPath`/`outputName`만 있는)
  하위-attrset이다. `d.out.out`처럼 더 파고들면 없는 속성이라 에러난다
  (실제 Nix라면 계속 자기 자신을 가리킴).
- **`appendContext`/`getContext`가 얕은 스코프.** 어떤 store-path에
  의존하는지, 그리고 `path`/`allOutputs`/`outputs` 세 가지 컨텍스트
  "종류" 구분까지는 추적하지만, 실제 Nix가 갖는 더 풍부한 세부 정보(예:
  `outputs` 종류가 가리키는 각 출력이 실제로 존재하는 유효한 파생물
  출력인지 검증하는 것 등)는 검증하지 않는다 — 문자열만 넣으면 그대로
  인코딩/디코딩된다.
- **`ctx-string-in-args?` fail-closed 게이트는 얕은 스캔이다.**
  최상위 인자 + 벡터 인자 한 단계까지만 보고, 아직 강제평가 안 된 `Cell`
  뒤에 숨은 리스트 원소는 못 본다. 오라클로 직접 확인한 실제 동작:
  컨텍스트 있는 문자열이 `sort`/`filter` 같은 non-allowlisted 빌트인의
  리스트 인자 **안에 중첩**돼 있으면 거부되지 않고 그냥 통과한다(참고
  구현도 동일) — 게이트가 신뢰성 있게 잡는 건 바로 그 자리에 스칼라로
  전달된 컨텍스트 문자열뿐이다. 오라클과 동일한 동작이라 이 호스트만의
  결함이 아니고, 일부러 더 엄격하게 만들지 않았다(오라클과 정확히 같은
  강도로 fail-closed하도록).
- **`toJSON`이 attrset을 `__toString`/`outPath`로 강제 변환하지 않는다
  (이번 작업 이전부터 있던, 무관한 사전 존재 격차).** 실제 Nix와 참고
  구현은 `toJSON { outPath = "/x"; ... }`를 `"/x"`(문자열)로 직렬화하지만,
  이 호스트의 `to-json-value`는 attrset을 항상 순수 JSON 객체로
  직렬화한다 — `derivation` 결과처럼 `outPath`가 있는 attrset을 통째로
  `toJSON`에 넘기면 실제 Nix와 다른 모양이 나온다. 이번 작업은
  `ContextStringValue` 인식만 `to-json-value`에 추가했고, 이 attrset
  강제변환 격차는 문자열 컨텍스트와 무관한 기존 동작이라 손대지 않았다
  — 고치려면 `to-json-value`의 `AttrsetValue` 분기에 `__toString`/
  `outPath` 우선순위 코어스를 추가해야 한다.

## Path 값 타입 시뮬레이션 한계 (2026-08-20, 버그 아님)

[`IMPLEMENTATION.md`](IMPLEMENTATION.md) §9에서 신설한 `PathValue`도
**순수 시뮬레이션**이다 — 아래는 전부 의도된 한계지 고쳐야 할 버그가
아니다:

- **경로 리터럴이 현재 파일 디렉터리로 절대화되지 않는다.** 실제 Nix는
  상대 경로 리터럴을 그 파일의 디렉터리 기준 절대 경로로 만들지만, 이
  호스트는 리터럴 텍스트를 정규화만 하고 그대로 유지한다(`+` 연결도
  cur-dir을 전혀 모른 채 텍스트만 이어붙임) — 다른 두 호스트도 같은
  선택을 하는 걸 오라클로 교차검증하고 따른, 세 호스트가 공유하는 의도된
  설계다.
- **`${...}` 문자열 보간의 의사 store path가 문자열 컨텍스트에 안
  얹힌다.** 경로를 `"${p}"`로 보간하면 `/nix/store/<hash>-<basename>`
  형태의 의사 store path 텍스트로 치환되지만(진짜 store 접근 없음), 그
  결과 문자열의 컨텍스트에는 아무 것도 추가되지 않는다 — 참고로 삼은
  최신 구현을 그대로 포팅한 결과다. 문자열 컨텍스트 기능 전체의 존재
  이유가 "이 문자열이 어떤 store-path에 의존하는가" 추적인데, 이 경로만
  추적에서 빠지는 셈이라 잠재적 gap으로 보인다 — 일부러 고치지 않고
  참고 구현 그대로 재현했다(§9 "판단 콜" 참고). 나중에 다시 볼 가치가
  있는 지점.
- **경로 리터럴 안에서 `${...}` 동적 보간을 못 쓴다** (`./a/${x}` 같은
  구문). 렉서가 경로 문자로 `{`/`}`를 애초에 인정하지 않아서, 이 구문은
  경로 토큰 파싱이 중간에 끊기고 별개의 집합 리터럴처럼 잘못 토큰화된다
  — 오라클 교차검증 결과 참고 호스트들도 이 구문에서 에러가 나는 걸
  확인했으므로 이 호스트만의 새 제한이 아니다. 이번 작업 스코프 밖(동적
  경로 보간은 렉서 차원의 별개 기능).
- **`toPath`가 `PathValue` 인자를 거부한다** (`builtins.toPath ./x`는
  type-error). 실제 Nix의 `builtins.toPath : string -> path` 시그니처를
  그대로 따른 선택이고, 참고 호스트 중 하나(문자열 컨텍스트 작업의 원
  참고 구현)도 같은 인자를 주면 에러가 나는 걸 오라클로 확인했다 — 다른
  참고 구현은 Path 인자를 그대로 통과시키는 더 관대한 선택을 했지만, 이
  호스트는 실제 Nix 시그니처 + 그 오라클 둘 다에 맞춰 거부 쪽을 택했다.
