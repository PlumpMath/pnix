# pnix-hy — pnix의 Python/Hy 호스트

당신은 **pnix-hy** 안에 있다. pnix 언어의 한 호스트 임베딩이다. 이 트리는
**자기완결**이다: 형제 저장소에 의존하지 않으며, 다른 호스트와 corpus, gate,
`.px` 코어를 공유하지 않는다. 빌드·게이트에 필요한 모든 것이 여기 있다.

다음 두 정체성을 분리 유지:

- **hy-meta**는 이 호스트 언어의 self-host 증명 + native 가속을 소유;
  pnix-agnostic이다.
- **pnix-hy**는 이 호스트의 pnix RUNTIME을 소유: pnix parse/evaluate,
  `hy-meta`에 가속 연결, 브릿지 제공(effect/capability adapter +
  canonical-result emission).

여기서 타협 불가: **meta first, never cram** — `hy-meta` 기반보다 앞서 이
호스트 제품 표면을 키우지 않는다. **Non-regression** — 이 repo 게이트를
green 유지. 이 repo 자체 `SCOPE_LOCK.md`(있으면)가 로컬 범위를 지배한다.

## 이중 축 + 호스트 라이브러리 (혼동 금지)

정식 monorepo 문서: [`../HOST_DEV_ENV.md`](../HOST_DEV_ENV.md).

| 축 | 진입점 | 역할 |
|------|-------|------|
| **host-main** | `pnix-hy-python` / `pnix-hy-hy` (bare `python`/`hy`) | `PYTHONPATH` → `pnix_hy` |
| **pnix-main** | `pnix-hy-pnix` | pnix REPL / `.px` 평가 |
| **library** | installable `pnix_hy`; `PNIX_HY_HOME` / `PNIX_HY_LIBRARY` | 호스트 바인딩 Python 패키지 |
| **meta** | `hy-meta` | pnix-agnostic |

호스트 언어 `.px` import: `import pnix_hy as ph; ph.eval_file("x.px")` (`run_px`).  
nix overlay에서 `pkgs.python311`을 전역 오버라이드하지 말 것 — PATH join만.  
HM: `~/dot-nix/dev/py`.
