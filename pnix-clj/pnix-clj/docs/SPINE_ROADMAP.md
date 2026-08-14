# 메타순환 증거-저장소 SPINE — 연구 검증 로드맵

체크리스트 증거-저장소 spine (§3/5/8/9/10/13.1/6.6-6.7/15)을 feat 브랜치에
구축하기 위한 상세·검증된 기법 계획. `/deep-research` (95 agents, adversarial
verification, 2026-07-04)로 동료 심사 출처에 대해 검증. 무엇이 있고 이 계획이
무엇을 짓는지는 `docs/META_CIRCULAR_AUDIT.md` 참고; 공백은
`resources/pnix_clj/roadmap.edn`에도 기계 등록.

## 0. THE 하중 원칙 (이것 틀리면 아무것도 건전하지 않음)

**Content hash는 동등성 증명이 아니라 PROPOSE 필터.** 스타일 메모가 아니라
증명됨:

- α-equivalence modulo 해싱 (Maziarz, Ellis, Lawrence, Fitzgibbon & Peyton
  Jones, **PLDI 2021**, arXiv:2105.02856)은 α-동등 서브트리가 동일 해시됨을
  보장 (한 방향, 결정적), 그러나 역 (same-hash ⇒ α-동등)은 낮은 충돌 확률로만
  성립 — Lemma 6.6이 `(|a|+|b|)/2^b`로 상한, 그것도 random-oracle 모델 아래;
  실제 seeded hash로의 확장은 *증명 없이* 주장 (Khuong,
  pvk.ca/Blog/2022/12/29).
- 경험적으로: Nix over 709,816 packages / 17 revisions (Malka, Zacchiroli &
  Zimmermann 2025, arXiv:2501.15919)는 input-addressing에도 불구하고 bitwise
  재현성이 **69–91%**만 — input-hash 동등이 동일 출력을 보장하지 않음; 실패
  ~15%는 임베드된 빌드 날짜.

**⇒ 모든 spine 능력 규칙:** hash-hit은 빠른 경로 (eval 스킵 / dedup / cache)를
허가하지만, 진리로 다루기 전 정확한 구조/α 검사 (그리고 결정성에 대해 실제
재실행)로 CONFIRMED 되어야 함. pnix-clj 헌법 "proven-vs-heuristic 경계"의
구체화.

## 1. 구축 순서 (의존성 존중 — 재정렬 금지)

```
§3 term store  →  §5 event log  →  §10 + §13.1 reflection snapshots
     →  §8 snapshot determinism  →  §9 purity-as-events
     →  §17 search  →  §6.6-6.7 mirror drift  →  §15 witness (capstone)
```
근거: 모든 것이 §3 term 해시로 키잉; §8 resolve-term은 §3 + 그것이 pin하는
§10/§13.1 스냅샷 필요; §9는 §3(hash)+§5(events)+§8(snapshot) 필요; §17은 §3
open-term summary 필요; §6.6-6.7은 §5 필요; §15가 전부 통합.

## 2. 능력별 계획 (기법 · 참조 · 함정 · 스케치)

### §3 — Canonicalization + content-addressed TERM store  [FOUNDATION]
- **기법**: 먼저 canonicalize, 그다음 hash, term GRAPH 위.
  1. 순서 독립 attrset/let 바인딩; first-order term graph로 letrec
     → **bisimulation collapse** → read back (Grabmayer & Rochel, *Maximal
     Sharing in the Lambda Calculus with letrec*, **ICFP 2014**,
     arXiv:1401.1460). UNFOLDING-equivalence (구조 공유) 결정, isomorphism까지
     정규 — NOT β/η. 붕괴 그래프 위 hash-cons.
  2. positional / de-Bruijn binders + dependencies-by-hash (**Unison** 레시피:
     "each definition identified by a hash of its syntax tree"; named args →
     positional refs; deps → their hashes; AST는 텍스트가 아니라 해시로 키잉
     저장).
  3. α-aware content hash (Maziarz PLDI'21) → `term-key`.
- **함정**: 지름길 "de Bruijn + sha256 the Merkle tree"는 OPEN 서브텀에
  UNSOUND — false negative (다른 binder 깊이의 같은 서브텀) AND false positive
  (격리 시 동일해 보이는 서로 다른 open 서브텀) — Blaauwbroek, Olšák & Geuvers,
  *Hashing Modulo Context-Sensitive α-Equivalence* (2024, arXiv:2401.02948).
  닫힌 WHOLE-term α-equivalence via de Bruijn은 sound; unsoundness는 open
  서브텀 / 유사성에 한정.
- **스케치 (pnix-clj)**: `pnix-clj.cas` — `canonical-form` (parser AST →
  붕괴 그래프), `term-hash`/`term-key` (α-aware), `put-term!`/`get-term`;
  hash-hit 시 dedup 전 `alpha-equivalent?` (붕괴 form 위 그래프 isomorphism).
  `hash.clj` 재사용. 가변/정체성-bearing 호스트 객체는 앞에서 거부 (§5와 공유
  guard).
- **§3c open-term summary**: 익명 skeleton + free-variable summary +
  context-sensitive hashing으로 structural distance (Blaauwbroek 2024), §17용.

### §5 — Append-only EVENT log (verifying traces)
- **기법**: *verifying trace* (Mokhov, Mitchell & Peyton Jones,
  *Build Systems à la Carte*, **ICFP 2018 / JFP 2020**): 해시만 저장
  (compact; early cutoff; dynamic deps). Scheduler
  (topological/restarting/suspending)는 rebuilder와 ORTHOGONAL — 분리 유지.
  값은 같은 해시로 키잉된 cached-eval/§3에 남김 (로그에 값 복제 금지).
  `open-store`, `append!`, event hash, event seq, index-by-kind/hash/field,
  pointer-movement-as-event.
- **함정 (hermeticity 오염 — `append!`에서 거부)**: Bazel hermeticity가 로그에
  절대 들어가면 안 되는 것을 정확히 열거 — build ID & 타임스탬프
  (`java.util.Date`, `System.currentTimeMillis`), 호스트 가변 바이너리 / 절대
  경로 / 시스템 컴파일러, 소스 트리 쓰기. 순수-EDN payload 규율; `append!`
  경계의 `contamination?` predicate가 정체성-bearing/런타임 객체 거부.
- **스케치**: `pnix-clj.store` (append-only, EDN-only) + `pnix-clj.evidence`
  (event 스키마); receipt 형태 재사용.

### §10 + §13.1 — Reflection 스냅샷
- all-ns / ns-publics / var-root / var-meta / dynamic-binding SNAPSHOTS + diff +
  증인; classpath + JVM-version 스냅샷/해시. §8이 PIN해야 하는 호스트-가변
  입력 (Bazel: pin host-varying binaries). 결정적 직렬화 (정렬, 정체성 없음).
  `pnix-clj.meta.namespace` / `.var` / `.classpath` / `.jvm`.

### §8 — Snapshot 결정성
- **기법**: content-addressed skip은 단말 입력에 키잉된 결정성 가정 아래에서만
  sound (deep constructive traces, Nix/Buck 클래스; *Build Systems à la Carte*
  §4.2.4 — **Frankenbuild** 예가 n≥2 비결정성이 정확성을 깨뜨릴 수 있음을 증명).
  Bazel 통합 패턴: CAS (content hash 값) + 관찰된 dep 해시로 주석된 command
  history가 결과 해시를 예측하고 eval 우회.
- **스케치**: `:snapshot/id` = (evaluator-version ⊕ symbol-version ⊕
  §10/§13.1 reflection 스냅샷) 해시; `assert-snapshot-runtime-match!`는
  mismatch 시 FAIL CLOSED; 스냅샷 아래 `resolve-term`. Futamura residual
  바이트코드는 (source term-hash + snapshot-id)로 content-addressed.

### §9 — Purity / determinism as EVENTS
- **기법**: 결정성을 정적으로 강제하지 않음. 재연/fork에서 잡히는 EVENTS로
  강제 — 위반은 한 번 잘 실행되고 나중에 재현 실패한 FIRST 이벤트에 PIN된
  발산으로 잡힘 (Nakajima 2026, *The Log is the Agent*; record-replay
  발산 탐지는 독립 확립). ★검증된 주의: 로그 위 결정적 fold는 자동으로
  byte-identical이 아니고, 비결정적 effect의 content-addressed caching이 재연을
  결정적으로 만들지 않음 — effect는 RECORD되어야 하고 결정성은 가정하지 말고
  실제 재실행으로 WITNESSED.
- **스케치**: 반복-eval 결정성 (같은 source+term-hash+snapshot+runtime-version
  → ACTUAL 재실행 + diff로 하나의 결과 해시); mutation isolation
  (이후 commit 후 옛 스냅샷 결과 immutable); threaded stress;
  비결정성 → first-divergent event에 pin된 위반 증거 (= §15 증인 anchor) +
  fail closed; payload의 date/timestamp/build-ID 패턴 스캔을 detector로.

### §17 — Search
- content-address lookup + event index + structural-similarity (§3c open-term
  skeleton distance). `pnix-clj.search`.

### §6.6-6.7 — Mirror drift + chain 수렴
- mirror DRIFT events + 반복-실행 chain-convergence 안정성, §5 events로 기록.
  (실행당 cross-mirror verdict는 이미 `mirror.clj`에 존재.)

### §15 — 증인 스키마 + admission lattice  [CAPSTONE]
- 명시 증인 (`:witness/id`, input/output/term/result hash, runtime/
  compiler/evaluator version, snapshot id, stage, status, evidence events) +
  admission/status lattice (held/candidate/admitted/rejected/evidence/failed/ok),
  in-toto / SLSA-shaped. 전부 통합: cross-mirror tower 판정을 §5 events로 기록해
  §15에 공급; §9 first-divergent-event가 증인 anchor.

## 3. 붙여넣기 가능한 TODO 시퀀스 (연구 순서)

1. **§3a canonicalizer** — AST → 순서 독립 attrset/let; term-graph +
   bisimulation collapse로 letrec (Grabmayer-Rochel); 가변/정체성 호스트 객체
   거부; positional binders + dependency-by-hash (Unison).
2. **§3b α-aware content hash** (Maziarz) → term-key → put/get; hash-hit =
   PROPOSE only → 붕괴 form 위 graph-isomorphism / α-check로 확인.
3. **§3c open-term summary** (skeleton + free-var summary + structural distance)
   via context-sensitive hashing (Blaauwbroek), NOT raw de-Bruijn Merkle.
4. **§5 event log** — open-store/append!/event-hash/seq/index/pointer-as-event;
   순수-EDN 규율; build-IDs/timestamps/host-varying/mutable 거부 CAS guard
   (hermeticity 클래스).
5. **§10 + §13.1** — ns/var/meta/dynamic-binding 스냅샷 + diff + 증인;
   classpath + JVM-version 스냅샷/해시.
6. **§8 snapshot** — :snapshot/id + §10/§13.1 스냅샷을 묶는 evaluator/symbol-
   version; runtime-match 게이트 FAIL-CLOSED; 스냅샷 아래 resolve-term.
7. **§9 purity-as-events** — 실제 재실행+diff로 반복-eval 결정성;
   mutation isolation; threaded stress; 비결정성 → first-divergent event에
   pin된 증거 + fail closed; payload date/timestamp/build-ID 스캔.
8. **§17 search** — content-address + event index + structural similarity.
9. **§6.6-6.7** — mirror drift events + chain-convergence 안정성.
10. **§15 증인 + admission lattice** (capstone) — 전체 스키마, in-toto/SLSA
    shaped; residual 바이트코드 (term-hash + snapshot-id) content-addressed;
    cross-mirror 판정을 §5 events로 §15에 공급.

## 4. 전제 소유자 결정 (변경 없음)

`origin/main`이 이미 §3/5/8/9/17 상당 부분 구현 (`cas.clj`/`store.clj`/
`term.clj`/`stage.clj`/`purity.clj`/`resolve.clj`/`evidence.clj`/`search.clj`).
구축 전 선택: **(A)** main spine을 feat로 port, **(B)** 분리 유지, **(C)**
rewrite 스타일로 feat에 최소 재구축. (A)/(C)이면 THIS 계획이
port/재구축 코드가 충족해야 할 정확성 명세 (특히 §0: hash = propose-filter,
정확히 확인).
