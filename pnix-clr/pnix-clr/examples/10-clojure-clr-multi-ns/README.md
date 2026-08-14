# 10 — ClojureCLR 다중 네임스페이스 (bootstrap)

## 무엇을

**pnix 게스트** 가 아니라 **호스트 ClojureCLR** 로 디스크 위 2개 네임스페이스를
로드하는 예제 포인터. 파사드 `clojure-clr -e` 가 아니라 **bootstrap** 경로.

## 실행

```bash
cd pnix-clr/examples/clojure-clr-project
./smoke
# => PASS (42)
```

`pnix-clr` 제품 CLI 와 별개 — 호스트 언어 쪽 day-1 경로.
