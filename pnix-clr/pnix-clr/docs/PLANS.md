# pnix-clr 미래 아이디어 / 확정 안 된 방향

이 파일은 방향이 아직 확정되지 않은 미래 아이디어와 로드맵을 모아둔다.
지금 당장 누가 집어서 할 수 있는 확정된 작업은 [`TODO.md`](TODO.md),
알려진 버그/의도된 제외는 [`BUGS.md`](BUGS.md), 뭐가 built됐는지는
[`IMPLEMENTATION.md`](IMPLEMENTATION.md) 참고.

## 1. 미래 아이디어 — pnixMounts / unsafeGetAttrPos 통일 (아직 예정 없음, 2026-08-19)

(원본: `pnix-clr/pnix-clr/todo.md`. 2026-08-20 이 문서로 통합, 옛
`todo.md`는 삭제됨. 내용은 원본 그대로 옮겨왔다.)

지금은 만들지 않는다. 기본 언어 기능(5개 호스트가 실제로 똑같이 동작해야 하는
핵심 부분)이 production 수준으로 완전히 갖춰지기 전까지는, 여기 적힌 건 전부
방향 제시용 메모일 뿐 확정된 설계가 아니다. 나중에 필요해지면 아래 단서를
참고해서 5개 호스트를 통일시키고 어디에 응용할지 결정한다.

### unsafeGetAttrPos

- Nix 실제 스펙: 속성이 정의된 위치를 `{ file; line; column; }` 모양으로
  돌려준다.
- 2026-08-19 5개 호스트 감사 결과:
  - hy: `{file; line; column;}` — Nix 스펙과 일치하는 모양. 나중에 통일할
    때 이게 목표 모양일 가능성이 높음.
  - clj: `{start; end; span;}`(바이트 오프셋) — clj 자신의 todo.md에 이미
    "line/column 추적 인프라가 생길 때까지의 임시방편"이라고 적어둠.
  - **clr (여기)**: 항상 `null` — 위치 추적 자체를 아직 안 함. `evaluator.clj`
    `:unsafeGetAttrPos` 케이스가 파서 span을 안 들고 있음.
  - cljs: 빌트인 이름은 등록돼있는데 호출하면 "not-callable" 에러 — 이름만
    있고 실제 구현이 없는 죽은 항목.
  - rs: 아예 등록 안 됨.
- 방향 아이디어(확정 아님): line/column 추적은 이 빌트인 하나만을 위한 게
  아니라 에러 메시지 품질 전반에 같이 쓸 수 있는 인프라다 — 파싱/평가 에러가
  지금은 대부분 바이트 오프셋(`:offset`)만 주는데, 실제 Nix처럼
  "파일:줄:컬럼"으로 보여주면 디버깅이 훨씬 편해진다. 이 하나만 따로
  만들기보다 에러 위치 표시 개선 작업과 묶는 게 나을 수 있다. clr의 `import`
  는 이미 `context`(root/file)로 "어느 파일인지"를 정확히 추적하고 있으니
  (2026-08-19 교차검증에서 확인), line/column만 추가하면 다른 호스트보다
  오히려 빨리 hy와 같은 모양에 도달할 수 있을 것 같다.

### pnixMounts

- Nix 실제 빌트인 아님 — `evaluator.clj`에 이미 `:type-error`,
  `"extension-not-wired"`, `:nix-builtin? false`로 명시돼있다. Nix 호환
  주장에서 의도적으로 제외된 pnix 자체 아이디어다.
- 이름과 프로젝트 전체 설계 방향(순수 평가기는 기본적으로 실제 OS
  파일시스템/store에 손을 못 댐 — `storePath`도 마찬가지로 여기서 명시적으로
  막혀있음)으로 미루어 짐작하면(확정 아님, 순전히 추측): "순수 평가기에게
  실제 OS 파일시스템 대신 미리 정해둔 가상 경로 목록(mount)만 제한적으로
  보여주는 기능"일 가능성이 있다.
- pnix-clj의 `import`가 `*import-modules*`(경로 문자열 -> pnix 소스 텍스트로
  된 순수 인메모리 맵)로 정확히 이 패턴을 이미 증명해뒀다. clr은 이미
  `import`가 `host/resolve-import` + `modules` atom으로 실제 파일시스템
  캐싱/순환참조 감지까지 잘 되어 있으니(2026-08-19 교차검증에서 확인 —
  다섯 호스트 중 가장 정교했음), 같은 인프라 위에 인메모리 mount 계층을
  얹는 게 자연스러워 보인다.
- 방향 아이디어(확정 아님): 나중에 필요해지면 `import`뿐 아니라
  `pathExists`/`readFile`/`readDir` 같은 다른 파일시스템 관련 빌트인들도
  똑같은 "인메모리 mount 맵" 패턴으로 확장하고, `pnixMounts`는 그 맵을
  읽기 전용으로 들여다보는 조회용 빌트인으로 만드는 게 자연스러워 보인다.
  정확한 시그니처/의미는 아직 미정 — 실제로 필요한 상황(재현 가능한 테스트,
  hermetic 빌드 등)이 생겼을 때 다시 설계해야 한다.

**중요**: 위 두 항목은 전부 방향 제시용 메모다. 5개 호스트를 실제로
통일시키는 작업은 기본 언어 기능이 production 수준으로 완전히 갖춰진 다음,
필요에 의해 결정한다.

## 2. 실제 백로그 — CAPABILITIES.md 자동 생성기 없음 (2026-08-19)

(원본: `pnix-clr/pnix-clr/todo.md`. 2026-08-20 이 문서로 통합, 옛
`todo.md`는 삭제됨.)

위 두 항목과 달리 이건 "아직 예정 없음" 메모가 아니라 **참고할 작동하는
구현이 이미 3개 있는** 실제 gap이다. clj(`docs/CAPABILITIES.md` +
`capabilities.clj`, `clojure -M:capabilities`로 재생성, drift 게이트
있음), hy(`docs/CAPABILITIES.md` + `pnix_hy/capabilities.py`,
`pnix-hy-project --capabilities`), rs(`docs/CAPABILITIES.md` +
`capabilities` 서브커맨드, `capabilities-check` 게이트) 셋 다 코드에서
자동 파생되고 drift가 나면 게이트가 잡는 능력 인덱스를 갖고 있다.

이 호스트(pnix-clr)와 pnix-cljs만 이게 없다 — `IMPLEMENTATION.md` §6(구
`docs/CLOJURE_CLR_ADMITTED_SURFACE.md`) 같은 지금 있는 문서는 전부 사람이
손으로 쓰고 갱신하는 것이라 코드가 바뀌어도 자동으로 어긋난다는 걸 보장
못 한다. 만들 때는 이 3개 호스트의 패턴(CLI 서브커맨드가 소스를 훑어
`docs/CAPABILITIES.md`를 재생성 + 그 결과와 실제 파일을 비교하는 drift
게이트)을 그대로 참고하면 된다 — 자세한 배경은
[`IMPLEMENTATION.md`](IMPLEMENTATION.md) §9.

## 3. 프로세스 내 평가기 스파이크 — 언제 experimental 딱지를 뗄지 (미정)

(원본: `docs/IN_PROCESS_EVAL.md` "수락 스케치" + "여전히 열린 항목" 중
아직 안 끝난 부분. 2026-08-20 이 문서로 통합, 옛 `IN_PROCESS_EVAL.md`는
삭제됨. 착륙한 스파이크 자체의 동작 방식/검증된 코퍼스는
[`IMPLEMENTATION.md`](IMPLEMENTATION.md) §8 참고, 지금 막혀 있는 기술적
제한(collectible ALC 등)은 [`BUGS.md`](BUGS.md) §4 참고.)

`Eval.SourceInProcess`/`FileInProcess`는 옵트인·실험 상태이고, 소유자가
언제 이걸 "admitted"(비실험) 상태로 끌어올릴지는 아직 정해지지 않았다.
승격을 고려할 때 남은 미정 사항:

- **net8 호스트 스토리를 명시적으로 확정할 것.** 현재도 사실상 "net8
  호스트는 process-spawn만 쓴다"가 맞는 답이고 TFM 정책(§7)에도 이미
  그렇게 적혀 있지만, in-process eval을 승격할 때 이걸 "그냥 원래
  그랬다"가 아니라 명시적으로 재확인/문서화해야 하는지는 아직 결정 안
  됨.
- **collectible ALC 언로드가 풀리기 전까지는 승격을 미룰지, 그 제한을
  안은 채로 승격할지.** 지금은 이 문제 자체가 upstream substrate의
  ALC-aware load 지원 여부에 막혀 있어서(`BUGS.md` §4), pnix-clr 쪽에서
  당장 할 수 있는 게 많지 않다 — substrate 쪽 상황이 바뀌면 다시 논의.
- **패리티 코퍼스를 기본 `pnix-clr-gate`의 필수 항목으로 승격할지.**
  지금은 substrate+artifact가 있을 때만 자동 연결되고
  `PNIX_CLR_INPROCESS_GATE=0`으로 옵트아웃 가능한 상태 — 이걸 항상 필수로
  바꿀지는 미정.

이 셋 다 "언젠가 결정할 것"이지 지금 누가 바로 집어서 끝낼 수 있는 확정된
작업이 아니라서 `TODO.md`가 아니라 여기 있다.
