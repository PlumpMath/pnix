# 호스트 언어 임포트 — pnix-cljs (Node / CLJS)

**정본 이중 축 교리:** [`../HOST_DEV_ENV.md`](../HOST_DEV_ENV.md)

제품 패키지는 **호스트 바인딩** JS 라이브러리를 실어 보낸다 (이식 가능한 멀티호스트 `.px` 아님).

---

## 레이아웃 (`nix build .#pnix-cljs` / HM install 이후)

```text
$out/share/pnix-cljs/
  package.json              # name @plumpmath/pnix-cljs, main: pnix-cljs-module.js
  pnix-cljs-module.js       # require 대상 (eval API)
  pnix-cljs.js              # CLI 진입 (bin/pnix-cljs로 래핑)

$out/lib/node_modules/@plumpmath/pnix-cljs/   # 동일 파일 (scoped require)
```

Env (HM `node` / `pnix-cljs-node` / shadow wrapper):

| 변수 | 의미 |
|------|------|
| `PNIX_CLJS_SHARE` / `PNIX_CLJS_LIBRARY` | `$out/share/pnix-cljs` |
| `PNIX_CLJS` | `pnix-cljs` CLI 경로 |
| `NODE_PATH` | `$out/lib/node_modules:$out/share/pnix-cljs:…` |

---

## Require + eval API

```js
// 권장 (scoped 패키지 — NODE_PATH에 lib/node_modules 필요)
const pnix = require('@plumpmath/pnix-cljs');

// flat 폴백 (NODE_PATH에 share/만 있어도 충분)
// const pnix = require('pnix-cljs-module.js');

// 인라인
pnix.evalSource('1 + 2');           // JS 프로젝션 객체
pnix.evalSourceJson('1 + 2');       // JSON 문자열
pnix.evalValueJson('1 + 2');        // value-only JSON (예: "3")

// 파일 (.px)
pnix.evalFile('prog.px');
pnix.evalFileJson('prog.px');
pnix.evalFileValueJson('prog.px');  // 스모크에 자주 사용: "3"
pnix.evalFileValue('prog.px');
```

### 스모크 (`pnix-cljs-host` 있는 HM 프로파일)

```bash
echo '1 + 2' > /tmp/t.px
node -e "const p=require('@plumpmath/pnix-cljs'); console.log(p.evalFileValueJson('/tmp/t.px'))"
# => 3   (lib/node_modules를 싣는 flake install 이후)

# 현재 share/가 NODE_PATH에 있으면 항상 동작:
node -e "const p=require('pnix-cljs-module.js'); console.log(p.evalFileValueJson('/tmp/t.px'))"

pnix-cljs-library   # env + 경로 출력
clojurescript -e '20 + 22'   # → pnix-cljs CLI
pnix-cljs-pnix               # pnix-main REPL
```

---

## 명명

| 이름 | 역할 |
|------|------|
| `pnix-cljs` | 런타임 CLI (eval / `--repl`) |
| `pnix-cljs-pnix` | pnix-main 대화형 REPL |
| `clojurescript` | bare host-main 별칭 → `pnix-cljs` |
| `pnix-cljs-cljs` / `cljs-meta` | host-meta fixed-point 표면 |
| `shadow-cljs` | **빌드 오케스트레이터**만; `PNIX_CLJS` / `NODE_PATH` 주입 |

---

## 주장하지 않음

- 이식 가능한 멀티호스트 `.px` 패키지  
- shadow-cljs 빌드 그래프 전체 대체  
- npm 레지스트리 게시 (이 소유자 제품 목표 아님 — 로컬 피드만)

## 로컬 export (개인 피드)

```bash
# pnix-cljs/dist/ 필요 (이전 build-cljs / nix build)
./bin/export-pnix-cljs-library          # → pnix-cljs/target/pnix-cljs-library
./bin/pnix-cljs-library-smoke
set -a; source pnix-cljs/target/pnix-cljs-library/refs.env; set +a
```
