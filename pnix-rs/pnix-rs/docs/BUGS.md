# pnix-rs — 알려진 버그 / 제한 / 의도적으로 안 고치는 것

목적: "왜 이거 안 되지"를 마주쳤을 때 진짜 버그인지 원래 그런 건지 빨리
판단하는 문서. **의도적으로 안 고치는 항목은 전부 "이건 버그 아니라
의도된 제한"이라고 명시했다** — 나중에 누가 실수로 "고치려" 하지 않도록.
2026-08-20 작성(옛 `SCOPE_LOCK.md`의 held 목록 + 옛 `todo.md`의 held/보류
항목을 흡수). 아키텍처/빌트인 지원 현황 자체는
[`docs/IMPLEMENTATION.md`](IMPLEMENTATION.md)를 볼 것 — 여기는 그 문서의
"다른 호스트와 알려진 차이점"(§3)과 겹치지 않는 항목만 다룬다.

## 1. 값 타입 — 의도적으로 없는 것들 (이건 버그 아니라 의도된 제한)

`PxVal`에 아예 variant가 없어서 못 하는 것들. 전부 SCOPE_LOCK §2의 "명시
미주장" 경계였고, 지금도 유효하다(2026-08-19 §5 정직 경계 재확인 — float/
toJSON/문자열 보간/`string+`/bool/`?`/`rec`/`with`는 이미 구현됐지만
아래는 여전히 진짜 갭).

- ~~경로(path) 값 타입이 없다~~ — **정정(2026-08-20): 더 이상 사실이
  아니다.** `PxVal::Path(String)`이 생겼다 — `typeOf`/`isPath`/`dirOf`/
  `baseNameOf`/`toPath`/`==`/`<`/`+`/`toJSON`/`toXML`/`${...}` 보간/
  rust-mirror/specialize 전부 갱신됨. 상세는 `IMPLEMENTATION.md` §1
  "경로(Path) 값" 절. 알아둘 만한 설계 선택: cur_dir(파일 위치)로
  절대화하지 않는다 — pnix-clj 오라클도 리터럴 텍스트만
  정규화하고 절대화는 안 하는 걸 확인하고 따름(실제 Nix는 절대화하는데,
  이건 `nix-instantiate`로 교차검증해서 확인한 의도적 divergence).
- ~~string-context 값이 없다~~ — **정정(2026-08-20): 더 이상 사실이
  아니다.** pnix-clj(오라클)/pnix-clr의 태그된-맵 설계를 그대로 이식해
  구현했다 — `PxVal`에 새 variant를 추가하지 않고, `PxVal::Attrs`를
  `__pnix_value_kind = "string-context"` 센티널 키로 태그하는 방식
  (`PxVal::Bytes`가 이미 쓰던 "특수 값 모양, 지정된 표면만 이해, 나머지는
  fail-closed"라는 선례를 그대로 따름). `hasContext`/`getContext`/
  `appendContext`/`unsafeDiscardStringContext`/`unsafeDiscardOutputDependency`
  + `builtins.derivation`/`derivationStrict`/`placeholder`가 전부 실제로
  동작한다. `+`/`${...}` 보간/동등성/순서 비교는 항상 context-aware(언어
  연산자라 게이트 대상이 아님); 그 외 빌트인은 고정 allowlist
  (`px_context_aware_builtin`, pnix-clj의 `context-aware-builtins`를 이름
  그대로 이식)에 없으면 top-level contextful 인자에 fail-closed
  (`px_builtin_exec`의 단일 chokepoint에서 얕은 스캔 — 강제되지 않은 list
  원소 안에 숨은 context는 오라클과 동일하게 통과함, 더 엄격하게 만들지
  않음). pure-simulation 알려진 한계(모두 pnix-clj/pnix-cljs와 동일한
  선상의 문서화된 제한이지 이 호스트만의 결함이 아님):
  - 의사(pseudo) 해시 — 실제 Nix store 해시와 바이트 호환 아님, 다른 pnix
    호스트의 해시와도 호환 아님(각 호스트가 독립적으로 시뮬레이션).
  - `derivation`이 만드는 `d.<output>`는 축약된(비순환) 파생물 attrset —
    실제 Nix의 `d.out == d` self-reference는 이 plain-`Attrs` 값 모델로는
    표현 불가.
  - `appendContext`는 pnix-clj가 정의한 단순화된 스코프(path/allOutputs/
    outputs 세 종류의 정보만) 그대로.
  - fail-closed 게이트는 얕다(shallow) — 의도적으로, 오라클 행동과
    일치시키기 위해서다.
  - **`.` select가 pnix-clj 오라클과 의도적으로 다르다**: 오라클은
    `eval-select`가 `attrset-value?`가 아니라 맨 `map?`을 써서 ctx-string
    맵을 실제 attrset처럼 취급해 `a.string`이 raw 표현("hello")을 그대로
    새어 나오게 한다(`?`/`//`는 같은 오라클에서도 올바르게 막혀 있음 —
    `eval-select` 한 함수만의 우연한 누락으로 보임). pnix-cljs 포트는
    애초에 별개 레코드 타입을 써서 이 누락을 재현하지 않았고, pnix-rs도
    그 판단을 따라 `.` select에서 ctx-string을 명시적으로 거부한다(실제
    Nix도 문자열에 `.`를 쓰면 타입 에러) — 유일하게 의도적으로 오라클과
    다른 지점.
- ~~URI 리터럴이 없다~~ — **정정(2026-08-20): 애초에 사실이 아니었다.**
  손대 보니 `px_uri_scheme_char`/`px_uri_body_char`/`px_uri_end`/
  `PxTok::Uri`가 이미 완전히 구현돼 있었고(실제 Nix 렉서 규칙과 정확히
  일치), substrate-check(`phase3_uri_literals`)/px-check 양쪽 다 이미
  통과 중이었다 — 이 항목은 낡은 기록이었을 뿐, 코드 변경 없음.
- ~~중첩 동적 attr 경로가 없다~~ — **정정(2026-08-20): 더 이상 사실이
  아니다.** `a.${x}.c = 1;`처럼 attrpath 중간(또는 첫 세그먼트 포함 어디든)에
  동적 세그먼트가 오는 형태가 이제 파싱된다(D21, pnix-clj의
  `parse-attr-path`/`path->nested` 이식). `parse_attrset_literal`의
  `parse_attr_path_segment`가 모든 세그먼트(첫 세그먼트뿐 아니라
  `.` 뒤 각 세그먼트)에서 정적 ident/키워드-이름과 동적
  문자열/`${...}`를 동일하게 허용하고, 정적 세그먼트는 기존처럼 순수
  `PxExpr::Attrs` 중첩으로, 동적 세그먼트는 **단일** 동적 키(`{ "${x}" = 1;
  }`, c10 corpus, 2026-07-02에 이미 개방됨)가 쓰던 것과 동일한
  `builtins.listToAttrs` 데슈가 경로(`px_wrap_dynamic_attr`)를 그 위치에서
  재사용해 감싼다. 가장 까다로웠던 상호작용은 정적-첫-세그먼트 형제가
  동적-중첩-세그먼트 값과 병합하는 경우(`{ a.b = 1; a.${x}.c = 2; }` — `a`가
  둘 다의 공통 정적 첫 세그먼트)였는데, `merge_attr_field`에 폴백 분기를
  추가해 해결했다: 기존 리터럴<->리터럴 재귀 병합 조건(둘 다 `PxExpr::Attrs`)이
  실패하면, 한쪽(혹은 양쪽)이 `px_dynamic_pairs`로 인식되는
  `builtins.listToAttrs [...]` 모양인지 확인하고, 맞다면 두 pair 리스트를
  이어붙인 새 `listToAttrs` 호출로 병합한다(진짜 병합 불가능한 값은 여전히
  파스-타임 "duplicate attrset key" 에러). 이어붙인 리스트의 실제 이름
  충돌(예: 동적 세그먼트가 런타임에 정적 형제와 같은 이름으로 평가됨)은
  §3의 기존 "동적 attrset 키 중복 = first-wins" divergence를 그대로
  따른다(오라클은 이 경우 eval-error, pnix-rs는 first-wins) — 새 divergence가
  아니라 기존 divergence가 새 경로(중첩 세그먼트)에서도 똑같이 적용되는
  것뿐이다. pnix-clj 오라클과 교차검증 완료(`let x = "b"; in { a.${x}.c = 1;
  }.a.b.c` => 1, `{ a.b = 1; a.${x} = 2; }.a` => `{b=1;c=2;}` 등). 부수적으로
  동적 **첫** 세그먼트 뒤의 점 이어짐도 이제 열렸다(`${x}.c = 1;`, 이전엔
  단일 동적 키만 가능했음) — pnix-clj의 `parse-attr-path`가 첫/후속 세그먼트를
  구분하지 않는 것과 동일하게 자연히 따라온 결과. `let` 바인딩
  (`parse_let`)도 같은 동적 세그먼트 경로를 쓴다 (`let a.${x}.c = 1; in
  ...`). 동적 첫 세그먼트는 Nix와 같이 "dynamic attributes not allowed
  in let"이다.
- **POSIX ERE 완전한 leftmost-longest 정합은 없다(2026-08-20 대부분 축소됨)**
  — `*`/`+`가 그룹(괄호 하위표현식)에도 적용되고 구간 반복
  `{m}`/`{m,}`/`{m,n}`도 이제 있다(`rx_repeat_group_try`/
  `try_parse_interval`, `src/px.rs`). 남은 진짜 갭 2개, 둘 다 backtracking
  엔진이라 구조적으로 안고 가는 것(POSIX-longest DFA로 다시 쓰지 않는 한
  못 고침, `nix-instantiate`로 확인해보니 **실제 Nix 자체도 이 두 갭에서
  POSIX-longest가 아니라 이 엔진과 같은 backtracking-첫-성공-승 방식**이라
  일부는 갭이 아니라 오라클과 이미 일치): (1) 그룹 반복 횟수가 초기 그리디
  전진 후 백오프될 때, 그 그룹 안에 중첩된 캡처는 백오프된 실제 반복
  횟수가 아니라 그리디 전진이 남긴 마지막 값을 그대로 유지한다(드문
  케이스). (2) 반복자 아래 중첩된 alternation(`(a|ab)*` 류)의 분기 선택은
  진짜 POSIX 최장 매치가 아니라 backtracking 순서를 따른다 — 단
  `nix-instantiate`로 직접 확인한 결과 실제 Nix도 이 정확한 동작을 보인다
  (`builtins.match "(a|ab)(c|bcd)(d*)" "abcd"`가 두 엔진 다
  `[ "a" "bcd" "" ]`).
- ~~JSON float exponent canonicalization이 없다~~ — **정정(2026-08-20):
  둘 다 이미 맞았거나 사소한 메시지 문제였다.** 비유한 float(NaN/inf)은
  이미 `toJSON`에서 에러였다(다만 메시지가 뭉뚱그려져 있어서
  `px_json_float_text`로 NaN/+inf/-inf를 구분하는 메시지로 교체). 유한
  float의 지수 표기(Rust `{:?}`)는 실측해보니 이미 유효한 JSON 숫자
  문법이었다(지수에 `+` 안 붙임; JSON 문법상 지수 앞에 소수부가 필수가
  아니라 `1e300` 자체가 이미 유효한 JSON 숫자라 별도 정규화가 필요
  없었음) — 코드 변경 불필요, proposal 0010의 우려는 실제로는 기우였다.
- **비유한 float(inf/NaN)의 canonical print가 유효한 px 소스가 아니다** —
  "print한 값을 다시 파싱하면 같은 값"이라는 이 프로젝트의 P1 성질에서
  유일한 예외(pnix-hy의 repr도 동일 예외를 가짐). `toJSON`/rust-mirror는
  비유한 값에서 명시 에러 또는 held(2026-07-03 감사 #2 기록). 관찰·비교는
  가능하지만 roundtrip되는 canonical print는 없다.

## 2. ~~의도적으로 held된 수학 확장 빌트인~~ — 정정(2026-08-20)

~~`sin cos tan sqrt exp ln log abs pow mod` — 호출하면 에러 나는 게
정상이다. 숫자 모델(B1) 전체를 어떻게 할지 아직 결정이 안 나서 의도적으로
묶어놓은 것.~~ **더 이상 사실이 아니다.** 이 10개는 다른 4개 호스트
(pnix-clj/pnix-clr/pnix-cljs/pnix-hy)가 전부 이미 동작하는 구현을 갖고
있던 4/5 합의 사례였고, "B1 숫자 모델" 우려는 실제로는 언어 전체의
int/float 승격 정책에 관한 것이었지 이런 단순 단항/이항 float 수학 함수
구현을 막는 이유가 아니었다 — hold를 풀고 실구현했다(순수 산술: Newton's
method/Taylor 급수. rs-meta의 인터프리트 Rust 부분집합은 f64 메서드
디스패치가 아예 없어서 — `substrate-check`가 이 파일 전체를 rs-meta
bootstrap으로 해석하는데 `interp.rs`의 `call_method`는 i64 계열만
숫자 타깃으로 인식한다 — `.sin()`/`.sqrt()`/`.exp()`/`.ln()`/`.powf()` 같은
표준 라이브러리 호출을 쓸 수 없다; 이 파일이 이미 같은 이유로 쓰던 관례
(`px_bit_op`의 bit-by-bit AND/OR/XOR, `px_round_to_int`의 cast-and-adjust
ceil/floor)를 그대로 따라 손으로 짰다). `atan2`(pnix-hy 오라클 고정)와
`mapAttrs'`(pnix-clj 오라클 고정)도 신규 추가됐다. 상세는
`IMPLEMENTATION.md` §4 역사 표 참고.

## 3. Nix와의 의도적 동작 차이 (divergence, 이건 버그 아니라 의도된 제한)

- **동적 attrset 키 중복 = first-wins.** Nix는 이 경우 eval-error를
  낸다. rs는 first-wins로 조용히 받아준다(2026-07-02 m3b, divergence로
  기록). B4 convergence 후속 과제로 남아있고 아직 착수 안 됨 — 당장은
  의도적 차이로 취급할 것.
- ~~int↔float 승격 없음~~ — **정정(2026-08-20): 더 이상 사실이 아니다.**
  옛 SCOPE_LOCK OWNER AMENDMENT가 이걸 "documented value divergence"로
  적어뒀지만, proposal 0010 phase 2(2026-07-10)에서 혼합 int/float 산술·
  비교·중첩 동등성/순서가 `nix-instantiate 2.34.7`과 합치하도록 이미
  구현됐다. 옛 문서를 그대로 믿고 여기 다시 갭으로 적지 말 것.
- ~~`builtins.sort`가 비안정~~ — **정정(2026-08-19 재확인, todo.md §5에
  기록됐던 내용): 사실이 아니다.** selection sort지만 비교자가 strict
  less-than일 때만 `min`을 갱신하는 tie-break 규칙 때문에 실제로는
  **안정 정렬**이다(동일 키 3개를 인터리브해도 원래 상대 순서 유지,
  직접 재확인됨). "비안정"이라고 적혀 있던 옛 문서가 틀렸었다 — corpus가
  distinct라서 몰랐던 게 아니라 애초에 안정적이었다.

## 4. 구현 경계 (v0 boundary, 이건 버그 아니라 의도된 제한)

- **incremental(P9)의 SCC 그룹 해시 v0 경계.** 상호 재귀(순환) 바인딩은
  그룹 단위로 하나의 해시를 공유하는데, **그룹 내부 멤버 이름이 그룹
  해시 텍스트에 포함된다** — 즉 이름을 알파-리네임하면 그룹 전체 해시가
  바뀐다(단일 바인딩의 dependency-substituted 해시가 이름 무관인 것과
  다름). 모듈 doc(`src/incremental.rs`)에 이미 명시돼 있는 v0 경계.
- **OS 수준 샌드박싱은 범위 밖.** `interop.rs`의 capability
  admission(`granted: &[String]`)은 **lane 내부 admission 규율**일 뿐,
  프로세스 샌드박싱이나 파일시스템 격리 같은 OS 수준 보장은 하지 않는다.
  P5 interop 설계 당시부터의 명시 미주장.
- **인라인/`px_parse` 경로의 `unsafeGetAttrPos.file`은 `"<pnix-px>"`다.**
  파일 eval(`-f` / `px_run_value_with_modules`)은 `px_parse_in`이 모듈
  키에서 경로를 굽는다. `tower::reify` 등 합성 문자열은 파일 개념이
  없어서 인라인 라벨을 유지한다.
- **cross-host 파일-대-파일 자동 비교는 아직 미가동.** `pnix-rs
  export-oracles`로 `proof/oracles-rs.tsv`를 만들고 `cross-host-check`로
  자기 정합(drift/스키마/기대값)은 검증하지만, pnix-clj/pnix-hy 쪽에
  **같은 TSV export가 아직 없어서** 진짜 파일 대 파일 자동 비교는 held —
  Python/Hy 파싱으로 우회하지 않는다(불가촉 원칙, zero-dep 원칙 위반).
  **버그 아님, 그리고 폐기된 것도 아님** — 자매 lane에 TSV export가
  생기면 재개할 사전 합의된 작업(rs 쪽 TSV 스키마는 이미 이 lane이
  제안한 형태로 고정돼 있음). cross-host parity 비교 전반이 지금은
  개발 속도를 위해 미룬 것이지 포기한 게 아니다.

## 5. 아직 열려 있는 수렴 작업 (proposal 0010, "제한"이라기보다 "다음 단계")

proposal 0010의 phase 1-2(2026-07-10)가 Nix builtin 표면과의 raw
presence/behavior 정합을 상당히 좁혔지만, 스스로 이렇게 열어뒀다:

> Raw-surface, path/context, and canonical-float convergence remain open.

(원문 인용은 그대로 둔다 — proposal 0010 문서 자체는 손대지 않음. 실제
현황은 **path/context/canonical-float 세 갈래 다 2026-08-20 기준 해소**:
string-context는 같은 날짜 앞선 세션에서, path 값 타입/JSON float
메시지는 이 절 작업에서. raw-surface 수렴(Nix 118종 대비 남은 builtin
격차)만 여전히 열려 있음.) 로드맵으로서의 우선순위/설계 방향은
[`docs/PLANS.md`](PLANS.md)를 볼 것(이건 "결정된 제한"이 아니라 "아직
설계 안 된 다음 단계"라 PLANS.md 쪽 성격에 더 가깝다).
