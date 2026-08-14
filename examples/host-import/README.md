# host-import 미니 예제 (P2.2 쉬운 시작)

아주 작은 **host-main** 데모: 각 호스트 언어가 자기 pnix 제품 라이브러리를
로드하고 `hello.px` (`1 + 2` → `3`) 를 평가한다.

**전제:** 이중 축 환경(HM 프로파일) 또는 동등한 env —  
[`../../HOST_IMPORT.md`](../../HOST_IMPORT.md) 참고.

**중요:** 먼저 **호스트 하위 디렉터리로 `cd`**.  
`examples/` 또는 `examples/host-import/` 에서 `clojure -M -m smoke` 를 돌리면
`Could not locate smoke__init.class` 로 실패한다 — 데모마다 자체 `deps.edn` /
진입 파일이 있다.

| 호스트 | 실행 방법 (모노레포 루트 또는 절대 경로) |
|--------|------------------------------------------|
| clj | `cd examples/host-import/clj && clojure -M -m smoke` |
| clj multi-module | `cd examples/host-import/clj-imports && clojure -M -m smoke` |
| hy | `cd examples/host-import/hy && python smoke.py` |
| cljs | `cd examples/host-import/cljs && node smoke.mjs` |
| rs | `cd examples/host-import/rs/pnix-rs-smoke && cargo run -q -- ../../hello.px` |
| clr | `cd examples/host-import/clr && ./smoke` (HelloPnix + export) |

공유 소스: [`hello.px`](hello.px).

회귀:
- 모노레포 `../../bin/host-import-smoke` (PATH 도구)
- 모노레포 `../../bin/host-library-smokes` (로컬 라이브러리 피드)

제품 예제 카탈로그 균형: [`../EXAMPLES_BALANCE.md`](../EXAMPLES_BALANCE.md).
