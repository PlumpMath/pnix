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

## 로컬 체크아웃을 다른 flake에서 오버라이드해 설치하기

이 저장소에는 **최상위 통합 `flake.nix`가 없다** — `pnix-clj/`, `pnix-cljs/`,
`pnix-clr/`, `pnix-hy/`, `pnix-rs/` 각각이 독립된 flake다. 다른 시스템 설정
(NixOS, Home Manager, 또는 그냥 개인 flake)에서 이 로컬 체크아웃을 소비하려면,
관심 있는 호스트 **디렉터리**를 flake input으로 가리키면 된다.

### 1) flake input으로 로컬 경로 지정

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    # 로컬 체크아웃을 직접 가리킨다 — 커밋/푸시 없이 편집이 바로 반영된다.
    # 경로는 이 monorepo를 clone한 실제 위치로 바꿀 것 (호스트 서브디렉터리까지).
    pnix-rs.url = "path:/home/YOU/pnix/pnix-rs";
    pnix-clj.url = "path:/home/YOU/pnix/pnix-clj";
    # 필요한 호스트만 추가하면 된다 (clj / cljs / clr / hy / rs).

    # 재현 가능한(로컬 체크아웃에 안 묶인) 설치가 필요하면 대신 이렇게:
    # pnix-rs.url = "github:PlumpMath/pnix?dir=pnix-rs";
  };

  outputs = { self, nixpkgs, pnix-rs, pnix-clj, ... }: {
    # 아래 참고
  };
}
```

`path:` input은 git이 추적하는 파일만 본다 — 새 파일을 추가했다면 평가 전에
`git add` 해야 flake가 그 파일을 볼 수 있다 (Nix flake의 일반적인 동작).

### 2) NixOS 시스템 설정에서 패키지로 소비

```nix
{ pnix-rs, ... }:
let system = "x86_64-linux"; in
{
  environment.systemPackages = [
    pnix-rs.packages.${system}.pnix-rs   # 런타임 CLI
    pnix-rs.packages.${system}.rs-meta   # 원하면 meta 절반도
  ];
}
```

### 3) Home Manager에서 소비

```nix
{ pnix-rs, ... }:
let system = "x86_64-linux"; in
{
  home.packages = [
    pnix-rs.packages.${system}.pnix-rs
  ];
}
```

`nix run`/`writeShellScriptBin`으로 PATH에 여러 호스트를 동시에 배선하는
더 정교한 패턴(호스트별 CLI 이름 충돌 회피, devShell 자동 진입 등)은 각자의
Home Manager 구성 스타일에 맞춰 직접 구성한다 — 이 README는 flake input
오버라이드 자체까지만 다룬다.

### 4) 커밋 없이 즉석으로 오버라이드 (CLI 한 줄)

기존 flake(예: 이미 `pnix-rs`를 `github:PlumpMath/pnix?dir=pnix-rs`로 고정해
둔 시스템 설정)를 건드리지 않고, 이번만 로컬 체크아웃으로 빌드/실행해보고 싶다면
(아래는 **당신의 시스템/HM flake 디렉터리**에서 실행):

```bash
# 이 pnix 저장소 자체를 바로 실행/빌드 — 오버라이드할 다른 flake가 필요 없다
cd pnix-rs && nix run .

# 다른 flake(당신의 NixOS/HM 설정)가 이미 pnix-rs를 input으로 갖고 있고,
# 이번만 로컬 체크아웃으로 바꿔치기해서 빌드하고 싶을 때
nix build --override-input pnix-rs path:/home/YOU/pnix/pnix-rs \
  .#nixosConfigurations.myhost.config.system.build.toplevel
```

### 5) 그냥 개발만 하려면

시스템에 설치할 필요 없이 이 체크아웃 안에서 바로 작업한다면 오버라이드가
전혀 필요 없다 — 위 "호스트 실행" 절에 있는 대로 해당 호스트 디렉터리에서
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
