# pnix-clj TODO

목적: 지금 당장 픽업 가능한, 아직 안 끝난 작업만 적는다. 이미 끝난 일은
[`IMPLEMENTATION.md`](IMPLEMENTATION.md) §4 역사 절에, 아직 착수가 확정
안 된 미래 방향은 [`PLANS.md`](PLANS.md)에 둔다 — 여기 섞지 않는다. 알려진
버그·의도된 제한은 [`BUGS.md`](BUGS.md) 참고.

## 언어 정합성 / 레인 커버리지

- **Rust-grounded/투영 코퍼스 확장.** 현재 10/10 Rust-grounded fixture와
  stage7 core 5/5는 전부 accepted. 새 fixture 배치가 들어오면 파서/평가기/
  lowering 확장을 이어서 진행할 것.

## Machine fragment (M-series)

- **기계 fragment 성장은 pillar가 실제로 필요로 할 때만.** M7 계열
  (`pnix-clj.machine`)의 differential corpus는 D22 + 2026-08-14 오라클
  핀까지 ~216행으로 자라 있다. 지금 우선순위 급한 작업은 아니고, 어떤
  pillar가 그걸 요구할 때 이어서 성장시킬 것.

## 호스트 라이브러리 제품 폴리시

([`IMPLEMENTATION.md`](IMPLEMENTATION.md) §11 참고 — 공개 API 표면.
옛 `docs/HOST_IMPORT.md`는 그 절로 흡수됐다.)

- **공개 Maven/local jar 좌표(선택).** 지금은 `bin/export-pnix-clj-library`
  로컬 export만 있다. 프로젝트가 monorepo 경로를 `local/root`로 잡지 않고도
  쓸 수 있게 하려면 공개 좌표가 필요한데, 아직 착수하지 않았다(제품
  목표인지도 확정 안 됨 — Maven Central 게시 자체는 소유자 정책상 비목표,
  로컬/사설 레지스트리 좌표라면 검토 여지).

## 완료로 판단해 여기서 뺀 것들

- **Oracle D-type wrong-VALUE / over-strict 스윕(2026-08-14)** — 충분히
  닫힌 상태로 판정됐다(`HOST_ENV_P2_P3.md` § Oracle D-type surface 참고).
  새로운 nix-instantiate 발산이 발견될 때만 재개.
- **F8 weval-style IR-level PE** — 스파이크로 랜딩 완료
  ([`IMPLEMENTATION.md`](IMPLEMENTATION.md) §4 역사 표, `pnix-clj.weval`).
- **게이트 리포트 캐시** — 랜딩 완료(13종 리포트 1-JVM 통합).
- **F4 · tools.analyzer.jvm AST-pass substrate** — 랜딩 완료
  (`pnix-clj.form-analysis`, `analyze-form`, 게이트
  `form-analysis-ast-pass-lane` / `synthesize-form-analysis-convergence`,
  example 38, WIKI `f4-analyzer-pass-lane`). Clojure-on-Clojure AST +
  host-interop / pure-core 분류; Python/Hy 대응물 없는 JVM 고유 레인.
- **String context 네 빌트인 + `all`** — `baseNameOf`/`dirOf`/
  `concatMapStringsSep`/`optionalString`을 context-aware allowlist에
  넣었고, `all`/`any`는 bool 결과라 컨텍스트가 새지 않아 같이 허용.
  서브 derivation 리스트와 보간 ctx-string 리스트 둘 다 게이트됨.
- **fold/map inspect + equality error-boundary** — 값을 읽는 fold/map은
  해당 슬롯만 강제; `==`는 길이/키셋이 다르면 슬롯을 안 보고, 같은
  모양이면 강제. Attrset는 Nix처럼 이름 정렬 순으로 비교(앞 키가
  다르면 뒤 에러 슬롯을 안 봄). 레인 전원이 동의하는 행만
  `mirror-pair`로 승격. clj-meta lowering `nix-equal`도 이제 이름 정렬
  순으로 비교한다 — `{ a = 1; z = 1/0; } == { a = 2; z = 1; }`가 4레인
  동의.
- **`.px` 에러 reason taxonomy** — `mk_error_as`가 `failure.class`를
  실어 호스트 machine class와 비교한다. 언어 에러 사이트(throw/abort/
  with/index/pattern/init/purity-gated)를 class로 분류했고,
  `runMirror.error_class`와 mirror-error 8건이 레인 class를 핀한다.
  모듈-스키마/`unsupported-expression` 잔여는 필요할 때마다 kind를 늘릴 것.
