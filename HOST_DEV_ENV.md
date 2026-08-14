# 호스트 개발 환경 — 이중 축 (정본)

**대상:** 사람, Claude/Codex 세션, `~/dot-nix` 또는 호스트 flake를 연결하는 모든 이.
세 번째 명명 체계를 발명하기 전에 이 문서를 읽을 것.

**최종 갱신:** 2026-08-14 (1일차 체크리스트 + 잔여 컷 닫힘)  
**HM 미러:** `~/dot-nix/dev/PNIX-HOSTS.md` (PATH 패키지, ShellCheck 규칙)  
**상태:** 이중 축 + host-import + **로컬** 라이브러리 피드는 일상 사용에
**충분히 닫혀 있음**. 선택 제품 트랙: [HOST_ENV_P2_P3.md](HOST_ENV_P2_P3.md).

---

## 1일차 체크리스트 (개발자 / 에이전트)

순서대로 진행. **로컬/개인 피드** 우선 — Maven/npm/crates.io/nuget.org 없음.

| 단계 | 명령 / 동작 | 기대 |
|------|-------------|------|
| 0 | 이 문서 + [HOST_IMPORT.md](HOST_IMPORT.md) 읽기 | 이중 축 명확 |
| 1 | HM 또는 호스트 flake: PATH에 library env (`pnix-*-library` / `*-refs`) | `PNIX_*` / `PYTHONPATH` / `NODE_PATH` 설정 |
| 2 | 체크아웃 스모크 (전체 제품 게이트 없음): | |
| 2a | `./bin/host-import-examples-smoke` | 데모 → `3` (도구 없으면 스킵) |
| 2b | `./bin/host-library-smokes` | 로컬 export → 도구 있는 곳에서 `3` |
| 2c | `./bin/host-env-residual-smoke` | 2a+2b 합산 |
| 3 | PATH 주입 스모크 (HM rebuild 후): `./bin/host-import-smoke` | 호스트별 PATH에서 `1+2` eval |
| 4 | 해당 호스트 변경 시에만 제품 게이트: `cd pnix-<h> && nix run .#gate` | 호스트 로컬 |

미니 데모: [examples/host-import/](examples/host-import/).  
CI: `.github/workflows/host-import.yml` (레이아웃 + clj/hy/rs 예제 + library print).

**일상 사용에 닫힘**의 의미: import API, 로컬 export 스크립트, 예제, CI 가드가
존재함. Stage15/N, 공개 레지스트리, 다섯 호스트 공통 `.px` 패키지를 의미하지 **않음**.

---

## 교리 (혼동 금지)

다섯 호스트 각각 **자체 완결**. 오늘 기준 이식 가능한 멀티호스트
`.px` 바이트코드 패키지는 **없음**. 호스트가 만드는 “라이브러리”는 *그*
호스트만을 위한 **호스트 언어 라이브러리**다.

| 패턴 | 방향 | 의미 |
|------|------|------|
| `pnix-<host>-pnix` | **pnix-main** | 이 호스트에서 **pnix** (`.px`) 평가 / REPL |
| `pnix-<host>-<lang>` | **host-main** | 이 호스트의 pnix 제품 라이브러리를 **로드**하는 일상 **호스트 언어** 툴체인 |
| `pnix-<host>-<host>` | **host-main** (축약) | lang 이름이 host id와 같을 때 위와 동일 (`-clj`, `-cljs`, `-hy`, `-rs`, `-clr`) |
| `pnix-<host>-library` / `*-refs` | 어느 쪽이든 | 제품 라이브러리 + env 계약 materialize 또는 출력 |
| `<host>-meta` | 메커니즘 | 호스트 언어 셀프호스트 / 스테이지; **pnix 비의존** |

완전한 호스트 생태계 설정에는 **두 방향 모두** 있어야 함:

1. **Host-main** — 개발자가 Clojure / Node / Python / Rust / C#에 거주; `require` /
   `import` / 링크 / MSBuild가 pnix 제품을 보도록 env와 함께 도구 기동.
2. **Pnix-main** — 개발자가 `.px`에 거주; `pnix-<host>-pnix` (및 eval CLI) 동작;
   호스트 도구는 PATH에 유지; 역 interop용 library env 유지.

역사적 **pnix-meta** “모든 호스트용 하나의 이식 가능 `.px` 코어”는 **이후**
트랙. PATH 래퍼나 호스트 라이브러리만으로 닫혔다고 주장하지 말 것.

---

## 매트릭스 (제품 + HM 이름)

| 호스트 | Host-main 진입 | 제품 라이브러리 | Env 계약 (제품 / HM) | Pnix-main | Meta |
|--------|----------------|-----------------|----------------------|-----------|------|
| **clj** | `pnix-clj-clj` (bare `clojure`) | classpath 위 `pnix-clj` 소스 (`-Sdeps` local/root) | `PNIX_CLJ_ROOT`, `PNIX_CLJ_LIBRARY` | `pnix-clj-pnix` | `clj-meta` |
| **cljs** | `clojurescript` → `pnix-cljs`; `node` via host join | `share/pnix-cljs` JS 모듈 | `NODE_PATH`, `PNIX_CLJS_SHARE`, `PNIX_CLJS_LIBRARY`, `PNIX_CLJS` | `pnix-cljs-pnix` | `cljs-meta` / `pnix-cljs-cljs` |
| **hy** | `pnix-hy-python` / `pnix-hy-hy` (bare `python`/`hy`) | 설치 가능 `pnix_hy` 패키지 | `PYTHONPATH`, `PNIX_HY_HOME`, `PNIX_HY_LIBRARY`, `PNIX_HY_PYTHON` | `pnix-hy-pnix` | `hy-meta` |
| **rs** | `pnix-rs-rs` (bare `cargo`/`rustc`) | `pnix-rs-library` (`libpnix_rs.*` + `pnix_rs.h`) | `PNIX_RS_LIB_DIR`, `PNIX_RS_INCLUDE_DIR`, `PNIX_RS_RUNTIME` | `pnix-rs-pnix` / `px-eval` | `rs-meta` |
| **clr** | `pnix-clr-clr` / `clojure-clr`; C# `pnix-clr-cs` | `export-pnix-clr-library` → `Pnix.Clr` + guest AOT DLL + MSBuild props | `PNIX_CLR`, `PNIX_CLR_ROOT`, `PNIX_CLR_ARTIFACT`, `PNIX_CLR_LIBRARY` | `pnix-clr-pnix` | `clr-meta` |

Flake apps (각 호스트 디렉터리 안):

```text
nix run .#pnix-<host>          # 런타임 CLI
nix run .#pnix-<host>-pnix     # pnix-main REPL
nix run .#gate
nix run .#pnix-<host>-library  # 라이브러리 경로 printer (전 호스트)
nix run .#pnix-<host>-refs     # library printer 별칭 (정의된 곳)
# rs 추가: packages.pnix-rs-library = 실제 rlib/dylib/header
# clr 추가: pnix-clr-library가 디스크에 export 트리 materialize
```

[HOST_IMPORT.md](HOST_IMPORT.md) § 패키징 티어 및 **개인/로컬 라이브러리
export** (공개 레지스트리 게시 없음) 참고. 합산:
`./bin/host-library-smokes`.

임포트 쿡북: **[HOST_IMPORT.md](HOST_IMPORT.md)**.  
PATH 스모크: **`./bin/host-import-smoke`**.

HM은 **`writeShellScriptBin`** 으로 셸 러너를 재구현 (darwin x86_64에서
raw `writeShellApplication` → ShellCheck/GHC 금지).
`~/dot-nix/dev/PNIX-HOSTS.md` 참고.

---

## 호스트 언어에서 `.px` 임포트 (API 치트시트)

라이브러리는 **호스트 바인딩**. 다음 진입점 우선:

| 호스트 | 호스트 언어에서 |
|--------|-----------------|
| clj | `(pnix-clj.core/eval-file "x.px")` — 공개 API: [docs/HOST_IMPORT.md](pnix-clj/pnix-clj/docs/HOST_IMPORT.md) |
| cljs | `require('@plumpmath/pnix-cljs')` → `evalFile` / `evalSource` ([HOST_IMPORT.md](pnix-cljs/HOST_IMPORT.md)) |
| hy | `import pnix_hy as ph; ph.eval_file("x.px")` (= `run_px`) |
| rs | `pnix_rs::eval_file("x.px")` / C ABI `pnix_rs_eval` |
| clr | `Pnix.Clr.Eval.File("x.px")` 또는 `pnix-clr x.px` (JSON CLI 결과) |

### CLR 라이브러리 레이아웃 (제품)

```bash
cd pnix-clr
./bin/export-pnix-clr-library   # → pnix-clr/target/pnix-clr-library/
# lib/net{8,10}.0/Pnix.Clr.dll
# lib/net10.0/runtime-artifact/pnix_clr.*.clj.dll + manifest.json
# build/Pnix.Clr.props|.targets
# share/pnix-clr/refs.env
```

C#:

```csharp
using Pnix.Clr;
var r = Eval.File("hello.px").EnsureDone();
```

Guest AOT `*.clj.dll`은 **ClojureCLR 어셈블리**이며 일반 C# API가 아님.
`PnixClrImportGuestDlls`는 `CLOJURE_LOAD_PATH` / assembly load로 로드하는
CLR 호스트에만 켜고 — 평범한 net8 앱 코드에는 쓰지 말 것.

소스: `pnix-clr/csharp/Pnix.Clr/`, flake apps `pnix-clr-library`, `pnix-clr-refs`.

### RS 라이브러리 레이아웃 (제품)

```text
packages.pnix-rs-library → $out/lib/libpnix_rs.* + $out/include/pnix_rs.h
```

### CLJS 라이브러리 레이아웃 (제품)

```text
packages.pnix-cljs →
  $out/share/pnix-cljs/                         # flat: pnix-cljs-module.js
  $out/lib/node_modules/@plumpmath/pnix-cljs/   # scoped require (권장)
NODE_PATH에 lib/node_modules 및/또는 share/ 포함 필요
# require('@plumpmath/pnix-cljs')  또는  require('pnix-cljs-module.js')
```

상세: [pnix-cljs/HOST_IMPORT.md](pnix-cljs/HOST_IMPORT.md).

### HY 라이브러리 레이아웃 (제품)

```text
packages.pnix-hy → site-packages/pnix_hy  (PYTHONPATH)
```

### CLJ 라이브러리 레이아웃 (제품)

```text
pnix-clj/ 소스 via -Sdeps {:deps {pnix/pnix-clj {:local/root …}}}
```

---

## 생태계 체크리스트 (호스트마다 전부 있어야 함)

개발자용으로 호스트가 “설정됨”이려면:

1. 제품 라이브러리를 주입하는 **Host-main** 래퍼  
2. **`pnix-<host>-library` 및/또는 `*-refs`** (또는 동등 flake 패키지)  
3. **`pnix-<host>-pnix`** (pnix-main)  
4. flake가 정의한 호스트 언어 REPL/툴체인 변형  
5. **`<host>-meta`**  
6. 호스트에 있을 때 **Gate**  

dot-nix는 `dev/{clj,cljs,py,rs,cs}/` 아래 (1)–(6)을 구현. PATH만으로 안 되는
제품 작업은 호스트 트리(이 monorepo)에 둔다.

---

## 에이전트가 하지 말아야 할 것

- 호스트 자체 게이트/문서가 말하지 않으면 **Stage15/N**, common-compiler,
  five-host gate 주장 금지 (특히 clr / cljs).
- 호스트 `*.dll` / rlib / JS share를 **공통** `.px` 패키지로 취급 금지.
- home-manager on darwin에 `writeShellApplication` 끌어오기 금지 (ShellCheck/GHC).
- Hy 주입을 위해 `pkgs.python311` / 전체 `python3Packages` 전역 오버라이드 금지
  (nixpkgs 빌더 깨짐). PATH join만.
- 하나의 `buildEnv`에 전체 `pnix-rs` + `pnix-rs-library` 혼합 금지 (dylib 충돌).
- Rhino **sdk_8** 과 pnix-clr **sdk_10** 혼동 금지.

---

## 호스트별 심화 문서

| 호스트 | 여기서 시작 |
|--------|-------------|
| clj | `pnix-clj/CLAUDE.md`, `pnix-clj/README.md`, `pnix-clj/pnix-clj/todo.md` (host import) |
| cljs | `pnix-cljs/CLAUDE.md`, `pnix-cljs/README.md`, `pnix-cljs/cljs-meta/todo.md` |
| hy | `pnix-hy/CLAUDE.md`, `pnix-hy/README.md`, `pnix-hy/pnix-hy/todo.md` |
| rs | `pnix-rs/CLAUDE.md`, `pnix-rs/README.md`, `pnix-rs/pnix-rs/todo.md` |
| clr | `pnix-clr/CLAUDE.md`, `pnix-clr/README.md`, `pnix-clr/csharp/Pnix.Clr/README.md`, `pnix-clr/clr-meta/todo.md` |

HM 패키징 진실: `~/dot-nix/dev/PNIX-HOSTS.md`.

---

## Hy 이름 충돌 (중요)

| 이름 | 의미 |
|------|------|
| flake `.#pnix-hy-hy` | `pnix-hy --repl hy` (소스 트리 / proof Python) |
| HM bin `pnix-hy-hy` | `pnix_hy`용 `PYTHONPATH`가 있는 bare **Hy 인터프리터** |

이름은 같지만 **같은 프로그램이 아님**. 문서에서는
“`pnix-hy-host` 경유 bare `hy`” vs “flake app `pnix-hy-hy` REPL 모드”로 구분.

## 스모크 (방향)

Monorepo 합산기 (권장):

```bash
./bin/host-import-examples-smoke   # examples/host-import/* 미니 데모
./bin/host-library-smokes          # 로컬 export 피드
./bin/host-env-residual-smoke      # 위 둘 모두
./bin/host-import-smoke            # HM 이후 PATH 도구 (PATH 없으면 스킵)
```

임시 host-main / pnix-main:

```bash
# host-main library inject (HM/library env 필요)
clojure -e '(+ 1 2)'                    # clj
python3 -c 'import pnix_hy'             # hy (PYTHONPATH 포함)
# cargo/rustc는 pnix-rs-rs가 PATH에 있으면 PNIX_RS_* 보유
# node는 pnix-cljs-host가 PATH에 있으면 NODE_PATH 보유

# pnix-main (해당 호스트 flake 루트에서)
# nix run .#pnix-clj-pnix
# nix run .#pnix-hy-pnix
# nix run .#pnix-rs-pnix
# nix run .#pnix-cljs-pnix
# nix run .#pnix-clr-pnix

# clr 로컬 라이브러리
cd pnix-clr && ./bin/export-pnix-clr-library && cat pnix-clr/target/pnix-clr-library/share/pnix-clr/refs.env
```
