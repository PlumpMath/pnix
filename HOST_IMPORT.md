# 호스트 언어 임포트 쿡북 (색인)

이중 축 교리: **[HOST_DEV_ENV.md](HOST_DEV_ENV.md)**.

호스트별 상세:

| 호스트 | 쿡북 |
|--------|------|
| clj | [pnix-clj/pnix-clj/docs/IMPLEMENTATION.md](pnix-clj/pnix-clj/docs/IMPLEMENTATION.md) §11 |
| cljs | [pnix-cljs/pnix-cljs/docs/IMPLEMENTATION.md](pnix-cljs/pnix-cljs/docs/IMPLEMENTATION.md) §3 |
| hy | 본 문서 § hy (패키지 자체가 라이브러리) |
| rs | 본 문서 § rs + [pnix-rs IMPLEMENTATION.md](pnix-rs/pnix-rs/docs/IMPLEMENTATION.md) §7 |
| clr | [pnix-clr/csharp/Pnix.Clr/README.md](pnix-clr/csharp/Pnix.Clr/README.md) |

HM 경로 헬퍼: `pnix-<host>-library` / `pnix-<host>-refs` (`~/dot-nix/dev/PNIX-HOSTS.md` 참고).

**P2/P3 로드맵:** [HOST_ENV_P2_P3.md](HOST_ENV_P2_P3.md) (host-env 잔여 **충분히 닫힘**)  
**1일차 체크리스트:** [HOST_DEV_ENV.md](HOST_DEV_ENV.md) § Day-1  
**미니 예제:** [examples/host-import/](examples/host-import/)  

| 스모크 | 시기 |
|--------|------|
| `./bin/host-import-examples-smoke` | 체크아웃 데모 (도구 없으면 스킵) |
| `./bin/host-library-smokes` | 로컬 export 피드 |
| `./bin/host-env-residual-smoke` | 예제 + 라이브러리 |
| `./bin/host-import-smoke` | HM 이후: PATH에 이미 도구가 있을 때 |

---

## 개인 / 로컬 라이브러리 export (공개 레지스트리 아님)

소유자 정책: Maven Central / npm / crates.io / nuget.org **게시 안 함**.
각 호스트에 **로컬 피드** materializer + 스모크가 있음:

| 호스트 | Export | 스모크 | 소비 측 |
|--------|--------|--------|---------|
| **clj** | `pnix-clj/pnix-clj/bin/export-pnix-clj-library` | `pnix-clj-library-smoke` | `{:local/root "…/pnix-clj"}` |
| **cljs** | `pnix-cljs/bin/export-pnix-cljs-library` | `pnix-cljs-library-smoke` | `NODE_PATH=…/lib/node_modules:…/share` |
| **hy** | `pnix-hy/pnix-hy/bin/export-pnix-hy-library` | `pnix-hy-library-smoke` | `PYTHONPATH=…/site` |
| **rs** | `pnix-rs/pnix-rs/bin/export-pnix-rs-library` | `pnix-rs-library-smoke` | path-dep 또는 `-L lib -I include` |
| **clr** | `pnix-clr/bin/export-pnix-clr-library` (+ pack) | `pnix-clr-library-smoke` | `PNIX_CLR_LIBRARY` / 로컬 nupkg 디렉터리 |

```bash
./bin/host-library-smokes   # clj hy rs cljs (+ clr는 이미 export된 경우)
```

---

## 라이브러리 패키징 티어 (과대 주장 금지)

| 호스트 | Flake `*-library` / `*-refs` | 라이브러리 본문 | HM 헬퍼 |
|--------|------------------------------|-----------------|---------|
| **clj** | app/printer `pnix-clj-library` | `pnix-clj` 소스 (`-Sdeps` local/root) | `pnix-clj-library` |
| **cljs** | app/printer `pnix-cljs-library` | share/ + `lib/node_modules/@plumpmath/pnix-cljs` | `pnix-cljs-library` |
| **hy** | app/printer `pnix-hy-library` | `packages.pnix-hy` site-packages | `pnix-hy-library` |
| **rs** | **package** `pnix-rs-library` + app `pnix-rs-refs` | rlib/a/dylib + 헤더 | `pnix-rs-refs` |
| **clr** | **export app** `pnix-clr-library` + `pnix-clr-refs` | `Pnix.Clr` + guest AOT + MSBuild props | `pnix-clr-refs` |

```text
nix run .#pnix-clj-library    # 경로 계약 (소스)
nix run .#pnix-hy-library
nix run .#pnix-cljs-library
nix run .#pnix-rs-library     # 실제 임베드 가능 아티팩트
nix run .#pnix-rs-refs
nix run .#pnix-clr-library    # export 트리 materialize
nix run .#pnix-clr-refs
```

---

## hy (Python)

```python
import pnix_hy as ph

ph.eval_source("1 + 2")
ph.eval_file("prog.px")   # run_px 별칭
ph.call_file("library.px", "double", [21])
ph.call_file_json("library.px", "mapDouble", "[[1,2,3]]")
```

공개 최상위 export: `pnix_hy.__all__` 참고 (`eval_source`, `eval_file`,
`run_px`, interop 헬퍼, …). 증명/메타 로더: `load_proof_api()`,
`load_meta_api()`.

선택적 호스트 전용 import 훅 (common-meta 아님):

```python
from pnix_hy import install_pnix_import_hook
# 루트를 설치해 Python import가 호스트 바인딩 .px 모듈을 pnix-hy로 로드하게 함.
# pnix_hy.interop.install_pnix_import_hook docstring 참고.
```

**이름 충돌:** flake app `.#pnix-hy-hy`는 `pnix-hy --repl hy` (소스 트리 필요).
HM PATH bin `pnix-hy-hy`는 **순수 Hy 인터프리터**이며 `pnix_hy`용 `PYTHONPATH`를 가짐.
둘을 동일시하지 말 것.

일상 PATH `python`은 `~/dot-nix/dev/{py,cuda}`의 과학 스택
(`python-with-packages`)이다. 게이트/투영의 Hy pin은 flake `proofPython`
(`PNIX_HY_PYTHON`). flake `packages.pnix-hy-proof-host`는 그 pin +
`pnix_hy` PATH join — HM `pnix-hy-host`와 다른 이름. 섞지 말 것 —
[`HOST_DEV_ENV.md`](HOST_DEV_ENV.md) § Hy 인터프리터 두 갈래.

```bash
python -c 'import pnix_hy as ph; print(ph.eval_file("prog.px"))'
pnix-hy-library
pnix-hy-pnix
```

---

## rs (Rust)

Cargo 패턴: [pnix-rs/pnix-rs/docs/IMPLEMENTATION.md](pnix-rs/pnix-rs/docs/IMPLEMENTATION.md) §7 (옛 `docs/CARGO_HOST_IMPORT.md` 흡수).


```rust
// -L $PNIX_RS_LIB_DIR 로 링크하고 C ABI용 pnix_rs.h 포함 후
// Native:
let s = pnix_rs::eval("1 + 2")?;
let s = pnix_rs::eval_file("prog.px")?;
let answer = pnix_rs::call_file_json("library.px", "double", "[21]")?;
```

```c
#include "pnix_rs.h"
char *out = NULL;
if (pnix_rs_eval("1 + 2", &out) == 0) { /* out 사용 */ pnix_rs_string_free(out); }
```

```bash
pnix-rs-refs
pnix-rs px-eval -c '1 + 2'
```

하나의 `buildEnv`에 전체 `pnix-rs` + `pnix-rs-library`를 넣지 말 것 (dylib 충돌).

---

## clr (C# / ClojureCLR)

`pnix-clr/csharp/Pnix.Clr/README.md` 참고.

```bash
./bin/export-pnix-clr-library
./bin/pack-pnix-clr-nupkg          # 선택: 로컬 nupkg
# MSBuild: csharp/Directory.Build.props.sample
```

```bash
pnix-clr-refs          # 첫 실행 시 library export 가능
pnix-clr -e '1 + 2'
# C#: $PNIX_CLR_LIBRARY/build/Pnix.Clr.props Import 후 Eval.File / Eval.Source
#      Eval.CallFile("library.px", "double", "[21]")
```

---

## 검증된 스모크 (2026-08-14)

또한: monorepo `./bin/host-import-smoke` (PATH 사용).

## 검증된 스모크 로그

| 호스트 | 명령 | 결과 |
|--------|------|------|
| clj | `(pnix-clj.core/eval-file …)` → `:value 3` | ok |
| hy | `pnix_hy.eval_file` → `3` | ok |
| cljs | `require('@plumpmath/pnix-cljs').evalSourceJson('1+2')` → value 3 | ok (user 2026-08-14) |
| rs | `pnix-rs px-eval -c '1 + 2'` → `3` | ok |
| clr | `pnix-clr -e '1 + 2'` → JSON value 3 | ok |
| helpers | `pnix-*-library` / `pnix-rs-refs` / `pnix-clr-refs` | 경로 출력 |

## 공통 pure-data stdlib 호출 경계 (2026-08-22)

다섯 host library는 `.px` attrset이 export한 curried 함수를 호스트에서 직접
호출한다. 공통 분모는 JSON-safe 값(정수/실수/문자열/bool/null/list/attrset)이며,
raw closure나 host-object identity는 경계를 넘지 않는다. 이 경계는 미래
`pnix-meta`의 순수 stdlib 함수를 쓰기 위한 것이고, callback/opaque/effect ABI와
혼동하지 않는다. 실행 정본은 `./bin/production-readiness-gate`다.
