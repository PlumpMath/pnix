# 03 — outcome projection

## 무엇을

생산 경로의 결과 모양(성공 값 vs 구조화 실패)을 CLI로 확인한다. clj 전 레인
receipt 타워와 동일 깊이는 아니다.

## 실행

```bash
cd pnix-clr
./bin/pnix-clr -e 'true && !false'
./bin/pnix-clr -e 'if true then 40 + 2 else 0'
./bin/pnix-clr pnix-clr/examples/03-outcome-projection/ok.px
# optional aggregate:
#   ./bin/pnix-clr-production-outcome-gate
```
