# pnix-clj BUGS

목적: 알려진 버그·한계, 그리고 **의도적으로 안 고치는** 항목을 적는다.
의도적으로 안 고치는 항목은 "이건 버그 아니라 의도된 제한"이라고 명시해서,
나중에 누가 실수로 "고치려고" 손대지 않게 한다.

## 의도된 제한 (버그 아님)

### 스코프 밖 레인 (구 `SCOPE_LOCK.md`)

`pnix-clj` 코어는 Clojure 호스팅 pnix 메타원형 증명 레인으로 한정된다
(전체 범위 선언은 [`IMPLEMENTATION.md`](IMPLEMENTATION.md) §5 참고). 아래
레인들은 "아직 구현 안 됨"이 아니라 **애초에 이 저장소의 스코프 밖으로
못박아둔 것**이다 — 이건 버그 아니라 의도된 제한:

- Hangul codec
- MSV / meaning sentence variants
- Korean dictionary / Korean mirror
- domain token matching
- graph-gate / gate-graph
- multi-language emit registry
- behavior-atom coding-agent emit
- puck-cli executor bridge
- autonomous tick runner
- redb ingest brain
- NL corpus / meaning graph / answer composer

`test/pnix_clj/scope_lock_test.clj`가 `src/`에 이 레인들의 토큰(`hangul`,
`gate-graph`, `tick-runner`, `puck-cli`, `redb-ingest`, `meaning-graph`,
`msv` 등)이 섞여 들어오지 않는지 게이트로 계속 확인한다.

### `import`/`scopedImport`는 파일시스템 캐시가 없다

`clj` 호스트는 import할 때마다 매번 새로 파싱+평가한다(clr의 canonical-path
캐시, rs의 AST splice와 다름). 순수 함수라 결과값은 항상 같아서 정합성
문제는 없지만, 같은 파일을 여러 번 import하면 다른 호스트보다 느릴 수
있다 — 성능 이슈일 뿐 정확성 이슈 아님. 자세한 내용은
[`IMPLEMENTATION.md`](IMPLEMENTATION.md) §1/§3 참고.

### `eval-source`는 기본적으로 파일시스템을 절대 안 건드린다

`*import-resolver*`가 `nil`이면 `import`는 무조건
`:import-evaluation-not-wired`로 실패한다. 이건 버그가 아니라 순도 보장을
위한 설계다(안전 샌드박스 용도로 이 호스트를 라이브러리처럼 쓰는 임베더를
위한 것) — CLI(`eval-file`/`repl.clj`)만 실제 리졸버를 바인딩한다. 자세한
내용은 [`IMPLEMENTATION.md`](IMPLEMENTATION.md) §1/§3 참고.

### `"`-in-`${}` splice leniency는 버그가 아니라 정답이다

한때 "고쳐야 할 관대함"으로 의심됐던 항목: `${...}` 안에 double-quoted
문자열이 중첩되는 걸 우리 파서(D7 balanced-scanner)가 허용하는 동작.
`/deep-research`(2026-07-08, 3-0 확인)로 조사한 결과 전제 자체가 틀렸다 —
실제 Nix도 `${…}` 안 double-quoted 문자열을 거부하지 않는다(`${`가 전체
식 컨텍스트를 열고, nixpkgs가 nested strings에 광범위 의존:
`"${"foo"}"`→`foo`, `"a${"b${"c"}d"}e"`→`abcde`, nix-instantiate 2.34.7
전부 수락). **D7의 현재 동작은 CORRECT Nix다 — 조이면 conformance가
줄어든다.** 코드/corpus 변경 없음, 결정만 기록. 남은 유일한 micro-quirk는
splice 안 `\"`-escaped quotes인데, 어느 쪽이든 마이그레이션 가치가 없다.

### 파생물(derivation)의 순환 self-reference는 모델링하지 않는다

`derivation`/`derivationStrict`는 순수 시뮬레이션(Tvix 스타일
evaluator/builder 분리, 실제 on-disk store 없음)이라 값 모델이 plain
map이다. 실제 Nix의 `d.out == d`처럼 파생물이 자기 자신을 순환 참조하는
형태는 모델링돼 있지 않다 — plain-map 값 모델의 근본적 한계라 값 모델을
바꾸기 전에는 고칠 수 없다.

### context 키의 store-path 유효성 검사는 느슨하다

실제 Nix는 문자열 context의 키가 진짜 store path인지 지연 검증한다
("context key ... is not a store path"). 이 저장소의 시뮬레이션은 임의의
키를 다 받아준다 — 문서화된 완화(relaxation)로, store를 시뮬레이션만 하는
설계 전체와 일관된다.

## 알려진 한계 (구조적, 버그로 취급하지 말 것)

### `.px` 내부 런타임의 scopedImport scope는 즉시 평가된다

`scopedImport`가 4개 레인 전부에서 동작하긴 하지만(2026-07-03), `.px`
레인만 scope를 호스트 경계를 넘기 전에 즉시(eager) 깊이 강제한다 — 그래서
직접/clj-meta 레인은 lazy하게 넘어가는 "쓰이지 않는 scope 키의 에러"가
`.px`에서는 `:scoped-import-scope-eval-held`로 걸린다. 고치려면 `.px`→호스트
lazy thunk bridge가 필요한데(같은 아키텍처 벽), 지금은 정직하게 held로
남겨뒀다.

### `.px` 에러 taxonomy는 언어 코어만 class로 맞춰 있다

`.px` `mk_error_as`는 `failure.class`(+ 빈 아닌 `evidence.reason`)를
실어 호스트 machine class와 비교한다. 코어 언어 에러(unknown-variable /
attribute-missing / assertion-failed / division-by-zero / not-callable /
non-boolean-condition / abort-builtin-called / interpolation)는
`mirror-error` 8건으로 핀돼 있다. 나머지 `mk_error` 기본값
(`unsupported-expression`, 모듈-스키마 검증 메시지)은 아직 메시지 위주라
크로스레인 비교가 안 된다 — 필요할 때마다 kind를 늘린다.

### 예전 `LANE_CLASSIFICATION.md`/`SCOPE_LOCK.md`/`clj-meta-separation.md`/
`docs/GENERATOR_DECISION.md`/`docs/HOST_IMPORT.md`/`docs/META_CIRCULAR_AUDIT.md`/
`docs/REMAINING_DECISION.md`/`docs/SPINE_ROADMAP.md`/`rec-forward-reference-taxonomy.md`
/예전 `todo.md`

2026-08-20에 이 문서 통합 작업으로 전부 [`IMPLEMENTATION.md`](IMPLEMENTATION.md)
(구현/아키텍처 내용), [`TODO.md`](TODO.md)(열린 작업), 또는 이 문서(버그·
한계)로 흡수되고 원본 파일은 삭제됐다. 예전 경로로 오는 링크가 있으면
위 세 문서 중 하나를 대신 참고할 것.
