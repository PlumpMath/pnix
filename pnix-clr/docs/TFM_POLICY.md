# TFM / SDK 정책 (pnix-clr vs Rhino)

**날짜:** 2026-08-14

| 경로 | TFM / SDK | 위치 |
|------|-----------|------|
| **pnix-clr 제품** (AOT guest, bootstrap, gates) | **net10.0** / `dotnet-sdk_10` | `pnix-clr/`, HM `dev/cs` runners |
| **Pnix.Clr managed Eval API** | multi-target **net8.0 + net10.0** | `csharp/Pnix.Clr/` — Rhino 측 net8이 Eval을 Reference 가능 |
| **Rhino / Grasshopper 플러그인** (Kimchi) | **net8.0** / sdk_8 cask 또는 pin | `dot-nix` Rhino 플러그인 경로 — pnix-clr AOT **아님** |

## 규칙

1. Rhino 플러그인 빌드를 조용히 sdk_10 / net10에 연결하지 **말 것**.
2. pnix-clr runtime-artifact를 sdk_8로 빌드하지 **말 것**.
3. `Pnix.Clr.Eval`만 필요한 host-main C#은 net8 사용 가능 (`pnix-clr`로의
   프로세스 스폰은 내부적으로 net10 호스트 런타임 사용).
4. 멀티-ns **ClojureCLR** 프로젝트 템플릿은 bootstrap **net10**만
   (`examples/clojure-clr-project`).

참고: monorepo `HOST_DEV_ENV.md`, `docs/CLOJURE_CLR_ADMITTED_SURFACE.md`.
