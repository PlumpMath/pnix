# 02 — 호스트 라이브러리 import (로컬 피드)

## 무엇을

host-main: Node 프로젝트가 **로컬 export**된 pnix-cljs 패키지를 require 하고
`.px` 파일을 평가한다. npm 레지스트리 배포가 아니다.

## 실행

```bash
cd pnix-cljs
./bin/pnix-cljs-library-smoke
# 또는 모노레포:
#   ./examples/host-import/cljs/smoke.mjs   # export 의 NODE_PATH 와 함께
```

## 관련

- 모노레포 `examples/host-import/cljs/`
- 모노레포 `HOST_IMPORT.md` § cljs
