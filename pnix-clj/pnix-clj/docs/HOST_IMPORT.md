# 호스트 언어 임포트 — pnix-clj (JVM)

**정본 이중 축 교리:** [`../../../HOST_DEV_ENV.md`](../../../HOST_DEV_ENV.md)

이 문서는 `clojure` / `pnix-clj-clj`가 아래를 주입한 뒤 호출 프로젝트가
`pnix-clj`를 호스트 라이브러리로 로드할 때의 **공개 API 표면**이다:

```clojure
{:deps {pnix/pnix-clj {:local/root "…/pnix-clj"}}}
```

Env (HM / 래퍼): `PNIX_CLJ_ROOT`, `PNIX_CLJ_LIBRARY` (같은 트리 루트).

---

## 지원 (host-main에 안정)

애플리케이션 코드는 이쪽을 우선. 그 외 네임스페이스는 tower/gate용이며
deprecation 주기 없이 바뀔 수 있다.

| 네임스페이스 | 진입점 | 역할 |
|--------------|--------|------|
| **`pnix-clj.core`** | `parse-source`, `eval-source`, **`eval-file`**, `eval-source-with-imports`, `eval-source-strict`, `eval-source-strict-audit`, `lower-source` | `.px` 파싱 / 평가 (1차 표면) |
| **`pnix-clj.machine-outcome`** | `eval-source-outcome` | 구조화 Done/Failed/Suspended 프로젝션 |
| **`pnix-clj.convenience`** | 예제용 헬퍼 | core 위 얇은 설탕 (신규 코드는 core 우선) |

### 최소 예제

```clojure
(require '[pnix-clj.core :as c])

;; 인라인 소스
(c/eval-source "1 + 2")
;; => {:status :ok, :value 3, …}

;; 파일 (호스트 언어에서 .px 프로그램 임포트)
(c/eval-file "path/to/prog.px")

;; 메모리 임포트만 (FS 없음): target-string -> source 맵
(c/eval-source-with-imports "import ./lib.px" {"./lib.px" "1 + 1"})
```

미니 멀티모듈 프로젝트: monorepo
`examples/host-import/clj-imports/` (`main.px`가 `./lib.px` import).

결과 형태: `:status` (`:ok` / `:failed` / `:suspended` …), `:value` 또는
`:error`, 파싱 메타를 담은 런타임 맵. `:ok`가 아니면 실패로 취급.

---

## 사용 가능하나 2차

tower/mirror 레인에 이미 익숙할 때만:

| 네임스페이스 | 메모 |
|--------------|------|
| `pnix-clj.mirror`, `pnix-clj.mirror-pair` | 크로스-substrate mirror 리포트 |
| `pnix-clj.interop` | 호스트 interop / opaque refs |
| `pnix-clj.capabilities` | capability 인덱스 |
| `pnix-clj.lowering` | lowering 레인 |
| `pnix-clj.parser` / `pnix-clj.evaluator` | 내부 — `core` 우선 |

증명 / 생성기 / fuzzer 네임스페이스 (`generate`, `grammar-fuzzer`,
`arith-proof`, …)는 애플리케이션용 **호스트 라이브러리 API가 아님**.

---

## 이것이 아닌 것

- 이식 가능한 멀티호스트 `.px` 바이트코드 패키지 아님.
- jar / `libexec`가 필요할 때 inject 래퍼 없는 stock `clojure` 대체 아님 —
  그때는 nix의 `clojure-stock` 사용.
- **로컬 라이브러리 export** (개인 피드, Maven Central 아님):
  ```bash
  ./bin/export-pnix-clj-library          # → target/pnix-clj-library/
  ./bin/pnix-clj-library-smoke
  # then: {:deps {pnix/pnix-clj {:local/root "…/pnix-clj-library/pnix-clj"}}}
  ```
- Maven Central / 공개 레지스트리 게시는 이 소유자의 **제품 목표 아님**
  (clr 로컬 nupkg와 동일 정책).

---

## 스모크

```bash
# HM clojure (= pnix-clj-clj) 기준
echo '1 + 2' > /tmp/t.px
clojure -M -e "(require '[pnix-clj.core :as c]) (println (:value (c/eval-file \"/tmp/t.px\")))"
# => 3

pnix-clj-library   # PNIX_CLJ_ROOT / local/root 힌트 출력
pnix-clj-pnix      # pnix-main REPL
```
