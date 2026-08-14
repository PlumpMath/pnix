# ClojureCLR admitted surface 인벤토리 (P3.2 step 1)

**날짜:** 2026-08-14  
**상태:** 인벤토리 전용 — 전체 ClojureCLR 교체를 **주장하지 않음**.  
**관련:** 모노레포 `HOST_ENV_P2_P3.md` § P3.2 · `clr-meta/todo.md` § Post host-env

이것은 **`bin/clojure-clr`**, **`bin/clr-meta`**, 그리고 upstream bootstrap이
**오늘** admit하는 것의 정직한 지도다. 이후 작업이 facade를 조용히 늘리지 않고
named profile로 확장할 수 있도록 한다.

---

## Named profile (혼동하지 말 것)

| Profile | Entrypoint | 역할 |
|---------|------------|------|
| **`tool-eval`** | `bin/clojure-clr` | 집중 facade: `-e` / **단일 폼** 파일 하나 |
| **`tool-eval-multi`** | `--multi-form FILE\|-` / `--multi-e FORM` | 옵트인: 여러 top-level form L→R, 마지막 값 (named gate) |
| **`bootstrap`** | `bin/clojure-clr-bootstrap` | Upstream Clojure.Main (substrate가 admit하는 전체 CLI 플래그) |
| **`bootstrap-project`** | `examples/clojure-clr-project/` | bootstrap + `CLOJURE_LOAD_PATH` 위 multi-ns 샘플 |
| **`meta`** | `bin/clr-meta` | Selfhost builder, gate, runtime-artifact, tool-eval family |

Named profile용 게이트 (`bin/pnix-clr-gate`에도 연결됨):

```bash
./bin/clojure-clr-profiles-smoke
# tool-eval + tool-eval-multi + bootstrap-project → 42 (5 checks)
```

TFM: **net10.0** 제품 경로; Rhino **sdk_8**은 별도 — `TFM_POLICY.md` 참조.

---

## `bin/clojure-clr` — admitted CLI

진실 소스: `bin/clojure-clr` (fail-closed).

| Admitted | Form | 동작 |
|----------|------|----------|
| Yes | `-e FORM` 또는 `--eval FORM` (정확히 2 argv) | `exec bin/clr-meta "$@"` (단일 폼) |
| Yes | 존재하는 파일인 단일 path (정확히 1 argv) | `exec bin/clr-meta FILE` (단일 폼; trailing 실패) |
| Yes | `-` (정확히 1 argv) | stdin에서 단일 폼 (trailing 실패) |
| Yes | `--multi-form FILE` (정확히 2 argv, 파일 존재) | `tool-eval-multi` — 모든 top-level form, 마지막 값 |
| Yes | `--multi-form -` | stdin에서 `tool-eval-multi` |
| Yes | `--multi-e FORM` / `--multi-eval FORM` | 인라인 문자열에서 `tool-eval-multi` |
| No | REPL, `-i`, `-M`, deps.edn, clojure CLI 패리티 | stderr + exit 2 |

에러 텍스트 (facade exit 2, non-admitted argv):

```text
clojure-clr compatibility: admitted surface is -e FORM, one FORM file, '-',
--multi-form FILE|-, or --multi-e FORM; use clojure-clr-bootstrap …
```

Surface matrix 게이트 (fail-closed 인벤토리):

```bash
./clr-meta/scripts/clr-meta-tool-surface-gate
# also wired into clr-meta-gate after tool-eval-multi
```

### clr-meta tool-eval에서 “FORM / file”의 의미

`pnix.clr-meta.main`(tool profile)에 위임, **전체** Clojure 아님:

- 정확히 **하나의** 폼 (reader evaluation 비활성; tagged/conditional reader
  거부), 단 `--multi-form` / `--multi-e` (tool-eval-multi profile) 제외.
- 값 도메인은 **admitted portable form domain**으로 제한 (밖이면 eval 전 fail closed).
- 평가는 **physical evaluator generation 2** 경유 (nested interpreter lane;
  Compiler Stage1–15/N **아님**).
- 이 tool surface에 `load-string` 경로 없음.

**결과 맵 (테스트 / Stage 게이트):** 성공·실패 tool-eval 결과는 최소한 다음을 포함:

| Key | 의미 |
|-----|---------|
| `:profile` | 예: `:tool-eval` 또는 multi-form profile |
| `:form-count` | 평가된 top-level form 수 (단일 `-e`는 1) |

pre-multi-form 형태에 대한 정확한 EDN/map 동등성을 assert하지 말 것; admitted
key + value를 고정하거나 named surface/multi 게이트를 사용.

따라서 `clojure-clr`는 **이름 호환 슬라이버**이지, “임의 프로젝트를 위한 CLR 위 Clojure”가 아니다.

---

## `bin/clr-meta` — 더 넓지만 여전히 profiled

| Profile | 예 | 비고 |
|---------|----------|--------|
| Tool-eval | `-e`, 단일 파일, `--gate` (eval-family) | form eval에 대해 위와 같은 reader/domain 규칙 |
| Runtime artifact | `--build-runtime PLAN OUT SRC` | **pnix-clr** product namespace용 hash-bound AOT |
| Compiler selfhost | `--build-compiler-selfhost-stageN …` | Stage ladder; `STATUS.md` / design 문서 참조 |
| Aggregate | `bin/clr-meta-gate` | 전체 family; promotion 주장하지 말 것 |

닫힌 compiler/selfhost claim은 `clr-meta/STATUS.md`와
`STAGE15_N_ROADMAP.md` Open claims에 나열 (정직히 남은 것: 일반 IL fixed point,
broad ClojureCLR compatibility, host promotion, …).

---

## Upstream substrate (trust root)

| 조각 | 위치 |
|-------|----------|
| NuGet pin | `clr-bootstrap/` 경유 `Clojure` 1.12.3-alpha8 |
| Publish | `bin/build-clr` → `clojure-clr-clojure-…/…/publish/` |
| Main assembly | `Clojure.Main.dll` (net10.0) |

더 넓은 upstream compiler/runtime 작업: **`clojure-clr-bootstrap`**,
`clojure-clr` facade 아님.

---

## 확장 로드맵 (단계 건너뛰지 말 것)

`clr-meta/todo.md` Post host-env / P3.2에서:

1. **[x] 인벤토리** (이 문서).
2. **[x] TFM 정책 정리** — [`TFM_POLICY.md`](TFM_POLICY.md) (net10 제품
   vs net8 Rhino / multi-target Pnix.Clr).
3. **[x] 프로젝트 템플릿 + smoke (bootstrap profile)** —
   `examples/clojure-clr-project/`가 **clojure-clr-bootstrap** +
   `CLOJURE_LOAD_PATH`로 **두 namespace** 로드 (facade 아님).
   `./smoke`는 `42` 기대. 여전히 **`clojure-clr` multi-file 아님**,
   deps.edn 패리티 아님.
4. **[x] Named profile + dual smoke** — `tool-eval` / `bootstrap` /
   `bootstrap-project` 문서화; `bin/clojure-clr-profiles-smoke` +
   `clojure-clr --help` (2026-08-14).
5. **[x] tool-eval-multi** — `--multi-form FILE` +
   `scripts/clr-meta-tool-eval-multi-gate` (`clr-meta-gate`에 연결);
   기본 단일 폼 trailing 거부 유지 (2026-08-14).
6. **[x] product aggregate에 profiles-smoke** — `bin/pnix-clr-gate`가
   `clojure-clr-profiles-smoke` 실행 (~17s, 2026-08-14).
7. **[x] 로컬 nupkg pack smoke** — `bin/pnix-clr-nupkg-smoke` (export layout +
   dual-TFM pack; 로컬 feed only, 2026-08-14).
8. **[x] nuget.org** — **하지 않음** (소유자: personal/local feed only, 2026-08-14).

금지된 지름길:

- 전체 ClojureCLR를 암시하도록 `clojure-clr` 이름 바꾸기.
- facade `-e`만으로 Stage15/N 또는 Trusting-Trust 주장.
- 명시되지 않은 profile에서 Rhino sdk_8과 pnix-clr net10 혼합.

---

## 빠른 smoke (facade only)

```bash
cd pnix-clr
./bin/build-clr                 # if substrate missing
./bin/clojure-clr -e '(+ 20 22)'   # => 42 via clr-meta gen2
echo '(+ 1 2)' > /tmp/t.clj
./bin/clojure-clr /tmp/t.clj
./bin/clojure-clr -M -e 1         # must fail closed (exit 2)
```
