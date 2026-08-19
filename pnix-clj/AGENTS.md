# pnix-clj — pnix의 Clojure/JVM 호스트

여기는 pnix 언어의 호스트 임베딩 중 하나인 **pnix-clj** 안입니다. 이 트리는
**자기 완결**입니다. 형제 저장소에 의존하지 않으며, 다른 호스트와 corpus, gate,
`.px` core를 공유하지 않습니다. 빌드와 게이트에 필요한 모든 것이 여기 있습니다.

다음 두 정체성을 분리해 유지하세요.

- **clj-meta**는 이 호스트 언어의 self-host 증명 + 네이티브 가속을 소유합니다.
  pnix와 무관(pnix-agnostic)합니다.
- **pnix-clj**는 이 호스트의 pnix RUNTIME을 소유합니다. pnix를 parse/evaluate하고,
  `clj-meta`에 가속을 연결하며, 브리지(effect/capability 어댑터 +
  canonical-result 방출)를 제공합니다.

여기서 타협하지 않는 원칙: **meta first, never cram** — `clj-meta` 기반보다
앞서 이 호스트의 제품 표면을 키우지 마세요. **Non-regression** — 이 저장소의
게이트를 녹색으로 유지하세요. 권위 있는 범위 선언은
[`pnix-clj/docs/IMPLEMENTATION.md`](pnix-clj/docs/IMPLEMENTATION.md) §5(스코프
경계)가 지배합니다 — 의도적으로 스코프 밖인 항목은
[`pnix-clj/docs/BUGS.md`](pnix-clj/docs/BUGS.md) 참고(예전 `SCOPE_LOCK.md`의
후신, 2026-08-20 통합). 열린 작업은
[`pnix-clj/docs/TODO.md`](pnix-clj/docs/TODO.md), 아직 착수 확정 안 된 방향은
[`pnix-clj/docs/PLANS.md`](pnix-clj/docs/PLANS.md) 참고.

## 이중 축 + 호스트 라이브러리 (혼동 금지)

정본 monorepo 문서: [`../HOST_DEV_ENV.md`](../HOST_DEV_ENV.md).

| 축 | 진입점 | 역할 |
|------|-------|------|
| **host-main** | `pnix-clj-clj` / bare `clojure` | `-Sdeps` local/root로 `pnix-clj` 주입 |
| **pnix-main** | `pnix-clj-pnix` | pnix REPL / `.px` eval |
| **library** | `pnix-clj` 소스; `PNIX_CLJ_ROOT` | 호스트 바인딩 JVM 라이브러리, 이식 가능 `.px` 아님 |
| **meta** | `clj-meta` | pnix-agnostic |

호스트 언어 `.px` import: `(pnix-clj.core/eval-file "x.px")` / `eval-source`.  
공개 표면: [`pnix-clj/docs/IMPLEMENTATION.md`](pnix-clj/docs/IMPLEMENTATION.md) §11.  
HM: `~/dot-nix/dev/clj` + overlay.
