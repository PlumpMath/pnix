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

- **경로(path) 값 타입이 없다.** `builtins.typeOf ./x`가 `"path"`가 아니라
  `"string"`을 돌려주고 `builtins.isPath`는 `false`. 상세 원인·비교는
  `IMPLEMENTATION.md` §3 참고. typeOf/isPath/JSON 투영/canonical print/
  동등성을 관통하는 새 값 타입 추가라 범위가 크다 — **mirror/gate 수요가
  생기면 그때 추가**한다는 게 SCOPE_LOCK의 결정이었고 아직 유효하다.
- **string-context 값이 없다.** context-carrying string(파생물 추적용)이
  아예 표현 불가 — `unsafeDiscardStringContext` 같은 빌트인은 이름만
  있고 진짜 context가 없으니 사실상 no-op. proposal 0010이 `hashString`
  등에서 이 갭을 명시: "clj/hy는 hash data context가 버려지고 algorithm
  context는 거부됨을 검증하지만, rs는 애초에 그 값을 표현 못 해서 검증
  대상이 없다."
- **URI 리터럴이 없다.**
- **중첩 동적 attr 경로가 없다** — `a.${x}.b = 1;`처럼 attrpath 중간에
  동적 세그먼트가 오는 형태. **단일** 동적 키(`{ "${x}" = 1; }`, c10
  corpus)는 2026-07-02에 이미 개방됐음 — 헷갈리지 말 것, 이건 그것보다
  더 일반적인 형태의 이야기다.
- **POSIX ERE 전체 정합이 없다** — `match`/`split` 등 정규식 빌트인은
  있지만 POSIX ERE 스펙 전체를 따르지는 않는다.
- **JSON float exponent canonicalization이 없다** — `toJSON`이 지수
  표기(`1e10` 등)를 Nix와 동일하게 정규화하지 않는다. proposal 0010이
  명시: "exponent spelling/shortest-roundtrip, direct non-finite encoding,
  common error classes, Nix-version policy remain B1 work."
- **비유한 float(inf/NaN)의 canonical print가 유효한 px 소스가 아니다** —
  "print한 값을 다시 파싱하면 같은 값"이라는 이 프로젝트의 P1 성질에서
  유일한 예외(pnix-hy의 repr도 동일 예외를 가짐). `toJSON`/rust-mirror는
  비유한 값에서 명시 에러 또는 held(2026-07-03 감사 #2 기록). 관찰·비교는
  가능하지만 roundtrip되는 canonical print는 없다.

## 2. 의도적으로 held된 수학 확장 빌트인 (이건 버그 아니라 의도된 제한)

`sin cos tan sqrt exp ln log abs pow mod`(그리고 `atan2`도 같은 부류) —
`docs/CAPABILITIES.md`/§2 빌트인 표에서는 O(등록됨)로 보이지만 **호출하면
에러 나는 게 정상**이다. 숫자 모델(B1) 전체를 어떻게 할지 아직 결정이 안
나서 의도적으로 묶어놓은 것 — `functionArgs`/`abs`처럼 개별적으로
un-hold된 것들과 이 부류는 성격이 다르다. 상세는
`IMPLEMENTATION.md` §3 참고.

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

즉 위 §1의 path/string-context/JSON float 갭들과 겹치는 부분이 이
proposal의 다음 단계다 — 로드맵으로서의 우선순위/설계 방향은
[`docs/PLANS.md`](PLANS.md)를 볼 것(이건 "결정된 제한"이 아니라 "아직
설계 안 된 다음 단계"라 PLANS.md 쪽 성격에 더 가깝다).
