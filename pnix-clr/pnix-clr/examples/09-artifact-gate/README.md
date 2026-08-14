# 09 — artifact 게이트 (fail-closed)

## 무엇을

제품 실행은 **검증된 AOT artifact** 에 묶인다. 없거나 깨지면 조용히
소스 경로로 떨어지지 않는다 (fail-closed).

## 실행

```bash
cd pnix-clr
./bin/build-pnix-clr-artifact
./bin/pnix-clr-artifact-gate --no-build
./bin/pnix-clr-gate
```

상세: 제품 `README.md` 의 artifact 계약 절.
