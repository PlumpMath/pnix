# pnix

**pnix**는 Nix와 유사한 표현 언어로, 다섯 호스트 언어에서 각각 독립적으로 구현된다.
각 구현은 자체 완결 저장소다: 공유 런타임, 공유 코퍼스, 형제 디렉터리 의존 없이
자체 빌드·실행·게이트한다.

## 다섯 호스트

모든 호스트는 쌍이다.
`*-meta` 절반은 해당 호스트 언어의 셀프호스트 증명과 네이티브 가속을 소유하며 pnix에 구애받지 않는다.
`pnix-*` 절반은 pnix 런타임 — 파싱, 평가, 호스트 값 브리지 — 을 소유한다.

| 디렉터리 | 호스트 언어 | 쌍 |
|---|---|---|
| [`pnix-clj/`](pnix-clj) | Clojure / JVM | `clj-meta` + `pnix-clj` |
| [`pnix-cljs/`](pnix-cljs) | ClojureScript / Node | `cljs-meta` + `pnix-cljs` |
| [`pnix-clr/`](pnix-clr) | ClojureCLR / .NET | `clr-meta` + `pnix-clr` |
| [`pnix-hy/`](pnix-hy) | Hy / Python | `hy-meta` + `pnix-hy` |
| [`pnix-rs/`](pnix-rs) | Rust | `rs-meta` + `pnix-rs` |

성숙도는 호스트마다 다르다.
`pnix-clj`가 가장 완성도가 높다. `pnix-clr`와 `pnix-cljs`는 명시적으로 실험적이다.
각 호스트의 `README.md`와 `CLAUDE.md`가 무엇을 주장하고 무엇을 주장하지 않는지 밝힌다 —
패리티를 가정하기 전에 읽을 것.

## 호스트 실행

모든 호스트는 Nix flake다. 이 최상위에서는 아무것도 빌드하지 않는다. 관심 있는 호스트로 들어간다.

다섯 호스트는 같은 이름으로 같은 진입점을 노출하므로, 하나에서 배운 내용이 다른 호스트로도 이어진다.
`<host>`는 `clj`, `cljs`, `clr`, `hy`, `rs` 중 하나:

| App | 내용 |
|---|---|
| `.#pnix-<host>` | pnix 런타임 CLI |
| `.#pnix-<host>-pnix` | 대화형 pnix REPL (**pnix-main**) |
| `.#pnix-<host>-<lang>` | 호스트 언어 툴체인 / REPL (**host-main**), 있는 경우 |
| `.#pnix-<host>-library` | 호스트 제품 라이브러리 export (해당 호스트가 제공하는 경우, 예: clr/rs) |
| `.#<host>-meta` | 해당 호스트 언어 메커니즘 CLI (`clj-meta`, `rs-meta`, …) |
| `.#gate` | 해당 호스트 full gate |
| `.#default` | `.#pnix-<host>`와 동일 |

### 이중 축 개발 (host-main ↔ pnix-main)

각 호스트는 **양쪽** 방향을 모두 지원해야 한다. 라이브러리는 **호스트 바인딩**
(공유 멀티호스트 `.px` 패키지 아님). 전체 매트릭스, env 계약, import API:

→ **[HOST_DEV_ENV.md](HOST_DEV_ENV.md)** (정본 이중 축 + **1일차 체크리스트**)  
→ **[HOST_IMPORT.md](HOST_IMPORT.md)** (호스트별 import 쿡북 + 패키징 티어)  
→ **[HOST_ENV_P2_P3.md](HOST_ENV_P2_P3.md)** (선택 P2/P3; host-env 컷 닫힘)  
→ **[examples/host-import/](examples/host-import/)** (미니 host-main 데모)  
→ **`./bin/host-import-examples-smoke`** · **`./bin/host-library-smokes`** ·  
  **`./bin/host-env-residual-smoke`** · **`./bin/host-import-smoke`** (PATH / HM)

| 방향 | 이름 패턴 | 의도 |
|------|-----------|------|
| **pnix-main** | `pnix-<host>-pnix` | `.px`에 거주; 이 호스트에서 eval/REPL |
| **host-main** | `pnix-<host>-<lang>` / `pnix-<host>-<host>` | Clojure/Node/Python/Rust/C#에 거주; 이 호스트의 pnix **라이브러리** 로드 |

HM 배선 (PATH, `writeShellScriptBin`, ShellCheck 없음): `~/dot-nix/dev/PNIX-HOSTS.md`.

따라서 pnix REPL은 JVM에서 `.#pnix-clj-pnix`, Rust에서 `.#pnix-rs-pnix`이고, 게이트는 어디서나 `.#gate`다:

```bash
cd pnix-rs && nix run .#pnix-rs-pnix
```

```bash
cd pnix-clj && nix run .#gate
```

개별 호스트는 자체 추가 기능을 둔다 — `nix run .#pnix-rs-px-eval`, `.#substrate-check`, `.#tower`, `.#deps-lock`, `.#pnix-clr-library`.
호스트 안에서 `nix flake show`로 노출 전체를 확인.

## NixOS / Home Manager 설정에서 pnix를 "오버라이드"로 설치하기

이 저장소에는 **최상위 통합 `flake.nix`가 없다** — `pnix-clj/`, `pnix-cljs/`,
`pnix-clr/`, `pnix-hy/`, `pnix-rs/` 각각이 독립된 flake다. 아래는 시스템
설정(NixOS/nix-darwin/Home Manager, flake 기반)에서 이 호스트들을 **기존
언어 커맨드(`clojure`, `python`, `cargo`, `node`, …) 자체를 대체하거나
PATH 우선순위로 끼워 넣는** 방식으로 통합하는, 실전에서 쓰이는 패턴이다.

### 1) 각 호스트를 flake input으로 추가

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    # 필요한 호스트만 추가 (clj / cljs / clr / hy / rs). dir= 로 monorepo
    # 안의 호스트 서브디렉터리를 골라낸다.
    pnix-clj.url  = "github:YOU/pnix?dir=pnix-clj";
    pnix-hy.url   = "github:YOU/pnix?dir=pnix-hy";
    pnix-rs.url   = "github:YOU/pnix?dir=pnix-rs";
    pnix-cljs.url = "github:YOU/pnix?dir=pnix-cljs";
    pnix-clr.url  = "github:YOU/pnix?dir=pnix-clr";
  };

  outputs = { self, nixpkgs, pnix-clj, pnix-hy, pnix-rs, ... }: {
    # 아래 절 참고
  };
}
```

**로컬에서 편집 중인 체크아웃을 커밋/푸시 없이 바로 반영**하려면 (다른
세션이 `~/pnix`를 계속 고치고 있을 때 유용) `flake.lock`을 로컬 경로로
오버라이드한다:

```bash
cd ~/your-system-flake-dir
nix flake lock \
  --override-input pnix-clj  path:$HOME/pnix/pnix-clj \
  --override-input pnix-hy   path:$HOME/pnix/pnix-hy \
  --override-input pnix-rs   path:$HOME/pnix/pnix-rs \
  --override-input pnix-cljs path:$HOME/pnix/pnix-cljs \
  --override-input pnix-clr  path:$HOME/pnix/pnix-clr
```

`path:` input은 **git이 추적하는 파일만** 본다 — 새 파일을 추가했다면 평가
전에 `git add` 해야 flake가 그 파일을 본다.

### 2) 두 가지 "오버라이드" 방식 — 언제 어느 쪽인가

호스트 CLI를 시스템 전역에 배선하는 방법은 둘로 나뉜다. **어느 쪽을 쓸지는
그 언어 패키지가 nixpkgs의 다른 빌드에서 내부적으로 쓰이는지**로 정해진다.

| 방식 | 언제 | 예 |
|---|---|---|
| **(A) nixpkgs overlay로 완전 교체** | 그 언어 실행파일이 nixpkgs 다른 패키지의 빌드 입력으로 안 쓰일 때 (Clojure CLI가 대표적) | `clojure` 자체를 pnix-aware 래퍼로 치환 |
| **(B) PATH 우선순위 래퍼(join)만 추가** | 그 언어 패키지 전체(`python3Packages`, `rustc` 등)가 nixpkgs 빌드 시스템 안에서 널리 쓰일 때 | `python`/`hy`, `cargo`/`rustc`, `node` |

**(B)가 기본값**이다 — `python3`/`rustc` 같은 패키지 그래프를 통째로
overlay로 바꿔치기하면 그 패키지들에 의존하는 무관한 nixpkgs 빌드가 깨질
수 있다(예: `cargo-auditable`, f-string 빌더). (A)는 그 언어의 **개발자용
CLI 하나만** 안전하게 완전 대체할 수 있다고 확신할 때만 쓴다.

### 2a) (A) overlay로 완전 교체 — Clojure 예시

`clojure` 실행 시 항상 `pnix-clj`가 로컬 의존성으로 클래스패스에 잡히도록
만드는 overlay:

```nix
# overlay.nix
final: prev:
let
  jdk = prev.jdk21;
  stockClojure = prev.clojure;
  pnixRoot = "${inputs.pnix-clj}/pnix-clj";  # flake input을 그대로 소스 경로로 사용
  path = prev.lib.makeBinPath [ stockClojure jdk prev.git prev.rlwrap ];

  pnixClojureCommand = prev.writeShellScriptBin "pnix-clj-clj" ''
    export PATH="${path}:$PATH"
    export JAVA_HOME="${jdk}"
    export PNIX_CLJ_ROOT="${pnixRoot}"
    pnix_deps='{:deps {pnix/pnix-clj {:local/root "${pnixRoot}"}}}'
    inject_pnix=1
    for arg in "$@"; do
      case "$arg" in -Sdeps|-Sdeps=*) inject_pnix=0 ;; esac
    done
    if [ "$inject_pnix" -eq 1 ]; then
      exec ${stockClojure}/bin/clojure -Sdeps "$pnix_deps" "$@"
    else
      exec ${stockClojure}/bin/clojure "$@"
    fi
  '';
  pnixClojure = prev.symlinkJoin {
    name = "pnix-clj-clj";
    paths = [ pnixClojureCommand ];
    postBuild = ''ln -sfn pnix-clj-clj "$out/bin/clojure"'';
    meta.mainProgram = "pnix-clj-clj";
  };
in {
  clojure-stock = stockClojure;  # 원본 clojure는 이 이름으로 남겨 둔다
  clojure = pnixClojure;         # nixpkgs.clojure 자체를 교체 (진짜 "오버라이드")
}
```

시스템 설정에서 이 overlay를 켜기만 하면 된다:

```nix
{ nixpkgs.overlays = [ (import ./overlay.nix) ]; }
```

`pkgs.clojure`를 쓰는 자리는 이제 전부 이 래퍼를 받는다 — jar/버전 정보가
필요하면 (래퍼가 아니라) `pkgs.clojure-stock`을 쓴다.

### 2b) (B) PATH 우선순위 래퍼 — Python/Hy 예시

`python3Packages`/`rustc` 전체 그래프는 그대로 두고, `home.packages`(또는
`environment.systemPackages`)에 **더 앞선 우선순위**로 얹는 얇은 래퍼만
추가한다:

```nix
{ pkgs, pnix-hy, ... }:
let
  system = pkgs.stdenv.hostPlatform.system;
  sitePackages = "${pnix-hy}/pnix-hy";  # pnix_hy 소스 루트
in {
  home.packages = [
    (pkgs.writeShellScriptBin "python" ''
      export PYTHONPATH="${sitePackages}''${PYTHONPATH:+:$PYTHONPATH}"
      exec ${pkgs.python311}/bin/python3 "$@"
    '')
  ];
}
```

`home.packages`는 nixpkgs의 `python3`/`python3Packages` 자체를 안 건드리고
**이 사용자 세션의 PATH에서만** `python`을 이 래퍼로 가린다 — 다른 파생물이
빌드 중에 `pkgs.python3`을 참조해도 원본 그대로 쓴다. 같은 패턴을
`cargo`/`rustc`(rs), `node`/`clojurescript`(cljs), `clojure-clr`(clr)에도
반복한다 — 언어별 세부 env var(`PNIX_HY_HOME`, `PNIX_RS_LIB_DIR`,
`NODE_PATH`, `PNIX_CLR_ROOT` 등)는 각 호스트의 `README.md`/`HOST_IMPORT.md`
참고.

### 3) 공통 함정

- **`writeShellApplication`을 쓰지 말 것** — `writeShellScriptBin`만 쓴다.
  전자는 항상 `shellcheck-minimal`(→ Haskell/GHC)을 끌고 들어와 무관한
  플랫폼(특히 x86_64-darwin)에서 재빌드 시간이 급증하거나 substitute가
  깨진다.
- **`symlinkJoin`으로 host join과 그 구성원을 동시에 `home.packages`에
  넣지 말 것** — 예를 들어 `pnix-hy-host`(join)와 flake의 `hy` 패키지를
  같이 넣으면 같은 `bin/` 파일 이름이 충돌한다. join **하나만** 넣는다.
- **패키지 전체 그래프(`python3Packages`, `rustc` 등)를 overlay로 바꿔치기
  하지 말 것** — 2b)의 PATH 방식을 쓴다. 예외는 그 실행파일이 다른
  nixpkgs 빌드의 입력으로 전혀 안 쓰인다고 확신할 때뿐이다(Clojure CLI가
  그런 경우).

### 4) 그냥 개발만 하려면

시스템에 설치할 필요 없이 이 체크아웃 안에서 바로 작업한다면 위 오버라이드가
전혀 필요 없다 — "호스트 실행" 절에 있는 대로 해당 호스트 디렉터리에서
`nix develop`(devShell) / `nix run .#...` 를 쓰면 된다.

## 업스트림 substrate

호스트 언어 컴파일러는 고정된 업스트림 릴리스 패키지로 소비하며, 이 트리에 벤더하지 않는다:

- ClojureScript — Maven의 `org.clojure/clojurescript`, `pnix-cljs/deps-lock.json`에 pin
- ClojureCLR — `Clojure` NuGet 패키지, `pnix-clr/clr-bootstrap/`에 pin
- Hy — 업스트림 `hylang/hy` 태그, `pnix-hy/flake.lock`에 pin

빌드 산출물 (`work/`, `dist/`, `target/`)은 각 호스트 게이트가 재생성하며 추적하지 않는다.

## 상태

활발히 개발 중인 연구 코드다.
인터페이스는 바뀌며, 호스트는 같은 입력에서 동일한 동작을 보장하지 않는다.

## 라이선스

MIT — [LICENSE](LICENSE) 참고.
업스트림 호스트 언어 컴파일러는 빌드 시 각자 레지스트리에서 가져오며 자체 라이선스를 유지한다.
