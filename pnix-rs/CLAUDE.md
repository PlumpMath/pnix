# pnix-rs — pnix의 Rust 호스트

당신은 **pnix-rs** 안에 있다. pnix 언어의 한 호스트 임베딩이다. 이 트리는
**자기완결**이다: 형제 저장소에 의존하지 않으며, 다른 호스트와 corpus, gate,
`.px` 코어를 공유하지 않는다. 빌드·게이트에 필요한 모든 것이 여기 있다.

다음 두 정체성을 분리 유지:

- **rs-meta**는 이 호스트 언어의 self-host 증명 + native 가속을 소유;
  pnix-agnostic이다.
- **pnix-rs**는 이 호스트의 pnix RUNTIME을 소유: pnix parse/evaluate,
  `rs-meta`에 가속 연결, 브릿지 제공(effect/capability adapter +
  canonical-result emission).

여기서 타협 불가: **meta first, never cram** — `rs-meta` 기반보다 앞서 이
호스트 제품 표면을 키우지 않는다. **Non-regression** — 이 repo 게이트를
green 유지. 이 repo 자체 `SCOPE_LOCK.md`(있으면)가 로컬 범위를 지배한다.

## 이중 축 + 호스트 라이브러리 (혼동 금지)

정식 monorepo 문서: [`../HOST_DEV_ENV.md`](../HOST_DEV_ENV.md).

| 축 | 진입점 | 역할 |
|------|-------|------|
| **host-main** | `pnix-rs-rs` (bare `cargo`/`rustc`) | `PNIX_RS_LIB_DIR` + link/include env |
| **pnix-main** | `pnix-rs-pnix` / `px-eval` | pnix REPL / one-shot eval |
| **library** | flake `packages.pnix-rs-library` | `libpnix_rs.*` + `include/pnix_rs.h` |
| **meta** | `rs-meta` / `bootstrap` | pnix-agnostic |

호스트 언어 `.px` import: `pnix_rs::eval_file` / C `pnix_rs_eval`.  
한 `buildEnv`에 full `pnix-rs` + `pnix-rs-library`를 함께 넣지 말 것 (dylib clash).  
HM: `~/dot-nix/dev/rs`.
