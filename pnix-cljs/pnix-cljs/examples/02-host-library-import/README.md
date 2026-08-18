# 02 — 호스트 라이브러리 import (로컬 피드)

## 쉽게 말하면 (비유)
`00`/`01`은 pnix-cljs를 **이 저장소 안에서** `dist/`를 직접 `require`했다.
이건 그 반대쪽 질문이다 — **다른 Node 프로젝트**가 pnix-cljs를 (npm 레지스트리
가 아니라) 로컬 export된 패키지로 `require`해서 `.px` 파일을 평가할 수 있는가.

## 무엇을
host-main 축: Node 프로젝트가 **로컬 export**된 pnix-cljs 패키지를
`require("@plumpmath/pnix-cljs")`(또는 `NODE_PATH`로 잡은 `pnix-cljs-module.js`)
로 불러와 `.px` 파일을 평가한다. npm 레지스트리 배포가 아니다 — 로컬
`export-pnix-cljs-library`가 만든 디렉터리를 가리킨다.

## plain Node의 한계
npm 패키지가 아닌 이웃 프로젝트의 코드를 "그냥 가져다 쓰는" 표준 방법은
상대 경로 `require`뿐이다 — 버전/export 표면이 명시적으로 안 굳어 있으면
빌드 산출물이 바뀔 때마다 소비하는 쪽이 깨진다.

## pnix-cljs의 방식
```js
const pnix = require("@plumpmath/pnix-cljs"); // export 후 NODE_PATH
console.log(pnix.evalFileValueJson("hello.px"));
```
`export-pnix-cljs-library`가 `share/` + scoped npm 패키지 레이아웃으로 export를
고정하고, `pnix-cljs-library-smoke`가 그 레이아웃에서 실제로
`require('@plumpmath/pnix-cljs')`/`require('pnix-cljs-module.js')` 둘 다
`evalFile*`이 동작하는지 검증한다.

## 어디에 쓰나
pnix-cljs를 별도 Node 프로젝트의 의존성으로 로컬에서 먼저 검증하고 싶을 때
(CI/개발 중, npm 공개 배포 전).

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
