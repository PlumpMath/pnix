# pnix-hy 미래 계획 / proposal 인덱스

이 문서는 아직 **확정 안 된** 방향과, 확정된 proposal들의 **인덱스**를
담는다. proposal 문서 자체(`docs/proposals/NNNN-*.md`)를 여기 흡수하거나
지우지 않는다 — 이 문서는 1-2줄 요약 + 링크만, 본문은 각 proposal 파일이
정본이다.

새 아이디어가 proposal이 될 준비가 되면 `docs/proposals/NNNN-<slug>.md`로
시작한다(`docs/IMPLEMENTATION.md` §4.7). `docs/TODO.md`에 `[ ]`로 바로
추가하지 않는다 — 사람이 수락한 뒤에만 TODO로 넘어간다.

## 1. 미확정 미래 방향 (proposal 없음, 설계도 미완)

### 1.1 pnixMounts / unsafeGetAttrPos 5개 호스트 통일 (2026-08-19, 아직 예정 없음)

지금은 만들지 않는다. 기본 언어 기능(5개 호스트가 실제로 똑같이 동작해야
하는 핵심 부분)이 production 수준으로 완전히 갖춰지기 전까지는, 여기 적힌
건 전부 방향 제시용 메모일 뿐 확정된 설계가 아니다. 나중에 필요해지면
아래 단서를 참고해서 5개 호스트를 통일시키고 어디에 응용할지 결정한다.

**unsafeGetAttrPos**

- Nix 실제 스펙: 속성이 정의된 위치를 `{ file; line; column; }` 모양으로
  돌려준다.
- 2026-08-21 기준 5개 호스트 모두 `{file; line; column;}` (인라인 파일
  라벨 `"<pnix-px>"`, 생성 attrset은 `null`). **hy(여기)** 가 Nix 스펙
  모양의 기준이었고, clj는 2026-08-21에 파서 span에서 변환했다.
- 방향 아이디어(확정 아님): 다섯 곳 출력 형태가 정말 똑같은지(파일 경로
  표기 방식 등 세부 포맷 차이 포함) 다시 교차검증하면 된다. line/column
  추적은 에러 메시지 품질에도 같이 쓸 수 있다.

**pnixMounts**

- Nix 실제 빌트인 아님 — pnix-clj/pnix-clr 소스에 `:nix-builtin? false`,
  `:policy :non-faithful-extension-not-nix-coverage`로 명시돼있다. Nix
  호환 주장에서 의도적으로 제외된 pnix 자체 아이디어다. 현재 없는 이유의
  "고치지 말 것" 쪽 근거는 `docs/BUGS.md` §2.
- 이름과 프로젝트 전체 설계 방향(순수 평가기는 기본적으로 실제 OS
  파일시스템/store에 손을 못 댐 — `storePath`도, 원래 `import`도 전부 이
  원칙 때문에 막혀있다가 2026-08-19에 `import`만 실제 파일 읽기로
  확장됨)으로 미루어 짐작하면(확정 아님, 순전히 추측): "순수 평가기에게
  실제 OS 파일시스템 대신 미리 정해둔 가상 경로 목록(mount)만 제한적으로
  보여주는 기능"일 가능성이 있다.
- pnix-clj의 `import`가 `*import-modules*`(경로 문자열 -> pnix 소스
  텍스트로 된 순수 인메모리 맵)로 정확히 이 패턴을 이미 증명해뒀다. hy는
  이미 `import`가 처음부터 실제 파일시스템으로 잘 동작하고 있었으니
  (2026-08-19 교차검증에서 확인), 인메모리 mount 계층은 그 위에 얹는
  추가 옵션 정도로 생각하면 될 것 같다.
- 방향 아이디어(확정 아님): 나중에 필요해지면 `import`뿐 아니라
  `pathExists`/`readFile`/`readDir` 같은 다른 파일시스템 관련 빌트인들도
  똑같은 "인메모리 mount 맵" 패턴으로 확장하고, `pnixMounts`는 그 맵을
  읽기 전용으로 들여다보는 조회용 빌트인으로 만드는 게 자연스러워 보인다.
  정확한 시그니처/의미는 아직 미정 — 실제로 필요한 상황(재현 가능한
  테스트, hermetic 빌드 등)이 생겼을 때 다시 설계해야 한다.

**중요**: 위 두 항목은 전부 방향 제시용 메모다. 5개 호스트를 실제로
통일시키는 작업은 기본 언어 기능이 production 수준으로 완전히 갖춰진
다음, 필요에 의해 결정한다.

### 1.2 specializer/BTA 연구 잔여 (딥리서치 #1/#2/#3, `docs/audits/` 근거)

Q1(laziness×부분평가)/Q2(stage-poly) 연구 트랙 대부분은 SHIPPED 또는
CLOSED됐다(`docs/IMPLEMENTATION.md` §8.2, WON'T-DO 결정은 `docs/BUGS.md`
§3). 남은 건 이 셋뿐 — 전부 미착수, proposal 없음:

- **Q1-3 CPS로 작성한 specializer (Bondorf BTI)** — 큰 작업이라 별도
  proposal 권장. specializer를 CPS로 쓰면 source-CPS의 BTI(binding-time
  improvement) 효과를 얻으면서도 출력이 부풀지 않는다(Bondorf LFP'92) —
  단 BTI는 non-Jones-optimal specializer를 보상하지 못하므로 건전한
  reducer가 전제. 하려면 `poly_specialize`를 CPS 스타일로 재작성하거나
  새 변형을 만들어야 하는 대규모 작업.
- **R5 — scheduler×rebuilder 분류 적용** (`docs/proposals/0013-*.md`
  카탈로그의 항목 중 유일하게 아직 어떤 proposal로도 승격 안 된 것).
  Build Systems à la Carte(Mokhov+) 식으로 캐시를 verifying/constructive
  trace로 분류하고 suspending scheduler + constructive trace 지점
  ("Cloud Shake")을 아티팩트 캐시 설계 기준으로 삼는 아이디어. 0019(해시
  키 검사 캐시)/0023(증분 평가)이 인접 영역을 이미 shipped했지만, 이
  scheduler×rebuilder 분류 자체는 별도로 다뤄진 적 없다.
- **0028 P2 — optimal cogen (3차 Futamura 사영의 실행 성능벽)** — 4개
  실험(tree-walker/thunk/closure/스케일스윕)으로 "naive cogen 아티팩트의
  병리적 bloat이 근본 원인, 런타임/스케일 튜닝으로는 해결 불가"까지
  확인 후 **중단**(`docs/proposals/0028-compiled-runtime.md` 참고). 다시
  시도하려면 새로운 cogen 생성 전략 자체가 필요 — 현재 아이디어 없음.
- **(연구는 열려있지만 결정과 무관)** Q2.2(c) LMS `Rep[T]` 타입-스테이징
  심화, Q2.3 등가-보존 검증 방법론(translation validation/refinement/
  bisimulation) — 둘 다 `docs/BUGS.md` §3의 "stage-poly 안 함" 결정
  자체에는 영향 없는 순수 연구 질문. 필요해지면 4차 딥리서치 대상.

## 2. Proposal 인덱스 (`docs/proposals/`)

전부 `docs/IMPLEMENTATION.md` §4.7의 변경 프로세스를 따른다 — 새 proposal은
scope, 의도적 placeholder/OUT-of-scope 접촉 여부, 수락 여부를 명시해야
한다. 상태는 각 proposal 문서 상단의 "Status" 줄이 정본 — 아래는 요약.
0000/0013은 **후보 카탈로그**(그 자체로는 구현 안 됨, 개별 항목이 이후
proposal로 승격됨)이고 나머지는 전부 개별 구현 proposal이다.

| # | 제목 | 상태 | 요약 |
|---|---|---|---|
| [0000](proposals/0000-interop-language-feature-candidates.md) | interop 언어기능 후보 카탈로그 | 후보 카탈로그(대부분 0001-0007로 승격) | Hy/Python↔pnix interop 확장 후보 20+개, A(값/opaque)/B(callable/module)/C(macro/quote)/D(경계-ABI) 클러스터 |
| [0001](proposals/0001-roundtrip-host-value-and-loss-fidelity.md) | roundtrip-host-value + loss fidelity | SHIPPED 2026-07-01 | `interop.py` 값 왕복의 loss 마킹 정밀화(A1-A6) |
| [0002](proposals/0002-host-callable-into-pnix-eval.md) | host-callable-into-pnix-eval | SHIPPED 2026-07-01 | pnix 소스에서 host callable을 capability-gated로 호출(`host_callable_to_pnix` 등) |
| [0003](proposals/0003-hy-macro-quasiquote-over-pnix.md) | hy-macro-quasiquote-over-pnix | SHIPPED 2026-07-01 | Hy 매크로/quasiquote를 pnix-투영 폼 위에서 관찰(pnix 쪽엔 매크로 추가 안 함) |
| [0004](proposals/0004-interop-diagnostics-and-invariants.md) | interop 진단 & 불변식 | SHIPPED 2026-07-01 | drift 분류기, Hy-쪽 reification, mirror-off에서도 interop 동작 불변식 |
| [0005](proposals/0005-hy-reader-macro-embeds-pnix.md) | Hy reader macro가 pnix 임베드 | SHIPPED 2026-07-01 | Hy `#px` reader macro로 read-time에 pnix 식 삽입 |
| [0006](proposals/0006-interop-error-contract-and-role-matrix.md) | 경계 에러 계약 + role matrix | SHIPPED 2026-07-01 | `InteropError` 명확화 + `docs/INTEROP_ROLE_MATRIX.md` 신설(2026-08-20에 `docs/IMPLEMENTATION.md` §6으로 재통합) |
| [0007](proposals/0007-opaque-lifecycle-and-correspondence-abi.md) | opaque lifecycle + correspondence ABI | SHIPPED 2026-07-01 | opaque-ref 수명 추적(leak-countable) + 버전드 correspondence 아티팩트 |
| [0008](proposals/0008-meta-circular-repls.md) | meta-circular REPL 5모드 | SHIPPED 2026-07-01 | pnix/hy/python REPL(pnix-hy) + hy/python REPL(hy-meta), 전부 context 유지 |
| [0009](proposals/0009-pnix-semantic-action-vm.md) | pnix 시맨틱/action VM | SHIPPED 2026-07-02 | `action.py` — accepted/held/rejected verdict로 LLM/알고리즘 스텝을 묶는 checkpoint 레이어 |
| [0010](proposals/0010-module-distribution-tiers.md) | 모듈 배포 티어 | SHIPPED 2026-07-02 | 설치 가능한 `import pnix_hy`, CORE/projection/full 티어, `PNIX_HY_HOME` off-tree 지원 |
| [0011](proposals/0011-docs-as-code-capability-index.md) | docs-as-code 인덱스 + drift 게이트 | SHIPPED 2026-07-02 | `capabilities.py`/`docs/CAPABILITIES.md` 자동 생성 + `docs_drift_report` |
| [0012](proposals/0012-ci-enforced-management-gate.md) | CI 강제 관리 게이트 | SHIPPED 2026-07-02 | 0011을 CI에서 강제(로컬 `make ci`; GitHub Actions는 현재 disabled) |
| [0013](proposals/0013-meta-circular-and-interop-candidates.md) | meta-circular/interop 확장 후보 카탈로그 | 후보 카탈로그(대부분 0014-0030으로 승격) | T(타워/Futamura)·I(interop 강화)·P(phase/hygiene)·R(재현성)·G(감사가 찾은 진짜 gap) 5개 클러스터, R5만 미승격(§1.2) |
| [0014](proposals/0014-jones-optimality-gate.md) | Jones-optimality 수용 게이트 | SHIPPED 2026-07-02 | 코퍼스 전체 `ir_of(p) == ir_of(emit(parse(p)))` 해시 동등 게이트 |
| [0015](proposals/0015-interop-numeric-losslessness.md) | 경계 수치 무손실 술어 | SHIPPED 2026-07-02 | `numeric_fits()` — GraalVM fitsIn* 스타일 사전 손실 판정 |
| [0016](proposals/0016-opaque-own-borrow-lifecycle.md) | opaque own/borrow 수명 규율 | SHIPPED 2026-07-02 | Wasm Canonical ABI 스타일 own/borrow 비트 + num_lends, 대여 중 release는 거부 |
| [0017](proposals/0017-hygiene-self-check.md) | hygiene self-check | SHIPPED 2026-07-02 | sets-of-scopes 기반 매크로 포획 검출 리포트 |
| [0018](proposals/0018-ir-diff-and-pass-reification.md) | IR 구조 diff + 패스 물화 | SHIPPED 2026-07-02 | `ir.ir_diff`(노드-경로 diff) + `ir.ir_pipeline`(패스 델타 물화) |
| [0019](proposals/0019-hash-keyed-check-cache.md) | 해시-키 검사 캐시 | SHIPPED 2026-07-02 | `check_cache.py` — verifying-trace 캐시(opt-in `--check --cached`) |
| [0020](proposals/0020-interop-hardening.md) | interop 하드닝 웨이브 | SHIPPED 2026-07-02 | 런타임 회수 가능 capability, context-수명 opaque, blame 판정, harden 표면-witness |
| [0021](proposals/0021-compartment-isolation.md) | compartment 격리 | SHIPPED 2026-07-02 | `compartment.py` — SES Compartment 스타일 구획별 env+모듈 테이블, 완전 격리 |
| [0022](proposals/0022-phase-separation-gates.md) | phase 산술 + 분리 게이트 | SHIPPED 2026-07-02 | `phase.py` — phase ±정수 대수 + lowering 무부작용/관측 무관성 |
| [0023](proposals/0023-incremental-evaluation.md) | 증분 평가 | SHIPPED 2026-07-02 | `incremental.py` — 정의-단위 내용주소 재계산 + realisation 조기중단 |
| [0024](proposals/0024-typed-witness-attestation.md) | predicate-typed witness | SHIPPED 2026-07-02 | in-toto 스타일 버전드 predicate URI(envelope 무변경 설계) |
| [0025](proposals/0025-pe-annotations-respecialization.md) | PE 어노테이션 + 재특화 | SHIPPED 2026-07-02 | assumptions/boundaries + 의미변경 감지 시 `respecialize_if_drifted` |
| [0026](proposals/0026-tower-ladder-milestones.md) | 타워 사다리 마일스톤 | SHIPPED 2026-07-02(허용 scope 내 CLOSED) | `tower.py` — Futamura 1·2·3차 사영을 pnix로 표현/생성/실행, `--futamura` 통합 |
| [0027](proposals/0027-host-artifact-gaps.md) | host artifact 잔여 gap | SHIPPED 2026-07-02 | hy-meta 레인: `form_sha256`, 변수-단위 `env_diff`(bootstrap.py 본체 무접촉) |
| [0028](proposals/0028-compiled-runtime.md) | pnix compiled runtime | P1+P3 SHIPPED, **P2 중단**(§1.2) | `compiled.py` + `--ceval` fast-path; optimal cogen 실행 성능벽은 미해결로 남음 |
| [0029](proposals/0029-efficient-cogen.md) | efficient cogen (cogen approach) | SHIPPED 2026-07-02 | `cogen.py` — 손수 작성한 생성확장으로 인터프리터→컴파일러 0.003초(naive는 150초+) |
| [0030](proposals/0030-context-propagation.md) | context propagation | P1 SHIPPED 2026-07-03 | commuting conversion으로 Bondorf CPS 효과를 CPS 재작성 없이 realize |
