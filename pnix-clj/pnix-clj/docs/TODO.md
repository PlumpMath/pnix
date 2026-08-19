# pnix-clj TODO

목적: 지금 당장 픽업 가능한, 아직 안 끝난 작업만 적는다. 이미 끝난 일은
[`IMPLEMENTATION.md`](IMPLEMENTATION.md) §4 역사 절에, 아직 착수가 확정
안 된 미래 방향은 [`PLANS.md`](PLANS.md)에 둔다 — 여기 섞지 않는다. 알려진
버그·의도된 제한은 [`BUGS.md`](BUGS.md) 참고.

## 언어 정합성 / 레인 커버리지

- **builtin-by-builtin laziness exactness 감사 계속.** 현재 selector/shape/
  equality/force-boundary 슬라이스는 끝났지만, 값을 실제로 inspect하는
  fold/map류와 equality error-boundary receipt는 아직 감사 전이다. 모든
  레인이 동의할 때만 `mirror-pair` 또는 전용 receipt로 승격할 것.
  ([`IMPLEMENTATION.md`](IMPLEMENTATION.md) §1 laziness 설계 참고.)
- **`.px` 에러 reason taxonomy 확장.** 현재 한계는 [`BUGS.md`](BUGS.md)
  "알려진 한계" 참고 — `mk_error`가 메시지 하나만 담아 다른 레인과 구조적
  비교가 안 된다. 필요할 때마다 kind를 늘릴 것.
- **String context 커버리지 마무리 확인.** `baseNameOf`/`dirOf`/
  `concatMapStringsSep`/`optionalString`이 다른 문자열 빌트인들처럼
  context-aware인지 재점검할 것(대부분의 문자열 빌트인은 이미 처리됨,
  이 넷만 미확인 상태로 남아있었다). `all`이 sub-derivation을 담은
  리스트를 순회할 때 context 처리가 맞는지도 확인.
- **Rust-grounded/투영 코퍼스 확장.** 현재 10/10 Rust-grounded fixture와
  stage7 core 5/5는 전부 accepted. 새 fixture 배치가 들어오면 파서/평가기/
  lowering 확장을 이어서 진행할 것.
- **F4 · tools.analyzer.jvm AST-pass substrate 노출.** clj-meta가 이미
  `tools.analyzer.jvm`을 쓰고 있으니, 이를 재사용 가능한 pass 레인으로
  노출하는 작업 — Clojure-on-Clojure AST + 커스텀 pass pipeline(Python/Hy엔
  대응물이 없는 Clojure/JVM 고유 능력). 아직 착수 전.

## Machine fragment (M-series)

- **기계 fragment 성장은 pillar가 실제로 필요로 할 때만.** M7 계열
  (`pnix-clj.machine`)의 differential corpus는 D22 + 2026-08-14 오라클
  핀까지 ~216행으로 자라 있다. 지금 우선순위 급한 작업은 아니고, 어떤
  pillar가 그걸 요구할 때 이어서 성장시킬 것.

## 호스트 라이브러리 제품 폴리시

([`IMPLEMENTATION.md`](IMPLEMENTATION.md) §11 참고 — 공개 API 표면은 이미
`docs/HOST_IMPORT.md` 흡수분으로 문서화됨.)

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
