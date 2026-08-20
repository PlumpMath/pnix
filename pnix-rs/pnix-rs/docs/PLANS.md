# pnix-rs — 미확정 로드맵 / 미래 방향

목적: 아직 설계가 끝나지 않았거나, 착수 여부조차 결정 안 된 방향을
한 곳에 모은 인덱스. **여기 있는 건 전부 "확정된 계획"이 아니라 "방향
제시용 메모"다** — proposal이 있으면 proposal이 권위, 없으면 아래 텍스트가
현재 유일한 기록이다. 2026-08-20 작성(옛 `REGISTRY.md` §2 로드맵 +
옛 `todo.md`의 "미래 아이디어" 절을 흡수). 실제 구현 현황은
[`docs/IMPLEMENTATION.md`](IMPLEMENTATION.md), 알려진 제한/의도적 held는
[`docs/BUGS.md`](BUGS.md), 지금 당장 할 수 있는 열린 작업은
[`docs/TODO.md`](TODO.md)를 볼 것.

**원본은 그대로 둔다**: 아래는 `docs/proposals/000N-*.md`(10개)와
`docs/research/2026-07-03-metacircular-frontier.md`의 요약+링크일 뿐,
그 파일들 자체를 여기 흡수하지 않았다 — 자세한 설계/수용 기준은 원본을
열어볼 것.

## 1. 로드맵 순위 (옛 REGISTRY.md §2, 근거: deep-research)

근거: `docs/research/2026-07-03-metacircular-frontier.md`(5각 검색 · 15소스
· 3표 적대검증 · 6 findings 전부 high-confidence).

| # | 능력 | 성격 | lane/모듈 | proposal |
|---|---|---|---|---|
| 1 | **full 3차 사영** — feature-rich specialiser 자기적용 (bounded cogen DONE; full은 연구 지평) | 연구 프론티어 | pnix-rs tower/bta | [0004](proposals/0004-hand-written-cogen.md) |
| 2 | **P6 v9+** — 트레이트 solving / 클로저 projection (수요 시) | 기계적 확장 | pnix-rs rust_mirror | [0001](proposals/0001-rust-ast-projection.md) |
| 3 | **runtime 표면(수요 시)** — 중첩 동적 attr 경로, POSIX ERE의 leftmost-longest 완전 정합(backtracking 엔진 구조상 남은 두 갭, `docs/BUGS.md` §1 참고) (경로 값/URI 리터럴/JSON float exponent canonicalization/string-context 값은 2026-08-20 기준 구현 완료 — `docs/IMPLEMENTATION.md` §1의 경로(Path) 값 절, `docs/BUGS.md` §1 참고. 비유한 float의 canonical **print**(px_print, toJSON과는 별개)가 유효한 px 소스가 아닌 문제는 여전히 open) | 기계적 확장 | pnix-rs px | [0006](proposals/0006-runtime-surface-on-demand.md) |
| 4 | **Nix builtin 표면 완전 수렴** — raw presence 전체(남은 유일한 항목) (path 값/canonical JSON float/string-context 값/`derivation`/`derivationStrict`/`placeholder`는 2026-08-20에 구현 완료) | 기계적 확장 | pnix-rs px / builtin overlay | [0010](proposals/0010-builtin-surface-convergence.md) |
| 5 | **full S=L** + stage-polymorphic | 연구 지평 | pnix-rs tower | [0007](proposals/0007-research-frontier-index.md) |
| 6 | research open — step-level bisimulation · CompCert류 certified compilation · N-레벨 collapsing tower [incremental·proof-carrying·finite reflective tower는 이미 DONE] · poly-optimizations(sharing/eta/let-insertion) 명시 게이트 미구현 | 후속 리서치 | pnix-rs tower | [0007](proposals/0007-research-frontier-index.md) |
| ext | 자매 lane(pnix-clj/pnix-hy) TSV 파일-대-파일 비교 | external 대기 | pnix-rs cross-host | [0007](proposals/0007-research-frontier-index.md) — 상세 상태는 [`docs/BUGS.md`](BUGS.md) §4 |

핵심 통찰(finding 5): fv-제한 등 subject BTA는 **Jones-optimality를 못
올리는 강도 천장** — 다음은 "더 coarsen"이 아니라 위 게이트들. finding 6:
#1이 언어별 meta-circular 잠재력 차이의 정수(Rust만 싸게 얻는 정적 보증).

## 2. proposal 전체 인덱스 (옛 REGISTRY.md §3 + 0010 추가)

`docs/proposals/`의 10개 파일 전체. DONE인 것도 설계 기록으로 남겨둔다 —
"왜 이렇게 만들었는지"가 여기 있다.

| # | 제목 | 상태(2026-08-20 기준) | 한 줄 요약 |
|---|---|---|---|
| [0001](proposals/0001-rust-ast-projection.md) | Rust AST 구조 축 projection (P6 v1) | v8 DONE(제네릭 struct/impl/method), 트레이트 solving·클로저 projection·비균형-브래킷 char 리터럴은 held | px 값이 아니라 Rust AST 구조 자체를 px로 reify/reflect하는 축 — sig 브래킷 트리 왕복부터 제네릭까지 단계적으로 확장 |
| [0002](proposals/0002-px-attrs-sorted-lookup.md) | px 값 attrset의 정렬 표현 + 이진 탐색 조회 | DONE(2026-07-03) | 2차 Futamura 사영이 선형 스캔 Vec 때문에 못 끝나던 것 — `PxVal::Attrs`를 이름-정렬 불변식으로 바꿔 조회 O(log n) |
| [0003](proposals/0003-px-call-by-need.md) | px 재귀 let의 call-by-name → call-by-need | DONE(2026-07-03) | 재귀 let 참조마다 재평가되던 걸 바인딩별 memo로 — 2차 사영이 1h40m+ 미종결에서 ~0.1초로 완주한 결정적 한 수 |
| [0004](proposals/0004-hand-written-cogen.md) | 손으로 쓴 cogen (3차 Futamura 사영을 자기적용 없이) | bounded DONE(산술 객체언어), full 3rd projection은 연구 지평으로 held | 자기적용 대신 cogen을 손으로 작성해 3차 사영의 이득을 우회 획득(Leuschel 접근) |
| [0005](proposals/0005-well-typed-residual-gate.md) | 잘-타입된 residual 게이트 | DONE(2026-07-03) | px→Rust residual이 rs-meta 플로어 typeck로 구성상 타입-정합임을 증명 — Rust 정적 강점을 동적 Lisp이 못 얻는 성질 |
| [0006](proposals/0006-runtime-surface-on-demand.md) | px runtime 표면 확장(수요 기반) | HELD(로드맵 순위 3) | SCOPE_LOCK held 표면(int↔float 승격 등 — 일부는 0010으로 이미 개방됨, 나머지는 §1 로드맵 참고) 확장 원칙: 수요 없이 확장 금지 |
| [0007](proposals/0007-research-frontier-index.md) | 연구 프론티어 인덱스 | 인덱스(다수 항목 DONE, 일부 open) | deep-research가 다루지 못한 후속 리서치 대상(bisimulation, N-레벨 collapsing tower 등)을 한 곳에 등록 |
| [0008](proposals/0008-peer-engine-adapter.md) | peer-engine adapter | v1 DONE(2026-07-03, `src/engine.rs`) | rs-meta의 Rust translation-validation 결과를 pnix-hy/clj류 peer engine이 이해할 공통 `.px` verdict 봉투로 매핑 — rs-meta는 여전히 pnix를 모름 |
| [0009](proposals/0009-canonical-rust-ir.md) | canonical Rust IR + hash | v0 DONE(2026-07-03) | peer-engine verdict에 포맷 불변 content-address(`ir_hash`)를 채워 TV 실패 위치 추적/캐시 키로 쓸 수 있게 |
| [0010](proposals/0010-builtin-surface-convergence.md) | builtin surface convergence | phase 1-2 DONE(2026-07-10), path·context·canonical-float 수렴 DONE(2026-08-20), raw-surface 수렴만 open | Nix 118종 대비 rs 77→91종으로 좁힘, checked i64/mixed int-float/hashString을 `nix-instantiate 2.34.7` 오라클에 pin — REGISTRY.md 원본에는 목록이 빠져 있었음(2026-08-20 발견, 여기 추가) |

## 3. 연구 프론티어(0007)가 다루지 못한 부분 — 후속 리서치 대상

`docs/research/2026-07-03-metacircular-frontier.md`(요약만, 원본은 그대로
둠) 기준 아직 open인 것:

- **(c) translation-validation/certified-compilation** — proof-carrying
  residual(certify-check)은 DONE이지만, step-level bisimulation과
  CompCert류 certified compilation은 여전히 open.
- **(d) reflective tower / collapsing towers**(Amin&Rompf, 3-Lisp/Black) —
  finite 2-레벨 coherent form(reflect-tower-check)은 DONE, **N-레벨
  collapsing tower**(self_interp 인코딩의 자기적용 무게 벽)는 open.
- **(g) pnix-hy 예제 매핑 중 미완**: 21-poly-optimizations(sharing/eta/
  let-insertion)는 etaBody/fv-제한/call-by-need로 부분 보유하지만 명시
  게이트는 아직 없음.

## 4. pnixMounts / unsafeGetAttrPos — 통일 방향 (아직 예정 없음, 옛 todo.md "미래 아이디어" 절)

**지금은 만들지 않는다.** 기본 언어 기능(5개 호스트가 실제로 똑같이
동작해야 하는 핵심 부분)이 production 수준으로 완전히 갖춰지기 전까지는,
아래는 전부 방향 제시용 메모일 뿐 확정된 설계가 아니다. 나중에 필요해지면
아래 단서를 참고해서 5개 호스트를 통일시키고 어디에 응용할지 결정한다.
**이 두 항목은 이번 문서 정리에서 "참고할 실제 레퍼런스 구현이 어디에도
없는" 유일한 예외라 proposal로 승격하지 않고 메모로만 남겨둔다.**

### unsafeGetAttrPos

Nix 실제 스펙: 속성이 정의된 위치를 `{ file; line; column; }` 모양으로
돌려준다. 2026-08-19 5개 호스트 감사 결과:

- **hy**: `{file; line; column;}` — Nix 스펙과 일치하는 모양. 나중에
  통일할 때 이게 목표 모양일 가능성이 높음.
- **clj**: `{start; end; span;}`(바이트 오프셋) — clj 자신의 문서에 이미
  "line/column 추적 인프라가 생길 때까지의 임시방편"이라고 적어둠.
- **clr**: 2026-08-20에 `{file; line; column}` 구현됨 (hy 오라클과 동일).
- **cljs**: 2026-08-20에 `{file; line; column}` 구현됨 (hy 오라클과 동일).
- **rs(여기)**: 2026-08-20에 `{file; line; column}` 구현됨 (hy 오라클과 동일).

방향 아이디어(확정 아님): line/column 추적은 이 빌트인 하나만을 위한 게
아니라 에러 메시지 품질 전반에 같이 쓸 수 있는 인프라다 — 파싱/평가
에러가 지금은 대부분 바이트 오프셋만 주는데, 실제 Nix처럼
"파일:줄:컬럼"으로 보여주면 디버깅이 훨씬 편해진다. 이 하나만 따로
만들기보다 에러 위치 표시 개선 작업과 묶는 게 나을 수 있다. `import`가
2026-08-19에 실제 파일시스템 읽기로 열렸으니(rs도 이미 파일 모드 `-f`로
잘 동작함), "어느 파일인지" 추적하는 것도 이제 실제로 의미가 생겼다.

### pnixMounts

Nix 실제 빌트인 아님 — pnix-clj/pnix-clr 소스에 `:nix-builtin? false`,
`:policy :non-faithful-extension-not-nix-coverage`로 명시돼있다. Nix 호환
주장에서 의도적으로 제외된 pnix 자체 아이디어다.

이름과 프로젝트 전체 설계 방향(순수 평가기는 기본적으로 실제 OS
파일시스템/store에 손을 못 댐 — `storePath`도, 원래 `import`도 전부 이
원칙 때문에 막혀있다가 2026-08-19에 `import`만 실제 파일 읽기로
확장됨)으로 미루어 짐작하면(확정 아님, 순전히 추측): "순수 평가기에게
실제 OS 파일시스템 대신 미리 정해둔 가상 경로 목록(mount)만 제한적으로
보여주는 기능"일 가능성이 있다.

pnix-clj의 `import`가 `*import-modules*`(경로 문자열 -> pnix 소스 텍스트로
된 순수 인메모리 맵)로 정확히 이 패턴을 이미 증명해뒀다. rs 쪽에서
비슷한 걸 만든다면, `import`의 자체 evaluable-subset 제약(px.rs가
rs-meta로 검증 가능해야 함)을 그대로 지키면서 같은 아이디어를 이식할 수
있는지가 관건.

방향 아이디어(확정 아님): 나중에 필요해지면 `import`뿐 아니라
`pathExists`/`readFile`/`readDir` 같은 다른 파일시스템 관련 빌트인들도
똑같은 "인메모리 mount 맵" 패턴으로 확장하고, `pnixMounts`는 그 맵을
읽기 전용으로 들여다보는 조회용 빌트인으로 만드는 게 자연스러워 보인다.
정확한 시그니처/의미는 아직 미정 — 실제로 필요한 상황(재현 가능한 테스트,
hermetic 빌드 등)이 생겼을 때 다시 설계해야 한다.

**중요**: 위 두 항목은 전부 방향 제시용 메모다. 5개 호스트를 실제로
통일시키는 작업은 기본 언어 기능이 production 수준으로 완전히 갖춰진
다음, 필요에 의해 결정한다.
