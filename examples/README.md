# examples/

두 층이 있다.

1. **호스트 제품 카탈로그** — `pnix-<host>/pnix-<host>/examples/`  
   테마 균형·성숙도: **[EXAMPLES_BALANCE.md](EXAMPLES_BALANCE.md)**
2. **모노레포 host-import 스모크** — **`host-import/`** (이중 축 day-1 데모)

## 호스트 제품 카탈로그 (이 디렉터리가 아님)

| 호스트 | 경로 | 깊이 |
|--------|------|------|
| clj | `pnix-clj/pnix-clj/examples/` | 조밀 (~90) |
| hy | `pnix-hy/pnix-hy/examples/` | 조밀 (~35) |
| rs | `pnix-rs/pnix-rs/examples/` | 중간 (~15) |
| cljs | `pnix-cljs/pnix-cljs/examples/` | 코어 00–06 |
| clr | `pnix-clr/pnix-clr/examples/` | 코어 00–06 |

## host-import (이 트리)

```bash
# 단일 파일 eval-file
cd host-import/clj && clojure -M -m smoke
# => 3

# multi-module import ./lib.px
cd host-import/clj-imports && clojure -M -m smoke
# => 3
```

다른 호스트: [`host-import/README.md`](host-import/README.md).

`examples/` 자체에서 `clojure -M -m smoke` 를 돌리지 말 것 — 여기에
`deps.edn` / `smoke.clj` classpath 가 없다.
