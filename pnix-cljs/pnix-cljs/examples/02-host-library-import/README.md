# 02 — host library import (local feed)

## 무엇을

host-main: Node 프로젝트가 **로컬 export**된 pnix-cljs 패키지를 require 하고
`.px` 파일을 평가한다. npm registry 배포가 아니다.

## 실행

```bash
cd pnix-cljs
./bin/pnix-cljs-library-smoke
# or monorepo:
#   ./examples/host-import/cljs/smoke.mjs   # with NODE_PATH from export
```

## 관련

- monorepo `examples/host-import/cljs/`
- monorepo `HOST_IMPORT.md` § cljs
