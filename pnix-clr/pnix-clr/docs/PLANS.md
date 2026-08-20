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
- 2026-08-21 기준 5개 호스트 모두 `{file; line; column;}` (인라인 파일
  라벨 `"<pnix-px>"`, 생성 attrset은 `null`, `inherit` 이름은 inherit 절
  위치). **clr (여기)** 는 2026-08-20에 hy 오라클과 같은 모양, 2026-08-21에
  inherit 이름 위치와 점선 attrpath 첫 세그먼트 공유를 맞춤.
- 방향 아이디어(확정 아님): line/column 추적은 에러 메시지 품질 전반에
  같이 쓸 수 있는 인프라다 — 파싱/평가 에러가 지금은 대부분 바이트
  오프셋(`:offset`)만 주는데, 실제 Nix처럼 "파일:줄:컬럼"으로 보여주면
  디버깅이 훨씬 편해진다.

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

## 2. 해결됨 — CAPABILITIES.md 자동 생성기 없음 (2026-08-19 제기, 2026-08-20 해결)

(원본: `pnix-clr/pnix-clr/todo.md`. 2026-08-20 이 문서로 통합, 옛
`todo.md`는 삭제됨.)

clj/hy/rs 세 호스트를 참고해서 이 호스트에도 만들었다: `bin/pnix-clr
capabilities`가 `pnix-clr.evaluator/builtin-names`(root `builtins-entries`
등록 테이블에서 직접 introspect, 손으로 옮겨 적지 않음)와 `pnix-clr.main/
cli-commands`에서 [`CAPABILITIES.md`](CAPABILITIES.md)를 렌더링하고,
`bin/pnix-clr capabilities-check`가 drift 게이트로 `bin/pnix-clr-gate`에
연결돼 있다. 자세한 배경은 [`IMPLEMENTATION.md`](IMPLEMENTATION.md) §9,
생성기 구현은 `pnix-clr/src/pnix_clr/main.clj`
(`capabilities-doc`/`capabilities-check!`)와
`pnix-clr/src/pnix_clr/evaluator.clj`(`builtin-names`) 참고.

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
